//! The reviewed tax ledger layer: what the UK tax engine consumes.
//!
//! A [`TaxLedgerEvent`] is **derived** data: it is built from normalised
//! events plus human review overrides, and is never edited by hand. Raw
//! evidence and `normalised_events.jsonl` are never mutated; every human
//! change lives in `review_overrides.jsonl` and is re-applied on each
//! `ledger build`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::error::CoreError;
use crate::event::{Direction, EventType};
use crate::review::ReviewAction;

/// UK tax treatment of one ledger event. This is the reviewer-facing
/// vocabulary: machine classification only ever *suggests* one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxEventType {
    /// Bought or otherwise acquired for consideration; enters the pool at cost.
    Acquisition,
    /// Sold for fiat or otherwise disposed of; CGT disposal.
    Disposal,
    /// The "sell" leg of a crypto-to-crypto swap; CGT disposal at market value.
    SwapDisposal,
    /// The "buy" leg of a crypto-to-crypto swap; enters the pool at market value.
    SwapAcquisition,
    /// Movement between the user's own wallets/accounts; not taxable.
    TransferIn,
    /// Movement between the user's own wallets/accounts; not taxable.
    TransferOut,
    /// Bridge receipt of the same economic asset; treated as non-taxable transfer.
    BridgeIn,
    /// Bridge send of the same economic asset; treated as non-taxable transfer.
    BridgeOut,
    /// Network/exchange fee paid in crypto; a small disposal of the fee asset.
    Fee,
    /// Staking reward: income at receipt, enters the pool at that value.
    StakingReward,
    /// Mining reward: income at receipt, enters the pool at that value.
    MiningReward,
    /// Airdrop not received in return for anything: acquisition at market
    /// value (not income by default — see assumptions).
    Airdrop,
    /// Tokens from a chain fork. Base cost is user-supplied (apportioned);
    /// defaults to zero, never auto-priced.
    Fork,
    /// Crypto received from an employer: employment income at receipt.
    EmploymentIncome,
    /// Crypto received as self-employment/trading income.
    SelfEmploymentIncome,
    /// Miscellaneous income (HMRC's catch-all for e.g. some staking/lending).
    MiscIncome,
    /// Compensation/reimbursement for lost or stolen cryptoassets.
    Compensation,
    /// Paying for goods or services with crypto; a CGT disposal.
    GoodsOrServicesSpend,
    /// Human decided this row has no tax effect (dust, spam token, test tx).
    Ignore,
    /// Not yet classified; blocks tax calculation until resolved.
    Unknown,
}

impl TaxEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Acquisition => "acquisition",
            Self::Disposal => "disposal",
            Self::SwapDisposal => "swap_disposal",
            Self::SwapAcquisition => "swap_acquisition",
            Self::TransferIn => "transfer_in",
            Self::TransferOut => "transfer_out",
            Self::BridgeIn => "bridge_in",
            Self::BridgeOut => "bridge_out",
            Self::Fee => "fee",
            Self::StakingReward => "staking_reward",
            Self::MiningReward => "mining_reward",
            Self::Airdrop => "airdrop",
            Self::Fork => "fork",
            Self::EmploymentIncome => "employment_income",
            Self::SelfEmploymentIncome => "self_employment_income",
            Self::MiscIncome => "misc_income",
            Self::Compensation => "compensation",
            Self::GoodsOrServicesSpend => "goods_or_services_spend",
            Self::Ignore => "ignore",
            Self::Unknown => "unknown",
        }
    }

    /// Adds tokens to the Section 104 pool (or same-day/30-day matching).
    pub fn is_pool_entry(&self) -> bool {
        self.is_purchase_like() || self.is_income()
    }

    /// Acquired for consideration: cost basis is what was paid (market value
    /// for swap legs).
    pub fn is_purchase_like(&self) -> bool {
        matches!(
            self,
            Self::Acquisition | Self::SwapAcquisition | Self::Airdrop | Self::Fork
        )
    }

    /// Income at receipt: taxable value at market price, which then becomes
    /// the CGT cost basis.
    pub fn is_income(&self) -> bool {
        matches!(
            self,
            Self::StakingReward
                | Self::MiningReward
                | Self::EmploymentIncome
                | Self::SelfEmploymentIncome
                | Self::MiscIncome
                | Self::Compensation
        )
    }

    /// CGT disposal: removes tokens via same-day → 30-day → S104 matching.
    pub fn is_disposal(&self) -> bool {
        matches!(
            self,
            Self::Disposal | Self::SwapDisposal | Self::GoodsOrServicesSpend | Self::Fee
        )
    }

    /// No tax effect and no pool effect.
    pub fn is_non_taxable(&self) -> bool {
        matches!(
            self,
            Self::TransferIn | Self::TransferOut | Self::BridgeIn | Self::BridgeOut | Self::Ignore
        )
    }

    /// Machine suggestion from the normalised classification. Never a final
    /// decision: the reviewer can override every row.
    pub fn suggest(event_type: EventType, direction: Direction) -> Self {
        let incoming = matches!(direction, Direction::In);
        match event_type {
            EventType::NativeTransfer | EventType::TokenTransfer => match direction {
                Direction::In => Self::Acquisition,
                Direction::Out => Self::Disposal,
                Direction::SelfTransfer => Self::TransferOut,
                Direction::Unknown => Self::Unknown,
            },
            EventType::Fee => Self::Fee,
            EventType::PossibleSwap => {
                if incoming {
                    Self::SwapAcquisition
                } else {
                    Self::SwapDisposal
                }
            }
            EventType::PossibleBridge => {
                if incoming {
                    Self::BridgeIn
                } else {
                    Self::BridgeOut
                }
            }
            EventType::PossibleAirdrop => Self::Airdrop,
            EventType::PossibleStakingReward => Self::StakingReward,
            EventType::ContractCall | EventType::Unknown => Self::Unknown,
        }
    }

    /// Resolve a coarse [`ReviewAction`] (the milestone-1 review vocabulary)
    /// into a tax event type, using the event's direction where needed.
    pub fn from_review_action(
        action: ReviewAction,
        detected: EventType,
        direction: Direction,
    ) -> Self {
        let incoming = matches!(direction, Direction::In);
        match action {
            ReviewAction::Keep => Self::suggest(detected, direction),
            ReviewAction::Ignore => Self::Ignore,
            ReviewAction::Transfer => {
                if incoming {
                    Self::TransferIn
                } else {
                    Self::TransferOut
                }
            }
            ReviewAction::Swap => {
                if incoming {
                    Self::SwapAcquisition
                } else {
                    Self::SwapDisposal
                }
            }
            ReviewAction::Bridge => {
                if incoming {
                    Self::BridgeIn
                } else {
                    Self::BridgeOut
                }
            }
            ReviewAction::StakingReward => Self::StakingReward,
            ReviewAction::Airdrop => Self::Airdrop,
            ReviewAction::Income => Self::MiscIncome,
            ReviewAction::Fee => Self::Fee,
            ReviewAction::Unknown => Self::Unknown,
        }
    }
}

