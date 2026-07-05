use anyhow::{Context, Result};
use async_trait::async_trait;
use tinotax_core::WalletSource;
use tinotax_store::{Cursor, EndpointCache, ProjectPaths, RawFileManifestEntry, RawManifest};
use tracing::info;

use crate::fetcher::{FetchContext, FetchReport, WalletFetcher};
use crate::http::HttpClient;

/// Safety cap so a pathological pagination loop can't run forever. The
/// cursor is persisted, so a capped fetch resumes where it stopped.
const MAX_PAGES_PER_ENDPOINT: u64 = 2_000;
const POLITENESS_DELAY_MS: u64 = 250;

/// Blockscout v2 API fetcher. One instance per base URL — Lisk EVM and
/// IOTA EVM both use this with different `base_url`s.
pub struct BlockscoutFetcher {
    base_url: String,
    http: HttpClient,
}

/// (directory name under raw/, URL path segment)
const ENDPOINTS: &[(&str, &str)] = &[
    ("transactions", "transactions"),
    ("token_transfers", "token-transfers"),
];

impl BlockscoutFetcher {
    pub fn new(base_url: String, http: HttpClient) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
        }
    }

    async fn fetch_endpoint(
        &self,
        ctx: FetchContext<'_>,
        wallet: &WalletSource,
        dir_name: &str,
        url_segment: &str,
        manifest: &mut RawManifest,
        manifest_path: &camino::Utf8Path,
    ) -> Result<(u64, u64, bool)> {
        let paths = ProjectPaths::new(ctx.project_dir.to_owned());
        let cache = EndpointCache::open(&paths, &wallet.chain, &wallet.address, dir_name)?;

        let existing = if ctx.resume {
            cache.read_cursor()?
        } else {
            None
        };
        let resumed = existing.is_some();
        let mut cursor = existing.unwrap_or_else(Cursor::start);
        if cursor.done {
            info!(wallet = %wallet.address, endpoint = dir_name, "already complete, skipping");
            return Ok((0, 0, true));
        }

        let url = format!(
            "{}/addresses/{}/{}",
            self.base_url, wallet.address, url_segment
        );
        let mut pages_fetched = 0u64;
        let mut items_fetched = 0u64;

        loop {
            let query = params_from_value(cursor.next_params.as_ref());
            let body = self
                .http
                .get_json(&url, &query, &[])
                .await
                .with_context(|| format!("fetching {url} page {}", cursor.next_page))?;

            let item_count = body
                .get("items")
                .and_then(|v| v.as_array())
                .map(|a| a.len() as u64)
                .unwrap_or(0);

            if item_count > 0 {
                let page = cursor.next_page;
                let (rel_path, hash) = cache.write_page(page, &body)?;
                manifest.upsert(RawFileManifestEntry {
                    source_id: wallet.id.clone(),
                    chain: wallet.chain.clone(),
                    wallet: wallet.address.clone(),
                    endpoint: dir_name.to_string(),
                    page,
                    path: rel_path,
                    blake3: hash,
                    fetched_at: tinotax_store::now_rfc3339(),
                    item_count,
                });
                manifest.save(manifest_path)?;
                pages_fetched += 1;
                items_fetched += item_count;
                info!(
                    wallet = %wallet.address,
                    endpoint = dir_name,
                    page,
                    items = item_count,
                    "cached page"
                );
            }

            let next = body
                .get("next_page_params")
                .filter(|v| !v.is_null())
                .cloned();
            match next {
                Some(params) if item_count > 0 => {
                    cursor.next_page += 1;
                    cursor.next_params = Some(params);
                    cursor.updated_at = tinotax_store::now_rfc3339();
                    cache.write_cursor(&cursor)?;
                }
                _ => {
                    cursor.done = true;
                    cursor.updated_at = tinotax_store::now_rfc3339();
                    cache.write_cursor(&cursor)?;
                    break;
                }
            }

            if cursor.next_page > MAX_PAGES_PER_ENDPOINT {
                anyhow::bail!(
                    "{url}: exceeded {MAX_PAGES_PER_ENDPOINT} pages; cursor saved, re-run with --resume"
                );
            }
            tokio::time::sleep(std::time::Duration::from_millis(POLITENESS_DELAY_MS)).await;
        }

        Ok((pages_fetched, items_fetched, resumed))
    }
}

#[async_trait]
impl WalletFetcher for BlockscoutFetcher {
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
            RawManifest::load_or_new(&manifest_path, &wallet.id, &wallet.chain, &wallet.address);

        let mut report = FetchReport {
            source_id: wallet.id.clone(),
            chain: wallet.chain.clone(),
            wallet: wallet.address.clone(),
            pages_fetched: 0,
            items_fetched: 0,
            resumed: false,
        };

        for (dir_name, url_segment) in ENDPOINTS {
            let (pages, items, resumed) = self
                .fetch_endpoint(
                    ctx,
                    wallet,
                    dir_name,
                    url_segment,
                    &mut manifest,
                    &manifest_path,
                )
                .await?;
            report.pages_fetched += pages;
            report.items_fetched += items;
            report.resumed |= resumed;
        }

        manifest.save(&manifest_path)?;
        Ok(report)
    }
}

/// Turn Blockscout's `next_page_params` object into query parameters.
fn params_from_value(value: Option<&serde_json::Value>) -> Vec<(String, String)> {
    let Some(serde_json::Value::Object(map)) = value else {
        return Vec::new();
    };
    map.iter()
        .filter(|(_, v)| !v.is_null())
        .map(|(k, v)| {
            let text = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            (k.clone(), text)
        })
        .collect()
}
