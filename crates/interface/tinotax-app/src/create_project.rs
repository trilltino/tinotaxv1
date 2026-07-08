//! Create a project from just a wallet address.
//!
//! The desktop onboarding path: paste an address, and TinoTax probes the
//! chains it can fetch, writes a validated project config, and hands back a
//! path the caller can run the startup workflow against. No pre-existing
//! folder layout or hand-written TOML required, so it works on any machine.

use anyhow::{anyhow, bail, Result};
use camino::Utf8PathBuf;
use serde::Serialize;
use tinotax_config::ProjectConfig;
use tinotax_connectors::HttpClient;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectResult {
    /// Standalone config the startup workflow initialises the project from.
    pub config_path: String,
    /// Folder the project (raw/, staging/, out/, project.toml) will live in.
    pub project_path: String,
    pub name: String,
    pub detected: Vec<DetectedChainDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedChainDto {
    pub chain: String,
    pub label: String,
    pub address: String,
}

/// One chain TinoTax can fetch, with everything needed to write its config.
struct KnownChain {
    chain: &'static str,
    label: &'static str,
    provider_id: &'static str,
    provider_kind: &'static str,
    base_url: &'static str,
    wallet_id: &'static str,
    wallet_name: &'static str,
}

const LISK: KnownChain = KnownChain {
    chain: "lisk-evm",
    label: "Lisk EVM",
    provider_id: "lisk_blockscout",
    provider_kind: "blockscout",
    base_url: "https://blockscout.lisk.com/api/v2",
    wallet_id: "lisk_main",
    wallet_name: "Lisk EVM wallet",
};
const IOTA: KnownChain = KnownChain {
    chain: "iota-evm",
    label: "IOTA EVM",
    provider_id: "iota_blockscout",
    provider_kind: "blockscout",
    base_url: "https://explorer.evm.iota.org/api/v2",
    wallet_id: "iota_main",
    wallet_name: "IOTA EVM wallet",
};
const NEAR: KnownChain = KnownChain {
    chain: "near",
    label: "NEAR",
    provider_id: "nearblocks",
    provider_kind: "nearblocks",
    base_url: "https://api.nearblocks.io/v1",
    wallet_id: "near_main",
    wallet_name: "NEAR wallet",
};

const ETHEREUM: KnownChain = KnownChain {
    chain: "ethereum-evm",
    label: "Ethereum",
    provider_id: "ethereum_blockscout",
    provider_kind: "blockscout",
    base_url: "https://eth.blockscout.com/api/v2",
    wallet_id: "ethereum_main",
    wallet_name: "Ethereum wallet",
};
const BASE: KnownChain = KnownChain {
    chain: "base-evm",
    label: "Base",
    provider_id: "base_blockscout",
    provider_kind: "blockscout",
    base_url: "https://base.blockscout.com/api/v2",
    wallet_id: "base_main",
    wallet_name: "Base wallet",
};
const ARBITRUM: KnownChain = KnownChain {
    chain: "arbitrum-evm",
    label: "Arbitrum",
    provider_id: "arbitrum_blockscout",
    provider_kind: "blockscout",
    base_url: "https://arbitrum.blockscout.com/api/v2",
    wallet_id: "arbitrum_main",
    wallet_name: "Arbitrum wallet",
};
const GNOSIS: KnownChain = KnownChain {
    chain: "gnosis-evm",
    label: "Gnosis",
    provider_id: "gnosis_blockscout",
    provider_kind: "blockscout",
    base_url: "https://gnosis.blockscout.com/api/v2",
    wallet_id: "gnosis_main",
    wallet_name: "Gnosis wallet",
};

/// EVM chains with a public Blockscout v2 API that TinoTax can fetch. Each
/// has a matching native symbol in `Chain::native_symbol` — don't add one
/// without the other, or its native transfers normalise as "NATIVE".
const EVM_CHAINS: &[&KnownChain] = &[&LISK, &IOTA, &ETHEREUM, &BASE, &ARBITRUM, &GNOSIS];

enum AddressKind {
    Evm,
    Near,
    Unknown,
}

/// Probe the supported chains for `address`, write a config into `parent_dir`,
/// and return the paths + what was detected. Does not fetch — the caller runs
/// the startup workflow against `config_path`/`project_path`.
pub async fn desktop_create_project_from_address(
    parent_dir: &str,
    address: &str,
    name: Option<&str>,
) -> Result<CreateProjectResult> {
    let address = address.trim();
    if address.is_empty() {
        bail!("enter a wallet address");
    }

    let http = HttpClient::new()?;
    let detected: Vec<&KnownChain> = match classify_address(address) {
        AddressKind::Evm => {
            // Probe every supported EVM chain at once so one slow or down
            // explorer can't stall the whole detection.
            let mut set = tokio::task::JoinSet::new();
            for chain in EVM_CHAINS {
                let http = http.clone();
                let base_url = chain.base_url;
                let address = address.to_string();
                set.spawn(async move {
                    (base_url, evm_has_activity(&http, base_url, &address).await)
                });
            }
            let mut active: std::collections::HashSet<&'static str> = std::collections::HashSet::new();
            while let Some(result) = set.join_next().await {
                if let Ok((base_url, true)) = result {
                    active.insert(base_url);
                }
            }
            // Rebuild in EVM_CHAINS order so the config is deterministic.
            let found: Vec<&KnownChain> = EVM_CHAINS
                .iter()
                .copied()
                .filter(|chain| active.contains(chain.base_url))
                .collect();
            if found.is_empty() {
                let names: Vec<&str> = EVM_CHAINS.iter().map(|chain| chain.label).collect();
                bail!(
                    "no activity found for {address} on the EVM chains TinoTax can fetch \
                     ({}). Double-check the address, or add the chain manually in the config.",
                    names.join(", ")
                );
            }
            found
        }
        // A NEAR-shaped account only exists on NEAR; NearBlocks needs a key, so
        // include it by shape rather than probing here (the fetch validates it).
        AddressKind::Near => vec![&NEAR],
        AddressKind::Unknown => bail!(
            "{address:?} doesn't look like an EVM (0x…) or NEAR (name.near) address"
        ),
    };

    let name = name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_name(address, &detected));
    let slug = slugify(&name);
    if slug.is_empty() {
        bail!("project name must contain letters or digits");
    }

    let parent = Utf8PathBuf::from(parent_dir);
    std::fs::create_dir_all(&parent)
        .map_err(|e| anyhow!("creating projects folder {parent}: {e}"))?;
    let config_path = parent.join(format!("{slug}.toml"));
    let project_path = parent.join(&slug);
    if config_path.exists() || project_path.join("project.toml").exists() {
        bail!("a project called {slug:?} already exists in {parent} — pick another name");
    }

    let toml = build_project_toml(&name, address, &detected);
    std::fs::write(&config_path, &toml).map_err(|e| anyhow!("writing {config_path}: {e}"))?;
    // The generated config must round-trip through full validation.
    if let Err(err) = ProjectConfig::load(&config_path) {
        let _ = std::fs::remove_file(&config_path);
        bail!("generated config was invalid: {err}");
    }

    Ok(CreateProjectResult {
        config_path: config_path.to_string(),
        project_path: project_path.to_string(),
        name,
        detected: detected
            .iter()
            .map(|c| DetectedChainDto {
                chain: c.chain.to_string(),
                label: c.label.to_string(),
                address: address.to_string(),
            })
            .collect(),
    })
}

