//! Environment and provider health checks for local development.
//!
//! `doctor` is safe to run before a project exists. It reports the CLI
//! version, optional config validation, provider reachability, and whether the
//! expected API-key environment variables are present.

use anyhow::Result;
use camino::Utf8PathBuf;
use tinotax_config::{ProjectConfig, ProviderKind};
use tinotax_connectors::HttpClient;

/// Environment sanity checks. Reports findings and always exits cleanly —
/// doctor diagnoses, it does not fail the build.
pub async fn doctor() -> Result<()> {
    println!("tinotax {}", env!("CARGO_PKG_VERSION"));

    let config_path = Utf8PathBuf::from("wallets.toml");
    if !config_path.exists() {
        println!(
            "config: no wallets.toml in the current directory (pass --config to other commands)"
        );
        return Ok(());
    }

    match ProjectConfig::load(&config_path) {
        Err(err) => {
            println!("config: wallets.toml FAILED to parse/validate:\n  {err}");
            return Ok(());
        }
        Ok(config) => {
            println!(
                "config: wallets.toml ok — project {:?}, {} wallet(s), {} provider(s)",
                config.project.name,
                config.wallets.len(),
                config.providers.len()
            );

            if std::env::var("NEARBLOCKS_API_KEY").is_ok() {
                println!("env: NEARBLOCKS_API_KEY is set");
            } else if config
                .providers
                .values()
                .any(|p| p.kind == ProviderKind::Nearblocks)
            {
                println!(
                    "env: NEARBLOCKS_API_KEY not set — NEAR fetching will use the slow anonymous tier"
                );
            }

            if std::env::var("COINGECKO_PRO_API_KEY").is_ok() {
                println!("env: COINGECKO_PRO_API_KEY is set");
            } else if std::env::var("COINGECKO_DEMO_API_KEY").is_ok()
                || std::env::var("COINGECKO_API_KEY").is_ok()
            {
                println!("env: COINGECKO_API_KEY is set");
            } else {
                println!(
                    "env: no CoinGecko API key set — `prices fetch` may fail on historical GBP pricing"
                );
            }

            let http = HttpClient::new()?;
            for (name, provider) in &config.providers {
                let url = format!("{}/stats", provider.base_url.trim_end_matches('/'));
                match http.get_json_once(&url, &[], &[]).await {
                    Ok(_) => println!("provider {name}: reachable ({url})"),
                    Err(err) => println!("provider {name}: NOT reachable — {err}"),
                }
            }
        }
    }

    Ok(())
}
