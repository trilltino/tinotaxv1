//! normalised events + review overrides → reviewed tax ledger.

use anyhow::{Context, Result};
use tinotax_core::{
    uk_tax_year, NormalisedEvent, PriceConfidence, PriceSource, ReviewOverride, ReviewStatus,
    SourceKind, TaxEventType, TaxLedgerEvent,
};
use tinotax_review::{load_all_events, load_latest_overrides};
use tinotax_store::{read_jsonl, JsonlWriter, ProjectPaths};

#[derive(Debug, Clone, Copy, Default)]
pub struct LedgerSummary {
    pub total: u64,
    pub reviewed: u64,
    pub needs_review: u64,
    pub ignored: u64,
    pub unknown: u64,
}

/// Build `staging/reviewed_ledger.jsonl` and `out/reviewed_ledger.csv`.
pub fn build_ledger(paths: &ProjectPaths) -> Result<LedgerSummary> {
    let events = load_all_events(paths)?;
    let overrides = load_latest_overrides(paths)?;

    let mut summary = LedgerSummary::default();
    let mut ledger = Vec::with_capacity(events.len());
    for event in &events {
        let row = to_ledger_event(event, overrides.get(&event.event_id))
            .with_context(|| format!("event {}", event.event_id))?;
        summary.total += 1;
        match row.review_status {
            ReviewStatus::Reviewed => summary.reviewed += 1,
            ReviewStatus::NeedsReview => summary.needs_review += 1,
            ReviewStatus::Auto => {}
        }
        if row.tax_event_type == TaxEventType::Ignore {
            summary.ignored += 1;
        }
        if row.tax_event_type == TaxEventType::Unknown {
            summary.unknown += 1;
        }
        ledger.push(row);
    }

    // load_all_events is timestamp-sorted; keep that order on disk.
    std::fs::create_dir_all(paths.staging())?;
    let mut writer = JsonlWriter::create(&paths.reviewed_ledger_jsonl())?;
    for row in &ledger {
        writer.write(row)?;
    }
    writer.finish()?;

    crate::csv_export::export_ledger_csv(paths, &ledger, "reviewed_ledger.csv")?;
    Ok(summary)
}

/// Read the reviewed ledger back (for pricing, tax calc, evidence).
pub fn load_reviewed_ledger(paths: &ProjectPaths) -> Result<Vec<TaxLedgerEvent>> {
    read_jsonl(&paths.reviewed_ledger_jsonl())
        .context("reading staging/reviewed_ledger.jsonl — run `ledger build` first")
}

/// Deterministic ledger id derived from the source event id.
fn ledger_event_id(event_id: &str) -> String {
    let hash = blake3::hash(format!("ledger|{event_id}").as_bytes()).to_hex();
    format!("lev_{}", &hash.as_str()[..16])
}

