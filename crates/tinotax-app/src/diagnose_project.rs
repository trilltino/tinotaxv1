use anyhow::Result;
use tinotax_diagnostics::Diagnostics;

use crate::open_project;

/// Compute + write diagnostics, and print the human summary.
pub fn diagnose_project(project: &str) -> Result<Diagnostics> {
    let (paths, config) = open_project(project)?;
    let diagnostics = tinotax_diagnostics::run(&paths, &config.project.name)?;

    println!(
        "project {}: {} events",
        diagnostics.project, diagnostics.total_events
    );
    println!(
        "{:<10} {:<44} {:>8} {:>6} {:>6} {:>6} {:>6} {:>7}",
        "chain", "wallet", "events", "in", "out", "self", "fees", "review"
    );
    for w in &diagnostics.wallets {
        println!(
            "{:<10} {:<44} {:>8} {:>6} {:>6} {:>6} {:>6} {:>7}",
            w.chain,
            w.wallet,
            w.events,
            w.in_events,
            w.out_events,
            w.self_transfers,
            w.fees,
            w.needs_review
        );
    }
    println!(
        "review queue: {} ({} contract calls, {} possible swaps, {} possible bridges)",
        diagnostics.review.needs_review,
        diagnostics.review.unknown_contract_calls,
        diagnostics.review.possible_swaps,
        diagnostics.review.possible_bridges
    );
    if diagnostics.duplicate_event_ids > 0 {
        println!(
            "WARNING: {} duplicate event ids in staging — this is a bug, please report it",
            diagnostics.duplicate_event_ids
        );
    }
    Ok(diagnostics)
}
