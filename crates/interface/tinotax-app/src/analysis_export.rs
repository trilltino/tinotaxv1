//! Analysis-ready CSV export.
//!
//! This file is deliberately denormalised: it joins normalised event context
//! with reviewed/priced ledger fields when those later pipeline stages exist.
//! Spreadsheet users and BI tools should be able to pivot from this one file.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use tinotax_core::{NormalisedEvent, TaxLedgerEvent};

use crate::open_project;

const ANALYSIS_COLUMNS: [&str; 47] = [
    "event_id",
    "ledger_event_id",
    "timestamp",
    "tax_year",
    "source_id",
    "source_kind",
    "platform",
    "chain",
    "wallet",
    "tx_hash",
    "block_number",
    "detected_event_type",
    "detected_direction",
    "tax_event_type",
    "asset_symbol",
    "asset_contract",
    "quantity",
    "raw_amount",
    "token_decimals",
    "from_address",
    "to_address",
    "counterparty",
    "method",
    "fee_asset",
    "fee_amount",
    "proceeds_gbp",
    "cost_gbp",
    "income_gbp",
    "fee_gbp",
    "price_source",
    "price_confidence",
    "review_status",
    "needs_review",
    "review_reasons",
    "user_note",
    "raw_file",
    "raw_page",
    "json_path",
    "log_index",
    "movement_index",
    "year",
    "month",
    "date",
    "signed_quantity",
    "gross_value_gbp",
    "activity_class",
    "visual_bucket",
];

