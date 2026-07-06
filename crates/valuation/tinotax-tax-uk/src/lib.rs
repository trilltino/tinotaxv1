//! UK CGT + income engine.
//!
//! - `tax_year`   — UK tax year (6 April – 5 April) boundaries
//! - `validation` — refuses unresolved/unpriced rows instead of guessing
//! - `matching`   — day aggregation and the `calculate` entry point
//! - `same_day`   — TCGA92 s105 same-day matching
//! - `thirty_day` — TCGA92 s106A 30-day "bed & breakfast" matching
//! - `s104_pool`  — Section 104 average-cost pooling per asset
//! - `disposals`  — per-disposal calculation rows
//! - `income`     — income receipts at market value
//! - `fees`       — crypto-fee disposal totals
//! - `reports`    — `tax/<year>/` CSVs + assumptions doc
//!
//! `matching::calculate` is pure and deterministic; all IO lives in
//! `reports` and `domain::load_opening_pools`.

pub mod disposals;
pub mod domain;
pub mod fees;
pub mod income;
pub mod matching;
pub mod reports;
pub mod s104_pool;
pub mod same_day;
pub mod tax_year;
pub mod thirty_day;
pub mod validation;

pub use domain::{
    load_opening_pools, DisposalCalculation, IncomeCalculation, OpeningPool, PoolMovement,
    PoolYearState, TaxError, UkTaxCalculation, UkTaxSummary, UnresolvedTaxItem,
};
pub use matching::calculate;
pub use reports::write_reports;
pub use tax_year::TaxYear;

#[cfg(test)]
mod tests {
    use std::error::Error;

    use rust_decimal::{dec, Decimal};

    use super::*;
    use tinotax_core::{PriceConfidence, ReviewStatus, TaxEventType, TaxLedgerEvent};

    fn tax_year_label(timestamp: &str) -> String {
        match tinotax_core::uk_tax_year(timestamp) {
            Ok(label) => label,
            Err(err) => format!("invalid-test-timestamp:{err}"),
        }
    }

    fn event(
        id: &str,
        timestamp: &str,
        t: TaxEventType,
        asset: &str,
        quantity: Decimal,
        gbp: Option<Decimal>,
    ) -> TaxLedgerEvent {
        let (proceeds, cost, income) = if t.is_disposal() {
            (gbp, None, None)
        } else if t.is_income() {
            (None, None, gbp)
        } else {
            (None, gbp, None)
        };
        TaxLedgerEvent {
            ledger_event_id: id.to_string(),
            source_event_ids: vec![format!("src_{id}")],
            source_refs: vec![],
            timestamp: timestamp.to_string(),
            tax_year: tax_year_label(timestamp),
            platform: None,
            chain: Some("test".into()),
            wallet: Some("w".into()),
            tx_hash: None,
            tax_event_type: t,
            asset_symbol: asset.to_string(),
            asset_contract: None,
            quantity,
            proceeds_gbp: proceeds,
            cost_gbp: cost,
            income_gbp: income,
            fee_gbp: None,
            price_source: gbp.map(|_| "manual".to_string()),
            price_confidence: if gbp.is_some() {
                PriceConfidence::High
            } else {
                PriceConfidence::Missing
            },
            review_status: ReviewStatus::Auto,
            user_note: None,
        }
    }

    fn year() -> TaxYear {
        TaxYear { start_year: 2024 }
    }

    fn calc(events: &[TaxLedgerEvent]) -> Result<UkTaxCalculation, TaxError> {
        calculate(events, &[], year(), false)
    }

