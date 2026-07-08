//! Orchestration layer: the CLI calls these; they wire the pipeline crates
//! together. No business logic lives here or in the CLI.

pub mod analysis_export;
pub mod cex_import;
pub mod create_project;
pub mod desktop_api;
pub mod diagnose_project;
pub mod event_cache;
pub mod doctor;
pub mod export_review;
pub mod fetch_project;
pub mod normalise_project;
pub mod pipeline;
pub mod preflight;
pub mod project_ops;
pub mod readiness;
pub mod run_demo;
pub mod wallet_insights;

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use tinotax_config::ProjectConfig;
use tinotax_store::ProjectPaths;

pub use cex_import::{desktop_import_cex, CexImportResultDto};
pub use create_project::{
    desktop_create_project_from_address, CreateProjectResult, DetectedChainDto,
};
pub use desktop_api::{
    auto_classify_contract_calls, bulk_set_review, desktop_config_wallets, desktop_default_project,
    desktop_project_data_view, desktop_project_paths, desktop_project_status,
    export_hmrc_questionnaire, load_review_page, load_review_rows, save_review_overrides,
    DataArtifactDto, HmrcQuestionnaireExportResult, HmrcQuestionnaireResponseDraft, ProjectDataViewDto,
    ProjectPathsDto, ProjectStatusDto, ReviewOverrideDraft, ReviewPage, ReviewQuery, ReviewRowsResult,
    SaveReviewResult, WalletConfigResult, WalletSourceDto,
};
pub use diagnose_project::diagnose_project;
pub use doctor::doctor;
pub use export_review::{apply_review, export_review};
pub use fetch_project::{fetch_project, fetch_project_wallets, FetchHooks};
pub use normalise_project::normalise_project;
pub use pipeline::{
    calculate_uk, export_review_all, import_cex, import_cex_if_declared, ledger_build, ledger_price,
    pack_hmrc, prices_fetch, prices_import, prices_missing,
};
pub use preflight::preflight;
pub use project_ops::{
    project_clean, project_clean_confirm, project_clean_plan, project_paths, project_status,
    workflow_finalize_year, workflow_prepare, workflow_rebuild_ledger, workflow_refresh_review,
    workflow_startup, workflow_sync_wallets, CleanPlanEntry, CleanTarget,
};
pub use readiness::readiness;
pub use run_demo::run_demo;
pub use wallet_insights::{desktop_wallet_insights, WalletInsightsResult};

/// Create a project folder from a config file (`project init`).
pub fn project_init(config: &str, out: &str) -> Result<ProjectPaths> {
    let config_path = resolve_config_path(config)?;
    // Validate before creating anything.
    ProjectConfig::load(&config_path)?;
    let paths = ProjectPaths::init_from_config(Utf8PathBuf::from(out), &config_path)?;
    println!("initialised project at {}", paths.root);
    Ok(paths)
}

pub(crate) fn resolve_config_path(config: &str) -> Result<Utf8PathBuf> {
    let requested = Utf8PathBuf::from(config);
    if requested.is_absolute() || requested.exists() {
        return Ok(requested);
    }

    let cwd = std::env::current_dir().context("resolving current directory")?;
    if let Ok(cwd) = Utf8PathBuf::from_path_buf(cwd) {
        if let Some(found) = find_relative_config(&cwd, &requested) {
            return Ok(found);
        }
    }

    let manifest_dir = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(found) = find_relative_config(&manifest_dir, &requested) {
        return Ok(found);
    }

    Ok(requested)
}

fn find_relative_config(start: &Utf8PathBuf, requested: &Utf8PathBuf) -> Option<Utf8PathBuf> {
    start.ancestors().find_map(|dir| {
        let candidate = dir.join(requested);
        candidate.exists().then_some(candidate)
    })
}

/// Open an existing project folder and its installed `project.toml`.
pub(crate) fn open_project(project: &str) -> Result<(ProjectPaths, ProjectConfig)> {
    let paths = ProjectPaths::new(Utf8PathBuf::from(project));
    let config = ProjectConfig::load(&paths.config_file()).with_context(|| {
        format!(
            "no usable project at {} — run `tinotax project init` (or `demo`) first",
            paths.root
        )
    })?;
    Ok((paths, config))
}

/// Generate `out/` reports: transactions CSV, events JSON, and the audit
/// manifest (always last, so it hashes the other outputs).
pub fn export_reports(project: &str) -> Result<()> {
    let (paths, config) = open_project(project)?;
    let rows = tinotax_report::export_transactions_csv(&paths)?;
    tinotax_report::export_events_json(&paths)?;
    let analysis_rows = analysis_export::export_analysis_csv(project)?;
    tinotax_report::write_audit_manifest(&paths, &config.project.name)?;
    println!(
        "wrote normalised_transactions.csv ({rows} rows), analysis_export.csv ({analysis_rows} rows), and audit_manifest.json"
    );
    Ok(())
}
