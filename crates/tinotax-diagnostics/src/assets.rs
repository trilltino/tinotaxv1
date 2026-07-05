//! Asset movement diagnostics.
//!
//! This module summarises observed asset quantities and directions so users
//! can spot unexpected tokens, missing rows, or one-sided activity before tax
//! calculation.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tinotax_core::{Direction, NormalisedEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDiagnostics {
    pub chain: String,
    pub symbol: String,
    pub contract: Option<String>,
    pub events: u64,
    pub in_count: u64,
    pub out_count: u64,
    pub self_transfer_count: u64,
}

pub fn compute(events: &[NormalisedEvent]) -> Vec<AssetDiagnostics> {
    // Keyed by (chain, symbol, contract) — symbols alone are spoofable.
    let mut assets: BTreeMap<(String, String, Option<String>), AssetDiagnostics> = BTreeMap::new();
    for event in events {
        let key = (
            event.chain.clone(),
            event.asset_symbol.clone(),
            event.asset_contract.clone(),
        );
        let entry = assets.entry(key).or_insert_with(|| AssetDiagnostics {
            chain: event.chain.clone(),
            symbol: event.asset_symbol.clone(),
            contract: event.asset_contract.clone(),
            events: 0,
            in_count: 0,
            out_count: 0,
            self_transfer_count: 0,
        });
        entry.events += 1;
        match event.direction {
            Direction::In => entry.in_count += 1,
            Direction::Out => entry.out_count += 1,
            Direction::SelfTransfer => entry.self_transfer_count += 1,
            Direction::Unknown => {}
        }
    }
    assets.into_values().collect()
}
