use rust_decimal::Decimal;
use std::str::FromStr;

use crate::error::CoreError;

/// A raw integer chain amount scaled into a human-readable `Decimal`.
///
/// `exact` is false when the value carried more significant digits than
/// `Decimal` can hold (96-bit mantissa) and trailing fractional digits were
/// dropped. Lossy conversions must be surfaced as review flags downstream;
/// the untouched raw string is always preserved on the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaledAmount {
    pub value: Decimal,
    pub exact: bool,
}

impl ScaledAmount {
    /// Scale a raw base-unit integer string (e.g. wei, yoctoNEAR) by
    /// `decimals` without ever passing through a float.
    pub fn from_raw(raw: &str, decimals: u32) -> Result<Self, CoreError> {
        let raw = raw.trim();
        if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
            return Err(CoreError::InvalidAmount {
                raw: raw.to_string(),
                reason: "expected an unsigned integer string".to_string(),
            });
        }

        let digits = raw.trim_start_matches('0');
        let digits = if digits.is_empty() { "0" } else { digits };

        let mut text = if decimals == 0 {
            digits.to_string()
        } else {
            let d = decimals as usize;
            if digits.len() > d {
                format!("{}.{}", &digits[..digits.len() - d], &digits[digits.len() - d..])
            } else {
                format!("0.{}{}", "0".repeat(d - digits.len()), digits)
            }
        };

        // Decimal holds 28-29 significant digits. If the value overflows,
        // drop least-significant fractional digits until it fits and report
        // the conversion as inexact.
        let mut exact = true;
        loop {
            match Decimal::from_str(&text) {
                Ok(value) => return Ok(Self { value, exact }),
                Err(_) => {
                    let Some(dot) = text.find('.') else {
                        return Err(CoreError::InvalidAmount {
                            raw: raw.to_string(),
                            reason: "integer part exceeds Decimal precision".to_string(),
                        });
                    };
                    if text.len() - dot <= 2 {
                        // nothing fractional left to trim
                        text.truncate(dot);
                    } else {
                        text.truncate(text.len() - 1);
                    }
                    exact = false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn scales_wei_to_ether() {
        let a = ScaledAmount::from_raw("1500000000000000000", 18).unwrap();
        assert_eq!(a.value.to_string(), "1.500000000000000000");
        assert!(a.exact);
    }

    #[test]
    fn scales_small_values_with_leading_zeros() {
        let a = ScaledAmount::from_raw("000042", 6).unwrap();
        assert_eq!(a.value.to_string(), "0.000042");
    }

    #[test]
    fn zero_decimals_passthrough() {
        let a = ScaledAmount::from_raw("12345", 0).unwrap();
        assert_eq!(a.value.to_string(), "12345");
    }

    #[test]
    fn large_yocto_near_is_lossy_not_fatal() {
        // 100_000 NEAR in yocto = 1e29: too many digits for Decimal at scale 24.
        let a = ScaledAmount::from_raw("100000000000000000000000000000", 24).unwrap();
        assert!(!a.exact);
        assert_eq!(a.value.trunc().to_string(), "100000");
    }

    #[test]
    fn rejects_non_integer_input() {
        assert!(ScaledAmount::from_raw("1.5", 18).is_err());
        assert!(ScaledAmount::from_raw("", 18).is_err());
        assert!(ScaledAmount::from_raw("1e24", 24).is_err());
    }
}