fn to_ledger_event(
    event: &NormalisedEvent,
    override_: Option<&ReviewOverride>,
) -> Result<TaxLedgerEvent> {
    // Precedence: precise user_tax_type > coarse user_action > machine suggestion.
    let tax_event_type = match override_ {
        Some(o) => o
            .user_tax_type
            .or_else(|| {
                o.user_action
                    .map(|a| TaxEventType::from_review_action(a, event.event_type, event.direction))
            })
            .unwrap_or_else(|| TaxEventType::suggest(event.event_type, event.direction)),
        None => TaxEventType::suggest(event.event_type, event.direction),
    };

    let review_status = match override_ {
        Some(_) => ReviewStatus::Reviewed,
        None if event.needs_review => ReviewStatus::NeedsReview,
        None => ReviewStatus::Auto,
    };

    let user_gbp_given = override_.is_some_and(|o| {
        o.user_proceeds_gbp.is_some()
            || o.user_cost_gbp.is_some()
            || o.user_income_gbp.is_some()
            || o.user_fee_gbp.is_some()
    });

    let platform = match event.source_kind {
        SourceKind::CexCsv => Some(event.chain.clone()),
        SourceKind::Wallet | SourceKind::Manual => None,
    };

    Ok(TaxLedgerEvent {
        ledger_event_id: ledger_event_id(&event.event_id),
        source_event_ids: vec![event.event_id.clone()],
        source_refs: vec![event.source_ref.clone()],
        timestamp: event.timestamp.clone(),
        tax_year: uk_tax_year(&event.timestamp)?,
        platform,
        chain: Some(event.chain.clone()),
        wallet: Some(event.wallet.clone()),
        tx_hash: (!event.tx_hash.is_empty()).then(|| event.tx_hash.clone()),
        tax_event_type,
        asset_symbol: override_
            .and_then(|o| o.user_asset_symbol.clone())
            .unwrap_or_else(|| event.asset_symbol.clone()),
        asset_contract: event.asset_contract.clone(),
        quantity: override_
            .and_then(|o| o.user_quantity)
            .unwrap_or(event.amount),
        proceeds_gbp: override_.and_then(|o| o.user_proceeds_gbp),
        cost_gbp: override_.and_then(|o| o.user_cost_gbp),
        income_gbp: override_.and_then(|o| o.user_income_gbp),
        fee_gbp: override_.and_then(|o| o.user_fee_gbp),
        price_source: user_gbp_given.then(|| PriceSource::UserProvided.as_str().to_string()),
        price_confidence: if user_gbp_given {
            PriceConfidence::High
        } else {
            PriceConfidence::Missing
        },
        review_status,
        user_note: override_.and_then(|o| o.user_note.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use tinotax_core::{Confidence, Direction, EventType, SourceRef};

    fn sample_event() -> NormalisedEvent {
        NormalisedEvent {
            event_id: "evt_1".into(),
            project_id: "p".into(),
            source_id: "near_main".into(),
            source_kind: SourceKind::Wallet,
            chain: "near".into(),
            wallet: "example.near".into(),
            timestamp: "2024-06-01T12:00:00Z".into(),
            block_number: Some(1),
            tx_hash: "0xabc".into(),
            event_type: EventType::TokenTransfer,
            direction: Direction::In,
            asset_symbol: "NEAR".into(),
            asset_contract: None,
            amount: Decimal::new(5, 0),
            raw_amount: None,
            token_decimals: None,
            from_address: None,
            to_address: None,
            fee_asset: None,
            fee_amount: None,
            counterparty: None,
            method: None,
            confidence: Confidence::High,
            needs_review: false,
            review_reasons: vec![],
            source_ref: SourceRef {
                raw_file: "raw/near/example.near/p1.json".into(),
                raw_page: Some(1),
                json_path: None,
                log_index: None,
                movement_index: None,
            },
        }
    }

    #[test]
    fn machine_suggestion_without_override() {
        let row = to_ledger_event(&sample_event(), None).unwrap();
        assert_eq!(row.tax_event_type, TaxEventType::Acquisition);
        assert_eq!(row.review_status, ReviewStatus::Auto);
        assert_eq!(row.tax_year, "2024-2025");
        assert_eq!(row.price_confidence, PriceConfidence::Missing);
    }

    #[test]
    fn user_tax_type_wins_over_action_and_suggestion() {
        let o = ReviewOverride {
            event_id: "evt_1".into(),
            user_action: Some(tinotax_core::ReviewAction::Swap),
            user_tax_type: Some(TaxEventType::TransferIn),
            user_asset_symbol: None,
            user_quantity: Some(Decimal::new(7, 0)),
            user_proceeds_gbp: None,
            user_cost_gbp: Some(Decimal::new(100, 0)),
            user_income_gbp: None,
            user_fee_gbp: None,
            user_price_source: None,
            user_note: Some("moved from ledger nano".into()),
            applied_at: "2026-01-01T00:00:00Z".into(),
            source_file: None,
        };
        let row = to_ledger_event(&sample_event(), Some(&o)).unwrap();
        assert_eq!(row.tax_event_type, TaxEventType::TransferIn);
        assert_eq!(row.quantity, Decimal::new(7, 0));
        assert_eq!(row.cost_gbp, Some(Decimal::new(100, 0)));
        assert_eq!(row.review_status, ReviewStatus::Reviewed);
        assert_eq!(row.price_confidence, PriceConfidence::High);
    }

    #[test]
    fn ledger_ids_are_deterministic() {
        assert_eq!(ledger_event_id("evt_1"), ledger_event_id("evt_1"));
        assert_ne!(ledger_event_id("evt_1"), ledger_event_id("evt_2"));
    }
}
