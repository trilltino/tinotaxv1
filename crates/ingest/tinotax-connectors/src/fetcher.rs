//! Connector traits and shared fetch result types.
//!
//! Application code calls this abstraction so each provider can implement its
//! own pagination and authentication details while preserving the same raw
//! cache contract.
use async_trait::async_trait;
use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use tinotax_core::WalletSource;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchReport {
    pub source_id: String,
    pub chain: String,
    pub wallet: String,
    pub pages_fetched: u64,
    pub items_fetched: u64,
    /// True when an existing cursor was honoured (including already-done).
    pub resumed: bool,
}

#[derive(Clone, Copy)]
pub struct FetchContext<'a> {
    pub project_dir: &'a Utf8Path,
    pub resume: bool,
    /// Nanoseconds since epoch. Fetchers that page oldest-first may stop once
    /// items pass this point (project period end plus any lookahead the tax
    /// rules need). `None` fetches the full history.
    pub stop_after_ns: Option<u64>,
    /// Called after each page is cached: `(endpoint, page, cumulative_items)`.
    /// Lets the app surface live fetch progress.
    pub on_page: Option<&'a (dyn Fn(&str, u64, u64) + Sync)>,
    /// Polled before each page; returning `true` aborts the fetch. The raw
    /// cache stays resumable (the cursor is already persisted), so a later run
    /// picks up where this one stopped.
    pub cancelled: Option<&'a (dyn Fn() -> bool + Sync)>,
}

impl<'a> FetchContext<'a> {
    /// A context with no progress/cancel hooks (the common case).
    pub fn new(project_dir: &'a Utf8Path, resume: bool, stop_after_ns: Option<u64>) -> Self {
        Self {
            project_dir,
            resume,
            stop_after_ns,
            on_page: None,
            cancelled: None,
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.map(|c| c()).unwrap_or(false)
    }

    pub(crate) fn report_page(&self, endpoint: &str, page: u64, items: u64) {
        if let Some(on_page) = self.on_page {
            on_page(endpoint, page, items);
        }
    }
}

/// One implementation per provider *API shape*, not per chain: the same
/// `BlockscoutFetcher` serves Lisk EVM and IOTA EVM with different base URLs.
#[async_trait]
pub trait WalletFetcher: Send + Sync {
    async fn fetch_wallet(
        &self,
        ctx: FetchContext<'_>,
        wallet: &WalletSource,
    ) -> anyhow::Result<FetchReport>;
}
