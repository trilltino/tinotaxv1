//! `counterparties.csv` (HMRC question 5): the distinct on-chain contracts and
//! protocols the wallets interacted with — so exchanges/DEXs/protocols can be
//! named from the data rather than from memory. `platforms_protocols_used.csv`
//! lists chains only, which doesn't answer Q5 on its own.
//!
//! Each row is one contract address the wallet touched, with the methods
//! called, how many times, and the active window. Well-known addresses are
//! labelled; the rest are left blank for a human to name — the value here is
//! surfacing *which* contracts and how often, not guessing at identities.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use camino::Utf8Path;
use tinotax_core::{EventType, NormalisedEvent};

/// Best-effort names for addresses that are the same across many chains. Kept
/// deliberately small and honest — unknowns are flagged, not fabricated.
fn known_name(address: &str) -> &'static str {
    match address.to_ascii_lowercase().as_str() {
        "0x4200000000000000000000000000000000000006" => "Wrapped ETH (OP-stack WETH)",
        "0x0000000000000000000000000000000000000000" => "Zero address (burn/mint)",
        _ => "",
    }
}

/// Whether an event represents interacting with a contract/protocol (as opposed
/// to a plain value transfer between externally-owned accounts).
fn is_protocol_interaction(event: &NormalisedEvent) -> bool {
    event.method.is_some()
        || matches!(
            event.event_type,
            EventType::ContractCall | EventType::PossibleSwap | EventType::PossibleBridge
        )
}

struct Counterparty {
    chain: String,
    methods: BTreeSet<String>,
    event_count: u64,
    first_seen: String,
    last_seen: String,
}

/// Write `counterparties.csv`, one row per (chain, contract address), sorted by
/// interaction count. Returns the number of distinct counterparties.
pub fn write_counterparties(events: &[NormalisedEvent], dir: &Utf8Path) -> Result<u64> {
    let mut by_addr: BTreeMap<(String, String), Counterparty> = BTreeMap::new();
    for event in events {
        if !is_protocol_interaction(event) {
            continue;
        }
        let Some(address) = event
            .to_address
            .clone()
            .or_else(|| event.counterparty.clone())
        else {
            continue;
        };
        let entry = by_addr
            .entry((event.chain.clone(), address))
            .or_insert_with(|| Counterparty {
                chain: event.chain.clone(),
                methods: BTreeSet::new(),
                event_count: 0,
                first_seen: event.timestamp.clone(),
                last_seen: event.timestamp.clone(),
            });
        entry.event_count += 1;
        if let Some(method) = &event.method {
            entry.methods.insert(method.clone());
        }
        if event.timestamp < entry.first_seen {
            entry.first_seen = event.timestamp.clone();
        }
        if event.timestamp > entry.last_seen {
            entry.last_seen = event.timestamp.clone();
        }
    }

    let mut rows: Vec<((String, String), Counterparty)> = by_addr.into_iter().collect();
    rows.sort_by(|a, b| {
        b.1.event_count
            .cmp(&a.1.event_count)
            .then_with(|| a.0.cmp(&b.0))
    });

    let path = dir.join("counterparties.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record([
        "chain",
        "address",
        "name",
        "methods",
        "event_count",
        "first_seen",
        "last_seen",
    ])?;
    for ((_, address), cp) in &rows {
        let methods: Vec<&str> = cp.methods.iter().map(String::as_str).collect();
        writer.write_record([
            cp.chain.as_str(),
            address.as_str(),
            known_name(address),
            &methods.join("; "),
            &cp.event_count.to_string(),
            cp.first_seen.get(..10).unwrap_or(&cp.first_seen),
            cp.last_seen.get(..10).unwrap_or(&cp.last_seen),
        ])?;
    }
    writer.flush()?;
    Ok(rows.len() as u64)
}
