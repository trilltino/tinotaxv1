//! Shared domain types for the TinoTax workspace.
//!
//! This crate owns stable records and helpers used across ingestion, review,
//! ledger, pricing, tax, and evidence crates. It deliberately avoids filesystem
//! and network IO.
pub mod amount;
pub mod asset;
pub mod chain;
pub mod date;
pub mod error;
pub mod event;
pub mod price;
pub mod review;
pub mod source;
pub mod tax_event;

pub use amount::ScaledAmount;
pub use asset::Asset;
pub use chain::Chain;
pub use error::CoreError;
pub use event::{Confidence, Direction, EventType, NormalisedEvent, SourceKind, SourceRef};
pub use price::PriceObservation;
pub use review::{ReviewAction, ReviewOverride, ReviewRow};
pub use source::WalletSource;
pub use tax_event::{
    parse_date_prefix, uk_tax_year, PriceConfidence, PriceSource, ReviewStatus, TaxEventType,
    TaxLedgerEvent,
};
