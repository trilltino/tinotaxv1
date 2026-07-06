//! Reviewed/priced ledger → CSV. One exporter serves both files: the priced
//! ledger is the same shape with GBP values filled in.

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use tinotax_core::TaxLedgerEvent;
use tinotax_store::ProjectPaths;

pub const LEDGER_COLUMNS: [&str; 21] = [
    "ledger_event_id",
    "source_event_ids",
    "timestamp",
    "tax_year",
    "platform",
    "chain",
    "wallet",
    "tx_hash",
    "tax_event_type",
    "asset_symbol",
    "asset_contract",
    "quantity",
    "proceeds_gbp",
    "cost_gbp",
    "income_gbp",
    "fee_gbp",
    "price_source",
    "price_confidence",
    "review_status",
    "user_note",
    "raw_files",
];

/// Write the ledger to `out/<file_name>`. Returns the number of rows.
pub fn export_ledger_csv(
    paths: &ProjectPaths,
    ledger: &[TaxLedgerEvent],
    file_name: &str,
) -> Result<u64> {
    std::fs::create_dir_all(paths.out())?;
    let path = paths.out().join(file_name);
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record(LEDGER_COLUMNS)?;
    let opt_dec = |d: Option<Decimal>| d.map(|v| v.to_string()).unwrap_or_default();
    for row in ledger {
        let raw_files: Vec<&str> = row
            .source_refs
            .iter()
            .map(|r| r.raw_file.as_str())
            .collect();
        writer.write_record([
            row.ledger_event_id.as_str(),
            &row.source_event_ids.join("; "),
            row.timestamp.as_str(),
            row.tax_year.as_str(),
            row.platform.as_deref().unwrap_or(""),
            row.chain.as_deref().unwrap_or(""),
            row.wallet.as_deref().unwrap_or(""),
            row.tx_hash.as_deref().unwrap_or(""),
            row.tax_event_type.as_str(),
            row.asset_symbol.as_str(),
            row.asset_contract.as_deref().unwrap_or(""),
            &row.quantity.to_string(),
            &opt_dec(row.proceeds_gbp),
            &opt_dec(row.cost_gbp),
            &opt_dec(row.income_gbp),
            &opt_dec(row.fee_gbp),
            row.price_source.as_deref().unwrap_or(""),
            row.price_confidence.as_str(),
            row.review_status.as_str(),
            row.user_note.as_deref().unwrap_or(""),
            &raw_files.join("; "),
        ])?;
    }
    writer.flush()?;
    Ok(ledger.len() as u64)
}
