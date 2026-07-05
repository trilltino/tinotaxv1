//! Historical GBP pricing.
//!
//! Observations (manual imports, CEX price hints, provider fetches) build a
//! per-day price book; `ledger price` values every taxable ledger row in
//! GBP or marks it `missing`. Tax calculation refuses to run on missing
//! values unless `--allow-unpriced` is passed.

pub mod audit;
pub mod manual_import;
pub mod missing;
pub mod price_book;
pub mod provider;
pub mod valuation;

pub use manual_import::{import_manual_prices, merge_observations};
pub use missing::{export_missing_prices, missing_prices};
pub use price_book::PriceBook;
pub use provider::fetch_missing_prices;
pub use valuation::{load_priced_ledger, price_ledger, PricingSummary};

/// Merge identity for stored observations: one row per (asset, day, source).
pub(crate) fn record_merge_key(
    obs: &tinotax_core::PriceObservation,
) -> anyhow::Result<(String, String, String)> {
    Ok((
        obs.asset_symbol.to_ascii_uppercase(),
        obs.date()?,
        obs.source.as_str().to_string(),
    ))
}
