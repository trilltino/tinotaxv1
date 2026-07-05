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

#[derive(Debug, Clone, Copy)]
pub struct FetchContext<'a> {
    pub project_dir: &'a Utf8Path,
    pub resume: bool,
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
