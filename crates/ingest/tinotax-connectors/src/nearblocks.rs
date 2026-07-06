//! NearBlocks wallet fetcher for NEAR account transaction pages.
//!
//! The fetcher supports anonymous and API-key modes, uses conservative rate
//! limits, and persists raw pages plus cursors for resumable evidence capture.
//!
//! Pagination is cursor-based (`cursor=` query param, `cursor` field in the
//! response): NearBlocks rejects `page` numbers past 200, which caps
//! page-numbered fetches at a few thousand rows on busy accounts. Pages are
//! requested oldest-first so the fetch can stop once it passes the project
//! period instead of paying for post-period history.
use anyhow::{Context, Result};
use async_trait::async_trait;
use tinotax_core::WalletSource;
use tinotax_store::{Cursor, EndpointCache, ProjectPaths, RawFileManifestEntry, RawManifest};
use tracing::{info, warn};

use crate::fetcher::{FetchContext, FetchReport, WalletFetcher};
use crate::http::HttpClient;

const PER_PAGE: u64 = 100;
/// NearBlocks bills one rate-limit credit per 25 items requested, so a
/// 100-item page costs 4 credits. Page size only changes round-trips and
/// file count, never quota.
const CREDITS_PER_PAGE: u64 = PER_PAGE.div_ceil(25);
const MAX_PAGES: u64 = 20_000;
/// Free API plan: 10 credits/minute. Paid plans go much higher; callers pass
/// their plan's per-minute credit budget through `credits_per_minute`.
const DEFAULT_CREDITS_PER_MINUTE_WITH_KEY: u64 = 10;
const DEFAULT_CREDITS_PER_MINUTE_ANONYMOUS: u64 = 6;

/// NearBlocks v1 API fetcher for NEAR accounts.
pub struct NearBlocksFetcher {
    base_url: String,
    api_key: Option<String>,
    credits_per_minute: Option<u64>,
    http: HttpClient,
}

impl NearBlocksFetcher {
    pub fn new(
        base_url: String,
        api_key: Option<String>,
        credits_per_minute: Option<u64>,
        http: HttpClient,
    ) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            credits_per_minute,
            http,
        }
    }

    fn delay_ms(&self) -> u64 {
        let budget = self
            .credits_per_minute
            .unwrap_or(if self.api_key.is_some() {
                DEFAULT_CREDITS_PER_MINUTE_WITH_KEY
            } else {
                DEFAULT_CREDITS_PER_MINUTE_ANONYMOUS
            });
        // NearBlocks enforces the budget over a rolling window, so uniform
        // spacing of exactly budget/minute still trips it: any 60s window
        // holds floor(60/s)+1 requests, one more than the average. Leave one
        // page of headroom: worst-case window credits stay within budget.
        60_000 * CREDITS_PER_PAGE / budget.saturating_sub(CREDITS_PER_PAGE).max(1)
    }
}

