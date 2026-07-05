//! Coinbase transaction report. Real exports carry a free-text preamble
//! before the header row, so the header is located by content, and GBP spot
//! prices are captured as price hints for the pricing stage.

use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use rust_decimal::Decimal;

use crate::record::{normalise_timestamp, parse_amount, CexRecord, CexRecordKind};

pub fn parse(path: &Utf8Path) -> Result<Vec<CexRecord>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    // Skip the preamble: the header is the first line naming both columns.
    let Some(header_offset) = text
        .lines()
        .position(|l| l.contains("Timestamp") && l.contains("Transaction Type"))
    else {
        bail!("{path}: no Coinbase header row (needs `Timestamp` and `Transaction Type` columns)");
    };
    let body: String = text
        .lines()
        .skip(header_offset)
        .collect::<Vec<_>>()
        .join("\n");

    let mut reader = csv::Reader::from_reader(body.as_bytes());
    let headers = reader.headers()?.clone();
    let col = |name: &str| headers.iter().position(|h| h.trim() == name);
    let col_req =
        |name: &str| col(name).with_context(|| format!("{path}: missing column {name:?}"));
    let time_col = col_req("Timestamp")?;
    let type_col = col_req("Transaction Type")?;
    let asset_col = col_req("Asset")?;
    let quantity_col = col_req("Quantity Transacted")?;
    let spot_currency_col = col("Spot Price Currency");
    let spot_price_col = col("Spot Price at Transaction");
    let fees_col = col("Fees and/or Spread");
    let notes_col = col("Notes");

    let mut records = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let row = (header_offset + i + 2) as u64;
        let get = |c: usize| record.get(c).unwrap_or("").trim();
        let opt = |c: Option<usize>| c.map(get).filter(|s| !s.is_empty());

        let tx_type = get(type_col);
        let quantity = parse_amount(get(quantity_col))
            .with_context(|| format!("{path} row {row}"))?
            .abs();
        // Coinbase quantities are unsigned; the type carries the direction.
        let outgoing = matches!(
            tx_type.to_ascii_lowercase().as_str(),
            "sell" | "send" | "convert" | "advanced trade sell" | "withdrawal"
        );
        let amount = if outgoing { -quantity } else { quantity };

        let kind = match tx_type.to_ascii_lowercase().as_str() {
            "buy" | "sell" | "convert" | "advanced trade buy" | "advanced trade sell" => {
                CexRecordKind::Trade
            }
            "send" | "withdrawal" => CexRecordKind::Withdrawal,
            "receive" | "deposit" => CexRecordKind::Deposit,
            t if t.contains("income") || t.contains("reward") || t.contains("earn") => {
                CexRecordKind::Reward
            }
            _ => CexRecordKind::Other,
        };

        // GBP valuations from the export become price hints; a GBP fee is
        // surfaced in the note for the reviewer (fees here are fiat, not a
        // separate crypto movement).
        let gbp_spot = matches!(opt(spot_currency_col), Some(c) if c.eq_ignore_ascii_case("GBP"));
        let price_gbp: Option<Decimal> = if gbp_spot {
            opt(spot_price_col).and_then(|p| parse_amount(p).ok())
        } else {
            None
        };
        let mut note = format!("coinbase {tx_type}");
        if let Some(notes) = opt(notes_col) {
            note = format!("{note}: {notes}");
        }
        if gbp_spot {
            if let Some(fees) = opt(fees_col) {
                note = format!("{note} (fees/spread £{fees})");
            }
        }

        records.push(CexRecord {
            row,
            timestamp: normalise_timestamp(get(time_col))
                .with_context(|| format!("{path} row {row}"))?,
            kind,
            asset: get(asset_col).to_ascii_uppercase(),
            amount,
            fee_asset: None,
            fee_amount: None,
            note: Some(note),
            price_gbp,
        });
    }
    Ok(records)
}
