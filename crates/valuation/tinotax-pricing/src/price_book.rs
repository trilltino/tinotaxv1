//! The merged per-day GBP price book.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use tinotax_core::{parse_date_prefix, PriceConfidence, PriceObservation};
use tinotax_store::{read_jsonl, ProjectPaths};

/// A resolved daily price with provenance.
#[derive(Debug, Clone)]
pub struct ResolvedPrice {
    pub price: Decimal,
    pub source: String,
    pub confidence: PriceConfidence,
    /// The day the observation actually covers (may differ from the asked
    /// day when a nearby day was used).
    pub observed_date: String,
}

/// (asset, date) → best observation. Merges `price_observations.jsonl`
/// (manual imports + provider fetches) and `cex_price_hints.jsonl`.
#[derive(Debug, Default)]
pub struct PriceBook {
    by_asset_date: BTreeMap<(String, String), PriceObservation>,
}

impl PriceBook {
    pub fn load(paths: &ProjectPaths) -> Result<Self> {
        let mut book = Self::default();
        for path in [paths.price_observations_jsonl(), paths.price_hints_jsonl()] {
            if !path.exists() {
                continue;
            }
            let observations: Vec<PriceObservation> =
                read_jsonl(&path).with_context(|| format!("reading {path}"))?;
            for obs in observations {
                book.insert(obs)?;
            }
        }
        Ok(book)
    }

    pub fn is_empty(&self) -> bool {
        self.by_asset_date.is_empty()
    }

    pub fn len(&self) -> usize {
        self.by_asset_date.len()
    }

    /// Keep the better observation per (asset, day): higher confidence wins,
    /// later fetch breaks ties.
    pub fn insert(&mut self, obs: PriceObservation) -> Result<()> {
        anyhow::ensure!(
            obs.currency.eq_ignore_ascii_case("GBP"),
            "price observation for {} is in {}, expected GBP",
            obs.asset_symbol,
            obs.currency
        );
        let key = (obs.asset_symbol.to_ascii_uppercase(), obs.date()?);
        let replace = match self.by_asset_date.get(&key) {
            None => true,
            Some(existing) => match obs.confidence.cmp(&existing.confidence) {
                std::cmp::Ordering::Less => true, // more confident
                std::cmp::Ordering::Greater => false,
                std::cmp::Ordering::Equal => obs.fetched_at >= existing.fetched_at,
            },
        };
        if replace {
            self.by_asset_date.insert(key, obs);
        }
        Ok(())
    }

    /// Price for an asset on the day of `timestamp`. Falls back to the
    /// nearest observation within ±3 days at reduced confidence.
    pub fn lookup(&self, asset: &str, timestamp: &str) -> Option<ResolvedPrice> {
        let asset = asset.to_ascii_uppercase();
        let (y, m, d) = parse_date_prefix(timestamp).ok()?;
        let day = days_from_epoch(y, m, d);
        // Exact day first, then spiral out ±1..=3.
        for (offset, confidence) in [
            (0i64, None),
            (-1, Some(PriceConfidence::Medium)),
            (1, Some(PriceConfidence::Medium)),
            (-2, Some(PriceConfidence::Low)),
            (2, Some(PriceConfidence::Low)),
            (-3, Some(PriceConfidence::Low)),
            (3, Some(PriceConfidence::Low)),
        ] {
            let date = date_string(day + offset);
            if let Some(obs) = self.by_asset_date.get(&(asset.clone(), date.clone())) {
                return Some(ResolvedPrice {
                    price: obs.price,
                    source: obs.source.as_str().to_string(),
                    // A nearby day can never claim more confidence than the
                    // observation itself.
                    confidence: confidence.map_or(obs.confidence, |c| c.max(obs.confidence)),
                    observed_date: date,
                });
            }
        }
        None
    }
}

pub use tinotax_core::date::{date_string, days_from_epoch};

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use tinotax_core::PriceSource;

    fn obs(asset: &str, timestamp: &str, price: i64) -> PriceObservation {
        PriceObservation {
            asset_symbol: asset.into(),
            asset_contract: None,
            timestamp: timestamp.into(),
            currency: "GBP".into(),
            price: Decimal::new(price, 0),
            source: PriceSource::Manual,
            confidence: PriceConfidence::High,
            fetched_at: "2026-01-01T00:00:00Z".into(),
            note: None,
        }
    }

    #[test]
    fn civil_date_round_trips() {
        assert_eq!(days_from_epoch(1970, 1, 1), 0);
        assert_eq!(date_string(0), "1970-01-01");
        assert_eq!(date_string(days_from_epoch(2024, 4, 6)), "2024-04-06");
        assert_eq!(date_string(days_from_epoch(2024, 3, 1) - 1), "2024-02-29");
    }

    #[test]
    fn exact_day_keeps_confidence() -> Result<(), Box<dyn Error>> {
        let mut book = PriceBook::default();
        book.insert(obs("BTC", "2024-06-01T15:00:00Z", 50000))?;
        let hit = book
            .lookup("btc", "2024-06-01T02:00:00Z")
            .ok_or_else(|| std::io::Error::other("expected exact-day price"))?;
        assert_eq!(hit.price, Decimal::new(50000, 0));
        assert_eq!(hit.confidence, PriceConfidence::High);
        Ok(())
    }

    #[test]
    fn nearby_day_downgrades_confidence() -> Result<(), Box<dyn Error>> {
        let mut book = PriceBook::default();
        book.insert(obs("BTC", "2024-06-01T00:00:00Z", 50000))?;
        let hit = book
            .lookup("BTC", "2024-06-02T00:00:00Z")
            .ok_or_else(|| std::io::Error::other("expected nearby-day price"))?;
        assert_eq!(hit.confidence, PriceConfidence::Medium);
        assert_eq!(hit.observed_date, "2024-06-01");
        assert!(book.lookup("BTC", "2024-06-10T00:00:00Z").is_none());
        Ok(())
    }
}
