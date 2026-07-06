//! Typed configuration loading for TinoTax projects.
//!
//! This crate converts TOML config into validated Rust structs. Downstream
//! crates should depend on these typed records instead of parsing TOML
//! themselves.
pub mod project_config;

pub use project_config::{
    CexCsvEntry, CexPlatform, ConfigError, ProjectConfig, ProjectSection, ProviderEntry,
    ProviderKind, WalletEntry,
};
