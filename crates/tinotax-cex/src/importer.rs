//! Orchestrates one `import-cex` run: copy + hash originals, parse each
//! export, convert records to normalised events, write the merged staging
//! files and the diagnostics CSV. Idempotent — the staging outputs are
//! fully rebuilt from the immutable raw copies every run.

use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use rust_decimal::Decimal;
use tinotax_config::{CexCsvEntry, CexPlatform, ProjectConfig};
use tinotax_core::{
    Confidence, Direction, EventType, NormalisedEvent, PriceConfidence, PriceObservation,
    PriceSource, SourceKind, SourceRef,
};
use tinotax_store::{hash_file, JsonlWriter, ProjectPaths};
use tracing::info;

use crate::record::{is_fiat, CexRecord, CexRecordKind};
use crate::report::{write_diagnostics, ImportReport};

/// Import every `[[cex_csvs]]` source. Returns one report per source.
pub fn import_all(paths: &ProjectPaths, config: &ProjectConfig) -> Result<Vec<ImportReport>> {
    if config.cex_csvs.is_empty() {
        bail!(
            "no [[cex_csvs]] sources declared in {} — see wallets.example.toml for the format",
            paths.config_file()
        );
    }

    let mut all_events: Vec<NormalisedEvent> = Vec::new();
    let mut all_hints: Vec<PriceObservation> = Vec::new();
    let mut reports = Vec::new();
    for entry in &config.cex_csvs {
        let report = import_one(paths, config, entry, &mut all_events, &mut all_hints)
            .with_context(|| format!("importing cex source {:?}", entry.id))?;
        info!(
            source = entry.id,
            events = report.events_emitted,
            "imported cex source"
        );
        reports.push(report);
    }

    all_events.sort_by(|a, b| {
        (a.timestamp.as_str(), a.event_id.as_str())
            .cmp(&(b.timestamp.as_str(), b.event_id.as_str()))
    });
    std::fs::create_dir_all(paths.staging())?;
    let mut writer = JsonlWriter::create(&paths.cex_events_jsonl())?;
    for event in &all_events {
        writer.write(event)?;
    }
    writer.finish()?;

    let mut hints = JsonlWriter::create(&paths.price_hints_jsonl())?;
    for hint in &all_hints {
        hints.write(hint)?;
    }
    hints.finish()?;

    write_diagnostics(paths, &reports)?;
    Ok(reports)
}

/// Copy the original file into `raw/cex/<id>/` (refusing silent
/// replacement), parse it, and convert to events + price hints.
fn import_one(
    paths: &ProjectPaths,
    config: &ProjectConfig,
    entry: &CexCsvEntry,
    events: &mut Vec<NormalisedEvent>,
    hints: &mut Vec<PriceObservation>,
) -> Result<ImportReport> {
    let source_path = Utf8PathBuf::from(&entry.path);
    if !source_path.exists() {
        bail!("file not found: {source_path}");
    }

    // Immutable evidence copy. Same content → fine; different content under
    // the same id → refuse, a new export needs a new id.
    let raw_dir = paths.cex_raw_dir(&entry.id);
    std::fs::create_dir_all(&raw_dir).with_context(|| format!("creating {raw_dir}"))?;
    let original = raw_dir.join("original.csv");
    let (source_hash, _) = hash_file(&source_path)?;
    if original.exists() {
        let (existing_hash, _) = hash_file(&original)?;
        if existing_hash != source_hash {
            bail!(
                "raw/cex/{}/original.csv already exists with different content — \
                 raw evidence is never overwritten; declare the new export as a new [[cex_csvs]] id",
                entry.id
            );
        }
    } else {
        std::fs::copy(source_path.as_std_path(), original.as_std_path())
            .with_context(|| format!("copying {source_path} to {original}"))?;
    }
    std::fs::write(
        raw_dir.join("original_hash.txt"),
        format!("blake3:{source_hash}\n"),
    )?;

    let records = match entry.platform {
        CexPlatform::Binance => crate::binance::parse(&original)?,
        CexPlatform::Coinbase => crate::coinbase::parse(&original)?,
        CexPlatform::Kraken => crate::kraken::parse(&original)?,
        CexPlatform::Awaken => crate::awaken::parse(&original)?,
        CexPlatform::Generic => {
            let mapping = entry.mapping.as_ref().expect("validated by config");
            crate::generic_csv::parse(&original, mapping)?
        }
    };

    let mut report = ImportReport {
        source_id: entry.id.clone(),
        platform: entry.platform.as_str().to_string(),
        ..Default::default()
    };
    let raw_file = paths.relative(&original);
    for record in &records {
        report.rows_read += 1;
        track_span(&mut report, &record.timestamp);

        if let Some(price) = record.price_gbp {
            hints.push(price_hint(record, price));
            report.price_hints += 1;
        }
        if record.amount.is_zero() {
            report.zero_amount_skipped += 1;
        } else if is_fiat(&record.asset) {
            report.fiat_movements_skipped += 1;
        } else {
            let event = to_event(config, entry, record, &raw_file, 0, false);
            if event.needs_review {
                report.needs_review += 1;
            }
            report.events_emitted += 1;
            events.push(event);
        }

        // Fees paid in crypto are their own movement (a small disposal).
        if let (Some(fee_asset), Some(fee_amount)) = (&record.fee_asset, record.fee_amount) {
            if !fee_amount.is_zero() && !is_fiat(fee_asset) {
                events.push(to_event(config, entry, record, &raw_file, 1, true));
                report.events_emitted += 1;
            }
        }
    }
    Ok(report)
}

