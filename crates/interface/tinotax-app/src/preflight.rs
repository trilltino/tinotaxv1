//! Fail-fast local startup checks.
//!
//! This command is intentionally stricter than `doctor`: it is used by
//! `just startup` before any fetch/import work begins.

use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use tinotax_config::{ProjectConfig, ProviderKind};

/// Validate the inputs and environment needed before a full local startup run.
pub fn preflight(config: &str, project: &str) -> Result<()> {
    let config_path = Utf8PathBuf::from(config);
    if !config_path.exists() {
        bail!("config file does not exist: {config_path}");
    }

    let config = ProjectConfig::load(&config_path)
        .with_context(|| format!("startup preflight failed for config {config_path}"))?;

    let project_path = Utf8PathBuf::from(project);
    if let Some(parent) = project_path.parent() {
        if !parent.as_str().is_empty() && !parent.exists() {
            bail!("project parent directory does not exist: {parent}");
        }
    }

    let mut failures = Vec::new();
    let mut warnings = Vec::new();

    if !config.project.base_currency.eq_ignore_ascii_case("GBP") {
        failures.push(format!(
            "project.base_currency is {:?}; TinoTax UK calculations currently require GBP",
            config.project.base_currency
        ));
    }

    for (name, provider) in &config.providers {
        if !(provider.base_url.starts_with("https://") || provider.base_url.starts_with("http://"))
        {
            failures.push(format!(
                "provider {name:?} has invalid base_url {:?}",
                provider.base_url
            ));
        }
    }

    if config
        .providers
        .values()
        .any(|provider| provider.kind == ProviderKind::Nearblocks)
        && env_key("NEARBLOCKS_API_KEY").is_none()
    {
        failures.push(
            "NEARBLOCKS_API_KEY is required for production startup with NearBlocks wallets"
                .to_string(),
        );
    }

    if env_key("COINGECKO_PRO_API_KEY")
        .or_else(|| env_key("COINGECKO_DEMO_API_KEY"))
        .or_else(|| env_key("COINGECKO_API_KEY"))
        .is_none()
    {
        warnings.push(
            "no CoinGecko API key found; startup can review data, but `prices fetch` will need one"
                .to_string(),
        );
    }

    for cex in &config.cex_csvs {
        let path = Utf8PathBuf::from(&cex.path);
        if !path.exists() {
            failures.push(format!(
                "cex_csvs {:?} input file does not exist: {}",
                cex.id, cex.path
            ));
        }
    }

    println!(
        "startup preflight: config {:?}, project {}, {} wallet(s), {} CEX import(s)",
        config.project.name,
        project_path,
        config.wallets.len(),
        config.cex_csvs.len()
    );

    for warning in &warnings {
        println!("[WARN] {warning}");
    }
    for failure in &failures {
        println!("[FAIL] {failure}");
    }

    if !failures.is_empty() {
        bail!("startup preflight failed: {} failure(s)", failures.len());
    }

    println!("startup preflight passed: {} warning(s)", warnings.len());
    Ok(())
}

fn env_key(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|key| !key.is_empty())
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn fails_for_missing_config() -> Result<(), Box<dyn Error>> {
        let err = match preflight("definitely-missing-wallets.toml", "./target/preflight-test") {
            Ok(()) => return Err(std::io::Error::other("missing config must fail").into()),
            Err(err) => err,
        };
        assert!(err.to_string().contains("config file does not exist"));
        Ok(())
    }
}
