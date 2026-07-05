//! Section 104 pooling (TCGA92 s104 via CRYPTO22200): each asset has one
//! pool holding total quantity and total allowable cost. Disposal
//! remainders draw cost at the pool average; unmatched acquisition
//! remainders top the pool up. Walked chronologically over the full
//! timeline so pools carry across tax years.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use tinotax_core::date::{date_string, days_from_epoch};

use crate::domain::{OpeningPool, PoolMovement, PoolYearState, TaxError};
use crate::matching::{tax_year_label_of_day, AssetDays};
use crate::tax_year::TaxYear;

pub(crate) struct PoolWalkOutcome {
    pub movements: Vec<PoolMovement>,
    pub year_states: Vec<PoolYearState>,
}

struct Pool {
    quantity: Decimal,
    cost: Decimal,
}

/// Process every asset's pool. Fails (listing every shortfall) if any
/// disposal exceeds the pool — that means missing acquisitions or a
/// missing `opening_pools.toml` entry, and calculating around it would
/// produce nonsense.
pub(crate) fn walk_pools(
    by_asset: &mut BTreeMap<String, AssetDays>,
    opening_pools: &[OpeningPool],
    tax_year: TaxYear,
) -> Result<PoolWalkOutcome, TaxError> {
    let year_start = days_from_epoch(tax_year.start_year, 4, 6);
    let year_end = days_from_epoch(tax_year.start_year + 1, 4, 5);

    let opening_by_asset: BTreeMap<String, &OpeningPool> = opening_pools
        .iter()
        .map(|p| (p.asset.to_ascii_uppercase(), p))
        .collect();

    let mut movements = Vec::new();
    let mut year_states = Vec::new();
    let mut shortfalls: Vec<String> = Vec::new();

    for (asset, days) in by_asset.iter_mut() {
        let mut pool = Pool {
            quantity: Decimal::ZERO,
            cost: Decimal::ZERO,
        };
        if let Some(opening) = opening_by_asset.get(asset) {
            pool.quantity = opening.quantity;
            pool.cost = opening.allowable_cost_gbp;
            movements.push(PoolMovement {
                asset: asset.clone(),
                date: opening
                    .as_of
                    .get(..10)
                    .unwrap_or(&opening.as_of)
                    .to_string(),
                tax_year: tinotax_core::uk_tax_year(&opening.as_of).unwrap_or_default(),
                kind: "opening".to_string(),
                quantity_delta: opening.quantity,
                cost_delta_gbp: opening.allowable_cost_gbp,
                quantity_after: pool.quantity,
                cost_after_gbp: pool.cost,
                note: "declared in opening_pools.toml".to_string(),
            });
        }

        let mut all_days: Vec<i64> = days
            .acquisitions
            .keys()
            .chain(days.disposals.keys())
            .copied()
            .collect();
        all_days.sort_unstable();
        all_days.dedup();

        let mut opening_state: Option<(Decimal, Decimal)> = None;
        let mut closing_state: Option<(Decimal, Decimal)> = None;
        for day in all_days {
            // Capture the requested year's opening/closing state while still
            // walking the full historical timeline for correct pool carryover.
            if opening_state.is_none() && day >= year_start {
                opening_state = Some((pool.quantity, pool.cost));
            }
            if closing_state.is_none() && day > year_end {
                closing_state = Some((pool.quantity, pool.cost));
            }

            // Disposals draw from the pool as it stood before this day's
            // leftover acquisitions (same-day matching already consumed
            // what it could).
            if let Some(disposal) = days.disposals.get_mut(&day) {
                if !disposal.remaining.is_zero() {
                    if pool.quantity < disposal.remaining {
                        shortfalls.push(format!(
                            "cannot calculate {asset} disposal on {}: needs {} from the pool but only {} available — no opening {asset} pool and no prior {asset} acquisition covers it",
                            date_string(day),
                            disposal.remaining,
                            pool.quantity
                        ));
                        continue;
                    }
                    let quantity = disposal.remaining;
                    let cost = if pool.quantity.is_zero() {
                        Decimal::ZERO
                    } else {
                        pool.cost * quantity / pool.quantity
                    };
                    pool.quantity -= quantity;
                    pool.cost -= cost;
                    disposal.pool_quantity = quantity;
                    disposal.pool_cost = cost;
                    disposal.remaining = Decimal::ZERO;
                    disposal.notes.push(format!(
                        "s104: {quantity} drawn from pool at cost £{:.2}",
                        cost
                    ));
                    movements.push(PoolMovement {
                        asset: asset.clone(),
                        date: date_string(day),
                        tax_year: tax_year_label_of_day(day),
                        kind: "disposal".to_string(),
                        quantity_delta: -quantity,
                        cost_delta_gbp: -cost,
                        quantity_after: pool.quantity,
                        cost_after_gbp: pool.cost,
                        note: format!("events: {}", disposal.event_ids.join("; ")),
                    });
                }
            }

            if let Some(acquisition) = days.acquisitions.get(&day) {
                if !acquisition.remaining.is_zero() {
                    // Only the part left after same-day and 30-day matching is
                    // admitted to the pool.
                    let cost = acquisition.cost_of(acquisition.remaining);
                    pool.quantity += acquisition.remaining;
                    pool.cost += cost;
                    movements.push(PoolMovement {
                        asset: asset.clone(),
                        date: date_string(day),
                        tax_year: tax_year_label_of_day(day),
                        kind: "acquisition".to_string(),
                        quantity_delta: acquisition.remaining,
                        cost_delta_gbp: cost,
                        quantity_after: pool.quantity,
                        cost_after_gbp: pool.cost,
                        note: format!("events: {}", acquisition.event_ids.join("; ")),
                    });
                }
            }
        }

        let (opening_quantity, opening_cost) = opening_state.unwrap_or((pool.quantity, pool.cost));
        let (closing_quantity, closing_cost) = closing_state.unwrap_or((pool.quantity, pool.cost));
        year_states.push(PoolYearState {
            asset: asset.clone(),
            opening_quantity,
            opening_cost_gbp: opening_cost,
            closing_quantity,
            closing_cost_gbp: closing_cost,
        });
    }

    if !shortfalls.is_empty() {
        return Err(TaxError::InsufficientPools {
            details: shortfalls.join("\n"),
        });
    }
    Ok(PoolWalkOutcome {
        movements,
        year_states,
    })
}
