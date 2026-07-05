//! `out/pricing_audit.csv`: every GBP value the pipeline derived, with the
//! price, the day it was observed, and where it came from. This is what an
//! accountant checks when a number looks odd.

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use tinotax_store::ProjectPaths;

#[derive(Debug, Clone)]
pub struct PricingAuditRow {
    pub ledger_event_id: String,
    pub timestamp: String,
    pub asset_symbol: String,
    pub tax_event_type: String,
    pub field: String,
    pub quantity: Decimal,
    pub price_gbp: Decimal,
    pub observed_date: String,
    pub value_gbp: Decimal,
    pub source: String,
    pub confidence: String,
}

pub fn write_pricing_audit(paths: &ProjectPaths, rows: &[PricingAuditRow]) -> Result<u64> {
    std::fs::create_dir_all(paths.out())?;
    let path = paths.out().join("pricing_audit.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record([
        "ledger_event_id",
        "timestamp",
        "asset_symbol",
        "tax_event_type",
        "field",
        "quantity",
        "price_gbp",
        "observed_date",
        "value_gbp",
        "source",
        "confidence",
    ])?;
    for r in rows {
        writer.write_record([
            r.ledger_event_id.as_str(),
            r.timestamp.as_str(),
            r.asset_symbol.as_str(),
            r.tax_event_type.as_str(),
            r.field.as_str(),
            &r.quantity.to_string(),
            &r.price_gbp.to_string(),
            r.observed_date.as_str(),
            &r.value_gbp.to_string(),
            r.source.as_str(),
            r.confidence.as_str(),
        ])?;
    }
    writer.flush()?;
    Ok(rows.len() as u64)
}
