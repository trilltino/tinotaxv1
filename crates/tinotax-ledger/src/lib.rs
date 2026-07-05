//! The reviewed tax ledger.
//!
//! `build_ledger` derives `staging/reviewed_ledger.jsonl` (and
//! `out/reviewed_ledger.csv`) from the merged normalised events plus the
//! latest human override per event. It is a pure re-derivation: run it as
//! often as you like, nothing upstream is ever mutated.

pub mod build;
pub mod csv_export;

pub use build::{build_ledger, load_reviewed_ledger, LedgerSummary};
pub use csv_export::export_ledger_csv;
