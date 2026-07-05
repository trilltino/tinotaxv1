//! Deterministic duplicate removal for normalised events.
//!
//! Normalisers can produce overlapping rows from multiple raw endpoints. This
//! module keeps the first event for each deterministic ID and drops repeats.
use std::collections::HashSet;

use tinotax_core::NormalisedEvent;

/// Drop events with duplicate `event_id`s (e.g. the same transfer appearing
/// on overlapping pages after a resume). First occurrence wins.
pub fn dedupe(events: Vec<NormalisedEvent>) -> (Vec<NormalisedEvent>, u64) {
    let mut seen: HashSet<String> = HashSet::with_capacity(events.len());
    let mut kept = Vec::with_capacity(events.len());
    let mut dropped = 0u64;
    for event in events {
        if seen.insert(event.event_id.clone()) {
            kept.push(event);
        } else {
            dropped += 1;
        }
    }
    (kept, dropped)
}
