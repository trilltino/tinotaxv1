//! Inputs, outputs and errors of the UK tax engine. The engine itself
//! (`matching::calculate`) is pure: ledger events in, calculation out.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::tax_year::TaxYear;

#[derive(Debug, Error)]
pub enum TaxError {
    #[error("invalid tax year {0:?} — expected e.g. 2024-2025")]
    BadTaxYear(String),

    #[error("invalid timestamp {0:?} in ledger")]
    InvalidTimestamp(String),

    #[error(
        "{count} ledger row(s) are not ready for tax calculation:\n{examples}\n\
         Resolve them via `review export-all` + `review apply` and `prices import`/`prices fetch`, \
         or pass --allow-unpriced to exclude and report them."
    )]
    UnresolvedItems { count: usize, examples: String },

    #[error(
        "cannot calculate: disposals exceed the available pool.\n{details}\n\
         Add opening_pools.toml (holdings acquired before the data window) or review the source data — \
         --allow-unpriced does not bypass this, it would produce nonsense tax outputs."
    )]
    InsufficientPools { details: String },
}

/// A Section 104 holding acquired before the data window, declared in
/// `opening_pools.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningPool {
    pub asset: String,
    pub quantity: Decimal,
    pub allowable_cost_gbp: Decimal,
    pub as_of: String,
}

/// `opening_pools.toml` top level: `[[pools]]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpeningPoolsFile {
    #[serde(default)]
    pub pools: Vec<OpeningPool>,
}

/// Current state of one asset's Section 104 pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolState {
    pub asset: String,
    pub quantity: Decimal,
    pub allowable_cost_gbp: Decimal,
}

/// One disposal (all disposals of one asset on one day are aggregated, per
/// TCGA92 s105) with its full matching breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisposalCalculation {
    pub disposal_id: String,
    pub asset: String,
    pub date: String,
    pub tax_year: String,
    pub quantity: Decimal,
    pub proceeds_gbp: Decimal,
    pub matched_same_day_quantity: Decimal,
    pub matched_same_day_cost_gbp: Decimal,
    pub matched_30_day_quantity: Decimal,
    pub matched_30_day_cost_gbp: Decimal,
    pub matched_s104_quantity: Decimal,
    pub matched_s104_cost_gbp: Decimal,
    pub allowable_cost_gbp: Decimal,
    pub gain_or_loss_gbp: Decimal,
    pub source_ledger_event_ids: Vec<String>,
    pub matching_notes: Vec<String>,
}

/// One change to one asset's Section 104 pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMovement {
    pub asset: String,
    pub date: String,
    pub tax_year: String,
    /// `opening` | `acquisition` | `disposal`
    pub kind: String,
    pub quantity_delta: Decimal,
    pub cost_delta_gbp: Decimal,
    pub quantity_after: Decimal,
    pub cost_after_gbp: Decimal,
    pub note: String,
}

/// Opening and closing pool balances for one asset across the tax year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolYearState {
    pub asset: String,
    pub opening_quantity: Decimal,
    pub opening_cost_gbp: Decimal,
    pub closing_quantity: Decimal,
    pub closing_cost_gbp: Decimal,
}

/// One income receipt (staking, mining, employment, …) at market value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeCalculation {
    pub ledger_event_id: String,
    pub asset: String,
    pub date: String,
    pub tax_year: String,
    /// The tax event type string, e.g. `staking_reward`.
    pub category: String,
    pub quantity: Decimal,
    pub income_gbp: Decimal,
    pub note: Option<String>,
}

/// A row excluded from (or worth flagging next to) the calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedTaxItem {
    pub ledger_event_id: String,
    pub asset: String,
    pub date: String,
    /// `blocker` (excluded from the calculation) or `warning` (included,
    /// but the reviewer should look).
    pub severity: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UkTaxSummary {
    pub tax_year: String,
    pub disposal_count: u64,
    pub total_proceeds_gbp: Decimal,
    pub total_allowable_costs_gbp: Decimal,
    pub total_gains_gbp: Decimal,
    pub total_losses_gbp: Decimal,
    pub net_gain_or_loss_gbp: Decimal,
    pub total_income_gbp: Decimal,
    pub income_by_category_gbp: BTreeMap<String, Decimal>,
    pub crypto_fees_disposed_gbp: Decimal,
    pub unresolved_blockers: u64,
    pub unresolved_warnings: u64,
}

/// Everything `calculate uk` produces for one tax year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UkTaxCalculation {
    pub tax_year: TaxYear,
    pub disposals: Vec<DisposalCalculation>,
    pub pool_movements: Vec<PoolMovement>,
    pub pool_year_states: Vec<PoolYearState>,
    pub income: Vec<IncomeCalculation>,
    pub unresolved: Vec<UnresolvedTaxItem>,
    pub summary: UkTaxSummary,
}

/// Load `opening_pools.toml` if present; a missing file means no opening
/// holdings, which is fine.
pub fn load_opening_pools(path: &camino::Utf8Path) -> anyhow::Result<Vec<OpeningPool>> {
    use anyhow::Context;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    let file: OpeningPoolsFile =
        toml::from_str(&text).with_context(|| format!("parsing {path}"))?;
    for pool in &file.pools {
        anyhow::ensure!(
            pool.quantity >= Decimal::ZERO && pool.allowable_cost_gbp >= Decimal::ZERO,
            "{path}: opening pool for {} must have non-negative quantity and cost",
            pool.asset
        );
    }
    Ok(file.pools)
}
