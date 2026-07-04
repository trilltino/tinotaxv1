use anyhow::{Context, Result};
use tinotax_core::{EventType, NormalisedEvent};
use tinotax_store::{read_jsonl, ProjectPaths};

/// Export only the uncertain rows to `out/manual_review.csv` for an
/// accountant/client to fill in `user_action` (+ optional `user_note`).
/// Returns the number of rows exported.
pub fn export_review(paths: &ProjectPaths) -> Result<u64> {
    let events: Vec<NormalisedEvent> = read_jsonl(&paths.events_jsonl())
        .context("reading staging/normalised_events.jsonl — run `normalise` first")?;

    std::fs::create_dir_all(paths.out())?;
    let path = paths.out().join("manual_review.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record([
        "event_id",
        "chain",
        "wallet",
        "timestamp",
        "tx_hash",
        "event_type",
        "direction",
        "asset_symbol",
        "amount",
        "from_address",
        "to_address",
        "fee_asset",
        "fee_amount",
        "confidence",
        "review_reasons",
        "suggested_action",
        "user_action",
        "user_note",
    ])?;

    let mut rows = 0u64;
    for event in events.iter().filter(|e| e.needs_review) {
        writer.write_record([
            event.event_id.as_str(),
            event.chain.as_str(),
            event.wallet.as_str(),
            event.timestamp.as_str(),
            event.tx_hash.as_str(),
            event.event_type.as_str(),
            event.direction.as_str(),
            event.asset_symbol.as_str(),
            &event.amount.to_string(),
            event.from_address.as_deref().unwrap_or(""),
            event.to_address.as_deref().unwrap_or(""),
            event.fee_asset.as_deref().unwrap_or(""),
            &event
                .fee_amount
                .map(|a| a.to_string())
                .unwrap_or_default(),
            event.confidence.as_str(),
            &event.review_reasons.join("; "),
            suggested_action(event),
            "", // user_action — filled by the reviewer
            "", // user_note
        ])?;
        rows += 1;
    }
    writer.flush()?;
    Ok(rows)
}

/// A starting point for the reviewer, never an automatic decision.
fn suggested_action(event: &NormalisedEvent) -> &'static str {
    match event.event_type {
        EventType::PossibleSwap => "swap",
        EventType::PossibleBridge => "bridge",
        EventType::PossibleAirdrop => "airdrop",
        EventType::PossibleStakingReward => "staking_reward",
        EventType::Fee => "fee",
        EventType::NativeTransfer | EventType::TokenTransfer => "keep",
        EventType::ContractCall | EventType::Unknown => "unknown",
    }
}
