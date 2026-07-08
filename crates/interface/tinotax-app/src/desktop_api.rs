//! JSON-friendly APIs used by the desktop/webview app.
//!
//! The CLI prints human output. The desktop app needs typed data, so this
//! module exposes thin DTOs over the same project and review primitives.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tinotax_core::{
    uk_tax_year, Chain, EventType, NormalisedEvent, PriceSource, ReviewAction, ReviewOverride,
    SourceKind, TaxEventType,
};
use tinotax_store::{JsonlWriter, ProjectPaths};

use crate::event_cache::load_events_cached;

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
pub struct ProjectDataViewDto {
    pub artifacts: Vec<DataArtifactDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataArtifactDto {
    pub stage: String,
    pub label: String,
    pub kind: String,
    pub path: String,
    pub exists: bool,
    pub bytes: u64,
    pub item_count: u64,
    pub item_label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRowsResult {
    pub rows: Vec<ReviewRowDto>,
    pub tax_event_types: Vec<String>,
    pub price_sources: Vec<String>,
}

/// A filtered, paginated request for review rows.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewQuery {
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub needs_review_only: bool,
    #[serde(default)]
    pub unknown_only: bool,
    /// Rows still wanting a human decision: flagged OR effective type unknown.
    #[serde(default)]
    pub needs_attention_only: bool,
    #[serde(default)]
    pub tax_year: Option<String>,
    #[serde(default)]
    pub asset: Option<String>,
    #[serde(default)]
    pub chain: Option<String>,
    /// Detected event type (fee, contract_call, token_transfer, …).
    #[serde(default)]
    pub event_type: Option<String>,
    /// Effective tax type (user override if set, else the machine suggestion).
    #[serde(default)]
    pub tax_type: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    /// Column to sort by: `time` (default), `asset`, `amount`, `type`,
    /// `taxType`, `chain`.
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub sort_desc: bool,
}

/// One page of review rows plus the facets the UI needs to render filters,
/// counts and pagination without loading the whole project.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPage {
    pub rows: Vec<ReviewRowDto>,
    pub offset: usize,
    pub limit: usize,
    /// Rows matching the current filter (across the whole project).
    pub total: usize,
    /// All rows in the project, before filtering.
    pub grand_total: usize,
    pub needs_review_count: usize,
    /// Rows wanting a human decision (flagged OR effective type unknown).
    pub needs_attention_count: usize,
    /// Zero-value contract calls still eligible for bulk-ignore.
    pub ignorable_contract_calls: usize,
    pub assets: Vec<String>,
    pub tax_years: Vec<String>,
    pub chains: Vec<String>,
    pub event_types: Vec<String>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletConfigResult {
    pub project_name: String,
    pub base_currency: String,
    pub period_start: String,
    pub period_end: String,
    pub cex_import_count: usize,
    pub price_provider: String,
    /// Whether a CoinGecko key is present for historical `prices fetch`.
    pub pricing_api_ready: bool,
    pub pricing_api_reason: String,
    pub wallets: Vec<WalletSourceDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletSourceDto {
    pub id: String,
    pub name: String,
    pub chain: String,
    pub address: String,
    pub provider: String,
    pub api_kind: String,
    pub api_url: String,
    pub native_asset: String,
    pub enabled: bool,
    pub disabled_reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HmrcQuestionnaireResponseDraft {
    pub id: String,
    pub question: String,
    pub answer: String,
    #[serde(default)]
    pub choice: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HmrcQuestionnaireExportResult {
    pub pdf_path: String,
    pub questionnaire_path: String,
}

pub fn desktop_config_wallets(config: &str) -> Result<WalletConfigResult> {
    let config_path = crate::resolve_config_path(config)?;
    let config = tinotax_config::ProjectConfig::load(&config_path)
        .with_context(|| format!("loading wallet config {config_path}"))?;

    // Gating is driven by which API a wallet actually needs, not a hardcoded
    // chain: public Blockscout explorers (Lisk, IOTA EVM) are keyless;
    // NearBlocks needs a key present before it can be synced.
    let nearblocks_ready = env_present("NEARBLOCKS_API_KEY");
    let wallets = config
        .wallets
        .iter()
        .map(|wallet| {
            let provider = config.provider_for(wallet);
            let chain = Chain::from(wallet.chain.clone());
            let (enabled, disabled_reason) = wallet_gate(provider.kind, nearblocks_ready);
            WalletSourceDto {
                id: wallet.id.clone(),
                name: wallet.name.clone(),
                chain: wallet.chain.clone(),
                address: wallet.address.clone(),
                provider: wallet.provider.clone(),
                api_kind: provider_kind_label(provider.kind).to_string(),
                api_url: provider.base_url.clone(),
                native_asset: chain.native_symbol().to_string(),
                enabled,
                disabled_reason,
            }
        })
        .collect();

    // Pricing is a project-level capability: `prices fetch` for anything
    // older than ~365 days needs a paid CoinGecko key.
    let pricing_api_ready = env_present("COINGECKO_PRO_API_KEY")
        || env_present("COINGECKO_DEMO_API_KEY")
        || env_present("COINGECKO_API_KEY");
    let pricing_api_reason = if pricing_api_ready {
        "CoinGecko key detected — historical GBP fetch available".to_string()
    } else {
        "No CoinGecko key set — historical GBP fetch (older than 365 days) needs a paid key"
            .to_string()
    };

    Ok(WalletConfigResult {
        project_name: config.project.name,
        base_currency: config.project.base_currency,
        period_start: config.project.period_start,
        period_end: config.project.period_end,
        cex_import_count: config.cex_csvs.len(),
        price_provider: "CoinGecko historical GBP".to_string(),
        pricing_api_ready,
        pricing_api_reason,
        wallets,
    })
}

/// Whether a wallet's provider can be synced from the desktop, and why not.
/// Blockscout is keyless; NearBlocks needs its API key present.
fn wallet_gate(kind: tinotax_config::ProviderKind, nearblocks_ready: bool) -> (bool, String) {
    match kind {
        tinotax_config::ProviderKind::Blockscout => (true, String::new()),
        tinotax_config::ProviderKind::Nearblocks => {
            if nearblocks_ready {
                (true, String::new())
            } else {
                (
                    false,
                    "Needs NEARBLOCKS_API_KEY (paid plan) — set it, then reload wallets".to_string(),
                )
            }
        }
    }
}

fn env_present(name: &str) -> bool {
    std::env::var(name).ok().filter(|v| !v.is_empty()).is_some()
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

pub fn desktop_default_project() -> Option<String> {
    // No auto-guessed default project. Returning users reopen via the Recent
    // projects list (persisted client-side); first-run users create or open one.
    // (Previously this searched for hardcoded `fox-project-*` folders, which
    // wrongly auto-opened the developer's project and broke test isolation.)
    None
}

pub fn desktop_project_data_view(
    project: &str,
    tax_year: Option<&str>,
) -> Result<ProjectDataViewDto> {
    let (paths, _) = crate::open_project(project)?;
    let tax_year = tax_year.unwrap_or("2024-2025");
    let tax_dir = paths.tax_dir(tax_year);
    let evidence_dir = paths.evidence_dir(tax_year);

    let mut artifacts = Vec::new();
    push_file(
        &mut artifacts,
        "Input",
        "Project config",
        paths.config_file(),
    )?;
    push_folder(
        &mut artifacts,
        "Input",
        "Raw wallet and CEX data",
        paths.raw(),
    )?;
    push_file(
        &mut artifacts,
        "Input",
        "HMRC questionnaire answers",
        paths.questionnaire_file(),
    )?;
    push_file(
        &mut artifacts,
        "Input",
        "Opening pools",
        paths.opening_pools_file(),
    )?;

    push_file(
        &mut artifacts,
        "Staging",
        "Wallet normalised events",
        paths.events_jsonl(),
    )?;
    push_file(
        &mut artifacts,
        "Staging",
        "CEX normalised events",
        paths.cex_events_jsonl(),
    )?;
    push_file(
        &mut artifacts,
        "Staging",
        "Rejected raw items",
        paths.rejected_jsonl(),
    )?;
    push_file(
        &mut artifacts,
        "Staging",
        "Warnings",
        paths.warnings_jsonl(),
    )?;

    push_file(
        &mut artifacts,
        "Review",
        "All review rows",
        paths.out().join("review_all_transactions.csv"),
    )?;
    push_file(
        &mut artifacts,
        "Review",
        "Manual review rows",
        paths.out().join("manual_review.csv"),
    )?;
    push_file(
        &mut artifacts,
        "Review",
        "Review overrides",
        paths.overrides_jsonl(),
    )?;
    push_file(
        &mut artifacts,
        "Review",
        "Reviewed ledger",
        paths.out().join("reviewed_ledger.csv"),
    )?;

    push_file(
        &mut artifacts,
        "Pricing",
        "Price observations",
        paths.price_observations_jsonl(),
    )?;
    push_file(
        &mut artifacts,
        "Pricing",
        "CEX price hints",
        paths.price_hints_jsonl(),
    )?;
    push_file(
        &mut artifacts,
        "Pricing",
        "Missing prices",
        paths.out().join("missing_prices.csv"),
    )?;
    push_file(
        &mut artifacts,
        "Pricing",
        "Priced ledger",
        paths.out().join("priced_ledger.csv"),
    )?;
    push_file(
        &mut artifacts,
        "Pricing",
        "Pricing audit",
        paths.out().join("pricing_audit.csv"),
    )?;

    push_file(
        &mut artifacts,
        "Tax",
        "Self Assessment summary",
        tax_dir.join("self_assessment_crypto_summary.csv"),
    )?;
    push_file(
        &mut artifacts,
        "Tax",
        "Disposals calculation",
        tax_dir.join("disposals_calculation.csv"),
    )?;
    push_file(
        &mut artifacts,
        "Tax",
        "S104 pool movements",
        tax_dir.join("s104_pool_movements.csv"),
    )?;
    push_file(
        &mut artifacts,
        "Tax",
        "S104 opening and closing pools",
        tax_dir.join("s104_pool_opening_closing.csv"),
    )?;
    push_file(
        &mut artifacts,
        "Tax",
        "Income summary",
        tax_dir.join("income_summary.csv"),
    )?;
    push_file(
        &mut artifacts,
        "Tax",
        "Unresolved tax items",
        tax_dir.join("unresolved_tax_items.csv"),
    )?;

    push_folder(
        &mut artifacts,
        "Evidence",
        "Evidence pack",
        evidence_dir.clone(),
    )?;
    push_file(
        &mut artifacts,
        "Evidence",
        "HMRC questions draft",
        evidence_dir.join("hmrc_questions_draft.md"),
    )?;
    push_file(
        &mut artifacts,
        "Evidence",
        "Assumptions and limitations",
        evidence_dir.join("assumptions_and_limitations.md"),
    )?;
    push_file(
        &mut artifacts,
        "Evidence",
        "Counterparties (Q5 protocols)",
        evidence_dir.join("counterparties.csv"),
    )?;
    push_file(
        &mut artifacts,
        "Evidence",
        "Questionnaire PDF",
        paths.out().join("hmrc_questionnaire_responses.pdf"),
    )?;
    push_file(
        &mut artifacts,
        "Evidence",
        "Audit manifest",
        paths.out().join("audit_manifest.json"),
    )?;

    Ok(ProjectDataViewDto { artifacts })
}

/// Build one review row DTO from an event and its latest override (if any).
fn build_review_row(event: &NormalisedEvent, o: Option<&ReviewOverride>) -> ReviewRowDto {
    let opt_dec = |d: Option<Decimal>| d.map(|v| v.to_string()).unwrap_or_default();
    let platform = match event.source_kind {
        SourceKind::CexCsv => event.chain.as_str(),
        SourceKind::Wallet | SourceKind::Manual => "",
    };
    ReviewRowDto {
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
        user_asset_symbol: o.and_then(|o| o.user_asset_symbol.clone()).unwrap_or_default(),
        user_quantity: opt_dec(o.and_then(|o| o.user_quantity)),
        user_proceeds_gbp: opt_dec(o.and_then(|o| o.user_proceeds_gbp)),
        user_cost_gbp: opt_dec(o.and_then(|o| o.user_cost_gbp)),
        user_income_gbp: opt_dec(o.and_then(|o| o.user_income_gbp)),
        user_fee_gbp: opt_dec(o.and_then(|o| o.user_fee_gbp)),
        user_price_source: o.and_then(|o| o.user_price_source.clone()).unwrap_or_default(),
        user_note: o.and_then(|o| o.user_note.clone()).unwrap_or_default(),
        raw_file: event.source_ref.raw_file.clone(),
        json_path: event.source_ref.json_path.clone().unwrap_or_default(),
    }
}

/// The tax type that currently applies to an event: the user's override if set,
/// otherwise the machine suggestion. Mirrors the frontend `effectiveTaxType`.
fn effective_tax_type(event: &NormalisedEvent, o: Option<&ReviewOverride>) -> TaxEventType {
    o.and_then(|o| o.user_tax_type)
        .unwrap_or_else(|| TaxEventType::suggest(event.event_type, event.direction))
}

/// A zero-value contract call with no user tax decision yet — the bulk-ignore
/// candidate (non-taxable envelope: no asset moved).
fn is_ignorable_contract_call(event: &NormalisedEvent, o: Option<&ReviewOverride>) -> bool {
    matches!(event.event_type, EventType::ContractCall)
        && event.amount.is_zero()
        && o.and_then(|o| o.user_tax_type).is_none()
}

/// Rows that still want a human decision: not yet given a tax type by the user,
/// and either machine-flagged or with an effective type still `unknown`. A row
/// the user has already classified (even to `ignore`) no longer needs attention.
fn needs_attention(event: &NormalisedEvent, o: Option<&ReviewOverride>) -> bool {
    if o.and_then(|o| o.user_tax_type).is_some() {
        return false;
    }
    event.needs_review || effective_tax_type(event, o) == TaxEventType::Unknown
}

fn matches_query(event: &NormalisedEvent, o: Option<&ReviewOverride>, q: &ReviewQuery) -> bool {
    if q.needs_review_only && !event.needs_review {
        return false;
    }
    if q.unknown_only && effective_tax_type(event, o) != TaxEventType::Unknown {
        return false;
    }
    if q.needs_attention_only && !needs_attention(event, o) {
        return false;
    }
    if let Some(year) = q.tax_year.as_deref().filter(|y| !y.is_empty()) {
        if uk_tax_year(&event.timestamp).unwrap_or_default() != year {
            return false;
        }
    }
    if let Some(asset) = q.asset.as_deref().filter(|a| !a.is_empty()) {
        if event.asset_symbol != asset {
            return false;
        }
    }
    if let Some(chain) = q.chain.as_deref().filter(|c| !c.is_empty()) {
        if event.chain != chain {
            return false;
        }
    }
    if let Some(event_type) = q.event_type.as_deref().filter(|e| !e.is_empty()) {
        if event.event_type.as_str() != event_type {
            return false;
        }
    }
    if let Some(tax_type) = q.tax_type.as_deref().filter(|t| !t.is_empty()) {
        if effective_tax_type(event, o).as_str() != tax_type {
            return false;
        }
    }
    if let Some(text) = q.text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let needle = text.to_lowercase();
        let note = o.and_then(|o| o.user_note.clone()).unwrap_or_default();
        let hay = [
            event.event_id.as_str(),
            event.tx_hash.as_str(),
            event.source_id.as_str(),
            event.asset_symbol.as_str(),
            event.wallet.as_str(),
            &event.review_reasons.join("; "),
            note.as_str(),
        ]
        .join(" ")
        .to_lowercase();
        if !hay.contains(&needle) {
            return false;
        }
    }
    true
}

/// Full (unpaginated) review rows — retained for the CLI/tests. The desktop app
/// uses [`load_review_page`] to avoid shipping every row over IPC.
pub fn load_review_rows(project: &str) -> Result<ReviewRowsResult> {
    let (paths, _) = crate::open_project(project)?;
    let events = load_events_cached(&paths)?;
    let overrides = tinotax_review::load_latest_overrides(&paths)?;
    let rows = events
        .iter()
        .map(|event| build_review_row(event, overrides.get(&event.event_id)))
        .collect();
    Ok(ReviewRowsResult {
        rows,
        tax_event_types: TAX_EVENT_TYPES.iter().map(|s| (*s).to_string()).collect(),
        price_sources: PRICE_SOURCES.iter().map(|s| (*s).to_string()).collect(),
    })
}

/// One filtered, paginated page of review rows plus the facets the UI needs
/// (distinct assets/years, counts). Only the page's rows are materialised as
/// DTOs, so the IPC payload is bounded regardless of project size.
pub fn load_review_page(project: &str, query: &ReviewQuery) -> Result<ReviewPage> {
    let (paths, _) = crate::open_project(project)?;
    let events = load_events_cached(&paths)?;
    let overrides = tinotax_review::load_latest_overrides(&paths)?;
    let limit = query.limit.clamp(1, 2000);

    let mut assets: BTreeSet<String> = BTreeSet::new();
    let mut tax_years: BTreeSet<String> = BTreeSet::new();
    let mut chains: BTreeSet<String> = BTreeSet::new();
    let mut event_types: BTreeSet<String> = BTreeSet::new();
    let mut needs_review_count = 0usize;
    let mut needs_attention_count = 0usize;
    let mut ignorable_contract_calls = 0usize;
    let mut matched: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        let o = overrides.get(&event.event_id);
        if !event.asset_symbol.is_empty() {
            assets.insert(event.asset_symbol.clone());
        }
        if let Ok(year) = uk_tax_year(&event.timestamp) {
            if !year.is_empty() {
                tax_years.insert(year);
            }
        }
        if !event.chain.is_empty() {
            chains.insert(event.chain.clone());
        }
        event_types.insert(event.event_type.as_str().to_string());
        if event.needs_review {
            needs_review_count += 1;
        }
        if needs_attention(event, o) {
            needs_attention_count += 1;
        }
        if is_ignorable_contract_call(event, o) {
            ignorable_contract_calls += 1;
        }
        if matches_query(event, o, query) {
            matched.push(index);
        }
    }

    // Sort the matched set before paging so ordering is stable across pages.
    // `time` (the default) is already the load order, so only re-sort otherwise.
    let ev = |i: usize| &events[i];
    match query.sort_by.as_deref() {
        Some("asset") => matched.sort_by(|&a, &b| ev(a).asset_symbol.cmp(&ev(b).asset_symbol)),
        Some("amount") => matched.sort_by(|&a, &b| ev(a).amount.cmp(&ev(b).amount)),
        Some("type") => {
            matched.sort_by(|&a, &b| ev(a).event_type.as_str().cmp(ev(b).event_type.as_str()))
        }
        Some("chain") => matched.sort_by(|&a, &b| ev(a).chain.cmp(&ev(b).chain)),
        Some("taxType") => matched.sort_by(|&a, &b| {
            let ta = effective_tax_type(ev(a), overrides.get(&ev(a).event_id));
            let tb = effective_tax_type(ev(b), overrides.get(&ev(b).event_id));
            ta.as_str().cmp(tb.as_str())
        }),
        _ => matched.sort_by(|&a, &b| ev(a).timestamp.cmp(&ev(b).timestamp)),
    }
    if query.sort_desc {
        matched.reverse();
    }

    let total = matched.len();
    let rows = matched
        .into_iter()
        .skip(query.offset)
        .take(limit)
        .map(|index| {
            let event = &events[index];
            build_review_row(event, overrides.get(&event.event_id))
        })
        .collect();

    Ok(ReviewPage {
        rows,
        offset: query.offset,
        limit,
        total,
        grand_total: events.len(),
        needs_review_count,
        needs_attention_count,
        ignorable_contract_calls,
        assets: assets.into_iter().collect(),
        tax_years: tax_years.into_iter().collect(),
        chains: chains.into_iter().collect(),
        event_types: event_types.into_iter().collect(),
        tax_event_types: TAX_EVENT_TYPES.iter().map(|s| (*s).to_string()).collect(),
        price_sources: PRICE_SOURCES.iter().map(|s| (*s).to_string()).collect(),
    })
}

/// Set `tax_type` on every row matching `query` (append-only overrides). Powers
/// the "set all matching rows" bulk action. Returns how many were written.
pub fn bulk_set_review(project: &str, query: &ReviewQuery, tax_type: &str) -> Result<SaveReviewResult> {
    let parsed: TaxEventType = tax_type
        .parse()
        .with_context(|| format!("unknown tax type {tax_type:?}"))?;
    let (paths, _) = crate::open_project(project)?;
    let events = load_events_cached(&paths)?;
    let overrides = tinotax_review::load_latest_overrides(&paths)?;

    let mut records = Vec::new();
    for event in events.iter() {
        let o = overrides.get(&event.event_id);
        if matches_query(event, o, query) {
            records.push(ReviewOverride {
                event_id: event.event_id.clone(),
                user_action: None,
                user_tax_type: Some(parsed),
                user_asset_symbol: None,
                user_quantity: None,
                user_proceeds_gbp: None,
                user_cost_gbp: None,
                user_income_gbp: None,
                user_fee_gbp: None,
                user_price_source: None,
                user_note: Some(format!("bulk: set to {} via review filter", parsed.as_str())),
                applied_at: tinotax_store::now_rfc3339(),
                source_file: Some("desktop_bulk_review".to_string()),
            });
        }
    }

    fs::create_dir_all(paths.staging())?;
    let mut writer = JsonlWriter::append(&paths.overrides_jsonl())?;
    for item in &records {
        writer.write(item)?;
    }
    let appended = writer.finish()?;
    tinotax_review::write_change_log(&paths).context("regenerating out/change_log.csv")?;
    Ok(SaveReviewResult {
        appended,
        change_log: paths.out().join("change_log.csv").to_string(),
    })
}

/// Bulk-classify every zero-value contract call as `ignore` (non-taxable),
/// writing auditable override records. Server-side so it never ships the whole
/// event set to the client. Returns how many overrides were appended.
pub fn auto_classify_contract_calls(project: &str) -> Result<SaveReviewResult> {
    let (paths, _) = crate::open_project(project)?;
    let events = load_events_cached(&paths)?;
    let overrides = tinotax_review::load_latest_overrides(&paths)?;

    let mut records = Vec::new();
    for event in events.iter() {
        if is_ignorable_contract_call(event, overrides.get(&event.event_id)) {
            records.push(ReviewOverride {
                event_id: event.event_id.clone(),
                user_action: None,
                user_tax_type: Some(TaxEventType::Ignore),
                user_asset_symbol: None,
                user_quantity: None,
                user_proceeds_gbp: None,
                user_cost_gbp: None,
                user_income_gbp: None,
                user_fee_gbp: None,
                user_price_source: None,
                user_note: Some("auto: zero-value contract call (non-taxable)".to_string()),
                applied_at: tinotax_store::now_rfc3339(),
                source_file: Some("desktop_auto_classify".to_string()),
            });
        }
    }

    fs::create_dir_all(paths.staging())?;
    let mut writer = JsonlWriter::append(&paths.overrides_jsonl())?;
    for item in &records {
        writer.write(item)?;
    }
    let appended = writer.finish()?;
    tinotax_review::write_change_log(&paths).context("regenerating out/change_log.csv")?;

    Ok(SaveReviewResult {
        appended,
        change_log: paths.out().join("change_log.csv").to_string(),
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

pub fn export_hmrc_questionnaire(
    project: &str,
    responses: Vec<HmrcQuestionnaireResponseDraft>,
) -> Result<HmrcQuestionnaireExportResult> {
    let (paths, config) = crate::open_project(project)?;
    fs::create_dir_all(paths.out())?;

    let questionnaire = build_questionnaire_toml(&config, &responses);
    let questionnaire_path = paths.questionnaire_file();
    fs::write(&questionnaire_path, questionnaire)
        .with_context(|| format!("writing {questionnaire_path}"))?;

    let pdf_path = paths.out().join("hmrc_questionnaire_responses.pdf");
    let pdf = build_questionnaire_pdf(&config.project.name, &responses)?;
    fs::write(&pdf_path, pdf).with_context(|| format!("writing {pdf_path}"))?;

    Ok(HmrcQuestionnaireExportResult {
        pdf_path: pdf_path.to_string(),
        questionnaire_path: questionnaire_path.to_string(),
    })
}

fn build_questionnaire_toml(
    config: &tinotax_config::ProjectConfig,
    responses: &[HmrcQuestionnaireResponseDraft],
) -> String {
    let answer = |id: &str| {
        responses
            .iter()
            .find(|response| response.id == id)
            .map(|response| response.answer.trim())
            .unwrap_or("")
    };
    let choice = |id: &str| {
        responses
            .iter()
            .find(|response| response.id == id)
            .and_then(|response| response.choice.as_deref())
            .and_then(choice_to_bool)
    };

    let mut text = String::new();
    text.push_str("# HMRC cryptoasset questionnaire answers exported from TinoTax desktop.\n");
    text.push_str("# Re-export from the app after changes, then re-run `tinotax pack hmrc`.\n\n");

    text.push_str("[activity]\n");
    push_key_value(&mut text, "began_on", answer("q1"));
    push_key_value(&mut text, "notes", "");
    text.push('\n');

    text.push_str("[source_of_funds]\n");
    push_key_value(&mut text, "summary", answer("q13"));
    text.push_str("bank_statement_refs = []\n\n");

    text.push_str("[forks]\n");
    push_optional_bool(&mut text, "received_forks", choice("q7"));
    push_key_value(&mut text, "notes", answer("q7"));
    text.push('\n');

    text.push_str("[airdrops]\n");
    push_optional_bool(&mut text, "received_airdrops", choice("q8"));
    push_key_value(&mut text, "notes", answer("q8"));
    text.push('\n');

    text.push_str("[compensation]\n");
    push_optional_bool(&mut text, "received_compensation", choice("q9"));
    push_key_value(&mut text, "notes", answer("q9"));
    text.push('\n');

    text.push_str("[employment]\n");
    push_optional_bool(&mut text, "received_crypto_from_employment", choice("q10"));
    push_key_value(&mut text, "paye_operated", "unknown");
    push_key_value(&mut text, "notes", answer("q10"));
    text.push('\n');

    text.push_str("[mining_staking]\n");
    push_optional_bool(&mut text, "engaged_in_mining_or_staking", choice("q11"));
    push_key_value(&mut text, "notes", answer("q11"));
    text.push('\n');

    text.push_str("[goods_services]\n");
    push_optional_bool(
        &mut text,
        "used_crypto_to_buy_goods_or_services",
        choice("q12"),
    );
    push_key_value(&mut text, "notes", answer("q12"));
    text.push('\n');

    text.push_str("[hmrc_questionnaire]\n");
    push_key_value(&mut text, "exported_at", &tinotax_store::now_rfc3339());
    push_key_value(&mut text, "project_name", &config.project.name);
    push_key_value(&mut text, "period_start", &config.project.period_start);
    push_key_value(&mut text, "period_end", &config.project.period_end);
    text.push('\n');

    for response in responses {
        text.push_str("[[hmrc_questionnaire.responses]]\n");
        push_key_value(&mut text, "id", &response.id);
        push_key_value(&mut text, "question", &response.question);
        if let Some(choice) = clean_optional_string(response.choice.clone()) {
            push_key_value(&mut text, "choice", &choice);
        }
        push_key_value(&mut text, "answer", &response.answer);
        text.push('\n');
    }

    text
}

fn build_questionnaire_pdf(
    project_name: &str,
    responses: &[HmrcQuestionnaireResponseDraft],
) -> Result<Vec<u8>> {
    let mut lines = Vec::new();
    lines.push("HMRC Cryptoasset Questionnaire Responses".to_string());
    lines.push(format!("Project: {}", printable_text(project_name)));
    lines.push(format!("Exported: {}", tinotax_store::now_rfc3339()));
    lines.push(String::new());

    for response in responses {
        lines.extend(wrap_line(
            &format!(
                "{} {}",
                response.id.to_ascii_uppercase(),
                response.question.trim()
            ),
            88,
        ));
        if let Some(choice) = clean_optional_string(response.choice.clone()) {
            lines.extend(wrap_line(&format!("Response: {choice}"), 88));
        }
        let answer = if response.answer.trim().is_empty() {
            "(not answered)"
        } else {
            response.answer.trim()
        };
        for paragraph in answer.lines() {
            lines.extend(wrap_line(&format!("Answer: {}", paragraph.trim()), 88));
        }
        lines.push(String::new());
    }

    write_text_pdf(&lines)
}

fn write_text_pdf(lines: &[String]) -> Result<Vec<u8>> {
    const LINES_PER_PAGE: usize = 54;
    let mut pages = Vec::new();
    let mut current = Vec::new();
    for line in lines {
        if current.len() >= LINES_PER_PAGE {
            pages.push(current);
            current = Vec::new();
        }
        current.push(line.clone());
    }
    if !current.is_empty() {
        pages.push(current);
    }
    if pages.is_empty() {
        pages.push(Vec::new());
    }

    let page_count = pages.len();
    let font_object_id = 3 + page_count * 2;
    let mut objects = Vec::new();
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_string());

    let kids = (0..page_count)
        .map(|index| format!("{} 0 R", 3 + index * 2))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(format!(
        "<< /Type /Pages /Kids [{kids}] /Count {page_count} >>"
    ));

    for (index, page_lines) in pages.iter().enumerate() {
        let page_object_id = 3 + index * 2;
        let content_object_id = page_object_id + 1;
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 {font_object_id} 0 R >> >> /Contents {content_object_id} 0 R >>"
        ));
        let stream = pdf_stream(page_lines);
        objects.push(format!(
            "<< /Length {} >>\nstream\n{}endstream",
            stream.len(),
            stream
        ));
    }

    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string());

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(pdf, "{} 0 obj", index + 1)?;
        pdf.extend_from_slice(object.as_bytes());
        pdf.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = pdf.len();
    writeln!(pdf, "xref")?;
    writeln!(pdf, "0 {}", objects.len() + 1)?;
    writeln!(pdf, "0000000000 65535 f ")?;
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ")?;
    }
    writeln!(pdf, "trailer")?;
    writeln!(pdf, "<< /Size {} /Root 1 0 R >>", objects.len() + 1)?;
    writeln!(pdf, "startxref")?;
    writeln!(pdf, "{xref_offset}")?;
    writeln!(pdf, "%%EOF")?;
    Ok(pdf)
}

fn pdf_stream(lines: &[String]) -> String {
    let mut stream = String::from("BT\n/F1 10 Tf\n14 TL\n50 800 Td\n");
    for line in lines {
        stream.push('(');
        stream.push_str(&escape_pdf_text(&printable_text(line)));
        stream.push_str(") Tj\nT*\n");
    }
    stream.push_str("ET\n");
    stream
}

fn wrap_line(text: &str, max_chars: usize) -> Vec<String> {
    let printable = printable_text(text);
    if printable.trim().is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in printable.split_whitespace() {
        let additional = if current.is_empty() { 0 } else { 1 };
        if !current.is_empty() && current.len() + additional + word.len() > max_chars {
            lines.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn printable_text(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '\r' | '\n' | '\t' => out.push(' '),
            '\u{00a3}' => out.push_str("GBP"),
            '\u{2018}' | '\u{2019}' => out.push('\''),
            '\u{201c}' | '\u{201d}' => out.push('"'),
            '\u{2013}' | '\u{2014}' => out.push('-'),
            c if c.is_ascii() && !c.is_control() => out.push(c),
            _ => out.push('?'),
        }
    }
    out
}

fn escape_pdf_text(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}

fn push_key_value(text: &mut String, key: &str, value: &str) {
    text.push_str(key);
    text.push_str(" = ");
    text.push_str(&toml_string(value));
    text.push('\n');
}

fn push_optional_bool(text: &mut String, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        text.push_str(key);
        text.push_str(" = ");
        text.push_str(if value { "true" } else { "false" });
        text.push('\n');
    }
}

fn toml_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn choice_to_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
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

fn push_folder(
    artifacts: &mut Vec<DataArtifactDto>,
    stage: &str,
    label: &str,
    path: Utf8PathBuf,
) -> Result<()> {
    let (item_count, bytes) = if path.exists() {
        dir_stats(&path)?
    } else {
        (0, 0)
    };
    artifacts.push(DataArtifactDto {
        stage: stage.to_string(),
        label: label.to_string(),
        kind: "folder".to_string(),
        path: path.to_string(),
        exists: path.exists(),
        bytes,
        item_count,
        item_label: "files".to_string(),
    });
    Ok(())
}

fn push_file(
    artifacts: &mut Vec<DataArtifactDto>,
    stage: &str,
    label: &str,
    path: Utf8PathBuf,
) -> Result<()> {
    let exists = path.exists();
    let bytes = if exists {
        fs::metadata(&path)
            .with_context(|| format!("reading metadata for {path}"))?
            .len()
    } else {
        0
    };
    let item_count = if exists && is_line_countable(&path) {
        count_non_empty_lines(&path)?
    } else {
        0
    };
    artifacts.push(DataArtifactDto {
        stage: stage.to_string(),
        label: label.to_string(),
        kind: "file".to_string(),
        path: path.to_string(),
        exists,
        bytes,
        item_count,
        item_label: if is_line_countable(&path) {
            "lines".to_string()
        } else {
            String::new()
        },
    });
    Ok(())
}

fn is_line_countable(path: &Utf8Path) -> bool {
    matches!(
        path.extension().unwrap_or_default(),
        "csv" | "json" | "jsonl" | "md" | "toml" | "txt"
    )
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

fn provider_kind_label(kind: tinotax_config::ProviderKind) -> &'static str {
    match kind {
        tinotax_config::ProviderKind::Blockscout => "blockscout",
        tinotax_config::ProviderKind::Nearblocks => "nearblocks",
    }
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
    fn wallet_gate_is_driven_by_provider_and_key() {
        use tinotax_config::ProviderKind;

        // Blockscout (Lisk, IOTA EVM) is keyless: always enabled.
        assert_eq!(wallet_gate(ProviderKind::Blockscout, false), (true, String::new()));
        assert_eq!(wallet_gate(ProviderKind::Blockscout, true), (true, String::new()));

        // NearBlocks is gated on its key.
        let (enabled, reason) = wallet_gate(ProviderKind::Nearblocks, false);
        assert!(!enabled);
        assert!(reason.contains("NEARBLOCKS_API_KEY"));
        assert_eq!(wallet_gate(ProviderKind::Nearblocks, true), (true, String::new()));
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

    #[test]
    fn hmrc_questionnaire_formats_toml_and_pdf() -> Result<(), Box<dyn Error>> {
        let config = tinotax_config::ProjectConfig {
            project: tinotax_config::ProjectSection {
                name: "demo".into(),
                base_currency: "GBP".into(),
                period_start: "2017-01-01T00:00:00Z".into(),
                period_end: "2025-04-05T23:59:59Z".into(),
            },
            wallets: Vec::new(),
            providers: Default::default(),
            cex_csvs: Vec::new(),
        };
        let responses = vec![
            HmrcQuestionnaireResponseDraft {
                id: "q1".into(),
                question: "When did you begin?".into(),
                answer: "2017".into(),
                choice: None,
            },
            HmrcQuestionnaireResponseDraft {
                id: "q7".into(),
                question: "Forks?".into(),
                answer: "No forks.".into(),
                choice: Some("no".into()),
            },
        ];

        let toml = build_questionnaire_toml(&config, &responses);
        assert!(toml.contains("began_on = \"2017\""));
        assert!(toml.contains("received_forks = false"));
        assert!(toml.contains("[[hmrc_questionnaire.responses]]"));

        let pdf = build_questionnaire_pdf("demo", &responses)?;
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(String::from_utf8_lossy(&pdf).contains("xref"));
        Ok(())
    }
}
