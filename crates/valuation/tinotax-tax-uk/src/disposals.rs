//! Assemble per-disposal calculation rows from the matched day aggregates.

use std::collections::BTreeMap;

use tinotax_core::date::date_string;

use crate::domain::DisposalCalculation;
use crate::matching::{tax_year_label_of_day, AssetDays};

pub(crate) fn build_rows(by_asset: &BTreeMap<String, AssetDays>) -> Vec<DisposalCalculation> {
    let mut rows = Vec::new();
    for (asset, days) in by_asset {
        for disposal in days.disposals.values() {
            let date = date_string(disposal.day);
            let allowable = disposal.same_day_cost + disposal.thirty_day_cost + disposal.pool_cost;
            rows.push(DisposalCalculation {
                disposal_id: format!("disp_{asset}_{date}"),
                asset: asset.clone(),
                date: date.clone(),
                tax_year: tax_year_label_of_day(disposal.day),
                quantity: disposal.quantity,
                proceeds_gbp: disposal.proceeds_gbp,
                matched_same_day_quantity: disposal.same_day_quantity,
                matched_same_day_cost_gbp: disposal.same_day_cost,
                matched_30_day_quantity: disposal.thirty_day_quantity,
                matched_30_day_cost_gbp: disposal.thirty_day_cost,
                matched_s104_quantity: disposal.pool_quantity,
                matched_s104_cost_gbp: disposal.pool_cost,
                allowable_cost_gbp: allowable,
                gain_or_loss_gbp: disposal.proceeds_gbp - allowable,
                source_ledger_event_ids: disposal.event_ids.clone(),
                matching_notes: disposal.notes.clone(),
            });
        }
    }
    rows.sort_by(|a, b| {
        (a.date.as_str(), a.asset.as_str()).cmp(&(b.date.as_str(), b.asset.as_str()))
    });
    rows
}
