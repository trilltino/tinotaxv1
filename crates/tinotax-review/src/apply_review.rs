use std::str::FromStr;

use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use tinotax_core::ReviewAction;
use tinotax_store::{JsonlWriter, ProjectPaths};
use tracing::warn;

/// One accepted decision, recorded to `staging/review_overrides.jsonl`.
/// The tax engine (milestone 2) consumes these; raw and normalised data
/// are never mutated by a review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewOverride {
    pub event_id: String,
    pub user_action: ReviewAction,
    pub user_note: Option<String>,
    pub applied_at: String,
}

/// Ingest an edited `manual_review.csv`. Rows with an empty `user_action`
/// are skipped; invalid actions fail the whole run so a typo can't silently
/// drop a decision.
pub fn apply_review(paths: &ProjectPaths, edited_csv: &Utf8Path) -> Result<u64> {
    let mut reader = csv::Reader::from_path(edited_csv)
        .with_context(|| format!("opening {edited_csv}"))?;
    let headers = reader.headers()?.clone();
    let col = |name: &str| headers.iter().position(|h| h == name);
    let (Some(id_col), Some(action_col)) = (col("event_id"), col("user_action")) else {
        bail!("{edited_csv} must have `event_id` and `user_action` columns");
    };
    let note_col = col("user_note");

    let mut overrides = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let event_id = record.get(id_col).unwrap_or("").trim();
        let action_text = record.get(action_col).unwrap_or("").trim();
        if action_text.is_empty() {
            continue;
        }
        if event_id.is_empty() {
            warn!(row = i + 2, "skipping row with user_action but no event_id");
            continue;
        }
        let user_action = ReviewAction::from_str(action_text)
            .with_context(|| format!("row {} of {edited_csv}", i + 2))?;
        let user_note = note_col
            .and_then(|c| record.get(c))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        overrides.push(ReviewOverride {
            event_id: event_id.to_string(),
            user_action,
            user_note,
            applied_at: tinotax_store::now_rfc3339(),
        });
    }

    std::fs::create_dir_all(paths.staging())?;
    let out_path = paths.staging().join("review_overrides.jsonl");
    let mut writer = JsonlWriter::create(&out_path)?;
    for o in &overrides {
        writer.write(o)?;
    }
    writer.finish()
}
