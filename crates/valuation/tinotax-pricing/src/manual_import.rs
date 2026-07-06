//! `prices import`: a hand-maintained CSV of daily GBP prices →
//! `staging/price_observations.jsonl`.
//!
//! Expected columns: `asset_symbol` (or `asset`), `date` (or `timestamp`),
//! `price_gbp` (or `price`), and optionally `source` and `note`. Imports
//! merge by (asset, day, source): re-importing a corrected file replaces
//! the old numbers instead of duplicating them.

use std::collections::BTreeMap;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use rust_decimal::Decimal;
use tinotax_core::{PriceConfidence, PriceObservation, PriceSource};
use tinotax_store::{read_jsonl, JsonlWriter, ProjectPaths};

use crate::record_merge_key;

/// Returns the number of observations imported from `file`.
pub fn import_manual_prices(paths: &ProjectPaths, file: &Utf8Path) -> Result<u64> {
    let mut reader = csv::Reader::from_path(file).with_context(|| format!("opening {file}"))?;
    let headers = reader.headers()?.clone();
    let col = |names: &[&str]| {
        headers
            .iter()
            .position(|h| names.contains(&h.trim().to_ascii_lowercase().as_str()))
    };
    let Some(asset_col) = col(&["asset_symbol", "asset"]) else {
        bail!("{file} needs an `asset_symbol` column");
    };
    let Some(date_col) = col(&["date", "timestamp"]) else {
        bail!("{file} needs a `date` (or `timestamp`) column");
    };
    let Some(price_col) = col(&["price_gbp", "price"]) else {
        bail!("{file} needs a `price_gbp` column");
    };
    let source_col = col(&["source"]);
    let note_col = col(&["note"]);

    let mut imported = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let row = i + 2;
        let get = |c: usize| record.get(c).unwrap_or("").trim();
        let asset = get(asset_col).to_ascii_uppercase();
        let date_text = get(date_col);
        if asset.is_empty() && date_text.is_empty() {
            continue; // blank line
        }
        if asset.is_empty() || date_text.is_empty() {
            bail!("{file} row {row}: asset and date are both required");
        }
        let price = Decimal::from_str(get(price_col))
            .with_context(|| format!("{file} row {row}: invalid price {:?}", get(price_col)))?;
        if price <= Decimal::ZERO {
            bail!("{file} row {row}: price must be > 0");
        }
        let timestamp = if date_text.contains('T') {
            date_text.to_string()
        } else {
            format!("{date_text}T00:00:00Z")
        };
        tinotax_core::parse_date_prefix(&timestamp)
            .map_err(|_| anyhow::anyhow!("{file} row {row}: invalid date {date_text:?}"))?;
        let source = match source_col.map(get).filter(|s| !s.is_empty()) {
            Some(text) => PriceSource::from_str(text)
                .with_context(|| format!("{file} row {row}: invalid source {text:?}"))?,
            None => PriceSource::Manual,
        };
        imported.push(PriceObservation {
            asset_symbol: asset,
            asset_contract: None,
            timestamp,
            currency: "GBP".to_string(),
            price,
            source,
            confidence: PriceConfidence::High,
            fetched_at: tinotax_store::now_rfc3339(),
            note: note_col
                .map(get)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
        });
    }

    let count = imported.len() as u64;
    merge_observations(paths, imported)?;
    Ok(count)
}

/// Merge new observations into `price_observations.jsonl`, replacing any
/// previous observation with the same (asset, day, source).
pub fn merge_observations(
    paths: &ProjectPaths,
    new_observations: Vec<PriceObservation>,
) -> Result<()> {
    let path = paths.price_observations_jsonl();
    let mut merged: BTreeMap<(String, String, String), PriceObservation> = BTreeMap::new();
    if path.exists() {
        for obs in read_jsonl::<PriceObservation>(&path)? {
            merged.insert(record_merge_key(&obs)?, obs);
        }
    }
    for obs in new_observations {
        merged.insert(record_merge_key(&obs)?, obs);
    }
    std::fs::create_dir_all(paths.staging())?;
    let mut writer = JsonlWriter::create(&path)?;
    for obs in merged.values() {
        writer.write(obs)?;
    }
    writer.finish()?;
    Ok(())
}
