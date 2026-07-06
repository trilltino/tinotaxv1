//! `prices missing`: which (asset, day) pairs still need a GBP price.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use tinotax_core::{parse_date_prefix, TaxLedgerEvent};
use tinotax_ledger::load_reviewed_ledger;
use tinotax_store::ProjectPaths;

use crate::price_book::PriceBook;
use crate::valuation::needed_field;

#[derive(Debug, Clone)]
pub struct MissingPrice {
    pub asset_symbol: String,
    pub date: String,
    pub events_needing_price: u64,
    pub example_event_id: String,
    pub example_timestamp: String,
}

/// Aggregate the ledger's unmet pricing needs per (asset, day).
pub fn missing_prices(ledger: &[TaxLedgerEvent], book: &PriceBook) -> Vec<MissingPrice> {
    let mut missing: BTreeMap<(String, String), MissingPrice> = BTreeMap::new();
    for row in ledger {
        if needed_field(row).is_none() {
            continue;
        }
        if book.lookup(&row.asset_symbol, &row.timestamp).is_some() {
            continue;
        }
        let Ok((y, m, d)) = parse_date_prefix(&row.timestamp) else {
            continue;
        };
        let date = format!("{y:04}-{m:02}-{d:02}");
        missing
            .entry((row.asset_symbol.to_ascii_uppercase(), date.clone()))
            .and_modify(|e| e.events_needing_price += 1)
            .or_insert_with(|| MissingPrice {
                asset_symbol: row.asset_symbol.to_ascii_uppercase(),
                date,
                events_needing_price: 1,
                example_event_id: row.ledger_event_id.clone(),
                example_timestamp: row.timestamp.clone(),
            });
    }
    missing.into_values().collect()
}

/// Write `out/missing_prices.csv`. Returns the number of (asset, day) rows.
pub fn export_missing_prices(paths: &ProjectPaths) -> Result<u64> {
    let ledger = load_reviewed_ledger(paths)?;
    let book = PriceBook::load(paths)?;
    let missing = missing_prices(&ledger, &book);

    std::fs::create_dir_all(paths.out())?;
    let path = paths.out().join("missing_prices.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record([
        "asset_symbol",
        "date",
        "events_needing_price",
        "example_event_id",
        "example_timestamp",
    ])?;
    for m in &missing {
        writer.write_record([
            m.asset_symbol.as_str(),
            m.date.as_str(),
            &m.events_needing_price.to_string(),
            m.example_event_id.as_str(),
            m.example_timestamp.as_str(),
        ])?;
    }
    writer.flush()?;
    Ok(missing.len() as u64)
}