fn track_span(report: &mut ImportReport, timestamp: &str) {
    if report.earliest.as_deref().is_none_or(|e| timestamp < e) {
        report.earliest = Some(timestamp.to_string());
    }
    if report.latest.as_deref().is_none_or(|l| timestamp > l) {
        report.latest = Some(timestamp.to_string());
    }
}

fn price_hint(record: &CexRecord, price: Decimal) -> PriceObservation {
    PriceObservation {
        asset_symbol: record.asset.clone(),
        asset_contract: None,
        timestamp: record.timestamp.clone(),
        currency: "GBP".to_string(),
        price,
        source: PriceSource::Cex,
        confidence: PriceConfidence::High,
        fetched_at: tinotax_store::now_rfc3339(),
        note: Some("spot price stated in the CEX export".to_string()),
    }
}

fn to_event(
    config: &ProjectConfig,
    entry: &CexCsvEntry,
    record: &CexRecord,
    raw_file: &str,
    movement_index: u64,
    is_fee: bool,
) -> NormalisedEvent {
    let (asset, amount, direction, event_type, confidence, needs_review, reasons) = if is_fee {
        (
            record.fee_asset.clone().unwrap_or_default(),
            record.fee_amount.unwrap_or_default(),
            Direction::Out,
            EventType::Fee,
            Confidence::High,
            false,
            Vec::new(),
        )
    } else {
        let direction = if record.amount.is_sign_negative() {
            Direction::Out
        } else {
            Direction::In
        };
        let (event_type, confidence, needs_review, reasons) = match record.kind {
            CexRecordKind::Trade => (EventType::PossibleSwap, Confidence::High, false, Vec::new()),
            CexRecordKind::Deposit | CexRecordKind::Withdrawal => (
                EventType::TokenTransfer,
                Confidence::High,
                false,
                Vec::new(),
            ),
            CexRecordKind::Fee => (EventType::Fee, Confidence::High, false, Vec::new()),
            CexRecordKind::Reward => (
                EventType::PossibleStakingReward,
                Confidence::High,
                false,
                Vec::new(),
            ),
            CexRecordKind::Airdrop => (
                EventType::PossibleAirdrop,
                Confidence::High,
                false,
                Vec::new(),
            ),
            CexRecordKind::Other => (
                EventType::Unknown,
                Confidence::Low,
                true,
                vec!["unrecognised CEX operation".to_string()],
            ),
        };
        (
            record.asset.clone(),
            record.amount.abs(),
            direction,
            event_type,
            confidence,
            needs_review,
            reasons,
        )
    };

    let event_id = {
        let hash = blake3::hash(
            format!(
                "cex|{}|{}|{}|{}|{}|{}",
                entry.id,
                record.row,
                movement_index,
                asset,
                amount,
                direction.as_str()
            )
            .as_bytes(),
        )
        .to_hex();
        format!("cex_{}", &hash.as_str()[..24])
    };

    NormalisedEvent {
        event_id,
        project_id: config.project.name.clone(),
        source_id: entry.id.clone(),
        source_kind: SourceKind::CexCsv,
        chain: entry.platform.as_str().to_string(),
        wallet: entry.id.clone(),
        timestamp: record.timestamp.clone(),
        block_number: None,
        tx_hash: String::new(),
        event_type,
        direction,
        asset_symbol: asset,
        asset_contract: None,
        amount,
        raw_amount: None,
        token_decimals: None,
        from_address: None,
        to_address: None,
        fee_asset: None,
        fee_amount: None,
        counterparty: None,
        method: None,
        confidence,
        needs_review,
        review_reasons: reasons,
        source_ref: SourceRef {
            raw_file: raw_file.to_string(),
            raw_page: None,
            json_path: Some(format!("row[{}]", record.row)),
            log_index: None,
            movement_index: Some(movement_index),
        },
    }
}
