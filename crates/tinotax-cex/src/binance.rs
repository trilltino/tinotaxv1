//! Binance "Transaction History" export:
//! `User_ID,UTC_Time,Account,Operation,Coin,Change,Remark`
//! One row per asset movement; `Change` is signed.

use anyhow::{Context, Result};
use camino::Utf8Path;

use crate::record::{normalise_timestamp, parse_amount, CexRecord, CexRecordKind};

pub fn parse(path: &Utf8Path) -> Result<Vec<CexRecord>> {
    let mut reader = csv::Reader::from_path(path).with_context(|| format!("opening {path}"))?;
    let headers = reader.headers()?.clone();
    let col = |name: &str| {
        headers
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case(name))
            .with_context(|| format!("{path}: missing column {name:?}"))
    };
    let time_col = col("UTC_Time")?;
    let operation_col = col("Operation")?;
    let coin_col = col("Coin")?;
    let change_col = col("Change")?;
    let remark_col = headers.iter().position(|h| h.trim() == "Remark");

    let mut records = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let row = (i + 2) as u64;
        let get = |c: usize| record.get(c).unwrap_or("").trim();
        let operation = get(operation_col);
        let amount = parse_amount(get(change_col)).with_context(|| format!("{path} row {row}"))?;
        records.push(CexRecord {
            row,
            timestamp: normalise_timestamp(get(time_col))
                .with_context(|| format!("{path} row {row}"))?,
            kind: CexRecordKind::from_operation(operation),
            asset: get(coin_col).to_ascii_uppercase(),
            amount,
            fee_asset: None,
            fee_amount: None,
            note: {
                let remark = remark_col.map(get).unwrap_or("");
                if remark.is_empty() {
                    Some(operation.to_string())
                } else {
                    Some(format!("{operation}: {remark}"))
                }
            },
            price_gbp: None,
        });
    }
    Ok(records)
}
