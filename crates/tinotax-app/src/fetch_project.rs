use anyhow::Result;
use tinotax_config::{ProviderEntry, ProviderKind};
use tinotax_connectors::{make_fetcher, FetchContext, FetchReport, ProviderSpec};

use crate::open_project;

fn provider_spec(entry: &ProviderEntry) -> ProviderSpec {
    match entry.kind {
        ProviderKind::Blockscout => ProviderSpec::Blockscout {
            base_url: entry.base_url.clone(),
        },
        ProviderKind::Nearblocks => ProviderSpec::NearBlocks {
            base_url: entry.base_url.clone(),
            api_key: std::env::var("NEARBLOCKS_API_KEY").ok().filter(|k| !k.is_empty()),
        },
    }
}

/// Fetch every configured wallet into the raw cache. With `resume`, honours
/// per-endpoint cursors, so a rate-limited or interrupted run picks up where
/// it stopped instead of re-downloading.
pub async fn fetch_project(project: &str, resume: bool) -> Result<Vec<FetchReport>> {
    let (paths, config) = open_project(project)?;
    let ctx = FetchContext {
        project_dir: &paths.root,
        resume,
    };

    let mut reports = Vec::new();
    for wallet in &config.wallets {
        let entry = config.provider_for(wallet);
        let fetcher = make_fetcher(&provider_spec(entry))?;
        let source = wallet.to_source();
        println!(
            "fetching {} ({} / {}) via {} ...",
            wallet.id, wallet.chain, wallet.address, wallet.provider
        );
        let report = fetcher.fetch_wallet(ctx, &source).await?;
        println!(
            "  {} pages, {} items{}",
            report.pages_fetched,
            report.items_fetched,
            if report.resumed { " (resumed)" } else { "" }
        );
        reports.push(report);
    }
    Ok(reports)
}
