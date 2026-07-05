//! `prices fetch`: pull missing daily GBP prices from a public provider.
//!
//! CoinGecko's `/coins/{id}/history?date=dd-mm-yyyy` endpoint gives the
//! price at 00:00 UTC for a day on the anonymous tier. Symbol → CoinGecko
//! id uses a built-in table for the assets this project actually meets;
//! anything unmapped is reported, not guessed.

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
        "USDC" => "usd-coin",
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
            "https://api.coingecko.com/api/v3/coins/{id}/history?date={d:02}-{mo:02}-{y:04}&localization=false"
        );
        info!("[{}/{}] {} on {}", i + 1, total, m.asset_symbol, m.date);
        let response = client.get(&url).send().await?;
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
