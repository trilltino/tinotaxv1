//! Kraken "Ledgers" export:
//! `txid,refid,time,type,subtype,aclass,asset,wallet,amount,fee,balance`
//! Amounts are signed; fees share the row's asset. Kraken's legacy asset
//! codes (XXBT, ZGBP, …) are translated to plain symbols.

use anyhow::{Context, Result};
use camino::Utf8Path;
use rust_decimal::Decimal;

use crate::record::{normalise_timestamp, parse_amount, CexRecord, CexRecordKind};

/// Kraken legacy asset code → conventional symbol.
pub fn normalise_asset(code: &str) -> String {
    let code = code.trim().to_ascii_uppercase();
    // Staking variants like `ETH2.S` / `DOT.S` are the same economic asset.
    let code = code.split('.').next().unwrap_or(&code).to_string();
    match code.as_str() {
        "XXBT" | "XBT" => "BTC".to_string(),
        "XETH" => "ETH".to_string(),
        "XXRP" => "XRP".to_string(),
        "XLTC" => "LTC".to_string(),
        "XXLM" => "XLM".to_string(),
        "XXMR" => "XMR".to_string(),
        "XZEC" => "ZEC".to_string(),
        "XETC" => "ETC".to_string(),
        "ZGBP" => "GBP".to_string(),
        "ZUSD" => "USD".to_string(),
        "ZEUR" => "EUR".to_string(),
        "ZJPY" => "JPY".to_string(),
        "ZCAD" => "CAD".to_string(),
        "ZAUD" => "AUD".to_string(),
        other => other.to_string(),
    }
}

pub fn parse(path: &Utf8Path) -> Result<Vec<CexRecord>> {
    let mut reader = csv::Reader::from_path(path).with_context(|| format!("opening {path}"))?;
    let headers = reader.headers()?.clone();
    let col = |name: &str| {
        headers
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case(name))
            .with_context(|| format!("{path}: missing column {name:?}"))
    };
    let time_col = col("time")?;
    let type_col = col("type")?;
    let asset_col = col("asset")?;
    let amount_col = col("amount")?;
    let fee_col = headers.iter().position(|h| h.trim() == "fee");
    let subtype_col = headers.iter().position(|h| h.trim() == "subtype");
    let refid_col = headers.iter().position(|h| h.trim() == "refid");

    let mut records = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let row = (i + 2) as u64;
        let get = |c: usize| record.get(c).unwrap_or("").trim();

        let ledger_type = get(type_col).to_ascii_lowercase();
        let kind = match ledger_type.as_str() {
            "trade" | "spend" | "receive" => CexRecordKind::Trade,
            "deposit" => CexRecordKind::Deposit,
            "withdrawal" => CexRecordKind::Withdrawal,
            "staking" | "earn" | "dividend" => CexRecordKind::Reward,
            _ => CexRecordKind::Other, // transfer, margin, rollover, adjustment, …
        };
        let amount = parse_amount(get(amount_col)).with_context(|| format!("{path} row {row}"))?;
        let fee_amount: Option<Decimal> = fee_col
            .map(|c| parse_amount(get(c)))
            .transpose()
            .with_context(|| format!("{path} row {row}"))?
            .filter(|f| !f.is_zero())
            .map(|f| f.abs());
        let asset = normalise_asset(get(asset_col));

        let mut note = format!("kraken {ledger_type}");
        if let Some(subtype) = subtype_col.map(get).filter(|s| !s.is_empty()) {
            note = format!("{note}/{subtype}");
        }
        if let Some(refid) = refid_col.map(get).filter(|s| !s.is_empty()) {
            note = format!("{note} ref {refid}");
        }

        records.push(CexRecord {
            row,
            timestamp: normalise_timestamp(get(time_col))
                .with_context(|| format!("{path} row {row}"))?,
            kind,
            asset: asset.clone(),
            amount,
            fee_asset: fee_amount.is_some().then_some(asset),
            fee_amount,
            note: Some(note),
            price_gbp: None,
        });
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kraken_asset_codes_normalise() {
        assert_eq!(normalise_asset("XXBT"), "BTC");
        assert_eq!(normalise_asset("ZGBP"), "GBP");
        assert_eq!(normalise_asset("DOT.S"), "DOT");
        assert_eq!(normalise_asset("SOL"), "SOL");
    }
}
