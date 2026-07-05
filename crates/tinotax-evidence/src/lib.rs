//! HMRC / Self Assessment evidence pack generator.
//!
//! `pack hmrc` assembles `evidence_pack/<year>/`: the tax calculation CSVs,
//! provenance (raw data index, hashes, pricing audit, review change log),
//! the HMRC questions draft, and the human questionnaire — one folder that
//! can be handed to the client or their accountant.

pub mod assumptions;
pub mod hmrc_questions;
pub mod markdown;
pub mod pack;
pub mod platforms;
pub mod raw_index;

pub use pack::build_pack;
