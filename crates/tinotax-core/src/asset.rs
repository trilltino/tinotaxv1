use serde::{Deserialize, Serialize};

/// An asset as observed on-chain. Identity is (chain, contract) for tokens
/// and (chain, symbol) for natives — symbols alone are not trustworthy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Asset {
    pub chain: String,
    pub symbol: String,
    /// None for the chain's native asset.
    pub contract: Option<String>,
    pub decimals: Option<u8>,
}

impl Asset {
    pub fn native(chain: &str, symbol: &str, decimals: u8) -> Self {
        Self {
            chain: chain.to_string(),
            symbol: symbol.to_string(),
            contract: None,
            decimals: Some(decimals),
        }
    }
}
