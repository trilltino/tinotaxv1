//! Shared loaders for the review/ledger layers: merged normalised events
//! (wallets + CEX) and the latest review override per event.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use tinotax_core::{NormalisedEvent, ReviewOverride};
use tinotax_store::{read_jsonl, ProjectPaths};

/// Every normalised event in the project: wallet events plus CEX events,
/// sorted by (timestamp, event_id) for deterministic output.
pub fn load_all_events(paths: &ProjectPaths) -> Result<Vec<NormalisedEvent>> {
    let wallet_path = paths.events_jsonl();
    let cex_path = paths.cex_events_jsonl();

    let mut events: Vec<NormalisedEvent> = Vec::new();
    if wallet_path.exists() {
        events.extend(
            read_jsonl::<NormalisedEvent>(&wallet_path)
                .with_context(|| format!("reading {wallet_path}"))?,
        );
    }
    if cex_path.exists() {
        events.extend(
            read_jsonl::<NormalisedEvent>(&cex_path)
                .with_context(|| format!("reading {cex_path}"))?,
        );
    }
    anyhow::ensure!(
        !events.is_empty(),
        "no normalised events found — run `normalise` (and `import-cex` if configured) first"
    );
    events.sort_by(|a, b| {
        (a.timestamp.as_str(), a.event_id.as_str())
            .cmp(&(b.timestamp.as_str(), b.event_id.as_str()))
    });
    Ok(events)
}

/// The full append-only override history, oldest first. Empty if no review
/// has been applied yet.
pub fn load_override_history(paths: &ProjectPaths) -> Result<Vec<ReviewOverride>> {
    let path = paths.overrides_jsonl();
    if !path.exists() {
        return Ok(Vec::new());
    }
    read_jsonl(&path).with_context(|| format!("reading {path}"))
}

/// Latest decision per event: later applies win, earlier history is kept on
/// disk for the change log.
pub fn load_latest_overrides(paths: &ProjectPaths) -> Result<BTreeMap<String, ReviewOverride>> {
    let mut latest = BTreeMap::new();
    for o in load_override_history(paths)? {
        latest.insert(o.event_id.clone(), o);
    }
    Ok(latest)
}
