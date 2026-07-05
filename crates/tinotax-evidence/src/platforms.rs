//! `platforms_protocols_used.csv` (HMRC question 5) and
//! `wallet_addresses.csv` — where the activity happened.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use camino::Utf8Path;
use tinotax_config::ProjectConfig;
use tinotax_core::TaxLedgerEvent;

#[derive(Default)]
struct PlatformSpan {
    first_seen: String,
    last_seen: String,
    event_count: u64,
}

pub fn write_platforms(ledger: &[TaxLedgerEvent], dir: &Utf8Path) -> Result<u64> {
    let mut spans: BTreeMap<(String, String), PlatformSpan> = BTreeMap::new();
    for event in ledger {
        let kind = if event.platform.is_some() {
            "exchange"
        } else {
            "chain"
        };
        let name = event
            .platform
            .clone()
            .or_else(|| event.chain.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let span = spans.entry((kind.to_string(), name)).or_default();
        if span.first_seen.is_empty() || event.timestamp < span.first_seen {
            span.first_seen = event.timestamp.clone();
        }
        if event.timestamp > span.last_seen {
            span.last_seen = event.timestamp.clone();
        }
        span.event_count += 1;
    }

    let path = dir.join("platforms_protocols_used.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record(["kind", "name", "first_seen", "last_seen", "event_count"])?;
    for ((kind, name), span) in &spans {
        writer.write_record([
            kind.as_str(),
            name.as_str(),
            span.first_seen.as_str(),
            span.last_seen.as_str(),
            &span.event_count.to_string(),
        ])?;
    }
    writer.flush()?;
    Ok(spans.len() as u64)
}

pub fn write_wallet_addresses(config: &ProjectConfig, dir: &Utf8Path) -> Result<()> {
    let path = dir.join("wallet_addresses.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record(["id", "name", "chain", "address", "provider"])?;
    for wallet in &config.wallets {
        writer.write_record([
            wallet.id.as_str(),
            wallet.name.as_str(),
            wallet.chain.as_str(),
            wallet.address.as_str(),
            wallet.provider.as_str(),
        ])?;
    }
    for cex in &config.cex_csvs {
        writer.write_record([
            cex.id.as_str(),
            cex.id.as_str(),
            cex.platform.as_str(),
            "(exchange account — see raw/cex/)",
            "csv_import",
        ])?;
    }
    writer.flush()?;
    Ok(())
}
