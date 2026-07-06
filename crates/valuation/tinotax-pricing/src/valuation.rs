//! `ledger price`: fill the GBP value every taxable row needs.
//!
//! Value precedence: reviewer-typed GBP values (already on the row from
//! `ledger build`) beat the price book; the book beats nothing — a row
//! with no source stays `missing` and blocks `calculate uk`.

use anyhow::Result;
use rust_decimal::Decimal;
use tinotax_core::{PriceConfidence, TaxEventType, TaxLedgerEvent};
use tinotax_ledger::{export_ledger_csv, load_reviewed_ledger};
use tinotax_store::{JsonlWriter, ProjectPaths};

use crate::audit::{write_pricing_audit, PricingAuditRow};
use crate::price_book::PriceBook;

/// Which GBP field a ledger row still needs before tax calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeededField {
    Proceeds,
    Cost,
    Income,
}

impl NeededField {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proceeds => "proceeds_gbp",
            Self::Cost => "cost_gbp",
            Self::Income => "income_gbp",
        }
    }
}

/// `None` = nothing to price (non-taxable, unknown, fork, or already valued).
pub fn needed_field(row: &TaxLedgerEvent) -> Option<NeededField> {
    let t = row.tax_event_type;
    if t.is_disposal() && row.proceeds_gbp.is_none() {
        Some(NeededField::Proceeds)
    } else if t.is_purchase_like() && t != TaxEventType::Fork && row.cost_gbp.is_none() {
        // Fork base cost is apportioned from the original asset — a market
        // price at receipt would be wrong, so forks are never auto-priced.
        Some(NeededField::Cost)
    } else if t.is_income() && row.income_gbp.is_none() {
        Some(NeededField::Income)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PricingSummary {
    pub total: u64,
    pub valued_from_book: u64,
    pub already_valued: u64,
    pub nothing_to_price: u64,
    pub still_missing: u64,
}

/// Build `staging/priced_ledger.jsonl`, `out/priced_ledger.csv` and
/// `out/pricing_audit.csv` from the reviewed ledger and the price book.
pub fn price_ledger(paths: &ProjectPaths) -> Result<PricingSummary> {
    let ledger = load_reviewed_ledger(paths)?;
    let book = PriceBook::load(paths)?;

    let mut summary = PricingSummary::default();
    let mut audit: Vec<PricingAuditRow> = Vec::new();
    let mut priced: Vec<TaxLedgerEvent> = Vec::with_capacity(ledger.len());
    for row in ledger {
        summary.total += 1;
        let mut row = row;
        // Work one row at a time so audit rows can point to the exact ledger
        // event and field that received an external price.
        match needed_field(&row) {
            None => {
                // Distinguish "user already valued it" from "not taxable".
                if row.tax_event_type.is_disposal() || row.tax_event_type.is_pool_entry() {
                    summary.already_valued += 1;
                } else {
                    summary.nothing_to_price += 1;
                }
            }
            Some(field) => match book.lookup(&row.asset_symbol, &row.timestamp) {
                Some(resolved) => {
                    // The price book stores unit GBP prices; ledger values are
                    // row quantity times unit price, rounded for HMRC reports.
                    let value = (row.quantity * resolved.price).round_dp(2);
                    match field {
                        NeededField::Proceeds => row.proceeds_gbp = Some(value),
                        NeededField::Cost => row.cost_gbp = Some(value),
                        NeededField::Income => row.income_gbp = Some(value),
                    }
                    // A fee event's value is both its proceeds and its cost
                    // to the payer; keep fee_gbp aligned for reporting.
                    if row.tax_event_type == TaxEventType::Fee && row.fee_gbp.is_none() {
                        row.fee_gbp = Some(value);
                    }
                    row.price_source = Some(resolved.source.clone());
                    row.price_confidence = resolved.confidence;
                    summary.valued_from_book += 1;
                    audit.push(PricingAuditRow {
                        ledger_event_id: row.ledger_event_id.clone(),
                        timestamp: row.timestamp.clone(),
                        asset_symbol: row.asset_symbol.clone(),
                        tax_event_type: row.tax_event_type.as_str().to_string(),
                        field: field.as_str().to_string(),
                        quantity: row.quantity,
                        price_gbp: resolved.price,
                        observed_date: resolved.observed_date,
                        value_gbp: value,
                        source: resolved.source,
                        confidence: resolved.confidence.as_str().to_string(),
                    });
                }
                None => {
                    row.price_confidence = PriceConfidence::Missing;
                    summary.still_missing += 1;
                }
            },
        }
        priced.push(row);
    }

    std::fs::create_dir_all(paths.staging())?;
    let mut writer = JsonlWriter::create(&paths.priced_ledger_jsonl())?;
    for row in &priced {
        writer.write(row)?;
    }
    writer.finish()?;

    export_ledger_csv(paths, &priced, "priced_ledger.csv")?;
    write_pricing_audit(paths, &audit)?;
    Ok(summary)
}

/// Read the priced ledger back (for `calculate uk` and the evidence pack).
pub fn load_priced_ledger(paths: &ProjectPaths) -> Result<Vec<TaxLedgerEvent>> {
    use anyhow::Context;
    tinotax_store::read_jsonl(&paths.priced_ledger_jsonl())
        .context("reading staging/priced_ledger.jsonl — run `ledger price` first")
}

/// GBP value rounding used across reports: 2 decimal places, banker's off.
pub fn round_gbp(value: Decimal) -> Decimal {
    value.round_dp(2)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use pretty_assertions::assert_eq;
    use tinotax_core::{
        PriceObservation, PriceSource, ReviewStatus, SourceRef, TaxEventType, TaxLedgerEvent,
    };
    use tinotax_store::{JsonlWriter, ProjectPaths};

    #[test]
    fn ledger_price_writes_gbp_values_to_csvs() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let paths = ProjectPaths::new(
            camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).map_err(|path| {
                std::io::Error::other(format!("non-UTF8 path {}", path.display()))
            })?,
        );
        paths.init()?;

        let ledger_row = TaxLedgerEvent {
            ledger_event_id: "lev_1".into(),
            source_event_ids: vec!["evt_1".into()],
            source_refs: vec![SourceRef {
                raw_file: "raw/lisk/page_000001.json".into(),
                raw_page: Some(1),
                json_path: Some("items[0]".into()),
                log_index: None,
                movement_index: Some(0),
            }],
            timestamp: "2025-01-09T16:34:35Z".into(),
            tax_year: "2024-2025".into(),
            platform: None,
            chain: Some("lisk-evm".into()),
            wallet: Some("0xwallet".into()),
            tx_hash: Some("0xtx".into()),
            tax_event_type: TaxEventType::Disposal,
            asset_symbol: "ETH".into(),
            asset_contract: None,
            quantity: Decimal::new(2, 0),
            proceeds_gbp: None,
            cost_gbp: None,
            income_gbp: None,
            fee_gbp: None,
            price_source: None,
            price_confidence: PriceConfidence::Missing,
            review_status: ReviewStatus::Auto,
            user_note: None,
        };

        let mut ledger_writer = JsonlWriter::create(&paths.reviewed_ledger_jsonl())?;
        ledger_writer.write(&ledger_row)?;
        ledger_writer.finish()?;

        let observation = PriceObservation {
            asset_symbol: "ETH".into(),
            asset_contract: None,
            timestamp: "2025-01-09T00:00:00Z".into(),
            currency: "GBP".into(),
            price: Decimal::new(2500, 0),
            source: PriceSource::Coingecko,
            confidence: PriceConfidence::High,
            fetched_at: "2026-07-06T00:00:00Z".into(),
            note: Some("test price".into()),
        };
        let mut price_writer = JsonlWriter::create(&paths.price_observations_jsonl())?;
        price_writer.write(&observation)?;
        price_writer.finish()?;

        let summary = price_ledger(&paths)?;
        assert_eq!(summary.valued_from_book, 1);
        assert_eq!(summary.still_missing, 0);

        let priced_csv = std::fs::read_to_string(paths.out().join("priced_ledger.csv"))?;
        assert!(priced_csv.contains("proceeds_gbp"));
        assert!(priced_csv.contains(",5000,"));
        assert!(priced_csv.contains(",coingecko,high,"));

        let audit_csv = std::fs::read_to_string(paths.out().join("pricing_audit.csv"))?;
        assert!(audit_csv.contains("price_gbp"));
        assert!(audit_csv.contains("2500"));
        assert!(audit_csv.contains("5000"));
        assert!(audit_csv.contains("coingecko"));
        Ok(())
    }
}
