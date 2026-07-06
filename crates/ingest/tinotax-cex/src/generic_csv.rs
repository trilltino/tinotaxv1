//! Generic single-row-per-movement mapper driven by a user-supplied column
//! mapping. `amount` must be signed (positive in, negative out).

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use camino::Utf8Path;

use crate::column_mapping::ColumnMap;
use crate::record::{normalise_timestamp, parse_amount, CexRecord, CexRecordKind};

pub fn parse(path: &Utf8Path, mapping: &BTreeMap<String, String>) -> Result<Vec<CexRecord>> {
    let mut reader = csv::Reader::from_path(path).with_context(|| format!("opening {path}"))?;
    let headers = reader.headers()?.clone();
    let map = ColumnMap::resolve(&headers, mapping).with_context(|| format!("{path}"))?;

    let mut records = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let row = (i + 2) as u64;
        let get = |c: usize| record.get(c).unwrap_or("").trim();
        let opt = |c: Option<usize>| c.map(get).filter(|s| !s.is_empty());

        let type_text = opt(map.r#type).unwrap_or("");
        let fee_amount = opt(map.fee_amount)
            .map(parse_amount)
            .transpose()
            .with_context(|| format!("{path} row {row}"))?
            .filter(|f| !f.is_zero())
            .map(|f| f.abs());
        records.push(CexRecord {
            row,
            timestamp: normalise_timestamp(get(map.timestamp))
                .with_context(|| format!("{path} row {row}"))?,
            kind: if type_text.is_empty() {
                CexRecordKind::Other
            } else {
                CexRecordKind::from_operation(type_text)
            },
            asset: get(map.asset).to_ascii_uppercase(),
            amount: parse_amount(get(map.amount)).with_context(|| format!("{path} row {row}"))?,
            fee_asset: opt(map.fee_asset)
                .map(str::to_ascii_uppercase)
                .or_else(|| fee_amount.map(|_| get(map.asset).to_ascii_uppercase())),
            fee_amount,
            note: opt(map.note)
                .map(str::to_string)
                .or_else(|| (!type_text.is_empty()).then(|| type_text.to_string())),
            price_gbp: None,
        });
    }
    Ok(records)
}
