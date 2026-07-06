//! `prices fetch`: pull missing daily GBP prices from a public provider.
//!
//! CoinGecko's `/coins/{id}/history?date=dd-mm-yyyy` endpoint gives the
//! price at 00:00 UTC for a day. Authentication is optional in development
//! but expected for production runs: set `COINGECKO_API_KEY`,
//! `COINGECKO_DEMO_API_KEY`, or `COINGECKO_PRO_API_KEY`.

use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use rust_decimal::Decimal;
use tinotax_core::{PriceConfidence, PriceObservation, PriceSource};
use tinotax_ledger::load_reviewed_ledger;
use tinotax_store::ProjectPaths;
use tracing::{info, warn};

use crate::manual_import::merge_observations;
use crate::missing::missing_prices;
use crate::price_book::PriceBook;

/// Built-in symbol → CoinGecko id map. Deliberately small and explicit:
/// a wrong id silently prices the wrong asset, so unknown symbols are
/// skipped and reported instead of fuzzy-matched.
fn coingecko_id(symbol: &str) -> Option<&'static str> {
    Some(match symbol.to_ascii_uppercase().as_str() {
        "BTC" | "WBTC" => "bitcoin",
        "ETH" | "WETH" => "ethereum",
        "NEAR" => "near",
        "LSK" => "lisk",
        "IOTA" | "MIOTA" => "iota",
        "USDT" => "tether",
        "USDC" | "USDC.E" => "usd-coin",
        "DAI" => "dai",
        "BNB" => "binancecoin",
        "SOL" => "solana",
        "ADA" => "cardano",
        "DOT" => "polkadot",
        "MATIC" | "POL" => "matic-network",
        "LTC" => "litecoin",
        "XRP" => "ripple",
        "DOGE" => "dogecoin",
        "AVAX" => "avalanche-2",
        "LINK" => "chainlink",
        "ATOM" => "cosmos",
        "XLM" => "stellar",
        "AURORA" => "aurora-near",
        _ => return None,
    })
}

/// Fetch every missing (asset, day) the provider can serve. Returns the
/// number of observations stored.
pub async fn fetch_missing_prices(paths: &ProjectPaths, provider: &str) -> Result<u64> {
    if !provider.eq_ignore_ascii_case("coingecko") {
        bail!("unknown price provider {provider:?} (supported: coingecko)");
    }

    let ledger = load_reviewed_ledger(paths)?;
    let book = PriceBook::load(paths)?;
    let missing = missing_prices(&ledger, &book);
    if missing.is_empty() {
        info!("no missing prices — nothing to fetch");
        return Ok(0);
    }

    let client = reqwest::Client::builder()
        .user_agent(concat!("tinotax/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let auth = CoingeckoAuth::from_env();
    let mut fetched = Vec::new();
    let mut unmapped: BTreeSet<String> = BTreeSet::new();
    let total = missing.len();
    for (i, m) in missing.iter().enumerate() {
        let Some(id) = coingecko_id(&m.asset_symbol) else {
            unmapped.insert(m.asset_symbol.clone());
            continue;
        };
        // dd-mm-yyyy, per the API contract.
        let (y, mo, d) = tinotax_core::parse_date_prefix(&m.date)
            .map_err(|_| anyhow::anyhow!("bad date {:?}", m.date))?;
        let url = format!(
            "{}/coins/{id}/history?date={d:02}-{mo:02}-{y:04}&localization=false",
            auth.base_url
        );
        info!("[{}/{}] {} on {}", i + 1, total, m.asset_symbol, m.date);
        let mut request = client.get(&url);
        if let Some((header, key)) = auth.header() {
            request = request.header(header, key);
        }
        let response = request.send().await?;
        if response.status().as_u16() == 401 {
            bail!(
                "CoinGecko rejected the price request as unauthorized. Set \
                 COINGECKO_API_KEY for a demo/public paid key, or \
                 COINGECKO_PRO_API_KEY for a Pro key."
            );
        }
        if response.status().as_u16() == 429 {
            warn!("rate limited by CoinGecko — stopping early; re-run to continue");
            break;
        }
        let body: serde_json::Value = response
            .error_for_status()
            .with_context(|| format!("fetching {url}"))?
            .json()
            .await?;
        // `to_string` renders the JSON literal digits exactly (the workspace
        // enables serde_json's arbitrary_precision, so no float round-trip).
        let Some(price) = body
            .pointer("/market_data/current_price/gbp")
            .map(|v| v.to_string())
            .map(|s| s.trim_matches('"').to_string())
            .and_then(|s| {
                Decimal::from_str_exact(&s)
                    .ok()
                    .or_else(|| Decimal::from_scientific(&s).ok())
            })
        else {
            warn!(
                "no GBP price for {} on {} — skipped",
                m.asset_symbol, m.date
            );
            continue;
        };
        fetched.push(PriceObservation {
            asset_symbol: m.asset_symbol.clone(),
            asset_contract: None,
            timestamp: format!("{}T00:00:00Z", m.date),
            currency: "GBP".to_string(),
            price,
            source: PriceSource::Coingecko,
            confidence: PriceConfidence::High,
            fetched_at: tinotax_store::now_rfc3339(),
            note: Some(format!("coingecko daily history for {id}")),
        });
        // Anonymous tier allows roughly a call every couple of seconds.
        if i + 1 < total {
            tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        }
    }

    if !unmapped.is_empty() {
        warn!(
            "no CoinGecko id known for: {} — import these via `prices import` instead",
            unmapped.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    let count = fetched.len() as u64;
    merge_observations(paths, fetched)?;
    Ok(count)
}

struct CoingeckoAuth {
    base_url: &'static str,
    header_name: Option<&'static str>,
    api_key: Option<String>,
}

impl CoingeckoAuth {
    fn from_env() -> Self {
        if let Some(key) = env_key("COINGECKO_PRO_API_KEY") {
            return Self {
                base_url: "https://pro-api.coingecko.com/api/v3",
                header_name: Some("x-cg-pro-api-key"),
                api_key: Some(key),
            };
        }

        let api_key = env_key("COINGECKO_DEMO_API_KEY").or_else(|| env_key("COINGECKO_API_KEY"));
        Self {
            base_url: "https://api.coingecko.com/api/v3",
            header_name: api_key.as_ref().map(|_| "x-cg-demo-api-key"),
            api_key,
        }
    }

    fn header(&self) -> Option<(&'static str, &str)> {
        Some((self.header_name?, self.api_key.as_deref()?))
    }
}

fn env_key(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|key| !key.is_empty())
}
