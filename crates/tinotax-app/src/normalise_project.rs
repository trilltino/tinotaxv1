use anyhow::Result;
use tinotax_normalise::NormaliseSummary;

use crate::open_project;

/// Regenerate `staging/` from `raw/`. Pure derivation — safe to re-run.
pub fn normalise_project(project: &str) -> Result<NormaliseSummary> {
    let (paths, config) = open_project(project)?;
    let wallets: Vec<_> = config.wallets.iter().map(|w| w.to_source()).collect();
    let summary = tinotax_normalise::normalise_project(&paths, &config.project.name, &wallets)?;

    println!(
        "normalised {} events ({} duplicates dropped, {} rejected, {} warnings)",
        summary.total_events, summary.duplicates_dropped, summary.rejected_items, summary.warnings
    );
    for w in &summary.per_wallet {
        println!(
            "  {:<16} {:<10} {:>8} events",
            w.source_id, w.chain, w.events
        );
    }
    Ok(summary)
}
