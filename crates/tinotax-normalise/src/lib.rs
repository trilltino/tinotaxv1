pub mod classify;
pub mod dedupe;
pub mod event_id;
pub mod evm;
pub mod near;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tinotax_core::{Chain, NormalisedEvent, WalletSource};
use tinotax_store::{JsonlWriter, ProjectPaths};
use tracing::{info, warn};

/// A raw item we could not turn into an event. Kept, never silently dropped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedItem {
    pub chain: String,
    pub wallet: String,
    pub raw_file: String,
    pub json_path: String,
    pub reason: String,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormaliseWarning {
    pub chain: String,
    pub wallet: String,
    pub message: String,
}

/// Accumulator threaded through the per-chain normalisers.
#[derive(Debug, Default)]
pub struct Batch {
    pub events: Vec<NormalisedEvent>,
    pub rejected: Vec<RejectedItem>,
    pub warnings: Vec<NormaliseWarning>,
}

impl Batch {
    pub fn warn(&mut self, chain: &str, wallet: &str, message: impl Into<String>) {
        self.warnings.push(NormaliseWarning {
            chain: chain.to_string(),
            wallet: wallet.to_string(),
            message: message.into(),
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormaliseSummary {
    pub total_events: u64,
    pub duplicates_dropped: u64,
    pub rejected_items: u64,
    pub warnings: u64,
    pub per_wallet: Vec<WalletEventCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletEventCount {
    pub source_id: String,
    pub chain: String,
    pub wallet: String,
    pub events: u64,
}

/// Read every cached raw page under `raw/`, produce
/// `staging/normalised_events.jsonl` (+ rejects and warnings). Derived
/// only — safe to delete staging and re-run.
pub fn normalise_project(
    paths: &ProjectPaths,
    project_id: &str,
    wallets: &[WalletSource],
) -> Result<NormaliseSummary> {
    let mut batch = Batch::default();

    for wallet in wallets {
        let before = batch.events.len();
        let chain = Chain::from(wallet.chain.clone());
        match chain {
            Chain::Near => near::normalise_near_wallet(paths, project_id, wallet, &mut batch)?,
            ref c if c.is_evm() => {
                evm::normalise_evm_wallet(paths, project_id, wallet, c, &mut batch)?
            }
            other => {
                warn!(chain = %other, wallet = %wallet.address, "no normaliser for chain");
                batch.warn(
                    &wallet.chain,
                    &wallet.address,
                    format!("no normaliser for chain {other}; raw data cached but not normalised"),
                );
            }
        }
        info!(
            wallet = %wallet.address,
            chain = %wallet.chain,
            events = batch.events.len() - before,
            "normalised wallet"
        );
    }

    let (mut events, duplicates_dropped) = dedupe::dedupe(std::mem::take(&mut batch.events));
    events.sort_by(|a, b| {
        (&a.timestamp, &a.tx_hash, &a.event_id).cmp(&(&b.timestamp, &b.tx_hash, &b.event_id))
    });

    let per_wallet = wallets
        .iter()
        .map(|w| WalletEventCount {
            source_id: w.id.clone(),
            chain: w.chain.clone(),
            wallet: w.address.clone(),
            events: events
                .iter()
                .filter(|e| e.source_id == w.id)
                .count() as u64,
        })
        .collect();

    std::fs::create_dir_all(paths.staging())?;
    let mut events_out = JsonlWriter::create(&paths.events_jsonl())?;
    for event in &events {
        events_out.write(event)?;
    }
    let total_events = events_out.finish()?;

    let mut rejected_out = JsonlWriter::create(&paths.rejected_jsonl())?;
    for item in &batch.rejected {
        rejected_out.write(item)?;
    }
    rejected_out.finish()?;

    let mut warnings_out = JsonlWriter::create(&paths.warnings_jsonl())?;
    for warning in &batch.warnings {
        warnings_out.write(warning)?;
    }
    warnings_out.finish()?;

    Ok(NormaliseSummary {
        total_events,
        duplicates_dropped,
        rejected_items: batch.rejected.len() as u64,
        warnings: batch.warnings.len() as u64,
        per_wallet,
    })
}
