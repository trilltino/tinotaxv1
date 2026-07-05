use anyhow::{Context, Result};
use async_trait::async_trait;
use tinotax_core::WalletSource;
use tinotax_store::{Cursor, EndpointCache, ProjectPaths, RawFileManifestEntry, RawManifest};
use tracing::{info, warn};

use crate::fetcher::{FetchContext, FetchReport, WalletFetcher};
use crate::http::HttpClient;

const PER_PAGE: u64 = 25;
const MAX_PAGES: u64 = 4_000;
/// NearBlocks' anonymous tier is heavily rate-limited; with an API key
/// (NEARBLOCKS_API_KEY) we can go much faster.
const DELAY_WITH_KEY_MS: u64 = 500;
const DELAY_ANONYMOUS_MS: u64 = 2_500;

/// NearBlocks v1 API fetcher for NEAR accounts.
pub struct NearBlocksFetcher {
    base_url: String,
    api_key: Option<String>,
    http: HttpClient,
}

impl NearBlocksFetcher {
    pub fn new(base_url: String, api_key: Option<String>, http: HttpClient) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            http,
        }
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
            RawManifest::load_or_new(&manifest_path, &wallet.id, &wallet.chain, &wallet.address);

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
        let delay = if self.api_key.is_some() {
            DELAY_WITH_KEY_MS
        } else {
            DELAY_ANONYMOUS_MS
        };

        loop {
            let page = cursor.next_page;
            let query = vec![
                ("page".to_string(), page.to_string()),
                ("per_page".to_string(), PER_PAGE.to_string()),
                ("order".to_string(), "desc".to_string()),
            ];
            let body = self
                .http
                .get_json(&url, &query, &headers)
                .await
                .with_context(|| format!("fetching {url} page {page}"))?;

            let item_count = body
                .get("txns")
                .and_then(|v| v.as_array())
                .map(|a| a.len() as u64)
                .unwrap_or(0);

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

            // A short page means we've reached the end.
            if item_count < PER_PAGE {
                cursor.done = true;
                cursor.updated_at = tinotax_store::now_rfc3339();
                cache.write_cursor(&cursor)?;
                break;
            }

            cursor.next_page += 1;
            cursor.updated_at = tinotax_store::now_rfc3339();
            cache.write_cursor(&cursor)?;

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
