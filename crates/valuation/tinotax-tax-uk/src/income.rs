//! Income receipts (staking, mining, airdrops-as-income, employment, …):
//! taxable at market value when received; that value then becomes the CGT
//! cost basis via the pool (handled in `matching::entry_cost`).
//!
//! Airdrops received for nothing are **not** treated as income by default —
//! they appear here with £0 income so HMRC question 8 can still be answered
//! from one file; the reviewer reclassifies to `misc_income` where the
//! airdrop was in return for a service.

use tinotax_core::{TaxEventType, TaxLedgerEvent};

use crate::domain::IncomeCalculation;

pub(crate) fn build_rows(events: &[TaxLedgerEvent]) -> Vec<IncomeCalculation> {
    let mut rows: Vec<IncomeCalculation> = events
        .iter()
        .filter(|e| e.tax_event_type.is_income() || e.tax_event_type == TaxEventType::Airdrop)
        .map(|e| IncomeCalculation {
            ledger_event_id: e.ledger_event_id.clone(),
            asset: e.asset_symbol.to_ascii_uppercase(),
            date: e.timestamp.get(..10).unwrap_or(&e.timestamp).to_string(),
            tax_year: e.tax_year.clone(),
            category: e.tax_event_type.as_str().to_string(),
            quantity: e.quantity,
            income_gbp: e.income_gbp.unwrap_or_default(),
            note: e.user_note.clone(),
        })
        .collect();
    rows.sort_by(|a, b| {
        (
            a.date.as_str(),
            a.asset.as_str(),
            a.ledger_event_id.as_str(),
        )
            .cmp(&(
                b.date.as_str(),
                b.asset.as_str(),
                b.ledger_event_id.as_str(),
            ))
    });
    rows
}
