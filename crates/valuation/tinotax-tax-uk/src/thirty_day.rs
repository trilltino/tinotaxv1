//! 30-day "bed & breakfast" matching (TCGA92 s106A via CRYPTO22210):
//! what remains of a disposal after same-day matching is matched against
//! acquisitions in the 30 days *following* it — earliest disposal claims
//! first, acquisitions taken in date order. Matched acquisitions never
//! reach the Section 104 pool.

use tinotax_core::date::date_string;

use crate::matching::AssetDays;

pub(crate) fn match_thirty_day(days: &mut AssetDays) {
    let acquisition_days: Vec<i64> = days.acquisitions.keys().copied().collect();
    for (day, disposal) in days.disposals.iter_mut() {
        if disposal.remaining.is_zero() {
            continue;
        }
        for acquisition_day in acquisition_days
            .iter()
            .copied()
            .filter(|a| *a > *day && *a <= day + 30)
        {
            let Some(acquisition) = days.acquisitions.get_mut(&acquisition_day) else {
                continue;
            };
            if acquisition.remaining.is_zero() {
                continue;
            }
            let quantity = disposal.remaining.min(acquisition.remaining);
            let cost = acquisition.cost_of(quantity);
            disposal.thirty_day_quantity += quantity;
            disposal.thirty_day_cost += cost;
            disposal.remaining -= quantity;
            acquisition.remaining -= quantity;
            disposal.notes.push(format!(
                "30-day: {quantity} matched to acquisition on {} at cost £{:.2}",
                date_string(acquisition_day),
                cost
            ));
            if disposal.remaining.is_zero() {
                break;
            }
        }
    }
}
