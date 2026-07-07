//! Minimal, defensive views over Blockscout v2 payloads. Every field is
//! optional except identities: raw JSON is the source of truth and is kept
//! on disk; these structs only need to be good enough to normalise.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Page {
    #[serde(default)]
    pub items: Vec<serde_json::Value>,
    #[serde(default)]
    pub next_page_params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub hash: String,
    #[serde(default)]
    pub block_number: Option<serde_json::Value>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub from: Option<AddressField>,
    #[serde(default)]
    pub to: Option<AddressField>,
    /// Native value in wei, as a string.
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub fee: Option<Fee>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tx_types: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct AddressField {
    #[serde(default)]
    pub hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Fee {
    #[serde(default)]
    pub value: Option<String>,
}

/// One internal call frame from `/addresses/{hash}/internal-transactions`.
/// Blockscout returns only sub-calls that touch the address — the top-level
/// call is reported by the `transactions` endpoint, so value seen here is
/// never a duplicate of the transaction's own `value`.
#[derive(Debug, Deserialize)]
pub struct InternalTransaction {
    #[serde(default, alias = "tx_hash")]
    pub transaction_hash: Option<String>,
    /// Position of the call frame within its transaction.
    #[serde(default)]
    pub index: Option<serde_json::Value>,
    #[serde(default)]
    pub block_number: Option<serde_json::Value>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub from: Option<AddressField>,
    #[serde(default)]
    pub to: Option<AddressField>,
    /// Native value in wei, as a string.
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub success: Option<bool>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenTransfer {
    #[serde(default, alias = "tx_hash")]
    pub transaction_hash: Option<String>,
    #[serde(default)]
    pub log_index: Option<serde_json::Value>,
    #[serde(default)]
    pub block_number: Option<serde_json::Value>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub from: Option<AddressField>,
    #[serde(default)]
    pub to: Option<AddressField>,
    #[serde(default)]
    pub token: Option<TokenInfo>,
    #[serde(default)]
    pub total: Option<TransferTotal>,
}

#[derive(Debug, Deserialize)]
pub struct TokenInfo {
    #[serde(default, alias = "address_hash")]
    pub address: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    /// Blockscout serialises decimals as a string.
    #[serde(default)]
    pub decimals: Option<String>,
    #[serde(default, rename = "type")]
    pub token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TransferTotal {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub decimals: Option<String>,
    #[serde(default)]
    pub token_id: Option<String>,
}
