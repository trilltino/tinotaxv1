//! Supported chain identifiers and parsing.
//!
//! Chain names are stored as stable string labels because they flow into raw
//! paths, source references, and review spreadsheets.
use serde::{Deserialize, Serialize};

/// Chains the demo knows about. Anything else is carried through as `Other`
/// rather than rejected — raw data must survive even if we can't classify it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum Chain {
    Near,
    LiskEvm,
    IotaEvm,
    Other(String),
}

impl Chain {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Near => "near",
            Self::LiskEvm => "lisk-evm",
            Self::IotaEvm => "iota-evm",
            Self::Other(s) => s,
        }
    }

    pub fn is_evm(&self) -> bool {
        matches!(self, Self::LiskEvm | Self::IotaEvm) || self.as_str().ends_with("-evm")
    }

    /// Native asset symbol used when normalising value transfers and gas fees.
    pub fn native_symbol(&self) -> &str {
        match self {
            Self::Near => "NEAR",
            Self::LiskEvm => "ETH",
            Self::IotaEvm => "IOTA",
            Self::Other(slug) => evm_native_symbol(slug),
        }
    }

    pub fn native_decimals(&self) -> u32 {
        match self {
            Self::Near => 24,
            _ => 18,
        }
    }
}

/// Native asset for the additional EVM chains `create project` can add by
/// slug (see `tinotax-app`'s address auto-detect). Unknown chains stay
/// "NATIVE" so a wrong symbol is never invented for data we can't classify.
fn evm_native_symbol(slug: &str) -> &'static str {
    match slug {
        "ethereum-evm" | "base-evm" | "optimism-evm" | "arbitrum-evm" => "ETH",
        "gnosis-evm" => "XDAI",
        _ => "NATIVE",
    }
}

impl From<String> for Chain {
    fn from(s: String) -> Self {
        match s.as_str() {
            "near" => Self::Near,
            "lisk-evm" => Self::LiskEvm,
            "iota-evm" => Self::IotaEvm,
            _ => Self::Other(s),
        }
    }
}

impl From<Chain> for String {
    fn from(c: Chain) -> Self {
        c.as_str().to_string()
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_evm_chains_route_and_report_native_symbols() {
        for slug in ["ethereum-evm", "base-evm", "arbitrum-evm", "gnosis-evm"] {
            assert!(Chain::from(slug.to_string()).is_evm(), "{slug} should be EVM");
        }
        assert_eq!(Chain::from("base-evm".to_string()).native_symbol(), "ETH");
        assert_eq!(Chain::from("gnosis-evm".to_string()).native_symbol(), "XDAI");
        // An unclassified chain never invents a symbol.
        assert_eq!(Chain::from("mystery".to_string()).native_symbol(), "NATIVE");
    }
}
