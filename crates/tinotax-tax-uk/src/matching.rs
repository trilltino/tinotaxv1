//! The engine: aggregate per day (TCGA92 s105), then match each disposal
//! same-day → 30-day ("bed & breakfast", TCGA92 s106A) → Section 104 pool,
//! per HMRC's Cryptoassets Manual (CRYPTO22200, CRYPTO22256).
//!
//! `calculate` is pure and deterministic: the full event timeline in,
//! one tax year's calculation out. The whole timeline is always processed —
//! pools carry across years — and outputs are filtered to the requested
//! year at the end.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use tinotax_core::date::day_of_timestamp;
use tinotax_core::{date::date_string, TaxLedgerEvent};

use crate::domain::{OpeningPool, TaxError, UkTaxCalculation, UkTaxSummary};
use crate::tax_year::TaxYear;
use crate::validation::validate;
use crate::{disposals, fees, income, s104_pool, same_day, thirty_day};

/// All acquisitions of one asset on one day, aggregated. (The day itself
/// is the `AssetDays::acquisitions` map key.)
#[derive(Debug, Clone)]
pub(crate) struct DayAcquisition {
    pub quantity: Decimal,
    pub cost_gbp: Decimal,
    /// Quantity not yet consumed by same-day/30-day matching (what will
    /// eventually enter the pool).
    pub remaining: Decimal,
    pub event_ids: Vec<String>,
}

impl DayAcquisition {
    /// Cost of `quantity` tokens at this day's average acquisition cost.
    pub fn cost_of(&self, quantity: Decimal) -> Decimal {
        if self.quantity.is_zero() {
            Decimal::ZERO
        } else {
            self.cost_gbp * quantity / self.quantity
        }
    }
}

/// All disposals of one asset on one day, aggregated, with the matching
/// breakdown filled in by the three passes.
#[derive(Debug, Clone)]
pub(crate) struct DayDisposal {
    pub day: i64,
    pub quantity: Decimal,
    pub proceeds_gbp: Decimal,
    pub remaining: Decimal,
    pub event_ids: Vec<String>,
    pub same_day_quantity: Decimal,
    pub same_day_cost: Decimal,
    pub thirty_day_quantity: Decimal,
    pub thirty_day_cost: Decimal,
    pub pool_quantity: Decimal,
    pub pool_cost: Decimal,
    pub notes: Vec<String>,
}

/// Per-asset aggregation of the whole timeline.
#[derive(Debug, Default)]
pub(crate) struct AssetDays {
    pub acquisitions: BTreeMap<i64, DayAcquisition>,
    pub disposals: BTreeMap<i64, DayDisposal>,
}

/// The GBP cost a pool entry brings with it: what was paid for purchases,
/// the taxed market value for income receipts, £0 for unvalued forks.
fn entry_cost(event: &TaxLedgerEvent) -> Decimal {
    if event.tax_event_type.is_income() {
        event.income_gbp.unwrap_or_default()
    } else {
        event.cost_gbp.unwrap_or_default()
    }
}

