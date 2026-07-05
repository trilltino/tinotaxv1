//! The platform-neutral intermediate every CEX mapper produces.

use anyhow::{bail, Result};
use rust_decimal::Decimal;

/// One asset movement parsed from one CSV row. A trade row that names both
/// legs (e.g. Awaken's sent/received columns) produces two records.
#[derive(Debug, Clone)]
pub struct CexRecord {
    /// 1-based data row in the original CSV (header = row 1).
    pub row: u64,
    /// RFC 3339.
    pub timestamp: String,
    pub kind: CexRecordKind,
    pub asset: String,
    /// Signed: positive into the account, negative out of it.
    pub amount: Decimal,
    pub fee_asset: Option<String>,
    /// Always positive when present.
    pub fee_amount: Option<Decimal>,
    pub note: Option<String>,
    /// GBP spot price for `asset` at `timestamp`, when the export states it.
    pub price_gbp: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CexRecordKind {
    /// One leg of an exchange trade (crypto↔crypto or crypto↔fiat).
    Trade,
    Deposit,
    Withdrawal,
    Fee,
    /// Staking/earn/interest reward paid by the exchange.
    Reward,
    Airdrop,
    /// Unrecognised operation — imported, flagged for review.
    Other,
}

impl CexRecordKind {
    /// Keyword classifier shared by the mappers that only give a free-text
    /// operation name (Binance, generic).
    pub fn from_operation(op: &str) -> Self {
        let op = op.to_ascii_lowercase();
        let has = |needle: &str| op.contains(needle);
        if has("fee") && !has("fee shared") {
            Self::Fee
        } else if has("deposit") {
            Self::Deposit
        } else if has("withdraw") {
            Self::Withdrawal
        } else if has("airdrop") || has("distribution") {
            Self::Airdrop
        } else if has("reward") || has("interest") || has("earn") || has("staking") || has("mining")
        {
            Self::Reward
        } else if has("buy")
            || has("sell")
            || has("trade")
            || has("convert")
            || has("exchange")
            || has("transaction related")
            || has("spend")
            || has("receive")
        {
            Self::Trade
        } else {
            Self::Other
        }
    }
}

/// Fiat currencies never become ledger events themselves (the crypto legs
/// carry the tax story); GBP legs do feed the price hints.
pub fn is_fiat(symbol: &str) -> bool {
    matches!(
        symbol.to_ascii_uppercase().as_str(),
        "GBP" | "USD" | "EUR" | "CHF" | "JPY" | "AUD" | "CAD"
    )
}

/// Parse a CSV amount cell: strips thousands separators and a leading `+`.
pub fn parse_amount(text: &str) -> Result<Decimal> {
    let cleaned = text.trim().replace(',', "");
    let cleaned = cleaned.strip_prefix('+').unwrap_or(&cleaned);
    match Decimal::from_str_exact(cleaned) {
        Ok(d) => Ok(d),
        Err(_) => bail!("invalid amount {text:?}"),
    }
}

/// Normalise the timestamp formats CEX exports actually use into RFC 3339.
/// Accepts `2021-01-02T03:04:05Z`, `2021-01-02 03:04:05[.frac]` (assumed
/// UTC), bare dates, and US-style `MM/DD/YYYY HH:MM[:SS]`.
pub fn normalise_timestamp(text: &str) -> Result<String> {
    let t = text.trim();
    if t.is_empty() {
        bail!("empty timestamp");
    }
    let candidate = if t.contains('/') {
        // MM/DD/YYYY [HH:MM[:SS]]
        let (date_part, time_part) = match t.split_once(' ') {
            Some((d, tm)) => (d, Some(tm)),
            None => (t, None),
        };
        let mut mdy = date_part.split('/');
        let (Some(m), Some(d), Some(y)) = (mdy.next(), mdy.next(), mdy.next()) else {
            bail!("unrecognised date {text:?}");
        };
        let time = match time_part {
            Some(tm) if tm.matches(':').count() == 1 => format!("{tm}:00"),
            Some(tm) => tm.to_string(),
            None => "00:00:00".to_string(),
        };
        format!("{y:0>4}-{m:0>2}-{d:0>2}T{time}Z")
    } else if let Some((date, time)) = t.split_once(' ') {
        format!("{date}T{time}Z")
    } else if t.contains('T') {
        let has_offset = t.get(10..).is_some_and(|rest| rest.contains('-'));
        if t.ends_with('Z') || t.contains('+') || has_offset {
            t.to_string()
        } else {
            format!("{t}Z")
        }
    } else {
        format!("{t}T00:00:00Z")
    };
    // Cheap structural validation; full RFC 3339 parsing is not needed for
    // ordering and tax-year assignment.
    tinotax_core::parse_date_prefix(&candidate)
        .map_err(|_| anyhow::anyhow!("unrecognised timestamp {text:?}"))?;
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_normalise() {
        assert_eq!(
            normalise_timestamp("2021-01-02 03:04:05").unwrap(),
            "2021-01-02T03:04:05Z"
        );
        assert_eq!(
            normalise_timestamp("2021-01-02 03:04:05.1234").unwrap(),
            "2021-01-02T03:04:05.1234Z"
        );
        assert_eq!(
            normalise_timestamp("2021-01-02T03:04:05Z").unwrap(),
            "2021-01-02T03:04:05Z"
        );
        assert_eq!(
            normalise_timestamp("1/2/2021 03:04").unwrap(),
            "2021-01-02T03:04:00Z"
        );
        assert_eq!(
            normalise_timestamp("2021-01-02").unwrap(),
            "2021-01-02T00:00:00Z"
        );
        assert!(normalise_timestamp("garbage").is_err());
    }

    #[test]
    fn amounts_parse() {
        assert_eq!(parse_amount("1,234.56").unwrap().to_string(), "1234.56");
        assert_eq!(parse_amount("+0.5").unwrap().to_string(), "0.5");
        assert_eq!(parse_amount("-2").unwrap().to_string(), "-2");
        assert!(parse_amount("abc").is_err());
    }

    #[test]
    fn operations_classify() {
        assert_eq!(
            CexRecordKind::from_operation("Deposit"),
            CexRecordKind::Deposit
        );
        assert_eq!(
            CexRecordKind::from_operation("Transaction Related"),
            CexRecordKind::Trade
        );
        assert_eq!(
            CexRecordKind::from_operation("Staking Rewards"),
            CexRecordKind::Reward
        );
        assert_eq!(CexRecordKind::from_operation("???"), CexRecordKind::Other);
    }
}
