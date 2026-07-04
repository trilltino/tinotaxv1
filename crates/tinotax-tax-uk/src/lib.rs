//! UK CGT engine — **milestone 2, deliberately not implemented yet**.
//!
//! Architecture principle 4: do not guess tax treatment too early. Milestone
//! 1 only ingests, normalises, and routes uncertainty to human review. This
//! crate exists so the workspace shape doesn't change when the real work
//! starts:
//!
//! - `tax_year`   — UK tax year (6 April – 5 April) boundaries
//! - `matching`   — same-day, then 30-day "bed & breakfast", then pool
//! - `s104_pool`  — Section 104 average-cost pooling per asset
//! - `income`     — income classification (staking, airdrops, ...)
//! - `reports`    — CGT computations / HMRC evidence pack
//!
//! Nothing here is called by the milestone-1 pipeline.

pub mod income;
pub mod matching;
pub mod reports;
pub mod s104_pool;
pub mod tax_year;