impl FromStr for TaxEventType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "acquisition" => Ok(Self::Acquisition),
            "disposal" => Ok(Self::Disposal),
            "swap_disposal" => Ok(Self::SwapDisposal),
            "swap_acquisition" => Ok(Self::SwapAcquisition),
            "transfer_in" => Ok(Self::TransferIn),
            "transfer_out" => Ok(Self::TransferOut),
            "bridge_in" => Ok(Self::BridgeIn),
            "bridge_out" => Ok(Self::BridgeOut),
            "fee" => Ok(Self::Fee),
            "staking_reward" => Ok(Self::StakingReward),
            "mining_reward" => Ok(Self::MiningReward),
            "airdrop" => Ok(Self::Airdrop),
            "fork" => Ok(Self::Fork),
            "employment_income" => Ok(Self::EmploymentIncome),
            "self_employment_income" => Ok(Self::SelfEmploymentIncome),
            "misc_income" | "income" => Ok(Self::MiscIncome),
            "compensation" => Ok(Self::Compensation),
            "goods_or_services_spend" | "spend" => Ok(Self::GoodsOrServicesSpend),
            "ignore" => Ok(Self::Ignore),
            "unknown" => Ok(Self::Unknown),
            other => Err(CoreError::UnknownTaxEventType(other.to_string())),
        }
    }
}

/// How a ledger row got its classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    /// Machine classification, high confidence, no override.
    Auto,
    /// Machine classification that was flagged for review and has no override yet.
    NeedsReview,
    /// A human decision from `review_overrides.jsonl` was applied.
    Reviewed,
}

impl ReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::NeedsReview => "needs_review",
            Self::Reviewed => "reviewed",
        }
    }
}

/// Where a GBP valuation came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    /// The reviewer typed the GBP value into the review CSV.
    UserProvided,
    /// Imported from a manual price CSV.
    Manual,
    /// Spot price captured from a CEX export (e.g. Coinbase GBP columns).
    Cex,
    /// Fetched from CoinGecko's daily history API.
    Coingecko,
}

impl PriceSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserProvided => "user_provided",
            Self::Manual => "manual",
            Self::Cex => "cex",
            Self::Coingecko => "coingecko",
        }
    }
}

impl FromStr for PriceSource {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "user_provided" | "user" => Ok(Self::UserProvided),
            "manual" => Ok(Self::Manual),
            "cex" => Ok(Self::Cex),
            "coingecko" => Ok(Self::Coingecko),
            other => Err(CoreError::UnknownPriceSource(other.to_string())),
        }
    }
}