    #[test]
    fn simple_buy_then_sell() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "b",
                "2024-05-01T00:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(10000)),
            ),
            event(
                "s",
                "2024-06-01T00:00:00Z",
                TaxEventType::Disposal,
                "BTC",
                dec!(1),
                Some(dec!(15000)),
            ),
        ];
        let c = calc(&events)?;
        assert_eq!(c.disposals.len(), 1);
        let d = &c.disposals[0];
        assert_eq!(d.matched_s104_cost_gbp, dec!(10000));
        assert_eq!(d.gain_or_loss_gbp, dec!(5000));
        assert_eq!(c.summary.net_gain_or_loss_gbp, dec!(5000));
        Ok(())
    }

    #[test]
    fn partial_disposal_takes_average_cost() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "b",
                "2024-05-01T00:00:00Z",
                TaxEventType::Acquisition,
                "ETH",
                dec!(10),
                Some(dec!(20000)),
            ),
            event(
                "s",
                "2024-07-01T00:00:00Z",
                TaxEventType::Disposal,
                "ETH",
                dec!(4),
                Some(dec!(12000)),
            ),
        ];
        let c = calc(&events)?;
        let d = &c.disposals[0];
        assert_eq!(d.allowable_cost_gbp, dec!(8000)); // 4/10 of 20k
        assert_eq!(d.gain_or_loss_gbp, dec!(4000));
        let pool = &c.pool_year_states[0];
        assert_eq!(pool.closing_quantity, dec!(6));
        assert_eq!(pool.closing_cost_gbp, dec!(12000));
        Ok(())
    }

    #[test]
    fn same_day_buy_and_sell_match_first() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "pool",
                "2024-05-01T00:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(10000)),
            ),
            event(
                "buy",
                "2024-06-01T09:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(20000)),
            ),
            event(
                "sell",
                "2024-06-01T17:00:00Z",
                TaxEventType::Disposal,
                "BTC",
                dec!(1),
                Some(dec!(21000)),
            ),
        ];
        let c = calc(&events)?;
        let d = &c.disposals[0];
        // Matched against the same-day buy (20k), not the older pool (10k).
        assert_eq!(d.matched_same_day_cost_gbp, dec!(20000));
        assert_eq!(d.matched_s104_cost_gbp, dec!(0));
        assert_eq!(d.gain_or_loss_gbp, dec!(1000));
        Ok(())
    }

    #[test]
    fn thirty_day_buyback_matches_before_pool() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "pool",
                "2023-01-01T00:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(5000)),
            ),
            event(
                "sell",
                "2024-06-01T00:00:00Z",
                TaxEventType::Disposal,
                "BTC",
                dec!(1),
                Some(dec!(30000)),
            ),
            event(
                "rebuy",
                "2024-06-20T00:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(29000)),
            ),
        ];
        let c = calc(&events)?;
        let d = &c.disposals[0];
        assert_eq!(d.matched_30_day_cost_gbp, dec!(29000));
        assert_eq!(d.matched_s104_cost_gbp, dec!(0));
        assert_eq!(d.gain_or_loss_gbp, dec!(1000));
        // The rebuy never reached the pool; the old pool is intact.
        let pool = &c.pool_year_states[0];
        assert_eq!(pool.closing_quantity, dec!(1));
        assert_eq!(pool.closing_cost_gbp, dec!(5000));
        Ok(())
    }

    #[test]
    fn buyback_after_30_days_stays_in_pool() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "pool",
                "2023-01-01T00:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(5000)),
            ),
            event(
                "sell",
                "2024-06-01T00:00:00Z",
                TaxEventType::Disposal,
                "BTC",
                dec!(1),
                Some(dec!(30000)),
            ),
            event(
                "rebuy",
                "2024-07-15T00:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(29000)),
            ),
        ];
        let c = calc(&events)?;
        let d = &c.disposals[0];
        assert_eq!(d.matched_30_day_cost_gbp, dec!(0));
        assert_eq!(d.matched_s104_cost_gbp, dec!(5000));
        assert_eq!(d.gain_or_loss_gbp, dec!(25000));
        Ok(())
    }

    #[test]
    fn s104_pool_averages_multiple_buys() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "b1",
                "2023-05-01T00:00:00Z",
                TaxEventType::Acquisition,
                "ETH",
                dec!(2),
                Some(dec!(2000)),
            ),
            event(
                "b2",
                "2023-08-01T00:00:00Z",
                TaxEventType::Acquisition,
                "ETH",
                dec!(2),
                Some(dec!(6000)),
            ),
            event(
                "s",
                "2024-06-01T00:00:00Z",
                TaxEventType::Disposal,
                "ETH",
                dec!(2),
                Some(dec!(7000)),
            ),
        ];
        let c = calc(&events)?;
        let d = &c.disposals[0];
        assert_eq!(d.allowable_cost_gbp, dec!(4000)); // half of 8k total cost
        assert_eq!(d.gain_or_loss_gbp, dec!(3000));
        Ok(())
    }

    #[test]
    fn swap_is_disposal_plus_acquisition() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "b",
                "2023-01-01T00:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(10000)),
            ),
            event(
                "out",
                "2024-06-01T00:00:00Z",
                TaxEventType::SwapDisposal,
                "BTC",
                dec!(1),
                Some(dec!(40000)),
            ),
            event(
                "in",
                "2024-06-01T00:00:01Z",
                TaxEventType::SwapAcquisition,
                "ETH",
                dec!(20),
                Some(dec!(40000)),
            ),
            event(
                "sell",
                "2025-01-10T00:00:00Z",
                TaxEventType::Disposal,
                "ETH",
                dec!(20),
                Some(dec!(50000)),
            ),
        ];
        let c = calc(&events)?;
        assert_eq!(c.disposals.len(), 2);
        let btc = c
            .disposals
            .iter()
            .find(|d| d.asset == "BTC")
            .ok_or_else(|| std::io::Error::other("missing BTC disposal"))?;
        assert_eq!(btc.gain_or_loss_gbp, dec!(30000));
        let eth = c
            .disposals
            .iter()
            .find(|d| d.asset == "ETH")
            .ok_or_else(|| std::io::Error::other("missing ETH disposal"))?;
        assert_eq!(eth.allowable_cost_gbp, dec!(40000));
        assert_eq!(eth.gain_or_loss_gbp, dec!(10000));
        Ok(())
    }

    #[test]
    fn staking_reward_is_income_then_cost_basis() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "r",
                "2024-05-01T00:00:00Z",
                TaxEventType::StakingReward,
                "NEAR",
                dec!(100),
                Some(dec!(500)),
            ),
            event(
                "s",
                "2024-08-01T00:00:00Z",
                TaxEventType::Disposal,
                "NEAR",
                dec!(100),
                Some(dec!(900)),
            ),
        ];
        let c = calc(&events)?;
        assert_eq!(c.summary.total_income_gbp, dec!(500));
        assert_eq!(c.disposals[0].allowable_cost_gbp, dec!(500));
        assert_eq!(c.disposals[0].gain_or_loss_gbp, dec!(400));
        assert_eq!(
            c.summary.income_by_category_gbp.get("staking_reward"),
            Some(&dec!(500))
        );
        Ok(())
    }

    #[test]
    fn airdrop_then_sale_uses_market_value_cost() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "a",
                "2024-05-01T00:00:00Z",
                TaxEventType::Airdrop,
                "XYZ",
                dec!(1000),
                Some(dec!(50)),
            ),
            event(
                "s",
                "2025-01-01T00:00:00Z",
                TaxEventType::Disposal,
                "XYZ",
                dec!(1000),
                Some(dec!(80)),
            ),
        ];
        let c = calc(&events)?;
        assert_eq!(c.disposals[0].gain_or_loss_gbp, dec!(30));
        // Airdrop appears in the income listing (for HMRC Q8) at zero income.
        assert_eq!(c.income.len(), 1);
        assert_eq!(c.income[0].income_gbp, dec!(0));
        assert_eq!(c.summary.total_income_gbp, dec!(0));
        Ok(())
    }

    #[test]
    fn fee_only_transaction_is_small_disposal() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "b",
                "2024-05-01T00:00:00Z",
                TaxEventType::Acquisition,
                "NEAR",
                dec!(10),
                Some(dec!(50)),
            ),
            event(
                "f",
                "2024-06-01T00:00:00Z",
                TaxEventType::Fee,
                "NEAR",
                dec!(1),
                Some(dec!(4)),
            ),
        ];
        let c = calc(&events)?;
        assert_eq!(c.disposals.len(), 1);
        assert_eq!(c.disposals[0].allowable_cost_gbp, dec!(5)); // 1/10 of £50
        assert_eq!(c.disposals[0].gain_or_loss_gbp, dec!(-1));
        assert_eq!(c.summary.crypto_fees_disposed_gbp, dec!(4));
        Ok(())
    }

    #[test]
    fn disposal_before_any_acquisition_fails_clearly() -> Result<(), Box<dyn Error>> {
        let events = vec![event(
            "s",
            "2024-06-01T00:00:00Z",
            TaxEventType::Disposal,
            "BTC",
            dec!(1),
            Some(dec!(30000)),
        )];
        let err = match calculate(&events, &[], year(), false) {
            Ok(_) => return Err(std::io::Error::other("expected insufficient-pool error").into()),
            Err(err) => err,
        };
        let text = err.to_string();
        assert!(text.contains("BTC"), "{text}");
        assert!(text.contains("opening_pools.toml"), "{text}");
        Ok(())
    }

    #[test]
    fn opening_pool_covers_early_disposals() -> Result<(), Box<dyn Error>> {
        let opening = OpeningPool {
            asset: "BTC".into(),
            quantity: dec!(1),
            allowable_cost_gbp: dec!(1200),
            as_of: "2017-01-01T00:00:00Z".into(),
        };
        let events = vec![event(
            "s",
            "2024-06-01T00:00:00Z",
            TaxEventType::Disposal,
            "BTC",
            dec!(0.5),
            Some(dec!(20000)),
        )];
        let c = calculate(&events, &[opening], year(), false)?;
        assert_eq!(c.disposals[0].allowable_cost_gbp, dec!(600));
        assert_eq!(c.disposals[0].gain_or_loss_gbp, dec!(19400));
        let pool = &c.pool_year_states[0];
        assert_eq!(pool.closing_quantity, dec!(0.5));
        assert_eq!(pool.closing_cost_gbp, dec!(600));
        Ok(())
    }

    #[test]
    fn negative_pool_is_prevented_even_partially() {
        let events = vec![
            event(
                "b",
                "2024-05-01T00:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(10000)),
            ),
            event(
                "s",
                "2024-06-01T00:00:00Z",
                TaxEventType::Disposal,
                "BTC",
                dec!(2),
                Some(dec!(60000)),
            ),
        ];
        assert!(matches!(
            calculate(&events, &[], year(), false),
            Err(TaxError::InsufficientPools { .. })
        ));
    }

    #[test]
    fn unresolved_rows_block_unless_allowed() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "b",
                "2024-05-01T00:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(10000)),
            ),
            event(
                "s",
                "2024-06-01T00:00:00Z",
                TaxEventType::Disposal,
                "BTC",
                dec!(1),
                None,
            ), // unpriced
            event(
                "u",
                "2024-06-02T00:00:00Z",
                TaxEventType::Unknown,
                "BTC",
                dec!(1),
                None,
            ),
        ];
        assert!(matches!(
            calculate(&events, &[], year(), false),
            Err(TaxError::UnresolvedItems { .. })
        ));
        let c = calculate(&events, &[], year(), true)?;
        assert_eq!(c.summary.unresolved_blockers, 2);
        assert_eq!(c.disposals.len(), 0); // the unpriced disposal was excluded
        assert_eq!(c.pool_year_states[0].closing_quantity, dec!(1));
        Ok(())
    }

    /// HMRC-style interaction example (CRYPTO22256 shape): a disposal is
    /// matched partly same-day, partly 30-day, remainder from the pool.
    #[test]
    fn same_day_then_thirty_day_then_pool_interaction() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "pool",
                "2023-01-01T00:00:00Z",
                TaxEventType::Acquisition,
                "TOK",
                dec!(1000),
                Some(dec!(1000)),
            ),
            event(
                "sell",
                "2024-06-03T10:00:00Z",
                TaxEventType::Disposal,
                "TOK",
                dec!(500),
                Some(dec!(2000)),
            ),
            event(
                "sameday",
                "2024-06-03T15:00:00Z",
                TaxEventType::Acquisition,
                "TOK",
                dec!(100),
                Some(dec!(350)),
            ),
            event(
                "rebuy",
                "2024-06-20T00:00:00Z",
                TaxEventType::Acquisition,
                "TOK",
                dec!(200),
                Some(dec!(650)),
            ),
        ];
        let c = calc(&events)?;
        let d = &c.disposals[0];
        assert_eq!(d.matched_same_day_quantity, dec!(100));
        assert_eq!(d.matched_same_day_cost_gbp, dec!(350));
        assert_eq!(d.matched_30_day_quantity, dec!(200));
        assert_eq!(d.matched_30_day_cost_gbp, dec!(650));
        assert_eq!(d.matched_s104_quantity, dec!(200));
        assert_eq!(d.matched_s104_cost_gbp, dec!(200)); // 200/1000 of £1000
        assert_eq!(d.allowable_cost_gbp, dec!(1200));
        assert_eq!(d.gain_or_loss_gbp, dec!(800));
        let pool = &c.pool_year_states[0];
        assert_eq!(pool.closing_quantity, dec!(800));
        assert_eq!(pool.closing_cost_gbp, dec!(800));
        Ok(())
    }

    #[test]
    fn transfers_and_ignored_rows_have_no_effect() -> Result<(), Box<dyn Error>> {
        let events = vec![
            event(
                "b",
                "2024-05-01T00:00:00Z",
                TaxEventType::Acquisition,
                "BTC",
                dec!(1),
                Some(dec!(10000)),
            ),
            event(
                "t",
                "2024-05-10T00:00:00Z",
                TaxEventType::TransferOut,
                "BTC",
                dec!(1),
                None,
            ),
            event(
                "i",
                "2024-05-11T00:00:00Z",
                TaxEventType::Ignore,
                "BTC",
                dec!(9),
                None,
            ),
            event(
                "s",
                "2024-06-01T00:00:00Z",
                TaxEventType::Disposal,
                "BTC",
                dec!(1),
                Some(dec!(15000)),
            ),
        ];
        let c = calc(&events)?;
        assert_eq!(c.disposals.len(), 1);
        assert_eq!(c.disposals[0].gain_or_loss_gbp, dec!(5000));
        Ok(())
    }
}