#[async_trait]
impl WalletFetcher for NearBlocksFetcher {
    async fn fetch_wallet(
        &self,
        ctx: FetchContext<'_>,
        wallet: &WalletSource,
    ) -> Result<FetchReport> {
        let paths = ProjectPaths::new(ctx.project_dir.to_owned());
        let wallet_dir = paths.wallet_raw_dir(&wallet.chain, &wallet.address);
        std::fs::create_dir_all(&wallet_dir)?;
        let manifest_path = wallet_dir.join("raw_manifest.json");
        let mut manifest =
            RawManifest::load_or_new(&manifest_path, &wallet.id, &wallet.chain, &wallet.address)?;

        let cache = EndpointCache::open(&paths, &wallet.chain, &wallet.address, "transactions")?;

        let existing = if ctx.resume {
            cache.read_cursor()?
        } else {
            None
        };
        let resumed = existing.is_some();
        let mut cursor = existing.unwrap_or_else(Cursor::start);

        let mut report = FetchReport {
            source_id: wallet.id.clone(),
            chain: wallet.chain.clone(),
            wallet: wallet.address.clone(),
            pages_fetched: 0,
            items_fetched: 0,
            resumed,
        };

        if cursor.done {
            info!(wallet = %wallet.address, "already complete, skipping");
            return Ok(report);
        }

        if self.api_key.is_none() {
            warn!(
                "no NEARBLOCKS_API_KEY set; using the anonymous tier with long delays — \
                 a free key from nearblocks.io makes this much faster"
            );
        }
        let headers: Vec<(&str, String)> = self
            .api_key
            .iter()
            .map(|key| ("Authorization", format!("Bearer {key}")))
            .collect();

        let url = format!("{}/account/{}/txns", self.base_url, wallet.address);
        let delay = self.delay_ms();

        loop {
            let page = cursor.next_page;
            let mut query = vec![
                ("per_page".to_string(), PER_PAGE.to_string()),
                ("order".to_string(), "asc".to_string()),
            ];
            if let Some(token) = cursor
                .next_params
                .as_ref()
                .and_then(|p| p.get("cursor"))
                .and_then(|c| c.as_str())
            {
                query.push(("cursor".to_string(), token.to_string()));
            }
            let body = self
                .http
                .get_json(&url, &query, &headers)
                .await
                .with_context(|| {
                    format!(
                        "fetching {url} page {page} \
                     (progress is saved; once the NearBlocks rate limit or daily quota \
                     resets, re-run fetch with --resume to continue)"
                    )
                })?;

            let items = body.get("txns").and_then(|v| v.as_array());
            let item_count = items.map(|a| a.len() as u64).unwrap_or(0);

            if item_count == 0 {
                cursor.done = true;
                cursor.updated_at = tinotax_store::now_rfc3339();
                cache.write_cursor(&cursor)?;
                break;
            }

            let (rel_path, hash) = cache.write_page(page, &body)?;
            manifest.upsert(RawFileManifestEntry {
                source_id: wallet.id.clone(),
                chain: wallet.chain.clone(),
                wallet: wallet.address.clone(),
                endpoint: "transactions".to_string(),
                page,
                path: rel_path,
                blake3: hash,
                fetched_at: tinotax_store::now_rfc3339(),
                item_count,
            });
            manifest.save(&manifest_path)?;
            report.pages_fetched += 1;
            report.items_fetched += item_count;
            info!(wallet = %wallet.address, page, items = item_count, "cached page");

            // Ascending order means once a page ends past the stop timestamp
            // the window is fully covered. The overshooting page is kept;
            // normalisation filters to the project period. NOTE: the cursor is
            // marked done for *this* stop point — extending period_end later
            // needs a fresh fetch (without --resume).
            let last_ts = items.and_then(|a| a.last()).and_then(item_timestamp_ns);
            if let (Some(stop), Some(ts)) = (ctx.stop_after_ns, last_ts) {
                if ts > stop {
                    info!(wallet = %wallet.address, "reached end of fetch window");
                    cursor.done = true;
                    cursor.updated_at = tinotax_store::now_rfc3339();
                    cache.write_cursor(&cursor)?;
                    break;
                }
            }

            let next_token = body
                .get("cursor")
                .and_then(|c| c.as_str())
                .filter(|s| !s.is_empty());
            match next_token {
                Some(token) => {
                    cursor.next_page += 1;
                    cursor.next_params = Some(serde_json::json!({ "cursor": token }));
                    cursor.updated_at = tinotax_store::now_rfc3339();
                    cache.write_cursor(&cursor)?;
                }
                None => {
                    cursor.done = true;
                    cursor.updated_at = tinotax_store::now_rfc3339();
                    cache.write_cursor(&cursor)?;
                    break;
                }
            }

            if cursor.next_page > MAX_PAGES {
                anyhow::bail!(
                    "{url}: exceeded {MAX_PAGES} pages; cursor saved, re-run with --resume"
                );
            }
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }

        Ok(report)
    }
}

/// Block timestamp of one txns row, in nanoseconds since epoch. NearBlocks
/// sends `block_timestamp` as a string or number, at the top level and/or
/// under `receipt_block`.
fn item_timestamp_ns(item: &serde_json::Value) -> Option<u64> {
    let raw = item.get("block_timestamp").or_else(|| {
        item.get("receipt_block")
            .and_then(|b| b.get("block_timestamp"))
    })?;
    match raw {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn timestamp_from_number_or_string_or_receipt_block() {
        let n = serde_json::json!({ "block_timestamp": 1638753233470037500u64 });
        assert_eq!(item_timestamp_ns(&n), Some(1638753233470037500));

        let s = serde_json::json!({ "block_timestamp": "1638753233470037500" });
        assert_eq!(item_timestamp_ns(&s), Some(1638753233470037500));

        let nested = serde_json::json!({
            "receipt_block": { "block_timestamp": 1638753233470037500u64 }
        });
        assert_eq!(item_timestamp_ns(&nested), Some(1638753233470037500));

        assert_eq!(item_timestamp_ns(&serde_json::json!({})), None);
    }

    #[test]
    fn credit_aware_delay() -> Result<(), Box<dyn Error>> {
        let http = HttpClient::new()?;
        // Free plan with key: 4 credits per 100-item page, 10 credits/min
        // budget, one page of headroom → 6 effective credits/min.
        let f = NearBlocksFetcher::new("x".into(), Some("k".into()), None, http.clone());
        assert_eq!(f.delay_ms(), 40_000);
        // Startup plan override: 190 credits/min.
        let f = NearBlocksFetcher::new("x".into(), Some("k".into()), Some(190), http.clone());
        assert_eq!(f.delay_ms(), 1_290);
        // Anonymous default: 6 credits/min budget leaves ~no headroom.
        let f = NearBlocksFetcher::new("x".into(), None, None, http);
        assert_eq!(f.delay_ms(), 120_000);
        Ok(())
    }
}