fn aggregate(events: &[TaxLedgerEvent]) -> Result<BTreeMap<String, AssetDays>, TaxError> {
    let mut by_asset: BTreeMap<String, AssetDays> = BTreeMap::new();
    for event in events {
        let t = event.tax_event_type;
        if event.quantity.is_zero() || (!t.is_pool_entry() && !t.is_disposal()) {
            continue;
        }
        let day = day_of_timestamp(&event.timestamp)
            .map_err(|_| TaxError::InvalidTimestamp(event.timestamp.clone()))?;
        let asset = event.asset_symbol.to_ascii_uppercase();
        let days = by_asset.entry(asset).or_default();
        if t.is_pool_entry() {
            let entry = days
                .acquisitions
                .entry(day)
                .or_insert_with(|| DayAcquisition {
                    quantity: Decimal::ZERO,
                    cost_gbp: Decimal::ZERO,
                    remaining: Decimal::ZERO,
                    event_ids: Vec::new(),
                });
            entry.quantity += event.quantity;
            entry.cost_gbp += entry_cost(event);
            entry.remaining += event.quantity;
            entry.event_ids.push(event.ledger_event_id.clone());
        } else {
            let entry = days.disposals.entry(day).or_insert_with(|| DayDisposal {
                day,
                quantity: Decimal::ZERO,
                proceeds_gbp: Decimal::ZERO,
                remaining: Decimal::ZERO,
                event_ids: Vec::new(),
                same_day_quantity: Decimal::ZERO,
                same_day_cost: Decimal::ZERO,
                thirty_day_quantity: Decimal::ZERO,
                thirty_day_cost: Decimal::ZERO,
                pool_quantity: Decimal::ZERO,
                pool_cost: Decimal::ZERO,
                notes: Vec::new(),
            });
            entry.quantity += event.quantity;
            entry.proceeds_gbp += event.proceeds_gbp.unwrap_or_default();
            entry.remaining += event.quantity;
            entry.event_ids.push(event.ledger_event_id.clone());
        }
    }
    Ok(by_asset)
}

/// Run the full calculation for one tax year.
pub fn calculate(
    events: &[TaxLedgerEvent],
    opening_pools: &[OpeningPool],
    tax_year: TaxYear,
    allow_unpriced: bool,
) -> Result<UkTaxCalculation, TaxError> {
    let (included, unresolved) = validate(events, allow_unpriced)?;
    let mut by_asset = aggregate(&included)?;

    for days in by_asset.values_mut() {
        same_day::match_same_day(days);
        thirty_day::match_thirty_day(days);
    }
    let pool_outcome = s104_pool::walk_pools(&mut by_asset, opening_pools, tax_year)?;

    let all_disposals = disposals::build_rows(&by_asset);
    let all_income = income::build_rows(&included);

    // Filter to the requested year: pools and matching always used the full
    // timeline, only the reporting window narrows.
    let year_label = tax_year.label();
    let disposals: Vec<_> = all_disposals
        .into_iter()
        .filter(|d| d.tax_year == year_label)
        .collect();
    let income: Vec<_> = all_income
        .into_iter()
        .filter(|i| i.tax_year == year_label)
        .collect();
    let pool_movements: Vec<_> = pool_outcome
        .movements
        .into_iter()
        .filter(|m| m.tax_year == year_label)
        .collect();

    let mut summary = UkTaxSummary {
        tax_year: year_label,
        ..Default::default()
    };
    for d in &disposals {
        summary.disposal_count += 1;
        summary.total_proceeds_gbp += d.proceeds_gbp;
        summary.total_allowable_costs_gbp += d.allowable_cost_gbp;
        if d.gain_or_loss_gbp >= Decimal::ZERO {
            summary.total_gains_gbp += d.gain_or_loss_gbp;
        } else {
            summary.total_losses_gbp += -d.gain_or_loss_gbp;
        }
    }
    summary.net_gain_or_loss_gbp = summary.total_gains_gbp - summary.total_losses_gbp;
    for i in &income {
        summary.total_income_gbp += i.income_gbp;
        *summary
            .income_by_category_gbp
            .entry(i.category.clone())
            .or_default() += i.income_gbp;
    }
    summary.crypto_fees_disposed_gbp = fees::fees_disposed_in_year(&included, tax_year);
    summary.unresolved_blockers = unresolved
        .iter()
        .filter(|u| u.severity == "blocker")
        .count() as u64;
    summary.unresolved_warnings = unresolved
        .iter()
        .filter(|u| u.severity == "warning")
        .count() as u64;

    Ok(UkTaxCalculation {
        tax_year,
        disposals,
        pool_movements,
        pool_year_states: pool_outcome.year_states,
        income,
        unresolved,
        summary,
    })
}

/// Tax-year label for a day number (used when building report rows).
pub(crate) fn tax_year_label_of_day(day: i64) -> String {
    tinotax_core::uk_tax_year(&date_string(day)).unwrap_or_default()
}
