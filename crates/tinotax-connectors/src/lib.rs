//! External provider connectors for wallet raw-data ingestion.
//!
//! The crate exposes provider factories and shared fetcher traits. It writes
//! raw evidence through `tinotax-store` but leaves interpretation to
//! `tinotax-normalise`.
pub mod blockscout;
pub mod fetcher;
pub mod http;
pub mod models;
pub mod nearblocks;

pub use blockscout::BlockscoutFetcher;
pub use fetcher::{FetchContext, FetchReport, WalletFetcher};
pub use http::HttpClient;
pub use nearblocks::NearBlocksFetcher;

/// Provider selection decoupled from the config crate so dependencies stay
/// one-way (`app` maps config entries into this).
#[derive(Debug, Clone)]
pub enum ProviderSpec {
    Blockscout {
        base_url: String,
    },
    NearBlocks {
        base_url: String,
        api_key: Option<String>,
    },
}

pub fn make_fetcher(spec: &ProviderSpec) -> anyhow::Result<Box<dyn WalletFetcher>> {
    let http = HttpClient::new()?;
    Ok(match spec {
        ProviderSpec::Blockscout { base_url } => {
            Box::new(BlockscoutFetcher::new(base_url.clone(), http))
        }
        ProviderSpec::NearBlocks { base_url, api_key } => Box::new(NearBlocksFetcher::new(
            base_url.clone(),
            api_key.clone(),
            http,
        )),
    })
}
