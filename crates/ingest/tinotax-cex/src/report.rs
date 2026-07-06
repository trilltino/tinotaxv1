//! Per-source import diagnostics: what was read, what was skipped, and why.

use anyhow::{Context, Result};
use tinotax_store::ProjectPaths;

#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub source_id: String,
    pub platform: String,
    pub rows_read: u64,
    pub events_emitted: u64,
    pub fiat_movements_skipped: u64,
    pub zero_amount_skipped: u64,
    pub needs_review: u64,
    pub price_hints: u64,
    pub earliest: Option<String>,
    pub latest: Option<String>,
    pub warnings: Vec<String>,
}

pub fn write_diagnostics(paths: &ProjectPaths, reports: &[ImportReport]) -> Result<()> {
    std::fs::create_dir_all(paths.out())?;
    let path = paths.out().join("cex_import_diagnostics.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record([
        "source_id",
        "platform",
        "rows_read",
        "events_emitted",
        "fiat_movements_skipped",
        "zero_amount_skipped",
        "needs_review",
        "price_hints",
        "earliest",
        "latest",
        "warnings",
    ])?;
    for r in reports {
        writer.write_record([
            r.source_id.as_str(),
            r.platform.as_str(),
            &r.rows_read.to_string(),
            &r.events_emitted.to_string(),
            &r.fiat_movements_skipped.to_string(),
            &r.zero_amount_skipped.to_string(),
            &r.needs_review.to_string(),
            &r.price_hints.to_string(),
            r.earliest.as_deref().unwrap_or(""),
            r.latest.as_deref().unwrap_or(""),
            &r.warnings.join("; "),
        ])?;
    }
    writer.flush()?;
    Ok(())
}