fn classify_address(address: &str) -> AddressKind {
    let a = address.trim();
    if a.len() == 42 && a.starts_with("0x") && a[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        AddressKind::Evm
    } else if is_near_account(a) {
        AddressKind::Near
    } else {
        AddressKind::Unknown
    }
}

fn is_near_account(address: &str) -> bool {
    let a = address.to_ascii_lowercase();
    // Implicit account: 64 lowercase hex chars.
    if a.len() == 64 && a.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    // Named account: allowed characters, and a dotted suffix (.near, .tg, …).
    let charset_ok = a
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'));
    charset_ok && a.contains('.') && a.len() >= 3 && !a.starts_with('.') && !a.ends_with('.')
}

async fn evm_has_activity(http: &HttpClient, base_url: &str, address: &str) -> bool {
    let url = format!("{base_url}/addresses/{address}/transactions");
    match http.get_json_once(&url, &[], &[]).await {
        Ok(body) => body
            .get("items")
            .and_then(|items| items.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false),
        Err(_) => false,
    }
}

fn default_name(address: &str, detected: &[&KnownChain]) -> String {
    let prefix = detected.first().map(|c| c.chain).unwrap_or("wallet");
    if address.starts_with("0x") && address.len() >= 8 {
        format!("{prefix} {}", &address[..8])
    } else {
        format!("{prefix} {address}")
    }
}

fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

fn build_project_toml(name: &str, address: &str, detected: &[&KnownChain]) -> String {
    let mut s = String::new();
    s.push_str("# Generated by TinoTax — new project from a wallet address.\n");
    s.push_str("# Edit freely: add wallets/CEX exports, then re-run the workflow.\n\n");

    s.push_str("[project]\n");
    push_kv(&mut s, "name", name);
    push_kv(&mut s, "base_currency", "GBP");
    push_kv(&mut s, "period_start", "2017-01-01T00:00:00Z");
    push_kv(&mut s, "period_end", &tinotax_store::now_rfc3339());
    s.push('\n');

    for c in detected {
        s.push_str("[[wallets]]\n");
        push_kv(&mut s, "id", c.wallet_id);
        push_kv(&mut s, "name", c.wallet_name);
        push_kv(&mut s, "chain", c.chain);
        push_kv(&mut s, "address", address);
        push_kv(&mut s, "provider", c.provider_id);
        s.push('\n');
    }

    let mut seen = std::collections::BTreeSet::new();
    for c in detected {
        if seen.insert(c.provider_id) {
            s.push_str(&format!("[providers.{}]\n", c.provider_id));
            push_kv(&mut s, "kind", c.provider_kind);
            push_kv(&mut s, "base_url", c.base_url);
            s.push('\n');
        }
    }
    s
}

fn push_kv(text: &mut String, key: &str, value: &str) {
    text.push_str(key);
    text.push_str(" = ");
    text.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => text.push_str("\\\\"),
            '"' => text.push_str("\\\""),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            c => text.push(c),
        }
    }
    text.push_str("\"\n");
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn classifies_addresses() {
        assert!(matches!(
            classify_address("0x1b4399A7c97ae092fB4CCDc1598b2767ECB79652"),
            AddressKind::Evm
        ));
        assert!(matches!(classify_address("foxboss.near"), AddressKind::Near));
        assert!(matches!(classify_address("alice.tg"), AddressKind::Near));
        assert!(matches!(classify_address("not an address"), AddressKind::Unknown));
        assert!(matches!(classify_address("0xdeadbeef"), AddressKind::Unknown));
    }

    #[test]
    fn slugifies() {
        assert_eq!(slugify("Fox — Lisk Wallet!"), "fox-lisk-wallet");
        assert_eq!(slugify("  spaces  "), "spaces");
        assert_eq!(slugify("###"), "");
    }

    #[test]
    fn builds_a_valid_multi_chain_config() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let path = camino::Utf8PathBuf::from_path_buf(tmp.path().join("gen.toml"))
            .map_err(|p| std::io::Error::other(format!("non-utf8 {}", p.display())))?;
        let toml = build_project_toml(
            "Fox multichain",
            "0x1b4399A7c97ae092fB4CCDc1598b2767ECB79652",
            &[&LISK, &IOTA],
        );
        std::fs::write(&path, toml)?;

        let config = ProjectConfig::load(&path)?;
        assert_eq!(config.project.name, "Fox multichain");
        assert_eq!(config.project.base_currency, "GBP");
        assert_eq!(config.wallets.len(), 2);
        assert_eq!(config.wallets[0].chain, "lisk-evm");
        assert_eq!(config.wallets[1].chain, "iota-evm");
        // Both EVM wallets share the address; providers are declared once each.
        assert_eq!(config.wallets[0].address, config.wallets[1].address);
        assert_eq!(config.providers.len(), 2);
        Ok(())
    }
}