/// Confidence in a GBP valuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceConfidence {
    /// Same-day observation or user-provided value.
    High,
    /// Nearby-day observation used as a stand-in.
    Medium,
    /// Weak stand-in; review recommended.
    Low,
    /// No GBP value available; blocks tax calculation for taxable rows.
    Missing,
}

impl PriceConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Missing => "missing",
        }
    }
}

/// One row of the reviewed (and, after `ledger price`, priced) tax ledger.
///
/// Derived from `NormalisedEvent` + review overrides; regenerated by
/// `ledger build`, GBP-valued by `ledger price`, consumed by `calculate uk`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxLedgerEvent {
    /// Deterministic: derived from the source event id.
    pub ledger_event_id: String,

    pub source_event_ids: Vec<String>,
    pub source_refs: Vec<crate::event::SourceRef>,

    /// RFC 3339.
    pub timestamp: String,
    /// UK tax year label, e.g. `2024-2025` (6 April 2024 – 5 April 2025).
    pub tax_year: String,

    /// Exchange/platform for CEX rows (e.g. `binance`), `None` for wallets.
    pub platform: Option<String>,
    pub chain: Option<String>,
    pub wallet: Option<String>,
    pub tx_hash: Option<String>,

    pub tax_event_type: TaxEventType,

    pub asset_symbol: String,
    pub asset_contract: Option<String>,
    /// Always >= 0; the type carries the direction of the tax effect.
    pub quantity: Decimal,

    pub proceeds_gbp: Option<Decimal>,
    pub cost_gbp: Option<Decimal>,
    pub income_gbp: Option<Decimal>,
    pub fee_gbp: Option<Decimal>,

    pub price_source: Option<String>,
    pub price_confidence: PriceConfidence,

    pub review_status: ReviewStatus,
    pub user_note: Option<String>,
}

/// The UK tax year (6 April – 5 April) containing an RFC 3339 timestamp,
/// as a label like `2024-2025`. Uses the UTC date; see assumptions doc.
pub fn uk_tax_year(timestamp: &str) -> Result<String, CoreError> {
    let (year, month, day) = parse_date_prefix(timestamp)?;
    // On/after 6 April: tax year starts this calendar year.
    let start = if (month, day) >= (4, 6) {
        year
    } else {
        year - 1
    };
    Ok(format!("{start}-{}", start + 1))
}

/// Parse the `YYYY-MM-DD` prefix of an RFC 3339 timestamp.
pub fn parse_date_prefix(timestamp: &str) -> Result<(i32, u32, u32), CoreError> {
    let date = timestamp.get(..10).unwrap_or(timestamp);
    let invalid = || CoreError::InvalidTimestamp(timestamp.to_string());
    let mut parts = date.splitn(3, '-');
    let year: i32 = parts
        .next()
        .and_then(|p| p.parse().ok())
        .ok_or_else(invalid)?;
    let month: u32 = parts
        .next()
        .and_then(|p| p.parse().ok())
        .ok_or_else(invalid)?;
    let day: u32 = parts
        .next()
        .and_then(|p| p.parse().ok())
        .ok_or_else(invalid)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(invalid());
    }
    Ok((year, month, day))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn tax_year_boundaries() -> Result<(), Box<dyn Error>> {
        assert_eq!(uk_tax_year("2024-04-05T23:59:59Z")?, "2023-2024");
        assert_eq!(uk_tax_year("2024-04-06T00:00:00Z")?, "2024-2025");
        assert_eq!(uk_tax_year("2025-01-01T12:00:00Z")?, "2024-2025");
        assert_eq!(uk_tax_year("2017-12-31T00:00:00Z")?, "2017-2018");
        Ok(())
    }

    #[test]
    fn rejects_garbage_timestamp() {
        assert!(uk_tax_year("not a date").is_err());
        assert!(uk_tax_year("2024-13-01T00:00:00Z").is_err());
    }

    #[test]
    fn tax_event_type_round_trips() -> Result<(), Box<dyn Error>> {
        for t in [
            TaxEventType::Acquisition,
            TaxEventType::SwapDisposal,
            TaxEventType::GoodsOrServicesSpend,
            TaxEventType::MiscIncome,
            TaxEventType::Unknown,
        ] {
            assert_eq!(t.as_str().parse::<TaxEventType>()?, t);
        }
        Ok(())
    }

    #[test]
    fn suggestion_follows_direction() {
        use crate::event::{Direction, EventType};
        assert_eq!(
            TaxEventType::suggest(EventType::PossibleSwap, Direction::In),
            TaxEventType::SwapAcquisition
        );
        assert_eq!(
            TaxEventType::suggest(EventType::TokenTransfer, Direction::Out),
            TaxEventType::Disposal
        );
        assert_eq!(
            TaxEventType::suggest(EventType::ContractCall, Direction::Unknown),
            TaxEventType::Unknown
        );
    }
}
