//! Fetch orchestration for configured wallet sources.
//!
//! This module loads provider config, constructs connector implementations,
//! and asks each connector to persist raw pages into the immutable project
//! cache. It does not interpret the fetched JSON.

use anyhow::{Context, Result};
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
            api_key: std::env::var("NEARBLOCKS_API_KEY")
                .ok()
                .filter(|k| !k.is_empty()),
            credits_per_minute: std::env::var("NEARBLOCKS_CREDITS_PER_MINUTE")
                .ok()
                .and_then(|v| v.parse().ok()),
        },
    }
}

/// UK CGT's 30-day bed-and-breakfast rule matches disposals against
/// acquisitions up to 30 days later, so a disposal on the last day of the
/// period needs visibility 30 days past `period_end`.
const FETCH_LOOKAHEAD_SECONDS: i64 = 30 * 24 * 3600;

fn stop_after_ns(period_end: &str) -> Result<u64> {
    let end: jiff::Timestamp = period_end
        .parse()
        .with_context(|| format!("invalid project period_end: {period_end}"))?;
    let ns = end.as_nanosecond() + i128::from(FETCH_LOOKAHEAD_SECONDS) * 1_000_000_000;
    u64::try_from(ns).context("period_end out of range")
}

/// Fetch every configured wallet into the raw cache. With `resume`, honours
/// per-endpoint cursors, so a rate-limited or interrupted run picks up where
/// it stopped instead of re-downloading.
pub async fn fetch_project(project: &str, resume: bool) -> Result<Vec<FetchReport>> {
    let (paths, config) = open_project(project)?;
    let ctx = FetchContext {
        project_dir: &paths.root,
        resume,
        stop_after_ns: Some(stop_after_ns(&config.project.period_end)?),
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
