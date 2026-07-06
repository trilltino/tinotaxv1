//! Human review of the normalised data.
//!
//! Two exports (all rows / uncertain rows only), one apply path. Human
//! decisions are recorded append-only to `staging/review_overrides.jsonl`;
//! raw and normalised data are never mutated.

pub mod apply_review;
pub mod export_all;
pub mod export_review;
pub mod load;

pub use apply_review::{apply_review, write_change_log};
pub use export_all::export_review_all;
pub use export_review::export_review;
pub use load::{load_all_events, load_latest_overrides, load_override_history};
