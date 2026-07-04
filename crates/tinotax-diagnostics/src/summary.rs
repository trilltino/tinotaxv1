use std::collections::BTreeMap;

use anyhow::{Context, Result};
use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use tinotax_core::{Direction, EventType, NormalisedEvent};

use crate::{assets, duplicates, review_flags};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostics {
    pub project: String,
    pub total_events: u64,
    pub wallets: Vec<WalletDiagnostics>,
    pub assets: Vec<assets::AssetDiagnostics>,
    pub review: review_flags::ReviewDiagnostics,
    /// Should always be zero after dedupe; non-zero means a pipeline bug.
    pub duplicate_event_ids: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletDiagnostics {
    pub chain: String,
    pub wallet: String,
    pub events: u64,
    pub in_events: u64,
    pub out_events: u64,
    pub self_transfers: u64,
    pub fees: u64,
    pub needs_review: u64,
}

pub fn compute(project: &str, events: &[NormalisedEvent]) -> Diagnostics {
    let mut wallets: BTreeMap<(String, String), WalletDiagnostics> = BTreeMap::new();
    for event in events {
        let entry = wallets
            .entry((event.chain.clone(), event.wallet.clone()))
            .or_insert_with(|| WalletDiagnostics {
                chain: event.chain.clone(),
                wallet: event.wallet.clone(),
                events: 0,
                in_events: 0,
                out_events: 0,
                self_transfers: 0,
                fees: 0,
                needs_review: 0,
            });
        entry.events += 1;
        if event.event_type == EventType::Fee {
            entry.fees += 1;
        } else {
            match event.direction {
                Direction::In => entry.in_events += 1,
                Direction::Out => entry.out_events += 1,
                Direction::SelfTransfer => entry.self_transfers += 1,
                Direction::Unknown => {}
            }
        }
        if event.needs_review {
            entry.needs_review += 1;
        }
    }

    Diagnostics {
        project: project.to_string(),
        total_events: events.len() as u64,
        wallets: wallets.into_values().collect(),
        assets: assets::compute(events),
        review: review_flags::compute(events),
        duplicate_event_ids: duplicates::count(events),
    }
}

pub fn write_wallet_summary_csv(path: &Utf8Path, diagnostics: &Diagnostics) -> Result<()> {
    let mut writer = csv::Writer::from_path(path).with_context(|| format!("creating {path}"))?;
    writer.write_record([
        "chain",
        "wallet",
        "total_events",
        "in_events",
        "out_events",
        "self_transfers",
        "fees",
        "needs_review",
    ])?;
    for w in &diagnostics.wallets {
        writer.write_record([
            w.chain.as_str(),
            w.wallet.as_str(),
            &w.events.to_string(),
            &w.in_events.to_string(),
            &w.out_events.to_string(),
            &w.self_transfers.to_string(),
            &w.fees.to_string(),
            &w.needs_review.to_string(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}
