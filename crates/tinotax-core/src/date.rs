//! Civil-date arithmetic without a calendar dependency: day numbers since
//! 1970-01-01 (proleptic Gregorian), via Howard Hinnant's algorithms.

/// Days since 1970-01-01 for a civil date.
pub fn days_from_epoch(y: i32, m: u32, d: u32) -> i64 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = i64::from((m + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of [`days_from_epoch`], as `YYYY-MM-DD`.
pub fn date_string(days: i64) -> String {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Day number for an RFC 3339 timestamp.
pub fn day_of_timestamp(timestamp: &str) -> Result<i64, crate::CoreError> {
    let (y, m, d) = crate::tax_event::parse_date_prefix(timestamp)?;
    Ok(days_from_epoch(y, m, d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        assert_eq!(days_from_epoch(1970, 1, 1), 0);
        assert_eq!(date_string(0), "1970-01-01");
        assert_eq!(date_string(days_from_epoch(2024, 4, 6)), "2024-04-06");
        assert_eq!(date_string(days_from_epoch(2024, 3, 1) - 1), "2024-02-29");
        assert_eq!(
            day_of_timestamp("2024-04-06T10:00:00Z").unwrap(),
            days_from_epoch(2024, 4, 6)
        );
    }
}
