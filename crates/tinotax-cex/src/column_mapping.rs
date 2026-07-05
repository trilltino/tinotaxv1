//! Column mapping for `platform = "generic"` imports: the project config
//! maps canonical column names onto whatever headers the CSV actually has.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};

/// Canonical columns the generic mapper understands. `timestamp`, `asset`
/// and `amount` are required; the rest are optional.
pub const CANONICAL_COLUMNS: [&str; 7] = [
    "timestamp",
    "type",
    "asset",
    "amount",
    "fee_asset",
    "fee_amount",
    "note",
];

/// Resolved column indices for one CSV.
#[derive(Debug, Clone)]
pub struct ColumnMap {
    pub timestamp: usize,
    pub r#type: Option<usize>,
    pub asset: usize,
    pub amount: usize,
    pub fee_asset: Option<usize>,
    pub fee_amount: Option<usize>,
    pub note: Option<usize>,
}

impl ColumnMap {
    /// `mapping` is canonical-name → actual-header from the project config.
    pub fn resolve(
        headers: &csv::StringRecord,
        mapping: &BTreeMap<String, String>,
    ) -> Result<Self> {
        for key in mapping.keys() {
            if !CANONICAL_COLUMNS.contains(&key.as_str()) {
                bail!(
                    "unknown canonical column {key:?} in [cex_csvs.mapping] — expected one of {CANONICAL_COLUMNS:?}"
                );
            }
        }
        let find = |canonical: &str| -> Result<Option<usize>> {
            let Some(actual) = mapping.get(canonical) else {
                return Ok(None);
            };
            let idx = headers
                .iter()
                .position(|h| h.trim() == actual.trim())
                .with_context(|| {
                    format!("mapped column {actual:?} (for {canonical}) not found in CSV header")
                })?;
            Ok(Some(idx))
        };
        let required = |canonical: &str| -> Result<usize> {
            find(canonical)?.with_context(|| {
                format!("[cex_csvs.mapping] must map the required column {canonical:?}")
            })
        };
        Ok(Self {
            timestamp: required("timestamp")?,
            r#type: find("type")?,
            asset: required("asset")?,
            amount: required("amount")?,
            fee_asset: find("fee_asset")?,
            fee_amount: find("fee_amount")?,
            note: find("note")?,
        })
    }
}
