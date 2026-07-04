use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// One normalised, source-traceable movement of value (or fee, or opaque
/// contract interaction) affecting one wallet.
///
/// Amounts are always positive; `direction` carries in/out. The untouched
/// chain amount is kept in `raw_amount` so nothing is lost to scaling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalisedEvent {
    /// Deterministic: blake3(chain|wallet|tx_hash|log_index|movement_index|asset|amount|direction).
    /// Re-running the import yields the same ID for the same event.
    pub event_id: String,

    pub project_id: String,
    pub source_id: String,
    pub source_kind: SourceKind,

    pub chain: String,
    pub wallet: String,

    /// RFC 3339.
    pub timestamp: String,
    pub block_number: Option<u64>,
    pub tx_hash: String,

    pub event_type: EventType,
    pub direction: Direction,

    pub asset_symbol: String,
    pub asset_contract: Option<String>,

    /// Human-readable exact amount (always >= 0).
    pub amount: Decimal,

    /// Raw chain amount (base units), where available.
    pub raw_amount: Option<String>,
    pub token_decimals: Option<u8>,

    pub from_address: Option<String>,
    pub to_address: Option<String>,

    pub fee_asset: Option<String>,
    pub fee_amount: Option<Decimal>,

    pub counterparty: Option<String>,
    pub method: Option<String>,

    pub confidence: Confidence,
    pub needs_review: bool,
    pub review_reasons: Vec<String>,

    pub source_ref: SourceRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Wallet,
    CexCsv,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    NativeTransfer,
    TokenTransfer,
    ContractCall,
    Fee,
    PossibleSwap,
    PossibleBridge,
    PossibleAirdrop,
    PossibleStakingReward,
    Unknown,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NativeTransfer => "native_transfer",
            Self::TokenTransfer => "token_transfer",
            Self::ContractCall => "contract_call",
            Self::Fee => "fee",
            Self::PossibleSwap => "possible_swap",
            Self::PossibleBridge => "possible_bridge",
            Self::PossibleAirdrop => "possible_airdrop",
            Self::PossibleStakingReward => "possible_staking_reward",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    In,
    Out,
    SelfTransfer,
    Unknown,
}

impl Direction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
            Self::SelfTransfer => "self_transfer",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// Pointer from a normalised event back to the exact raw evidence it came from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    /// Path relative to the project root, e.g. `raw/near/foxboss.near/transactions/page_000001.json`.
    pub raw_file: String,
    pub raw_page: Option<u64>,
    /// JSON pointer-ish locator inside the page, e.g. `items[17]`.
    pub json_path: Option<String>,
    pub log_index: Option<u64>,
    /// Disambiguates multiple events derived from one raw item (0 = value movement, 1 = fee).
    pub movement_index: Option<u64>,
}
