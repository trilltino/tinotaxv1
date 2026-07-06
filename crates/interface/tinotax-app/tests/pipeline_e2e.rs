//! End-to-end pipeline test on a synthetic project: pre-normalised events
//! go through review export → CSV edit → apply → ledger build → manual
//! prices → ledger price → UK calculation → evidence pack, exactly as the
//! CLI drives it.

use std::error::Error;

use camino::Utf8PathBuf;
use rust_decimal::Decimal;
use tinotax_core::{Confidence, Direction, EventType, NormalisedEvent, SourceKind, SourceRef};
use tinotax_store::{JsonlWriter, ProjectPaths};

const PROJECT_TOML: &str = r#"
[project]
name = "e2e-test"
base_currency = "GBP"
period_start = "2017-01-01T00:00:00Z"
period_end = "2025-04-05T23:59:59Z"

[[wallets]]
id = "near_test"
name = "test wallet"
chain = "near"
address = "test.near"
provider = "nearblocks"

[providers.nearblocks]
kind = "nearblocks"
base_url = "https://api.nearblocks.io/v1"
"#;

fn event(
    id: &str,
    timestamp: &str,
    event_type: EventType,
    direction: Direction,
    asset: &str,
    amount: &str,
    needs_review: bool,
) -> Result<NormalisedEvent, Box<dyn Error>> {
    Ok(NormalisedEvent {
        event_id: id.to_string(),
        project_id: "e2e-test".into(),
        source_id: "near_test".into(),
        source_kind: SourceKind::Wallet,
        chain: "near".into(),
        wallet: "test.near".into(),
        timestamp: timestamp.to_string(),
        block_number: Some(1),
        tx_hash: format!("0x{id}"),
        event_type,
        direction,
        asset_symbol: asset.to_string(),
        asset_contract: None,
        amount: amount.parse::<Decimal>()?,
        raw_amount: None,
        token_decimals: None,
        from_address: None,
        to_address: None,
        fee_asset: None,
        fee_amount: None,
        counterparty: None,
        method: None,
        confidence: if needs_review {
            Confidence::Low
        } else {
            Confidence::High
        },
        needs_review,
        review_reasons: if needs_review {
            vec!["synthetic uncertainty".into()]
        } else {
            vec![]
        },
        source_ref: SourceRef {
            raw_file: "raw/near/test.near/page_000001.json".into(),
            raw_page: Some(1),
            json_path: Some(format!("items[{id}]")),
            log_index: None,
            movement_index: Some(0),
        },
    })
}

#[test]
fn full_pipeline_from_normalised_events_to_evidence_pack() -> Result<(), Box<dyn Error>> {
    let tmp = tempfile::tempdir()?;
    let root = Utf8PathBuf::from_path_buf(tmp.path().join("proj"))
        .map_err(|path| std::io::Error::other(format!("non-UTF8 path {}", path.display())))?;
    let project = root.as_str();

    // A project as `normalise` would have left it.
    let paths = ProjectPaths::new(root.clone());
    paths.init()?;
    std::fs::write(paths.config_file(), PROJECT_TOML)?;
    let mut writer = JsonlWriter::create(&paths.events_jsonl())?;
    for e in [
        event(
            "buy",
            "2024-05-01T10:00:00Z",
            EventType::TokenTransfer,
            Direction::In,
            "BTC",
            "1",
            false,
        )?,
        event(
            "sell",
            "2024-06-01T10:00:00Z",
            EventType::TokenTransfer,
            Direction::Out,
            "BTC",
            "1",
            false,
        )?,
        event(
            "mystery",
            "2024-06-02T10:00:00Z",
            EventType::ContractCall,
            Direction::Unknown,
            "BTC",
            "0.1",
            true,
        )?,
    ] {
        writer.write(&e)?;
    }
    writer.finish()?;

    // 1. Export everything for review.
    let rows = tinotax_app::export_review_all(project)?;
    assert_eq!(rows, 3);
    let review_csv = paths.out().join("review_all_transactions.csv");
    assert!(review_csv.exists());

    // 2. The reviewer marks the mystery row as ignore.
    let edited = paths.out().join("review_all_transactions_edited.csv");
    std::fs::write(
        &edited,
        "event_id,user_tax_type,user_note\nmystery,ignore,failed contract call\n",
    )?;
    let applied = tinotax_app::apply_review(project, edited.as_str())?;
    assert_eq!(applied, 1);
    assert!(paths.out().join("change_log.csv").exists());

    // 3. Build the reviewed ledger.
    tinotax_app::ledger_build(project)?;
    assert!(paths.reviewed_ledger_jsonl().exists());
    assert!(paths.out().join("reviewed_ledger.csv").exists());

    // 4. Missing prices are visible, then filled by a manual import.
    tinotax_app::prices_missing(project)?;
    let missing = std::fs::read_to_string(paths.out().join("missing_prices.csv"))?;
    assert!(missing.contains("BTC,2024-05-01"), "{missing}");
    let prices = root.join("manual_prices.csv");
    std::fs::write(
        &prices,
        "asset_symbol,date,price_gbp\nBTC,2024-05-01,10000\nBTC,2024-06-01,15000\n",
    )?;
    tinotax_app::prices_import(project, prices.as_str())?;

    // 5. Price the ledger.
    tinotax_app::ledger_price(project)?;
    assert!(paths.priced_ledger_jsonl().exists());
    assert!(paths.out().join("pricing_audit.csv").exists());

    // 6. Calculate the year: £15k proceeds - £10k cost = £5k net gain.
    tinotax_app::calculate_uk(project, "2024-2025", false)?;
    let summary = std::fs::read_to_string(
        paths
            .tax_dir("2024-2025")
            .join("self_assessment_crypto_summary.csv"),
    )?;
    assert!(summary.contains("net_gain_or_loss_gbp,5000"), "{summary}");
    assert!(summary.contains("number_of_disposals,1"), "{summary}");

    // 7. Evidence pack.
    tinotax_app::pack_hmrc(project, "2024-2025")?;
    let pack = paths.evidence_dir("2024-2025");
    for name in [
        "README.md",
        "hmrc_questions_draft.md",
        "self_assessment_crypto_summary.csv",
        "disposals_calculation.csv",
        "s104_pool_movements.csv",
        "raw_data_index.csv",
        "manual_review_decisions.csv",
        "source_of_funds_notes.md",
        "calculator_statement.md",
        "assumptions_and_limitations.md",
    ] {
        assert!(pack.join(name).exists(), "missing {name} in evidence pack");
    }
    // The questionnaire template was created for the client to fill in.
    assert!(paths.questionnaire_file().exists());

    // The ignored row must not have produced a disposal.
    let disposals = std::fs::read_to_string(pack.join("disposals_calculation.csv"))?;
    assert_eq!(
        disposals.lines().count(),
        2,
        "header + exactly one disposal"
    );
    Ok(())
}
