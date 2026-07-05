//! The UK tax year runs 6 April – 5 April.

use serde::{Deserialize, Serialize};

use crate::domain::TaxError;

/// One UK tax year, identified by the calendar year it starts in.
/// `TaxYear { start_year: 2024 }` is 6 April 2024 – 5 April 2025,
/// labelled `2024-2025`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxYear {
    pub start_year: i32,
}

impl TaxYear {
    /// Accepts `2024-2025`, `2024-25`, `2024/25` and plain `2024`.
    pub fn parse(text: &str) -> Result<Self, TaxError> {
        let text = text.trim();
        let bad = || TaxError::BadTaxYear(text.to_string());
        let (start, end) = match text.split_once(['-', '/']) {
            Some((s, e)) => (s, Some(e)),
            None => (text, None),
        };
        let start_year: i32 = start.parse().map_err(|_| bad())?;
        if !(2000..=2100).contains(&start_year) {
            return Err(bad());
        }
        if let Some(end) = end {
            let end_year: i32 = end.parse().map_err(|_| bad())?;
            let expected_full = start_year + 1;
            let expected_short = expected_full % 100;
            if end_year != expected_full && end_year != expected_short {
                return Err(bad());
            }
        }
        Ok(Self { start_year })
    }

    /// Canonical label, e.g. `2024-2025` (used for folder names and CSVs).
    pub fn label(&self) -> String {
        format!("{}-{}", self.start_year, self.start_year + 1)
    }

    /// Does the civil date fall inside this tax year?
    pub fn contains(&self, year: i32, month: u32, day: u32) -> bool {
        let after_start =
            year > self.start_year || (year == self.start_year && (month, day) >= (4, 6));
        let end_year = self.start_year + 1;
        let before_end = year < end_year || (year == end_year && (month, day) <= (4, 5));
        after_start && before_end
    }

    /// Does the RFC 3339 timestamp fall inside this tax year?
    pub fn contains_timestamp(&self, timestamp: &str) -> bool {
        tinotax_core::parse_date_prefix(timestamp)
            .map(|(y, m, d)| self.contains(y, m, d))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_accepted_forms() {
        for text in ["2024-2025", "2024-25", "2024/25", "2024"] {
            assert_eq!(TaxYear::parse(text).unwrap().start_year, 2024, "{text}");
        }
        assert!(TaxYear::parse("2024-2026").is_err());
        assert!(TaxYear::parse("24-25").is_err());
        assert!(TaxYear::parse("garbage").is_err());
    }

    #[test]
    fn boundaries_are_6_april_to_5_april() {
        let y = TaxYear::parse("2024-2025").unwrap();
        assert!(!y.contains(2024, 4, 5));
        assert!(y.contains(2024, 4, 6));
        assert!(y.contains(2024, 12, 31));
        assert!(y.contains(2025, 4, 5));
        assert!(!y.contains(2025, 4, 6));
        assert_eq!(y.label(), "2024-2025");
    }
}
