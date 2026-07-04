//! Minimal, defensive views over NearBlocks v1 payloads.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TxnsPage {
    #[serde(default)]
    pub txns: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Txn {
    pub transaction_hash: String,
    /// Nanoseconds since epoch; NearBlocks sends it as a string or number.
    #[serde(default)]
    pub block_timestamp: Option<serde_json::Value>,
    #[serde(default)]
    pub signer_account_id: Option<String>,
    #[serde(default)]
    pub receiver_account_id: Option<String>,
    #[serde(default)]
    pub actions: Option<Vec<TxnAction>>,
    #[serde(default)]
    pub actions_agg: Option<ActionsAgg>,
    #[serde(default)]
    pub outcomes: Option<Outcomes>,
    #[serde(default)]
    pub block: Option<Block>,
}

#[derive(Debug, Deserialize)]
pub struct TxnAction {
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    /// yoctoNEAR
    #[serde(default)]
    pub deposit: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ActionsAgg {
    /// yoctoNEAR
    #[serde(default)]
    pub deposit: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Outcomes {
    #[serde(default)]
    pub status: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Block {
    #[serde(default)]
    pub block_height: Option<serde_json::Value>,
}
