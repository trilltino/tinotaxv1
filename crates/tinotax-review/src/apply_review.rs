//! `review apply`: ingest an edited review CSV.
//!
//! The rule (never broken anywhere in the codebase):
//!
//! ```text
//! Never mutate raw data.
//! Never mutate normalised_events.jsonl.
//! All human changes become review_overrides.jsonl (append-only).
//! reviewed_ledger.jsonl is derived from normalised_events + review_overrides.
//! ```
//!
//! Accepts both the milestone-1 `manual_review.csv` (event_id, user_action,
//! user_note) and the full `review_all_transactions.csv` (user_tax_type and
//! the user_* value columns). Any parse problem fails the whole run so a
//! typo cannot silently drop a decision.

use std::collections::BTreeSet;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use rust_decimal::Decimal;
use tinotax_core::{PriceSource, ReviewAction, ReviewOverride, TaxEventType};
use tinotax_store::{JsonlWriter, ProjectPaths};
use tracing::warn;

use crate::load::{load_all_events, load_override_history};

/// Parse the edited CSV, validate every filled-in cell, append the accepted
/// decisions to `staging/review_overrides.jsonl`, and regenerate
/// `out/change_log.csv`. Returns the number of decisions recorded.
pub fn apply_review(paths: &ProjectPaths, edited_csv: &Utf8Path) -> Result<u64> {
    let known_ids: BTreeSet<String> = load_all_events(paths)?
        .into_iter()
        .map(|e| e.event_id)
        .collect();

    let mut reader =
        csv::Reader::from_path(edited_csv).with_context(|| format!("opening {edited_csv}"))?;
    let headers = reader.headers()?.clone();
    let col = |name: &str| headers.iter().position(|h| h == name);

    let Some(id_col) = col("event_id") else {
        bail!("{edited_csv} must have an `event_id` column");
    };
    let action_col = col("user_action");
    let tax_type_col = col("user_tax_type");
    if action_col.is_none() && tax_type_col.is_none() {
        bail!("{edited_csv} must have a `user_action` or `user_tax_type` column");
    }
    // All user-editable columns are optional. Blank cells mean "keep the
    // machine value/current decision"; filled cells become an override record.
    let asset_col = col("user_asset_symbol");
    let quantity_col = col("user_quantity");
    let proceeds_col = col("user_proceeds_gbp");
    let cost_col = col("user_cost_gbp");
    let income_col = col("user_income_gbp");
    let fee_col = col("user_fee_gbp");
    let price_source_col = col("user_price_source");
    let note_col = col("user_note");

    let source_file = edited_csv.file_name().unwrap_or("review.csv").to_string();
    let mut overrides = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let row = i + 2; // 1-based + header row, matches what a spreadsheet shows
        let cell = |c: Option<usize>| {
            c.and_then(|c| record.get(c))
                .map(str::trim)
                .filter(|s| !s.is_empty())
        };
        let decimal_cell = |c: Option<usize>, name: &str| -> Result<Option<Decimal>> {
            cell(c)
                .map(|text| {
                    Decimal::from_str(text)
                        .with_context(|| format!("row {row}: invalid {name} {text:?}"))
                })
                .transpose()
        };

        let event_id = record.get(id_col).unwrap_or("").trim();

        // Parse enum-like columns before building the candidate so invalid
        // spreadsheet text fails with the same row number the user sees.
        let user_action = cell(action_col)
            .map(|text| {
                ReviewAction::from_str(text)
                    .with_context(|| format!("row {row}: invalid user_action {text:?}"))
            })
            .transpose()?;
        let user_tax_type = cell(tax_type_col)
            .map(|text| {
                TaxEventType::from_str(text)
                    .with_context(|| format!("row {row}: invalid user_tax_type {text:?}"))
            })
            .transpose()?;
        let user_price_source = cell(price_source_col)
            .map(|text| {
                PriceSource::from_str(text)
                    .with_context(|| format!("row {row}: invalid user_price_source {text:?}"))
                    .map(|s| s.as_str().to_string())
            })
            .transpose()?;

        let candidate = ReviewOverride {
            event_id: event_id.to_string(),
            user_action,
            user_tax_type,
            user_asset_symbol: cell(asset_col).map(str::to_string),
            user_quantity: decimal_cell(quantity_col, "user_quantity")?,
            user_proceeds_gbp: decimal_cell(proceeds_col, "user_proceeds_gbp")?,
            user_cost_gbp: decimal_cell(cost_col, "user_cost_gbp")?,
            user_income_gbp: decimal_cell(income_col, "user_income_gbp")?,
            user_fee_gbp: decimal_cell(fee_col, "user_fee_gbp")?,
            user_price_source,
            user_note: cell(note_col).map(str::to_string),
            applied_at: tinotax_store::now_rfc3339(),
            source_file: Some(source_file.clone()),
        };

        if !candidate.has_any_decision() {
            continue; // untouched row
        }
        if event_id.is_empty() {
            // A blank ID cannot be safely applied, but warning keeps accidental
            // notes below the table from killing an otherwise valid import.
            warn!(row, "skipping row with edits but no event_id");
            continue;
        }
        if !known_ids.contains(event_id) {
            bail!(
                "row {row}: event_id {event_id:?} does not exist in this project — \
                 was the CSV edited in a way that changed the id column?"
            );
        }
        if let Some(q) = candidate.user_quantity {
            if q < Decimal::ZERO {
                bail!("row {row}: user_quantity must be >= 0 (amounts are unsigned; the tax type carries direction)");
            }
        }
        overrides.push(candidate);
    }

    std::fs::create_dir_all(paths.staging())?;
    // Append only after every row has validated. This avoids half-applying a
    // spreadsheet where a later row contains an invalid value.
    let mut writer = JsonlWriter::append(&paths.overrides_jsonl())?;
    for o in &overrides {
        writer.write(o)?;
    }
    let appended = writer.finish()?;

    write_change_log(paths).context("regenerating out/change_log.csv")?;
    Ok(appended)
}

/// `out/change_log.csv`: the full, human-readable history of every review
/// decision ever applied — evidence that edits were deliberate and traceable.
pub fn write_change_log(paths: &ProjectPaths) -> Result<u64> {
    let history = load_override_history(paths)?;
    std::fs::create_dir_all(paths.out())?;
    let path = paths.out().join("change_log.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record([
        "applied_at",
        "event_id",
        "fields_set",
        "user_action",
        "user_tax_type",
        "user_asset_symbol",
        "user_quantity",
        "user_proceeds_gbp",
        "user_cost_gbp",
        "user_income_gbp",
        "user_fee_gbp",
        "user_price_source",
        "user_note",
        "source_file",
    ])?;
    let opt_dec = |d: Option<Decimal>| d.map(|v| v.to_string()).unwrap_or_default();
    for o in &history {
        writer.write_record([
            o.applied_at.as_str(),
            o.event_id.as_str(),
            &o.fields_set().join("; "),
            o.user_action.map(|a| a.as_str()).unwrap_or(""),
            o.user_tax_type.map(|t| t.as_str()).unwrap_or(""),
            o.user_asset_symbol.as_deref().unwrap_or(""),
            &opt_dec(o.user_quantity),
            &opt_dec(o.user_proceeds_gbp),
            &opt_dec(o.user_cost_gbp),
            &opt_dec(o.user_income_gbp),
            &opt_dec(o.user_fee_gbp),
            o.user_price_source.as_deref().unwrap_or(""),
            o.user_note.as_deref().unwrap_or(""),
            o.source_file.as_deref().unwrap_or(""),
        ])?;
    }
    writer.flush()?;
    Ok(history.len() as u64)
}
