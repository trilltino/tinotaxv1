use std::collections::BTreeMap;

use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tinotax_core::WalletSource;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read config {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot parse config {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: Box<toml::de::Error>,
    },

    #[error("wallet {wallet_id:?} references unknown provider {provider:?} (declared providers: {known:?})")]
    UnknownProvider {
        wallet_id: String,
        provider: String,
        known: Vec<String>,
    },

    #[error("duplicate wallet id {0:?}")]
    DuplicateWalletId(String),

    #[error("duplicate cex_csvs id {0:?}")]
    DuplicateCexId(String),

    #[error("cex_csvs {id:?} uses platform = \"generic\" but has no [cex_csvs.mapping] table")]
    GenericCexWithoutMapping { id: String },

    #[error("config declares no wallets")]
    NoWallets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectSection,
    #[serde(default)]
    pub wallets: Vec<WalletEntry>,
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderEntry>,
    #[serde(default)]
    pub cex_csvs: Vec<CexCsvEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    pub name: String,
    pub base_currency: String,
    pub period_start: String,
    pub period_end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletEntry {
    pub id: String,
    pub name: String,
    pub chain: String,
    pub address: String,
    pub provider: String,
}

impl WalletEntry {
    pub fn to_source(&self) -> WalletSource {
        WalletSource {
            id: self.id.clone(),
            name: self.name.clone(),
            chain: self.chain.clone(),
            address: self.address.clone(),
            provider: self.provider.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub kind: ProviderKind,
    pub base_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Blockscout,
    Nearblocks,
}

/// One CEX CSV export to import. The original file is copied unedited into
/// `raw/cex/<id>/` and hashed — HMRC asks for full, unedited trading data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CexCsvEntry {
    pub id: String,
    pub platform: CexPlatform,
    /// Path to the export, relative to wherever the command is run.
    pub path: String,
    /// For `platform = "generic"`: maps canonical column names
    /// (timestamp, type, asset, amount, fee_asset, fee_amount, note)
    /// onto the CSV's actual headers.
    #[serde(default)]
    pub mapping: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CexPlatform {
    Binance,
    Coinbase,
    Kraken,
    Awaken,
    Generic,
}

impl CexPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Binance => "binance",
            Self::Coinbase => "coinbase",
            Self::Kraken => "kraken",
            Self::Awaken => "awaken",
            Self::Generic => "generic",
        }
    }
}

impl ProjectConfig {
    pub fn load(path: &Utf8Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_string(),
            source,
        })?;
        let config: Self = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_string(),
            source: Box::new(source),
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.wallets.is_empty() {
            return Err(ConfigError::NoWallets);
        }
        let mut seen = std::collections::BTreeSet::new();
        for wallet in &self.wallets {
            if !seen.insert(&wallet.id) {
                return Err(ConfigError::DuplicateWalletId(wallet.id.clone()));
            }
            if !self.providers.contains_key(&wallet.provider) {
                return Err(ConfigError::UnknownProvider {
                    wallet_id: wallet.id.clone(),
                    provider: wallet.provider.clone(),
                    known: self.providers.keys().cloned().collect(),
                });
            }
        }
        let mut seen_cex = std::collections::BTreeSet::new();
        for cex in &self.cex_csvs {
            if !seen_cex.insert(&cex.id) {
                return Err(ConfigError::DuplicateCexId(cex.id.clone()));
            }
            if cex.platform == CexPlatform::Generic && cex.mapping.is_none() {
                return Err(ConfigError::GenericCexWithoutMapping { id: cex.id.clone() });
            }
        }
        Ok(())
    }

    pub fn provider_for(&self, wallet: &WalletEntry) -> &ProviderEntry {
        // validate() guarantees presence.
        &self.providers[&wallet.provider]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[project]
name = "demo"
base_currency = "GBP"
period_start = "2017-01-01T00:00:00Z"
period_end = "2025-04-05T23:59:59Z"

[[wallets]]
id = "near_foxboss"
name = "NEAR foxboss"
chain = "near"
address = "foxboss.near"
provider = "nearblocks"

[providers.nearblocks]
kind = "nearblocks"
base_url = "https://api.nearblocks.io/v1"
"#;

    #[test]
    fn parses_and_validates_sample() {
        let config: ProjectConfig = toml::from_str(SAMPLE).unwrap();
        config.validate().unwrap();
        assert_eq!(config.wallets.len(), 1);
        assert_eq!(
            config.provider_for(&config.wallets[0]).kind,
            ProviderKind::Nearblocks
        );
    }

    #[test]
    fn rejects_unknown_provider() {
        let broken = SAMPLE.replace("provider = \"nearblocks\"", "provider = \"missing\"");
        let config: ProjectConfig = toml::from_str(&broken).unwrap();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::UnknownProvider { .. })
        ));
    }
}
