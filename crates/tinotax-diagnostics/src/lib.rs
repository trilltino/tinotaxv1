//! Diagnostics for normalised project data.
//!
//! The crate writes deterministic data-quality reports from
//! `staging/normalised_events.jsonl` without mutating any project data.
pub mod assets;
pub mod duplicates;
pub mod review_flags;
pub mod summary;

use anyhow::{Context, Result};
use tinotax_core::NormalisedEvent;
use tinotax_store::{read_jsonl, ProjectPaths};

pub use summary::{Diagnostics, WalletDiagnostics};

/// Compute diagnostics from staged events and write
/// `out/diagnostics.json` + `out/wallet_activity_summary.csv`.
pub fn run(paths: &ProjectPaths, project_name: &str) -> Result<Diagnostics> {
    let events: Vec<NormalisedEvent> = read_jsonl(&paths.events_jsonl())
        .context("reading staging/normalised_events.jsonl — run `normalise` first")?;

    let diagnostics = summary::compute(project_name, &events);

    std::fs::create_dir_all(paths.out())?;
    let json_path = paths.out().join("diagnostics.json");
    std::fs::write(&json_path, serde_json::to_string_pretty(&diagnostics)?)
        .with_context(|| format!("writing {json_path}"))?;

    summary::write_wallet_summary_csv(
        &paths.out().join("wallet_activity_summary.csv"),
        &diagnostics,
    )?;

    Ok(diagnostics)
}
