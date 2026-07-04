use anyhow::{Context, Result};
use tinotax_core::NormalisedEvent;
use tinotax_store::{read_jsonl, ProjectPaths};

/// Flatten every normalised event into `out/normalised_transactions.csv`.
/// Every row carries its `event_id` and raw source reference — the audit
/// trail survives the export.
pub fn export_transactions_csv(paths: &ProjectPaths) -> Result<u64> {
    let events: Vec<NormalisedEvent> = read_jsonl(&paths.events_jsonl())
        .context("reading staging/normalised_events.jsonl — run `normalise` first")?;

    std::fs::create_dir_all(paths.out())?;
    let path = paths.out().join("normalised_transactions.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record([
        "event_id",
        "project_id",
        "source_id",
        "chain",
        "wallet",
        "timestamp",
        "block_number",
        "tx_hash",
        "event_type",
        "direction",
        "asset_symbol",
        "asset_contract",
        "amount",
        "raw_amount",
        "token_decimals",
        "from_address",
        "to_address",
        "fee_asset",
        "fee_amount",
        "counterparty",
        "method",
        "confidence",
        "needs_review",
        "review_reasons",
        "raw_file",
        "raw_page",
        "json_path",
        "log_index",
        "movement_index",
    ])?;

    for e in &events {
        writer.write_record([
            e.event_id.as_str(),
            e.project_id.as_str(),
            e.source_id.as_str(),
            e.chain.as_str(),
            e.wallet.as_str(),
            e.timestamp.as_str(),
            &e.block_number.map(|v| v.to_string()).unwrap_or_default(),
            e.tx_hash.as_str(),
            e.event_type.as_str(),
            e.direction.as_str(),
            e.asset_symbol.as_str(),
            e.asset_contract.as_deref().unwrap_or(""),
            &e.amount.to_string(),
            e.raw_amount.as_deref().unwrap_or(""),
            &e.token_decimals.map(|v| v.to_string()).unwrap_or_default(),
            e.from_address.as_deref().unwrap_or(""),
            e.to_address.as_deref().unwrap_or(""),
            e.fee_asset.as_deref().unwrap_or(""),
            &e.fee_amount.map(|v| v.to_string()).unwrap_or_default(),
            e.counterparty.as_deref().unwrap_or(""),
            e.method.as_deref().unwrap_or(""),
            e.confidence.as_str(),
            if e.needs_review { "true" } else { "false" },
            &e.review_reasons.join("; "),
            e.source_ref.raw_file.as_str(),
            &e.source_ref.raw_page.map(|v| v.to_string()).unwrap_or_default(),
            e.source_ref.json_path.as_deref().unwrap_or(""),
            &e.source_ref.log_index.map(|v| v.to_string()).unwrap_or_default(),
            &e.source_ref
                .movement_index
                .map(|v| v.to_string())
                .unwrap_or_default(),
        ])?;
    }
    writer.flush()?;
    Ok(events.len() as u64)
}
