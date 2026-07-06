//! JSON-friendly APIs used by the desktop/webview app.
//!
//! The CLI prints human output. The desktop app needs typed data, so this
//! module exposes thin DTOs over the same project and review primitives.

use std::collections::BTreeSet;
use std::fs;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tinotax_core::{
    uk_tax_year, PriceSource, ReviewAction, ReviewOverride, SourceKind, TaxEventType,
};
use tinotax_store::{JsonlWriter, ProjectPaths};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatusDto {
    pub root: String,
    pub name: String,
    pub base_currency: String,
    pub period_start: String,
    pub period_end: String,
    pub wallet_count: usize,
    pub cex_import_count: usize,
    pub provider_count: usize,
    pub folders: Vec<FolderStatusDto>,
    pub review_override_count: u64,
    pub price_observation_count: u64,
    pub questionnaire_present: bool,
    pub opening_pools_present: bool,
    pub outputs: Vec<OutputStatusDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderStatusDto {
    pub label: String,
    pub path: String,
    pub exists: bool,
    pub file_count: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputStatusDto {
    pub label: String,
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPathsDto {
    pub root: String,
    pub config: String,
    pub raw: String,
    pub staging: String,
    pub out: String,
    pub logs: String,
    pub questionnaire: String,
    pub opening_pools: String,
    pub tax: String,
    pub evidence_pack: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRowsResult {
    pub rows: Vec<ReviewRowDto>,
    pub tax_event_types: Vec<String>,
    pub price_sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRowDto {
    pub event_id: String,
    pub timestamp: String,
    pub tax_year: String,
    pub source_id: String,
    pub platform: String,
    pub chain: String,
    pub wallet: String,
    pub tx_hash: String,
    pub detected_event_type: String,
    pub detected_direction: String,
    pub asset_symbol: String,
    pub asset_contract: String,
    pub amount: String,
    pub fee_asset: String,
    pub fee_amount: String,
    pub from_address: String,
    pub to_address: String,
    pub confidence: String,
    pub needs_review: bool,
    pub review_reasons: String,
    pub suggested_tax_type: String,
    pub user_tax_type: String,
    pub user_asset_symbol: String,
    pub user_quantity: String,
    pub user_proceeds_gbp: String,
    pub user_cost_gbp: String,
    pub user_income_gbp: String,
    pub user_fee_gbp: String,
    pub user_price_source: String,
    pub user_note: String,
    pub raw_file: String,
    pub json_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewOverrideDraft {
    pub event_id: String,
    #[serde(default)]
    pub user_action: Option<String>,
    #[serde(default)]
    pub user_tax_type: Option<String>,
    #[serde(default)]
    pub user_asset_symbol: Option<String>,
    #[serde(default)]
    pub user_quantity: Option<String>,
    #[serde(default)]
    pub user_proceeds_gbp: Option<String>,
    #[serde(default)]
    pub user_cost_gbp: Option<String>,
    #[serde(default)]
    pub user_income_gbp: Option<String>,
    #[serde(default)]
    pub user_fee_gbp: Option<String>,
    #[serde(default)]
    pub user_price_source: Option<String>,
    #[serde(default)]
    pub user_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReviewResult {
    pub appended: u64,
    pub change_log: String,
}

pub fn desktop_project_status(project: &str) -> Result<ProjectStatusDto> {
    let (paths, config) = crate::open_project(project)?;
    let folders = [
        ("raw", paths.raw()),
        ("staging", paths.staging()),
        ("out", paths.out()),
        ("logs", paths.logs()),
        ("tax", paths.root.join("tax")),
        ("evidence_pack", paths.root.join("evidence_pack")),
    ]
    .into_iter()
    .map(|(label, path)| folder_status(label, &path))
    .collect::<Result<Vec<_>>>()?;

    let outputs = [
        (
            "review_all_transactions",
            paths.out().join("review_all_transactions.csv"),
        ),
        ("manual_review", paths.out().join("manual_review.csv")),
        (
            "normalised_transactions",
            paths.out().join("normalised_transactions.csv"),
        ),
        ("analysis_export", paths.out().join("analysis_export.csv")),
        ("reviewed_ledger", paths.out().join("reviewed_ledger.csv")),
        ("priced_ledger", paths.out().join("priced_ledger.csv")),
        ("pricing_audit", paths.out().join("pricing_audit.csv")),
        ("audit_manifest", paths.out().join("audit_manifest.json")),
    ]
    .into_iter()
    .map(|(label, path)| OutputStatusDto {
        label: label.to_string(),
        exists: path.exists(),
        path: path.to_string(),
    })
    .collect();

    Ok(ProjectStatusDto {
        root: paths.root.to_string(),
        name: config.project.name,
        base_currency: config.project.base_currency,
        period_start: config.project.period_start,
        period_end: config.project.period_end,
        wallet_count: config.wallets.len(),
        cex_import_count: config.cex_csvs.len(),
        provider_count: config.providers.len(),
        folders,
        review_override_count: count_non_empty_lines(&paths.overrides_jsonl())?,
        price_observation_count: count_non_empty_lines(&paths.price_observations_jsonl())?,
        questionnaire_present: paths.questionnaire_file().exists(),
        opening_pools_present: paths.opening_pools_file().exists(),
        outputs,
    })
}

pub fn desktop_project_paths(project: &str, tax_year: Option<&str>) -> ProjectPathsDto {
    let paths = ProjectPaths::new(Utf8PathBuf::from(project));
    let tax = tax_year
        .map(|year| paths.tax_dir(year))
        .unwrap_or_else(|| paths.root.join("tax"));
    let evidence_pack = tax_year
        .map(|year| paths.evidence_dir(year))
        .unwrap_or_else(|| paths.root.join("evidence_pack"));

    ProjectPathsDto {
        root: paths.root.to_string(),
        config: paths.config_file().to_string(),
        raw: paths.raw().to_string(),
        staging: paths.staging().to_string(),
        out: paths.out().to_string(),
        logs: paths.logs().to_string(),
        questionnaire: paths.questionnaire_file().to_string(),
        opening_pools: paths.opening_pools_file().to_string(),
        tax: tax.to_string(),
        evidence_pack: evidence_pack.to_string(),
    }
}

pub fn load_review_rows(project: &str) -> Result<ReviewRowsResult> {
    let (paths, _) = crate::open_project(project)?;
    let events = tinotax_review::load_all_events(&paths)?;
    let overrides = tinotax_review::load_latest_overrides(&paths)?;
    let mut rows = Vec::with_capacity(events.len());
    let opt_dec = |d: Option<Decimal>| d.map(|v| v.to_string()).unwrap_or_default();

    for event in &events {
        let o = overrides.get(&event.event_id);
        let platform = match event.source_kind {
            SourceKind::CexCsv => event.chain.as_str(),
            SourceKind::Wallet | SourceKind::Manual => "",
        };
        rows.push(ReviewRowDto {
            event_id: event.event_id.clone(),
            timestamp: event.timestamp.clone(),
            tax_year: uk_tax_year(&event.timestamp).unwrap_or_default(),
            source_id: event.source_id.clone(),
            platform: platform.to_string(),
            chain: event.chain.clone(),
            wallet: event.wallet.clone(),
            tx_hash: event.tx_hash.clone(),
            detected_event_type: event.event_type.as_str().to_string(),
            detected_direction: event.direction.as_str().to_string(),
            asset_symbol: event.asset_symbol.clone(),
            asset_contract: event.asset_contract.clone().unwrap_or_default(),
            amount: event.amount.to_string(),
            fee_asset: event.fee_asset.clone().unwrap_or_default(),
            fee_amount: opt_dec(event.fee_amount),
            from_address: event.from_address.clone().unwrap_or_default(),
            to_address: event.to_address.clone().unwrap_or_default(),
            confidence: event.confidence.as_str().to_string(),
            needs_review: event.needs_review,
            review_reasons: event.review_reasons.join("; "),
            suggested_tax_type: TaxEventType::suggest(event.event_type, event.direction)
                .as_str()
                .to_string(),
            user_tax_type: o
                .and_then(|o| o.user_tax_type)
                .map(|t| t.as_str().to_string())
                .unwrap_or_default(),
            user_asset_symbol: o
                .and_then(|o| o.user_asset_symbol.clone())
                .unwrap_or_default(),
            user_quantity: opt_dec(o.and_then(|o| o.user_quantity)),
            user_proceeds_gbp: opt_dec(o.and_then(|o| o.user_proceeds_gbp)),
            user_cost_gbp: opt_dec(o.and_then(|o| o.user_cost_gbp)),
            user_income_gbp: opt_dec(o.and_then(|o| o.user_income_gbp)),
            user_fee_gbp: opt_dec(o.and_then(|o| o.user_fee_gbp)),
            user_price_source: o
                .and_then(|o| o.user_price_source.clone())
                .unwrap_or_default(),
            user_note: o.and_then(|o| o.user_note.clone()).unwrap_or_default(),
            raw_file: event.source_ref.raw_file.clone(),
            json_path: event.source_ref.json_path.clone().unwrap_or_default(),
        });
    }

    Ok(ReviewRowsResult {
        rows,
        tax_event_types: TAX_EVENT_TYPES.iter().map(|s| (*s).to_string()).collect(),
        price_sources: PRICE_SOURCES.iter().map(|s| (*s).to_string()).collect(),
    })
}

pub fn save_review_overrides(
    project: &str,
    drafts: Vec<ReviewOverrideDraft>,
) -> Result<SaveReviewResult> {
    let (paths, _) = crate::open_project(project)?;
    let known_ids: BTreeSet<String> = tinotax_review::load_all_events(&paths)?
        .into_iter()
        .map(|event| event.event_id)
        .collect();

    let source_file = "desktop_review".to_string();
    let mut overrides = Vec::new();
    for (index, draft) in drafts.into_iter().enumerate() {
        let row = index + 1;
        let override_record = draft_to_override(draft, row, &known_ids, &source_file)?;
        if override_record.has_any_decision() {
            overrides.push(override_record);
        }
    }

    fs::create_dir_all(paths.staging())?;
    let mut writer = JsonlWriter::append(&paths.overrides_jsonl())?;
    for item in &overrides {
        writer.write(item)?;
    }
    let appended = writer.finish()?;
    tinotax_review::write_change_log(&paths).context("regenerating out/change_log.csv")?;

    Ok(SaveReviewResult {
        appended,
        change_log: paths.out().join("change_log.csv").to_string(),
    })
}

fn draft_to_override(
    draft: ReviewOverrideDraft,
    row: usize,
    known_ids: &BTreeSet<String>,
    source_file: &str,
) -> Result<ReviewOverride> {
    let event_id = draft.event_id.trim().to_string();
    if event_id.is_empty() {
        bail!("row {row}: event_id is required");
    }
    if !known_ids.contains(&event_id) {
        bail!("row {row}: event_id {event_id:?} does not exist in this project");
    }

    let user_action = parse_optional::<ReviewAction>(draft.user_action, row, "user_action")?;
    let user_tax_type = parse_optional::<TaxEventType>(draft.user_tax_type, row, "user_tax_type")?;
    let user_price_source =
        parse_optional::<PriceSource>(draft.user_price_source, row, "user_price_source")?
            .map(|source| source.as_str().to_string());
    let user_quantity = parse_optional_decimal(draft.user_quantity, row, "user_quantity")?;
    if let Some(quantity) = user_quantity {
        if quantity < Decimal::ZERO {
            bail!("row {row}: user_quantity must be >= 0");
        }
    }

    Ok(ReviewOverride {
        event_id,
        user_action,
        user_tax_type,
        user_asset_symbol: clean_optional_string(draft.user_asset_symbol),
        user_quantity,
        user_proceeds_gbp: parse_optional_decimal(
            draft.user_proceeds_gbp,
            row,
            "user_proceeds_gbp",
        )?,
        user_cost_gbp: parse_optional_decimal(draft.user_cost_gbp, row, "user_cost_gbp")?,
        user_income_gbp: parse_optional_decimal(draft.user_income_gbp, row, "user_income_gbp")?,
        user_fee_gbp: parse_optional_decimal(draft.user_fee_gbp, row, "user_fee_gbp")?,
        user_price_source,
        user_note: clean_optional_string(draft.user_note),
        applied_at: tinotax_store::now_rfc3339(),
        source_file: Some(source_file.to_string()),
    })
}

fn parse_optional<T>(value: Option<String>, row: usize, field: &str) -> Result<Option<T>>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    clean_optional_string(value)
        .map(|text| {
            T::from_str(&text).with_context(|| format!("row {row}: invalid {field} {text:?}"))
        })
        .transpose()
}

fn parse_optional_decimal(
    value: Option<String>,
    row: usize,
    field: &str,
) -> Result<Option<Decimal>> {
    clean_optional_string(value)
        .map(|text| {
            Decimal::from_str(&text).with_context(|| format!("row {row}: invalid {field} {text:?}"))
        })
        .transpose()
}

fn clean_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn folder_status(label: &str, path: &Utf8Path) -> Result<FolderStatusDto> {
    let (file_count, bytes) = if path.exists() {
        dir_stats(path)?
    } else {
        (0, 0)
    };
    Ok(FolderStatusDto {
        label: label.to_string(),
        path: path.to_string(),
        exists: path.exists(),
        file_count,
        bytes,
    })
}

fn dir_stats(dir: &Utf8Path) -> Result<(u64, u64)> {
    let mut files = 0u64;
    let mut bytes = 0u64;
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.with_context(|| format!("walking {dir}"))?;
        if entry.file_type().is_file() {
            files += 1;
            bytes += entry
                .metadata()
                .with_context(|| format!("reading metadata for {}", entry.path().display()))?
                .len();
        }
    }
    Ok((files, bytes))
}

fn count_non_empty_lines(path: &Utf8Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let text = fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    Ok(text.lines().filter(|line| !line.trim().is_empty()).count() as u64)
}

const TAX_EVENT_TYPES: &[&str] = &[
    "acquisition",
    "disposal",
    "swap_disposal",
    "swap_acquisition",
    "transfer_in",
    "transfer_out",
    "bridge_in",
    "bridge_out",
    "fee",
    "staking_reward",
    "mining_reward",
    "airdrop",
    "fork",
    "employment_income",
    "self_employment_income",
    "misc_income",
    "compensation",
    "goods_or_services_spend",
    "ignore",
    "unknown",
];

const PRICE_SOURCES: &[&str] = &["user_provided", "manual", "cex", "coingecko"];

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use rust_decimal::Decimal;

    fn known_ids() -> BTreeSet<String> {
        BTreeSet::from(["evt_1".to_string()])
    }

    #[test]
    fn desktop_review_draft_validates_and_parses() -> Result<(), Box<dyn Error>> {
        let draft = ReviewOverrideDraft {
            event_id: "evt_1".into(),
            user_action: None,
            user_tax_type: Some("staking_reward".into()),
            user_asset_symbol: Some(" near ".into()),
            user_quantity: Some("1.25".into()),
            user_proceeds_gbp: None,
            user_cost_gbp: None,
            user_income_gbp: Some("10.50".into()),
            user_fee_gbp: None,
            user_price_source: Some("user_provided".into()),
            user_note: Some("desktop edit".into()),
        };

        let parsed = draft_to_override(draft, 1, &known_ids(), "desktop")?;
        assert_eq!(parsed.user_tax_type, Some(TaxEventType::StakingReward));
        assert_eq!(parsed.user_asset_symbol.as_deref(), Some("near"));
        assert_eq!(parsed.user_quantity, Some(Decimal::new(125, 2)));
        assert_eq!(parsed.user_income_gbp, Some(Decimal::new(1050, 2)));
        assert_eq!(parsed.user_price_source.as_deref(), Some("user_provided"));
        Ok(())
    }

    #[test]
    fn desktop_review_draft_rejects_unknown_event() -> Result<(), Box<dyn Error>> {
        let draft = ReviewOverrideDraft {
            event_id: "missing".into(),
            user_action: None,
            user_tax_type: Some("ignore".into()),
            user_asset_symbol: None,
            user_quantity: None,
            user_proceeds_gbp: None,
            user_cost_gbp: None,
            user_income_gbp: None,
            user_fee_gbp: None,
            user_price_source: None,
            user_note: None,
        };

        let err = match draft_to_override(draft, 1, &known_ids(), "desktop") {
            Ok(_) => return Err(std::io::Error::other("expected unknown-event error").into()),
            Err(err) => err,
        };
        assert!(err.to_string().contains("does not exist"));
        Ok(())
    }

    #[test]
    fn desktop_review_draft_rejects_negative_quantity() -> Result<(), Box<dyn Error>> {
        let draft = ReviewOverrideDraft {
            event_id: "evt_1".into(),
            user_action: None,
            user_tax_type: Some("ignore".into()),
            user_asset_symbol: None,
            user_quantity: Some("-1".into()),
            user_proceeds_gbp: None,
            user_cost_gbp: None,
            user_income_gbp: None,
            user_fee_gbp: None,
            user_price_source: None,
            user_note: None,
        };

        let err = match draft_to_override(draft, 1, &known_ids(), "desktop") {
            Ok(_) => return Err(std::io::Error::other("expected negative-quantity error").into()),
            Err(err) => err,
        };
        assert!(err.to_string().contains("user_quantity"));
        Ok(())
    }
}
