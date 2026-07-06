//! CEX CSV ingestion.
//!
//! Each configured export is copied **unedited** into `raw/cex/<id>/` and
//! hashed (HMRC asks for full, unedited trading data files), then parsed by
//! a platform-specific mapper into the same `NormalisedEvent` shape wallet
//! data uses, landing in `staging/cex_normalised_events.jsonl`.
//!
//! Where an export carries GBP spot prices (e.g. Coinbase), they are kept
//! as price hints for the pricing stage — never discarded.

pub mod awaken;
pub mod binance;
pub mod coinbase;
pub mod column_mapping;
pub mod generic_csv;
pub mod importer;
pub mod kraken;
pub mod record;
pub mod report;

pub use importer::import_all;
pub use record::{CexRecord, CexRecordKind};
pub use report::ImportReport;
