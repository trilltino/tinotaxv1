//! Human review override model.
//!
//! Review overrides are append-only records that capture spreadsheet edits
//! without mutating raw or normalised source events.
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::error::CoreError;

/// What a human decided about an uncertain event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewAction {
    Keep,
    Ignore,
    Transfer,
    Swap,
    Bridge,
    StakingReward,
    Airdrop,
    Income,
    Fee,
    Unknown,
}

impl ReviewAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Ignore => "ignore",
            Self::Transfer => "transfer",
            Self::Swap => "swap",
            Self::Bridge => "bridge",
            Self::StakingReward => "staking_reward",
            Self::Airdrop => "airdrop",
            Self::Income => "income",
            Self::Fee => "fee",
            Self::Unknown => "unknown",
        }
    }
}

impl FromStr for ReviewAction {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "keep" => Ok(Self::Keep),
            "ignore" => Ok(Self::Ignore),
            "transfer" => Ok(Self::Transfer),
            "swap" => Ok(Self::Swap),
            "bridge" => Ok(Self::Bridge),
            "staking_reward" => Ok(Self::StakingReward),
            "airdrop" => Ok(Self::Airdrop),
            "income" => Ok(Self::Income),
            "fee" => Ok(Self::Fee),
            "unknown" => Ok(Self::Unknown),
            other => Err(CoreError::UnknownReviewAction(other.to_string())),
        }
    }
}

/// A row of `manual_review.csv` after the accountant/client edited it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRow {
    pub event_id: String,
    pub user_action: Option<ReviewAction>,
    pub user_note: Option<String>,
}

/// One accepted human decision, recorded append-only to
/// `staging/review_overrides.jsonl`.
///
/// This is the **only** place human changes live: raw data and
/// `normalised_events.jsonl` are never mutated. `ledger build` re-applies
/// the latest override per event on every run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewOverride {
    pub event_id: String,
    /// Coarse milestone-1 action (from `manual_review.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_action: Option<ReviewAction>,
    /// Precise tax classification (from `review_all_transactions.csv`);
    /// takes precedence over `user_action`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_tax_type: Option<crate::tax_event::TaxEventType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_asset_symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_quantity: Option<rust_decimal::Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_proceeds_gbp: Option<rust_decimal::Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_cost_gbp: Option<rust_decimal::Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_income_gbp: Option<rust_decimal::Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_fee_gbp: Option<rust_decimal::Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_price_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_note: Option<String>,
    pub applied_at: String,
    /// The edited CSV this decision came from (for the change log).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
}

impl ReviewOverride {
    /// Column names the reviewer actually filled in, for the change log.
    pub fn fields_set(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.user_action.is_some() {
            fields.push("user_action");
        }
        if self.user_tax_type.is_some() {
            fields.push("user_tax_type");
        }
        if self.user_asset_symbol.is_some() {
            fields.push("user_asset_symbol");
        }
        if self.user_quantity.is_some() {
            fields.push("user_quantity");
        }
        if self.user_proceeds_gbp.is_some() {
            fields.push("user_proceeds_gbp");
        }
        if self.user_cost_gbp.is_some() {
            fields.push("user_cost_gbp");
        }
        if self.user_income_gbp.is_some() {
            fields.push("user_income_gbp");
        }
        if self.user_fee_gbp.is_some() {
            fields.push("user_fee_gbp");
        }
        if self.user_price_source.is_some() {
            fields.push("user_price_source");
        }
        if self.user_note.is_some() {
            fields.push("user_note");
        }
        fields
    }

    /// True if the row carries at least one decision worth recording.
    pub fn has_any_decision(&self) -> bool {
        !self.fields_set().is_empty()
    }
}
