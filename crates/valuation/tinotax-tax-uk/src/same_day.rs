//! Same-day matching (TCGA92 s105 via CRYPTO22205): disposals are matched
//! first against acquisitions of the same asset on the same day, both sides
//! treated as single aggregated transactions.

use crate::matching::AssetDays;

pub(crate) fn match_same_day(days: &mut AssetDays) {
    for (day, disposal) in days.disposals.iter_mut() {
        let Some(acquisition) = days.acquisitions.get_mut(day) else {
            continue;
        };
        let quantity = disposal.remaining.min(acquisition.remaining);
        if quantity.is_zero() {
            continue;
        }
        let cost = acquisition.cost_of(quantity);
        disposal.same_day_quantity = quantity;
        disposal.same_day_cost = cost;
        disposal.remaining -= quantity;
        acquisition.remaining -= quantity;
        disposal
            .notes
            .push(format!("same-day: {quantity} matched at cost £{:.2}", cost));
    }
}
