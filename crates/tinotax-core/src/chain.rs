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
            Self::Other(_) => "NATIVE",
        }
    }

    pub fn native_decimals(&self) -> u32 {
        match self {
            Self::Near => 24,
            _ => 18,
        }
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
