//! Normalised event JSON export.
//!
//! The JSON export is a human/debugging view over the JSONL staging file and
//! remains regenerable from normalised events.
use anyhow::{Context, Result};
use tinotax_core::NormalisedEvent;
use tinotax_store::{read_jsonl, ProjectPaths};

/// Pretty-printed JSON copy of the events for consumers that prefer JSON
/// over JSONL/CSV. Derived output like everything else in `out/`.
pub fn export_events_json(paths: &ProjectPaths) -> Result<u64> {
    let events: Vec<NormalisedEvent> = read_jsonl(&paths.events_jsonl())
        .context("reading staging/normalised_events.jsonl — run `normalise` first")?;
    std::fs::create_dir_all(paths.out())?;
    let path = paths.out().join("normalised_events.json");
    std::fs::write(&path, serde_json::to_string_pretty(&events)?)
        .with_context(|| format!("writing {path}"))?;
    Ok(events.len() as u64)
}