/// Export `out/analysis_export.csv`, one wide row per normalised event.
pub fn export_analysis_csv(project: &str) -> Result<u64> {
    let (paths, _) = open_project(project)?;
    let events = tinotax_review::load_all_events(&paths)?;
    let ledger = load_best_ledger(&paths)?;
    let by_event = ledger_by_source_event(&ledger);

    std::fs::create_dir_all(paths.out())?;
    let path = paths.out().join("analysis_export.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record(ANALYSIS_COLUMNS)?;

    for event in &events {
        let row = by_event.get(&event.event_id);
        writer.write_record(analysis_record(event, row))?;
    }
    writer.flush()?;
    println!("wrote analysis_export.csv ({} rows)", events.len());
    Ok(events.len() as u64)
}

fn load_best_ledger(paths: &tinotax_store::ProjectPaths) -> Result<Vec<TaxLedgerEvent>> {
    if paths.priced_ledger_jsonl().exists() {
        return tinotax_pricing::load_priced_ledger(paths);
    }
    if paths.reviewed_ledger_jsonl().exists() {
        return tinotax_ledger::load_reviewed_ledger(paths);
    }
    Ok(Vec::new())
}

fn ledger_by_source_event(ledger: &[TaxLedgerEvent]) -> BTreeMap<String, &TaxLedgerEvent> {
    let mut out = BTreeMap::new();
    for row in ledger {
        for event_id in &row.source_event_ids {
            out.insert(event_id.clone(), row);
        }
    }
    out
}

fn analysis_record(event: &NormalisedEvent, ledger: Option<&&TaxLedgerEvent>) -> Vec<String> {
    let ledger = ledger.copied();
    let tax_year = ledger
        .map(|l| l.tax_year.clone())
        .unwrap_or_else(|| tinotax_core::uk_tax_year(&event.timestamp).unwrap_or_default());
    let asset_symbol = ledger
        .map(|l| l.asset_symbol.as_str())
        .unwrap_or(event.asset_symbol.as_str());
    let quantity = ledger.map(|l| l.quantity).unwrap_or(event.amount);
    let signed_quantity = signed_quantity(event, quantity);
    let gross_value_gbp = ledger
        .and_then(row_gbp_value)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let date = event.timestamp.get(..10).unwrap_or("").to_string();
    let year = date.get(..4).unwrap_or("").to_string();
    let month = date.get(..7).unwrap_or("").to_string();
    let tax_event_type = ledger
        .map(|l| l.tax_event_type.as_str())
        .unwrap_or("not_built");
    let activity_class = activity_class(tax_event_type);

    vec![
        event.event_id.clone(),
        ledger
            .map(|l| l.ledger_event_id.clone())
            .unwrap_or_default(),
        event.timestamp.clone(),
        tax_year,
        event.source_id.clone(),
        format!("{:?}", event.source_kind).to_ascii_lowercase(),
        ledger.and_then(|l| l.platform.clone()).unwrap_or_default(),
        event.chain.clone(),
        event.wallet.clone(),
        event.tx_hash.clone(),
        event
            .block_number
            .map(|v| v.to_string())
            .unwrap_or_default(),
        event.event_type.as_str().to_string(),
        event.direction.as_str().to_string(),
        tax_event_type.to_string(),
        asset_symbol.to_string(),
        event.asset_contract.clone().unwrap_or_default(),
        quantity.to_string(),
        event.raw_amount.clone().unwrap_or_default(),
        event
            .token_decimals
            .map(|v| v.to_string())
            .unwrap_or_default(),
        event.from_address.clone().unwrap_or_default(),
        event.to_address.clone().unwrap_or_default(),
        event.counterparty.clone().unwrap_or_default(),
        event.method.clone().unwrap_or_default(),
        event.fee_asset.clone().unwrap_or_default(),
        event.fee_amount.map(|v| v.to_string()).unwrap_or_default(),
        opt_dec(ledger.and_then(|l| l.proceeds_gbp)),
        opt_dec(ledger.and_then(|l| l.cost_gbp)),
        opt_dec(ledger.and_then(|l| l.income_gbp)),
        opt_dec(ledger.and_then(|l| l.fee_gbp)),
        ledger
            .and_then(|l| l.price_source.clone())
            .unwrap_or_default(),
        ledger
            .map(|l| l.price_confidence.as_str().to_string())
            .unwrap_or_default(),
        ledger
            .map(|l| l.review_status.as_str().to_string())
            .unwrap_or_default(),
        event.needs_review.to_string(),
        event.review_reasons.join("; "),
        ledger.and_then(|l| l.user_note.clone()).unwrap_or_default(),
        event.source_ref.raw_file.clone(),
        event
            .source_ref
            .raw_page
            .map(|v| v.to_string())
            .unwrap_or_default(),
        event.source_ref.json_path.clone().unwrap_or_default(),
        event
            .source_ref
            .log_index
            .map(|v| v.to_string())
            .unwrap_or_default(),
        event
            .source_ref
            .movement_index
            .map(|v| v.to_string())
            .unwrap_or_default(),
        year,
        month,
        date,
        signed_quantity.to_string(),
        gross_value_gbp,
        activity_class.to_string(),
        visual_bucket(event, tax_event_type).to_string(),
    ]
}

fn opt_dec(value: Option<Decimal>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

fn row_gbp_value(row: &TaxLedgerEvent) -> Option<Decimal> {
    row.proceeds_gbp
        .or(row.cost_gbp)
        .or(row.income_gbp)
        .or(row.fee_gbp)
}

fn signed_quantity(event: &NormalisedEvent, quantity: Decimal) -> Decimal {
    match event.direction {
        tinotax_core::Direction::Out => -quantity,
        _ => quantity,
    }
}

fn activity_class(tax_event_type: &str) -> &'static str {
    match tax_event_type {
        "disposal" | "swap_disposal" | "goods_or_services_spend" | "fee" => "cgt_disposal",
        "acquisition" | "swap_acquisition" | "airdrop" | "fork" => "pool_entry",
        "staking_reward"
        | "mining_reward"
        | "employment_income"
        | "self_employment_income"
        | "misc_income"
        | "compensation" => "income",
        "transfer_in" | "transfer_out" | "bridge_in" | "bridge_out" => "transfer",
        "ignore" => "ignored",
        "unknown" => "needs_classification",
        _ => "unbuilt",
    }
}

fn visual_bucket(event: &NormalisedEvent, tax_event_type: &str) -> &'static str {
    match activity_class(tax_event_type) {
        "cgt_disposal" => "taxable_outflow",
        "pool_entry" | "income" => "taxable_inflow",
        "transfer" => "internal_movement",
        "needs_classification" => "review_backlog",
        "ignored" => "ignored",
        _ if event.needs_review => "review_backlog",
        _ => "other",
    }
}
