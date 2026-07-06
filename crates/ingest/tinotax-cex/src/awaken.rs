//! Awaken.tax-style "universal" export: one row per transaction with
//! sent/received/fee columns. Header names are matched loosely because
//! aggregator exports vary in capitalisation and spacing.

use anyhow::{bail, Context, Result};
use camino::Utf8Path;

use crate::record::{normalise_timestamp, parse_amount, CexRecord, CexRecordKind};

fn find_col(headers: &csv::StringRecord, wanted: &str) -> Option<usize> {
    let wanted = wanted.to_ascii_lowercase();
    headers
        .iter()
        .position(|h| h.trim().to_ascii_lowercase().replace(' ', "_") == wanted)
}

pub fn parse(path: &Utf8Path) -> Result<Vec<CexRecord>> {
    let mut reader = csv::Reader::from_path(path).with_context(|| format!("opening {path}"))?;
    let headers = reader.headers()?.clone();

    let date_col = find_col(&headers, "date")
        .or_else(|| find_col(&headers, "timestamp"))
        .with_context(|| format!("{path}: missing Date/Timestamp column"))?;
    let type_col = find_col(&headers, "type");
    let sent_qty =
        find_col(&headers, "sent_quantity").or_else(|| find_col(&headers, "sent_amount"));
    let sent_cur = find_col(&headers, "sent_currency").or_else(|| find_col(&headers, "sent_asset"));
    let recv_qty =
        find_col(&headers, "received_quantity").or_else(|| find_col(&headers, "received_amount"));
    let recv_cur =
        find_col(&headers, "received_currency").or_else(|| find_col(&headers, "received_asset"));
    let fee_qty = find_col(&headers, "fee_amount").or_else(|| find_col(&headers, "fee_quantity"));
    let fee_cur = find_col(&headers, "fee_currency").or_else(|| find_col(&headers, "fee_asset"));
    let note_col = find_col(&headers, "description").or_else(|| find_col(&headers, "notes"));
    if sent_qty.is_none() && recv_qty.is_none() {
        bail!("{path}: expected Sent Quantity and/or Received Quantity columns");
    }

    let mut records = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let row = (i + 2) as u64;
        let cell = |c: Option<usize>| {
            c.and_then(|c| record.get(c))
                .map(str::trim)
                .filter(|s| !s.is_empty())
        };

        let timestamp = normalise_timestamp(record.get(date_col).unwrap_or("").trim())
            .with_context(|| format!("{path} row {row}"))?;
        let type_text = cell(type_col).unwrap_or("");
        let base_kind = match type_text.to_ascii_lowercase().as_str() {
            "" => None,
            t => Some(CexRecordKind::from_operation(t)),
        };
        let note = {
            let d = cell(note_col).unwrap_or("");
            let label = if type_text.is_empty() {
                "row"
            } else {
                type_text
            };
            Some(if d.is_empty() {
                format!("awaken {label}")
            } else {
                format!("awaken {label}: {d}")
            })
        };
        let fee_amount = cell(fee_qty)
            .map(parse_amount)
            .transpose()
            .with_context(|| format!("{path} row {row}"))?
            .filter(|f| !f.is_zero())
            .map(|f| f.abs());
        let fee_asset = fee_amount
            .is_some()
            .then(|| cell(fee_cur).unwrap_or("").to_ascii_uppercase())
            .filter(|s| !s.is_empty());

        let sent = match (cell(sent_qty), cell(sent_cur)) {
            (Some(q), Some(cur)) => Some((parse_amount(q)?, cur.to_ascii_uppercase())),
            _ => None,
        };
        let received = match (cell(recv_qty), cell(recv_cur)) {
            (Some(q), Some(cur)) => Some((parse_amount(q)?, cur.to_ascii_uppercase())),
            _ => None,
        };
        // Both legs present = a trade regardless of the type label.
        let both = sent.is_some() && received.is_some();

        if let Some((qty, asset)) = sent {
            records.push(CexRecord {
                row,
                timestamp: timestamp.clone(),
                kind: if both {
                    CexRecordKind::Trade
                } else {
                    base_kind.unwrap_or(CexRecordKind::Withdrawal)
                },
                asset,
                amount: -qty.abs(),
                fee_asset: fee_asset.clone(),
                fee_amount,
                note: note.clone(),
                price_gbp: None,
            });
        }
        if let Some((qty, asset)) = received {
            records.push(CexRecord {
                row,
                timestamp,
                kind: if both {
                    CexRecordKind::Trade
                } else {
                    base_kind.unwrap_or(CexRecordKind::Deposit)
                },
                asset,
                amount: qty.abs(),
                // Fee attaches to the sent leg when both exist.
                fee_asset: if both { None } else { fee_asset },
                fee_amount: if both { None } else { fee_amount },
                note,
                price_gbp: None,
            });
        }
    }
    Ok(records)
}
