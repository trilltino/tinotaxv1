//! Fee events are disposals of the fee asset at market value (paying a
//! network fee in crypto disposes of that crypto). They flow through the
//! normal matching passes; this module only totals them for the summary so
//! the reviewer can see the aggregate cost of fees at a glance.

use rust_decimal::Decimal;
use tinotax_core::{TaxEventType, TaxLedgerEvent};

use crate::tax_year::TaxYear;

pub(crate) fn fees_disposed_in_year(events: &[TaxLedgerEvent], year: TaxYear) -> Decimal {
    events
        .iter()
        .filter(|e| e.tax_event_type == TaxEventType::Fee && year.contains_timestamp(&e.timestamp))
        .filter_map(|e| e.proceeds_gbp.or(e.fee_gbp))
        .sum()
}
