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
