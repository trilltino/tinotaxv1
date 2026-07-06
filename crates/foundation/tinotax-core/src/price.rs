//! Historical price observations: the raw material of the GBP price book.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::tax_event::{parse_date_prefix, PriceConfidence, PriceSource};
use crate::CoreError;

/// One observed price for one asset at (or near) one moment, in one
/// currency. Observations accumulate in `staging/price_observations.jsonl`
/// (manual imports, provider fetches) and `staging/cex_price_hints.jsonl`
/// (spot prices captured from CEX exports); the price book merges them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceObservation {
    pub asset_symbol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_contract: Option<String>,
    /// RFC 3339. Daily-granularity sources use midnight UTC.
    pub timestamp: String,
    /// ISO currency code; always `GBP` in this pipeline.
    pub currency: String,
    pub price: Decimal,
    pub source: PriceSource,
    pub confidence: PriceConfidence,
    pub fetched_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl PriceObservation {
    /// The `YYYY-MM-DD` day this observation covers.
    pub fn date(&self) -> Result<String, CoreError> {
        let (y, m, d) = parse_date_prefix(&self.timestamp)?;
        Ok(format!("{y:04}-{m:02}-{d:02}"))
    }
}
