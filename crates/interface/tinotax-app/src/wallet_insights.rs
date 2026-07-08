//! Per-wallet insight aggregation for the desktop dashboard.
//!
//! Everything here is a read-only rollup of files the pipeline already
//! writes: normalised events for activity, the (priced) reviewed ledger for
//! GBP values and coverage, and the tax-year summary CSV for the headline
//! numbers. No new state, no recalculation of tax policy.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use serde::Serialize;
use tinotax_core::{
    Direction, EventType, NormalisedEvent, ReviewStatus, TaxEventType, TaxLedgerEvent,
};
use tinotax_pricing::valuation::needed_field;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletInsightsResult {
    /// Every wallet declared in the project, for the selector.
    pub wallets: Vec<WalletOptionDto>,
    /// Insights for the selected wallet; `None` when the project has none.
    pub insights: Option<WalletInsightsDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletOptionDto {
    pub id: String,
    pub name: String,
    pub chain: String,
    pub address: String,
    pub event_count: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletInsightsDto {
    pub wallet_id: String,
    pub name: String,
    pub chain: String,
    pub address: String,
    pub period_start: String,
    pub period_end: String,
    /// First/last event timestamps actually loaded; empty when no events.
    pub first_event: String,
    pub last_event: String,
    pub total_events: u64,
    pub events_in: u64,
    pub events_out: u64,
    pub fee_events: u64,
    pub needs_review: u64,
    pub monthly: Vec<MonthlyActivityDto>,
    pub assets: Vec<AssetInsightDto>,
    pub pricing: PricingCoverageDto,
    pub review: ReviewProgressDto,
    pub tax_year: String,
    /// Headline numbers from `tax/<year>/self_assessment_crypto_summary.csv`.
    /// Whole-project scope: S104 pools span wallets, so disposals cannot be
    /// attributed to a single wallet.
    pub tax_year_summary: Option<TaxYearSummaryDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyActivityDto {
    /// `YYYY-MM`.
    pub month: String,
    pub events: u64,
    pub inflows: u64,
    pub outflows: u64,
    pub fees: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInsightDto {
    pub symbol: String,
    pub events: u64,
    pub quantity_in: String,
    pub quantity_out: String,
    /// GBP sums from the ledger; empty string when nothing is valued yet.
    pub proceeds_gbp: String,
    pub cost_gbp: String,
    pub income_gbp: String,
    pub fee_gbp: String,
    pub unpriced_rows: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingCoverageDto {
    /// Ledger rows carrying a GBP value.
    pub valued_rows: u64,
    /// Rows that need a GBP value and do not have one.
    pub missing_rows: u64,
    /// Rows with no taxable value to price (opaque calls, ignores).
    pub nothing_to_price: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewProgressDto {
    pub total: u64,
    /// Machine-classified, not flagged, no override.
    pub auto_classified: u64,
    /// Rows with a human override applied.
    pub overridden: u64,
    /// Flagged or still classified `unknown`.
    pub outstanding: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxYearSummaryDto {
    pub tax_year: String,
    pub disposals: u64,
    pub proceeds_gbp: String,
    pub allowable_costs_gbp: String,
    pub gains_gbp: String,
    pub losses_gbp: String,
    pub net_gain_gbp: String,
    pub income_gbp: String,
    pub fees_gbp: String,
    pub unresolved_blockers: u64,
    pub unresolved_warnings: u64,
}

/// Aggregate insights for one wallet. `wallet_id = None` selects the wallet
/// with the most loaded events (ties broken by config order).
pub fn desktop_wallet_insights(
    project: &str,
    wallet_id: Option<&str>,
    tax_year: Option<&str>,
) -> Result<WalletInsightsResult> {
    let (paths, config) = crate::open_project(project)?;
    let tax_year = tax_year.unwrap_or("2024-2025").to_string();

    let events = crate::event_cache::load_events_cached(&paths)?;
    let overrides = tinotax_review::load_latest_overrides(&paths)?;

    let mut event_counts: BTreeMap<&str, u64> = BTreeMap::new();
    for event in events.iter() {
        *event_counts.entry(event.source_id.as_str()).or_default() += 1;
    }
    let wallets: Vec<WalletOptionDto> = config
        .wallets
        .iter()
        .map(|wallet| WalletOptionDto {
            id: wallet.id.clone(),
            name: wallet.name.clone(),
            chain: wallet.chain.clone(),
            address: wallet.address.clone(),
            event_count: event_counts.get(wallet.id.as_str()).copied().unwrap_or(0),
        })
        .collect();

    let selected = match wallet_id {
        Some(id) => wallets.iter().find(|wallet| wallet.id == id),
        None => wallets
            .iter()
            .max_by_key(|wallet| wallet.event_count)
            .or(wallets.first()),
    };
    let Some(selected) = selected.cloned() else {
        return Ok(WalletInsightsResult {
            wallets,
            insights: None,
        });
    };

    let wallet_events: Vec<&NormalisedEvent> = events
        .iter()
        .filter(|event| event.source_id == selected.id)
        .collect();

    // Prefer the priced ledger; fall back to the reviewed ledger so the view
    // still works before `ledger price` has run.
    let ledger: Vec<TaxLedgerEvent> = if paths.priced_ledger_jsonl().exists() {
        tinotax_pricing::load_priced_ledger(&paths)?
    } else if paths.reviewed_ledger_jsonl().exists() {
        tinotax_ledger::load_reviewed_ledger(&paths)?
    } else {
        Vec::new()
    };
    let wallet_ledger: Vec<&TaxLedgerEvent> = ledger
        .iter()
        .filter(|row| {
            row.wallet
                .as_deref()
                .is_some_and(|wallet| wallet.eq_ignore_ascii_case(&selected.address))
                && row.chain.as_deref() == Some(selected.chain.as_str())
        })
        .collect();

    let insights = build_insights(
        &selected,
        &config,
        &wallet_events,
        &wallet_ledger,
        &overrides,
        &tax_year,
        load_tax_year_summary(&paths, &tax_year)?,
    );

    Ok(WalletInsightsResult {
        wallets,
        insights: Some(insights),
    })
}

#[allow(clippy::too_many_arguments)]
fn build_insights(
    selected: &WalletOptionDto,
    config: &tinotax_config::ProjectConfig,
    wallet_events: &[&NormalisedEvent],
    wallet_ledger: &[&TaxLedgerEvent],
    overrides: &BTreeMap<String, tinotax_core::ReviewOverride>,
    tax_year: &str,
    tax_year_summary: Option<TaxYearSummaryDto>,
) -> WalletInsightsDto {
    let mut events_in = 0u64;
    let mut events_out = 0u64;
    let mut fee_events = 0u64;
    let mut needs_review = 0u64;
    let mut monthly: BTreeMap<String, MonthlyActivityDto> = BTreeMap::new();

    #[derive(Default)]
    struct AssetAccumulator {
        events: u64,
        quantity_in: Decimal,
        quantity_out: Decimal,
        proceeds: Option<Decimal>,
        cost: Option<Decimal>,
        income: Option<Decimal>,
        fee: Option<Decimal>,
        unpriced_rows: u64,
    }
    let mut assets: BTreeMap<String, AssetAccumulator> = BTreeMap::new();

    for event in wallet_events {
        let is_fee = event.event_type == EventType::Fee;
        if is_fee {
            fee_events += 1;
        } else {
            match event.direction {
                Direction::In => events_in += 1,
                Direction::Out => events_out += 1,
                Direction::SelfTransfer | Direction::Unknown => {}
            }
        }
        if event.needs_review {
            needs_review += 1;
        }

        if event.timestamp.len() >= 7 {
            let month = event.timestamp[..7].to_string();
            let bucket = monthly
                .entry(month.clone())
                .or_insert_with(|| MonthlyActivityDto {
                    month,
                    events: 0,
                    inflows: 0,
                    outflows: 0,
                    fees: 0,
                });
            bucket.events += 1;
            if is_fee {
                bucket.fees += 1;
            } else {
                match event.direction {
                    Direction::In => bucket.inflows += 1,
                    Direction::Out => bucket.outflows += 1,
                    Direction::SelfTransfer | Direction::Unknown => {}
                }
            }
        }

        let asset = assets.entry(event.asset_symbol.clone()).or_default();
        asset.events += 1;
        match event.direction {
            Direction::In => asset.quantity_in += event.amount,
            Direction::Out => asset.quantity_out += event.amount,
            Direction::SelfTransfer | Direction::Unknown => {}
        }
    }

    // GBP sums and coverage come from the ledger; quantities and counts from
    // events. The two views share asset symbols, so they land in one table.
    let mut valued_rows = 0u64;
    let mut missing_rows = 0u64;
    let mut nothing_to_price = 0u64;
    let mut unknown_rows = 0u64;
    let mut overridden_rows = 0u64;
    let mut flagged_rows = 0u64;
    for row in wallet_ledger {
        if needed_field(row).is_some() {
            missing_rows += 1;
            if let Some(asset) = assets.get_mut(&row.asset_symbol) {
                asset.unpriced_rows += 1;
            }
        } else if row.proceeds_gbp.is_some()
            || row.cost_gbp.is_some()
            || row.income_gbp.is_some()
            || row.fee_gbp.is_some()
        {
            valued_rows += 1;
        } else {
            nothing_to_price += 1;
        }

        match row.review_status {
            ReviewStatus::Reviewed => overridden_rows += 1,
            ReviewStatus::NeedsReview => flagged_rows += 1,
            ReviewStatus::Auto => {}
        }
        if row.tax_event_type == TaxEventType::Unknown {
            unknown_rows += 1;
        }

        if let Some(asset) = assets.get_mut(&row.asset_symbol) {
            add_opt(&mut asset.proceeds, row.proceeds_gbp);
            add_opt(&mut asset.cost, row.cost_gbp);
            add_opt(&mut asset.income, row.income_gbp);
            add_opt(&mut asset.fee, row.fee_gbp);
        }
    }

    // Review progress falls back to event-level data when the ledger has not
    // been built yet, so the tab is never empty on a fresh project.
    let review = if wallet_ledger.is_empty() {
        let overridden = wallet_events
            .iter()
            .filter(|event| overrides.contains_key(&event.event_id))
            .count() as u64;
        let total = wallet_events.len() as u64;
        ReviewProgressDto {
            total,
            auto_classified: total.saturating_sub(needs_review).saturating_sub(overridden),
            overridden,
            outstanding: needs_review,
        }
    } else {
        let total = wallet_ledger.len() as u64;
        let outstanding = flagged_rows.max(unknown_rows);
        ReviewProgressDto {
            total,
            auto_classified: total
                .saturating_sub(overridden_rows)
                .saturating_sub(outstanding),
            overridden: overridden_rows,
            outstanding,
        }
    };

    let mut asset_rows: Vec<AssetInsightDto> = assets
        .into_iter()
        .map(|(symbol, acc)| AssetInsightDto {
            symbol,
            events: acc.events,
            quantity_in: trim_decimal(acc.quantity_in),
            quantity_out: trim_decimal(acc.quantity_out),
            proceeds_gbp: opt_gbp(acc.proceeds),
            cost_gbp: opt_gbp(acc.cost),
            income_gbp: opt_gbp(acc.income),
            fee_gbp: opt_gbp(acc.fee),
            unpriced_rows: acc.unpriced_rows,
        })
        .collect();
    asset_rows.sort_by(|a, b| b.events.cmp(&a.events).then(a.symbol.cmp(&b.symbol)));

    WalletInsightsDto {
        wallet_id: selected.id.clone(),
        name: selected.name.clone(),
        chain: selected.chain.clone(),
        address: selected.address.clone(),
        period_start: config.project.period_start.clone(),
        period_end: config.project.period_end.clone(),
        first_event: wallet_events
            .iter()
            .map(|event| event.timestamp.as_str())
            .min()
            .unwrap_or_default()
            .to_string(),
        last_event: wallet_events
            .iter()
            .map(|event| event.timestamp.as_str())
            .max()
            .unwrap_or_default()
            .to_string(),
        total_events: wallet_events.len() as u64,
        events_in,
        events_out,
        fee_events,
        needs_review,
        monthly: monthly.into_values().collect(),
        assets: asset_rows,
        pricing: PricingCoverageDto {
            valued_rows,
            missing_rows,
            nothing_to_price,
        },
        review,
        tax_year: tax_year.to_string(),
        tax_year_summary,
    }
}

fn add_opt(total: &mut Option<Decimal>, value: Option<Decimal>) {
    if let Some(value) = value {
        *total = Some(total.unwrap_or_default() + value);
    }
}

fn opt_gbp(value: Option<Decimal>) -> String {
    value.map(|v| v.round_dp(2).to_string()).unwrap_or_default()
}

fn trim_decimal(value: Decimal) -> String {
    value.normalize().to_string()
}

/// Parse `tax/<year>/self_assessment_crypto_summary.csv` (item,value rows).
fn load_tax_year_summary(
    paths: &tinotax_store::ProjectPaths,
    tax_year: &str,
) -> Result<Option<TaxYearSummaryDto>> {
    let path = paths.tax_dir(tax_year).join("self_assessment_crypto_summary.csv");
    if !path.exists() {
        return Ok(None);
    }
    let mut reader = csv::Reader::from_path(&path).with_context(|| format!("opening {path}"))?;
    let mut items: BTreeMap<String, String> = BTreeMap::new();
    for record in reader.records() {
        let record = record?;
        let (Some(item), Some(value)) = (record.get(0), record.get(1)) else {
            continue;
        };
        items.insert(item.trim().to_string(), value.trim().to_string());
    }
    let text = |key: &str| items.get(key).cloned().unwrap_or_default();
    let count = |key: &str| items.get(key).and_then(|v| v.parse().ok()).unwrap_or(0);

    Ok(Some(TaxYearSummaryDto {
        tax_year: tax_year.to_string(),
        disposals: count("number_of_disposals"),
        proceeds_gbp: text("total_proceeds_gbp"),
        allowable_costs_gbp: text("total_allowable_costs_gbp"),
        gains_gbp: text("total_gains_gbp"),
        losses_gbp: text("total_losses_gbp"),
        net_gain_gbp: text("net_gain_or_loss_gbp"),
        income_gbp: text("total_income_gbp"),
        fees_gbp: text("crypto_fees_disposed_gbp"),
        unresolved_blockers: count("unresolved_blockers_excluded"),
        unresolved_warnings: count("unresolved_warnings"),
    }))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use camino::Utf8PathBuf;
    use tinotax_store::ProjectPaths;

    use super::*;

    const CONFIG: &str = r#"
[project]
name = "insights-test"
base_currency = "GBP"
period_start = "2024-01-01T00:00:00Z"
period_end = "2025-04-05T23:59:59Z"

[[wallets]]
id = "lisk_main"
name = "Lisk wallet"
chain = "lisk-evm"
address = "0xAAAA000000000000000000000000000000000001"
provider = "lisk_blockscout"

[[wallets]]
id = "near_side"
name = "NEAR wallet"
chain = "near"
address = "side.near"
provider = "nearblocks"

[providers.lisk_blockscout]
kind = "blockscout"
base_url = "https://blockscout.lisk.com/api/v2"

[providers.nearblocks]
kind = "nearblocks"
base_url = "https://api.nearblocks.io/v1"
"#;

    fn event(id: &str, source_id: &str, ts: &str, direction: &str, amount: &str) -> String {
        format!(
            r#"{{"event_id":"{id}","project_id":"insights-test","source_id":"{source_id}","source_kind":"wallet","chain":"lisk-evm","wallet":"0xAAAA000000000000000000000000000000000001","timestamp":"{ts}","block_number":1,"tx_hash":"tx_{id}","event_type":"native_transfer","direction":"{direction}","asset_symbol":"ETH","asset_contract":null,"amount":"{amount}","raw_amount":null,"token_decimals":18,"from_address":null,"to_address":null,"fee_asset":null,"fee_amount":null,"counterparty":null,"method":null,"confidence":"high","needs_review":false,"review_reasons":[],"source_ref":{{"raw_file":"raw/x.json","raw_page":1,"json_path":"items[0]","log_index":null,"movement_index":0}}}}"#
        )
    }

    fn seed_project() -> Result<(tempfile::TempDir, String), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
            .map_err(|p| std::io::Error::other(format!("non-UTF8 path {}", p.display())))?;
        let paths = ProjectPaths::new(root.clone());
        paths.init()?;
        std::fs::write(paths.config_file(), CONFIG)?;
        let events = [
            event("e1", "lisk_main", "2025-01-10T10:00:00Z", "in", "1"),
            event("e2", "lisk_main", "2025-01-20T10:00:00Z", "out", "0.4"),
            event("e3", "lisk_main", "2025-02-01T10:00:00Z", "in", "2"),
            event("e4", "near_side", "2025-02-02T10:00:00Z", "in", "5"),
        ]
        .join("\n");
        std::fs::write(paths.events_jsonl(), events + "\n")?;
        Ok((tmp, root.to_string()))
    }

    #[test]
    fn selects_busiest_wallet_and_rolls_up_activity() -> Result<(), Box<dyn Error>> {
        let (_tmp, project) = seed_project()?;
        let result = desktop_wallet_insights(&project, None, Some("2024-2025"))?;

        assert_eq!(result.wallets.len(), 2);
        let insights = result.insights.ok_or("insights for busiest wallet")?;
        assert_eq!(insights.wallet_id, "lisk_main");
        assert_eq!(insights.total_events, 3);
        assert_eq!(insights.events_in, 2);
        assert_eq!(insights.events_out, 1);
        assert_eq!(insights.first_event, "2025-01-10T10:00:00Z");
        assert_eq!(insights.last_event, "2025-02-01T10:00:00Z");
        assert_eq!(insights.monthly.len(), 2);
        assert_eq!(insights.monthly[0].month, "2025-01");
        assert_eq!(insights.monthly[0].events, 2);
        assert_eq!(insights.assets.len(), 1);
        assert_eq!(insights.assets[0].symbol, "ETH");
        assert_eq!(insights.assets[0].quantity_in, "3");
        assert_eq!(insights.assets[0].quantity_out, "0.4");
        // No ledger yet: review falls back to event counts, pricing is empty.
        assert_eq!(insights.review.total, 3);
        assert_eq!(insights.pricing.valued_rows, 0);
        assert!(insights.tax_year_summary.is_none());
        Ok(())
    }

    #[test]
    fn explicit_wallet_id_wins_and_unknown_id_yields_no_insights() -> Result<(), Box<dyn Error>> {
        let (_tmp, project) = seed_project()?;
        let result = desktop_wallet_insights(&project, Some("near_side"), None)?;
        let insights = result.insights.ok_or("explicit wallet insights")?;
        assert_eq!(insights.wallet_id, "near_side");
        assert_eq!(insights.total_events, 1);

        let missing = desktop_wallet_insights(&project, Some("nope"), None)?;
        assert!(missing.insights.is_none());
        assert_eq!(missing.wallets.len(), 2);
        Ok(())
    }

    #[test]
    fn tax_summary_csv_is_parsed_when_present() -> Result<(), Box<dyn Error>> {
        let (_tmp, project) = seed_project()?;
        let paths = ProjectPaths::new(Utf8PathBuf::from(&project));
        let tax_dir = paths.tax_dir("2024-2025");
        std::fs::create_dir_all(&tax_dir)?;
        std::fs::write(
            tax_dir.join("self_assessment_crypto_summary.csv"),
            "item,value\nnumber_of_disposals,122\ntotal_proceeds_gbp,42349.23\nnet_gain_or_loss_gbp,-150.62\nunresolved_blockers_excluded,83\n",
        )?;

        let result = desktop_wallet_insights(&project, Some("lisk_main"), Some("2024-2025"))?;
        let summary = result
            .insights
            .and_then(|insights| insights.tax_year_summary)
            .ok_or("tax summary parsed")?;
        assert_eq!(summary.disposals, 122);
        assert_eq!(summary.proceeds_gbp, "42349.23");
        assert_eq!(summary.net_gain_gbp, "-150.62");
        assert_eq!(summary.unresolved_blockers, 83);
        Ok(())
    }
}
