//! Duplicate event detection.
//!
//! Event IDs are expected to be deterministic and unique. Duplicate reporting
//! catches normalisation regressions before review or tax steps consume them.
use std::collections::HashSet;

use tinotax_core::NormalisedEvent;

/// Duplicate event IDs surviving into staging indicate a dedupe bug — this
/// is a self-check, not a normal statistic.
pub fn count(events: &[NormalisedEvent]) -> u64 {
    let mut seen = HashSet::with_capacity(events.len());
    events
        .iter()
        .filter(|e| !seen.insert(e.event_id.as_str()))
        .count() as u64
}
