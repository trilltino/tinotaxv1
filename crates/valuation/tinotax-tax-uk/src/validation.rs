//! Pre-flight checks: the engine refuses to calculate on unresolved rows
//! rather than guessing (`--allow-unpriced` downgrades the refusal to
//! exclude-and-report).

use rust_decimal::Decimal;
use tinotax_core::{ReviewStatus, TaxEventType, TaxLedgerEvent};

use crate::domain::{TaxError, UnresolvedTaxItem};

fn date_of(event: &TaxLedgerEvent) -> String {
    event
        .timestamp
        .get(..10)
        .unwrap_or(&event.timestamp)
        .to_string()
}

fn item(event: &TaxLedgerEvent, severity: &str, reason: String) -> UnresolvedTaxItem {
    UnresolvedTaxItem {
        ledger_event_id: event.ledger_event_id.clone(),
        asset: event.asset_symbol.clone(),
        date: date_of(event),
        severity: severity.to_string(),
        reason,
    }
}

/// Why a row cannot enter the calculation, if any.
fn blocker_reason(event: &TaxLedgerEvent) -> Option<String> {
    let t = event.tax_event_type;
    if t == TaxEventType::Unknown && !event.quantity.is_zero() {
        return Some("unclassified event (tax type is `unknown`)".to_string());
    }
    if t.is_disposal() && event.proceeds_gbp.is_none() {
        return Some("disposal has no GBP proceeds (missing price)".to_string());
    }
    if t.is_purchase_like() && t != TaxEventType::Fork && event.cost_gbp.is_none() {
        return Some("acquisition has no GBP cost (missing price)".to_string());
    }
    if t.is_income() && event.income_gbp.is_none() {
        return Some("income receipt has no GBP value (missing price)".to_string());
    }
    if event.quantity < Decimal::ZERO {
        return Some("negative quantity".to_string());
    }
    None
}

/// Split events into (included, unresolved). Errors when blockers exist and
/// `allow_unpriced` is false.
pub fn validate(
    events: &[TaxLedgerEvent],
    allow_unpriced: bool,
) -> Result<(Vec<TaxLedgerEvent>, Vec<UnresolvedTaxItem>), TaxError> {
    let mut included = Vec::with_capacity(events.len());
    let mut unresolved = Vec::new();
    for event in events {
        if event.tax_event_type.is_non_taxable() {
            included.push(event.clone()); // pass-through, no tax effect
            continue;
        }
        match blocker_reason(event) {
            Some(reason) => unresolved.push(item(event, "blocker", reason)),
            None => {
                if event.review_status == ReviewStatus::NeedsReview {
                    unresolved.push(item(
                        event,
                        "warning",
                        "included in the calculation but still flagged for review".to_string(),
                    ));
                }
                if event.tax_event_type == TaxEventType::Fork && event.cost_gbp.is_none() {
                    unresolved.push(item(
                        event,
                        "warning",
                        "fork with no user-supplied base cost — treated as £0 cost".to_string(),
                    ));
                }
                included.push(event.clone());
            }
        }
    }

    let blockers: Vec<&UnresolvedTaxItem> = unresolved
        .iter()
        .filter(|u| u.severity == "blocker")
        .collect();
    if !blockers.is_empty() && !allow_unpriced {
        let examples = blockers
            .iter()
            .take(10)
            .map(|u| {
                format!(
                    "  {} {} {} — {}",
                    u.date, u.asset, u.ledger_event_id, u.reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Err(TaxError::UnresolvedItems {
            count: blockers.len(),
            examples,
        });
    }
    Ok((included, unresolved))
}
