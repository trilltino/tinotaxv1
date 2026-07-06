//! `review export-all`: every event — certain or not — in one reviewable,
//! editable CSV. The `user_*` columns are the editable surface; everything
//! else is context. Existing override decisions are pre-filled so the file
//! always shows the current state of review.

use anyhow::{Context, Result};
use tinotax_core::{uk_tax_year, SourceKind, TaxEventType};
use tinotax_store::ProjectPaths;

use crate::load::{load_all_events, load_latest_overrides};

pub const REVIEW_ALL_COLUMNS: [&str; 32] = [
    "event_id",
    "timestamp",
    "tax_year",
    "source_id",
    "platform",
    "chain",
    "wallet",
    "tx_hash",
    "detected_event_type",
    "detected_direction",
    "asset_symbol",
    "asset_contract",
    "amount",
    "fee_asset",
    "fee_amount",
    "from_address",
    "to_address",
    "confidence",
    "needs_review",
    "review_reasons",
    "suggested_tax_type",
    "user_tax_type",
    "user_asset_symbol",
    "user_quantity",
    "user_proceeds_gbp",
    "user_cost_gbp",
    "user_income_gbp",
    "user_fee_gbp",
    "user_price_source",
    "user_note",
    "raw_file",
    "json_path",
];

/// Write `out/review_all_transactions.csv`. Returns the number of rows.
pub fn export_review_all(paths: &ProjectPaths) -> Result<u64> {
    let events = load_all_events(paths)?;
    let overrides = load_latest_overrides(paths)?;

    std::fs::create_dir_all(paths.out())?;
    let path = paths.out().join("review_all_transactions.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record(REVIEW_ALL_COLUMNS)?;

    let mut rows = 0u64;
    for event in &events {
        let o = overrides.get(&event.event_id);
        let platform = match event.source_kind {
            SourceKind::CexCsv => event.chain.as_str(),
            SourceKind::Wallet | SourceKind::Manual => "",
        };
        let opt_dec =
            |d: Option<rust_decimal::Decimal>| d.map(|v| v.to_string()).unwrap_or_default();
        writer.write_record([
            event.event_id.as_str(),
            event.timestamp.as_str(),
            &uk_tax_year(&event.timestamp).unwrap_or_default(),
            event.source_id.as_str(),
            platform,
            event.chain.as_str(),
            event.wallet.as_str(),
            event.tx_hash.as_str(),
            event.event_type.as_str(),
            event.direction.as_str(),
            event.asset_symbol.as_str(),
            event.asset_contract.as_deref().unwrap_or(""),
            &event.amount.to_string(),
            event.fee_asset.as_deref().unwrap_or(""),
            &opt_dec(event.fee_amount),
            event.from_address.as_deref().unwrap_or(""),
            event.to_address.as_deref().unwrap_or(""),
            event.confidence.as_str(),
            if event.needs_review { "true" } else { "false" },
            &event.review_reasons.join("; "),
            TaxEventType::suggest(event.event_type, event.direction).as_str(),
            o.and_then(|o| o.user_tax_type)
                .map(|t| t.as_str())
                .unwrap_or(""),
            o.and_then(|o| o.user_asset_symbol.as_deref()).unwrap_or(""),
            &opt_dec(o.and_then(|o| o.user_quantity)),
            &opt_dec(o.and_then(|o| o.user_proceeds_gbp)),
            &opt_dec(o.and_then(|o| o.user_cost_gbp)),
            &opt_dec(o.and_then(|o| o.user_income_gbp)),
            &opt_dec(o.and_then(|o| o.user_fee_gbp)),
            o.and_then(|o| o.user_price_source.as_deref()).unwrap_or(""),
            o.and_then(|o| o.user_note.as_deref()).unwrap_or(""),
            event.source_ref.raw_file.as_str(),
            event.source_ref.json_path.as_deref().unwrap_or(""),
        ])?;
        rows += 1;
    }
    writer.flush()?;
    Ok(rows)
}
