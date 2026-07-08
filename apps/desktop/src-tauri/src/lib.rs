use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowLog {
    message: String,
    level: &'static str,
}

/// Process-global cancel flag for the running `prepare` workflow. Set by
/// `cancel_prepare`, polled by the fetch loop; cleared at the start of each run.
fn prepare_cancel_flag() -> &'static Arc<AtomicBool> {
    static FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    FLAG.get_or_init(|| Arc::new(AtomicBool::new(false)))
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
struct ApiKeys {
    #[serde(default)]
    nearblocks: String,
    #[serde(default)]
    coingecko: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiKeysStatus {
    nearblocks_set: bool,
    coingecko_set: bool,
}

/// `<app_config_dir>/settings.toml`, where the API keys live between launches.
fn settings_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("settings.toml"))
}

fn load_api_keys(app: &AppHandle) -> ApiKeys {
    settings_file(app)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

/// Put the stored keys into the process environment so the existing pipeline
/// (which reads `NEARBLOCKS_API_KEY` / `COINGECKO_API_KEY`) picks them up. Safe
/// on edition 2021; called at startup before any command, and again on save.
fn apply_api_keys_to_env(keys: &ApiKeys) {
    if !keys.nearblocks.is_empty() {
        std::env::set_var("NEARBLOCKS_API_KEY", &keys.nearblocks);
    }
    if !keys.coingecko.is_empty() {
        std::env::set_var("COINGECKO_API_KEY", &keys.coingecko);
    }
}

fn env_key_set(name: &str) -> bool {
    std::env::var(name).map(|v| !v.is_empty()).unwrap_or(false)
}

#[tauri::command]
fn get_api_keys(app: AppHandle) -> ApiKeysStatus {
    let keys = load_api_keys(&app);
    ApiKeysStatus {
        nearblocks_set: !keys.nearblocks.is_empty() || env_key_set("NEARBLOCKS_API_KEY"),
        coingecko_set: !keys.coingecko.is_empty()
            || env_key_set("COINGECKO_API_KEY")
            || env_key_set("COINGECKO_DEMO_API_KEY")
            || env_key_set("COINGECKO_PRO_API_KEY"),
    }
}

#[tauri::command]
fn save_api_keys(app: AppHandle, nearblocks: String, coingecko: String) -> Result<ApiKeysStatus, String> {
    let keys = ApiKeys {
        nearblocks: nearblocks.trim().to_string(),
        coingecko: coingecko.trim().to_string(),
    };
    let path = settings_file(&app).ok_or_else(|| "could not locate the settings folder".to_string())?;
    let text = toml::to_string_pretty(&keys).map_err(|err| err.to_string())?;
    std::fs::write(&path, text).map_err(|err| format!("writing {}: {err}", path.display()))?;
    apply_api_keys_to_env(&keys);
    Ok(get_api_keys(app))
}

/// Where TinoTax keeps projects. Start every picker here so the dialog opens
/// on the user's data (Documents/TinoTax) rather than the app's working
/// directory. Falls back to Documents if the TinoTax folder does not exist yet.
fn projects_home(app: &AppHandle) -> Option<std::path::PathBuf> {
    let documents = app.path().document_dir().ok()?;
    let tinotax = documents.join("TinoTax");
    Some(if tinotax.is_dir() { tinotax } else { documents })
}

#[tauri::command]
async fn select_config_file(app: AppHandle) -> Option<String> {
    let (tx, rx) = oneshot::channel();
    let mut builder = app.dialog().file().add_filter("TOML", &["toml"]);
    if let Some(dir) = projects_home(&app) {
        builder = builder.set_directory(dir);
    }
    builder.pick_file(move |path| {
        if tx.send(path).is_err() {
            tracing::warn!("desktop config picker result receiver was dropped");
        }
    });
    match rx.await {
        Ok(path) => file_path_to_string(path),
        Err(err) => {
            tracing::warn!(%err, "desktop config picker callback was canceled");
            None
        }
    }
}

#[tauri::command]
async fn select_project_dir(app: AppHandle) -> Option<String> {
    let (tx, rx) = oneshot::channel();
    let mut builder = app.dialog().file();
    if let Some(dir) = projects_home(&app) {
        builder = builder.set_directory(dir);
    }
    builder.pick_folder(move |path| {
        if tx.send(path).is_err() {
            tracing::warn!("desktop project picker result receiver was dropped");
        }
    });
    match rx.await {
        Ok(path) => file_path_to_string(path),
        Err(err) => {
            tracing::warn!(%err, "desktop project picker callback was canceled");
            None
        }
    }
}

#[tauri::command]
async fn select_csv_file(app: AppHandle) -> Option<String> {
    let (tx, rx) = oneshot::channel();
    let mut builder = app.dialog().file().add_filter("CSV", &["csv"]);
    if let Some(dir) = projects_home(&app) {
        builder = builder.set_directory(dir);
    }
    builder.pick_file(move |path| {
        if tx.send(path).is_err() {
            tracing::warn!("desktop csv picker result receiver was dropped");
        }
    });
    match rx.await {
        Ok(path) => file_path_to_string(path),
        Err(err) => {
            tracing::warn!(%err, "desktop csv picker callback was canceled");
            None
        }
    }
}

#[tauri::command]
fn get_project_status(project: String) -> Result<tinotax_app::ProjectStatusDto, String> {
    tinotax_app::desktop_project_status(&project).map_err(error_text)
}

#[tauri::command]
fn get_project_paths(
    project: String,
    tax_year: Option<String>,
) -> Result<tinotax_app::ProjectPathsDto, String> {
    Ok(tinotax_app::desktop_project_paths(
        &project,
        tax_year.as_deref(),
    ))
}

#[tauri::command]
fn get_default_project() -> Option<String> {
    tinotax_app::desktop_default_project()
}

#[tauri::command]
fn get_project_data_view(
    project: String,
    tax_year: Option<String>,
) -> Result<tinotax_app::ProjectDataViewDto, String> {
    tinotax_app::desktop_project_data_view(&project, tax_year.as_deref()).map_err(error_text)
}

#[tauri::command]
fn load_config_wallets(config: String) -> Result<tinotax_app::WalletConfigResult, String> {
    tinotax_app::desktop_config_wallets(&config).map_err(error_text)
}

#[tauri::command]
async fn create_project_from_address(
    app: AppHandle,
    address: String,
    name: Option<String>,
) -> Result<tinotax_app::CreateProjectResult, String> {
    // Machine-agnostic default home for projects: Documents/TinoTax.
    let base = app
        .path()
        .document_dir()
        .map_err(|err| format!("could not locate your Documents folder: {err}"))?
        .join("TinoTax");
    let base = base.to_string_lossy().to_string();
    tinotax_app::desktop_create_project_from_address(&base, &address, name.as_deref())
        .await
        .map_err(error_text)
}

#[tauri::command]
fn import_cex_csv(
    project: String,
    source_id: String,
    platform: String,
    file: String,
    mapping: Option<std::collections::BTreeMap<String, String>>,
) -> Result<tinotax_app::CexImportResultDto, String> {
    tinotax_app::desktop_import_cex(&project, &source_id, &platform, &file, mapping)
        .map_err(error_text)
}

#[tauri::command]
fn get_wallet_insights(
    project: String,
    wallet_id: Option<String>,
    tax_year: Option<String>,
) -> Result<tinotax_app::WalletInsightsResult, String> {
    tinotax_app::desktop_wallet_insights(&project, wallet_id.as_deref(), tax_year.as_deref())
        .map_err(error_text)
}

#[tauri::command]
fn plan_project_clean(
    project: String,
    targets: Vec<String>,
    tax_year: Option<String>,
) -> Result<Vec<tinotax_app::CleanPlanEntry>, String> {
    let targets = parse_clean_targets(targets)?;
    tinotax_app::project_clean_plan(&project, &targets, tax_year.as_deref()).map_err(error_text)
}

#[tauri::command]
fn confirm_project_clean(
    project: String,
    targets: Vec<String>,
    tax_year: Option<String>,
) -> Result<Vec<tinotax_app::CleanPlanEntry>, String> {
    let targets = parse_clean_targets(targets)?;
    tinotax_app::project_clean_confirm(&project, &targets, tax_year.as_deref()).map_err(error_text)
}

#[tauri::command]
async fn run_startup_workflow(
    app: AppHandle,
    config: String,
    project: String,
    resume: bool,
) -> Result<(), String> {
    emit_log(&app, "startup workflow started", "info");
    match tinotax_app::workflow_startup(&config, &project, resume).await {
        Ok(()) => {
            emit_log(&app, "startup workflow completed", "info");
            Ok(())
        }
        Err(err) => {
            emit_log(&app, &format!("startup workflow failed: {err:#}"), "error");
            Err(error_text(err))
        }
    }
}

#[tauri::command]
async fn run_wallet_sync(
    app: AppHandle,
    config: String,
    project: String,
    wallet_ids: Vec<String>,
    resume: bool,
) -> Result<(), String> {
    emit_log(&app, "wallet sync started", "info");
    match tinotax_app::workflow_sync_wallets(
        &config,
        &project,
        &wallet_ids,
        resume,
        tinotax_app::FetchHooks::default(),
    )
    .await
    {
        Ok(()) => {
            emit_log(&app, "wallet sync completed", "info");
            Ok(())
        }
        Err(err) => {
            emit_log(&app, &format!("wallet sync failed: {err:#}"), "error");
            Err(error_text(err))
        }
    }
}

#[tauri::command]
async fn run_refresh_review(app: AppHandle, project: String) -> Result<(), String> {
    emit_log(&app, "refresh-review workflow started", "info");
    let project_for_task = project.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        tinotax_app::workflow_refresh_review(&project_for_task)
    })
    .await
    .map_err(|err| err.to_string())?;
    match result {
        Ok(()) => {
            emit_log(&app, "refresh-review workflow completed", "info");
            Ok(())
        }
        Err(err) => {
            emit_log(
                &app,
                &format!("refresh-review workflow failed: {err:#}"),
                "error",
            );
            Err(error_text(err))
        }
    }
}

#[tauri::command]
async fn run_finalize_year(
    app: AppHandle,
    project: String,
    tax_year: String,
    allow_unpriced: bool,
) -> Result<(), String> {
    emit_log(&app, "finalize-year workflow started", "info");
    let result = tauri::async_runtime::spawn_blocking(move || {
        tinotax_app::workflow_finalize_year(&project, &tax_year, allow_unpriced)
    })
    .await
    .map_err(|err| err.to_string())?;
    match result {
        Ok(()) => {
            emit_log(&app, "finalize-year workflow completed", "info");
            Ok(())
        }
        Err(err) => {
            emit_log(
                &app,
                &format!("finalize-year workflow failed: {err:#}"),
                "error",
            );
            Err(error_text(err))
        }
    }
}

#[tauri::command]
async fn run_prepare_wallet(
    app: AppHandle,
    config: String,
    project: String,
    wallet_ids: Vec<String>,
    tax_year: String,
    resume: bool,
    fetch_prices: bool,
) -> Result<(), String> {
    let cancel = Arc::clone(prepare_cancel_flag());
    cancel.store(false, Ordering::SeqCst);

    // Step progress ("fetching…", "building ledger…") as workflow-log events.
    let progress_app = app.clone();
    let progress = move |msg: &str| emit_log(&progress_app, msg, "info");

    // Per-page fetch progress so a long fetch shows live page counts.
    let page_app = app.clone();
    let on_page = move |wallet: &str, endpoint: &str, page: u64, items: u64| {
        emit_log(
            &page_app,
            &format!("{wallet} · {endpoint}: page {page} ({items} items)"),
            "info",
        );
    };
    let cancel_check = Arc::clone(&cancel);
    let cancelled = move || cancel_check.load(Ordering::SeqCst);
    let hooks = tinotax_app::FetchHooks {
        on_page: Some(&on_page),
        cancelled: Some(&cancelled),
    };

    emit_log(&app, "prepare started", "info");
    match tinotax_app::workflow_prepare(
        &config,
        &project,
        &wallet_ids,
        &tax_year,
        resume,
        fetch_prices,
        true,
        &progress,
        hooks,
    )
    .await
    {
        Ok(()) => {
            emit_log(&app, "prepare completed", "info");
            Ok(())
        }
        Err(err) => {
            let detail = format!("{err:#}");
            if detail.contains("cancelled") {
                // User-initiated stop — not a failure. Partial raw data is on
                // disk and resumable on the next run.
                emit_log(&app, "prepare cancelled", "info");
                Ok(())
            } else {
                emit_log(&app, &format!("prepare failed: {detail}"), "error");
                Err(error_text(err))
            }
        }
    }
}

#[tauri::command]
fn cancel_prepare() {
    prepare_cancel_flag().store(true, Ordering::SeqCst);
}

#[tauri::command]
async fn run_rebuild_ledger(app: AppHandle, project: String) -> Result<(), String> {
    emit_log(&app, "ledger rebuild started", "info");
    let result = tauri::async_runtime::spawn_blocking(move || {
        tinotax_app::workflow_rebuild_ledger(&project)
    })
    .await
    .map_err(|err| err.to_string())?;
    match result {
        Ok(()) => {
            emit_log(&app, "ledger rebuild completed", "info");
            Ok(())
        }
        Err(err) => {
            emit_log(&app, &format!("ledger rebuild failed: {err:#}"), "error");
            Err(error_text(err))
        }
    }
}

#[tauri::command]
fn load_review_rows(project: String) -> Result<tinotax_app::ReviewRowsResult, String> {
    tinotax_app::load_review_rows(&project).map_err(error_text)
}

#[tauri::command]
fn load_review_page(
    project: String,
    query: tinotax_app::ReviewQuery,
) -> Result<tinotax_app::ReviewPage, String> {
    tinotax_app::load_review_page(&project, &query).map_err(error_text)
}

#[tauri::command]
fn auto_classify_contract_calls(
    project: String,
) -> Result<tinotax_app::SaveReviewResult, String> {
    tinotax_app::auto_classify_contract_calls(&project).map_err(error_text)
}

#[tauri::command]
fn bulk_set_review(
    project: String,
    query: tinotax_app::ReviewQuery,
    tax_type: String,
) -> Result<tinotax_app::SaveReviewResult, String> {
    tinotax_app::bulk_set_review(&project, &query, &tax_type).map_err(error_text)
}

#[tauri::command]
fn save_review_overrides(
    project: String,
    drafts: Vec<tinotax_app::ReviewOverrideDraft>,
) -> Result<tinotax_app::SaveReviewResult, String> {
    tinotax_app::save_review_overrides(&project, drafts).map_err(error_text)
}

#[tauri::command]
fn export_hmrc_questionnaire(
    project: String,
    responses: Vec<tinotax_app::HmrcQuestionnaireResponseDraft>,
) -> Result<tinotax_app::HmrcQuestionnaireExportResult, String> {
    tinotax_app::export_hmrc_questionnaire(&project, responses).map_err(error_text)
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    open::that_detached(path).map_err(|err| err.to_string())
}

/// Save a copy of an existing project artifact to a location the user picks.
/// Returns the destination path, or `None` if the save dialog was cancelled.
#[tauri::command]
async fn save_file_copy(app: AppHandle, source: String) -> Result<Option<String>, String> {
    let source_path = std::path::PathBuf::from(&source);
    if !source_path.is_file() {
        return Err(format!("not a file: {source}"));
    }
    let suggested = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download")
        .to_string();

    let (tx, rx) = oneshot::channel();
    let mut builder = app.dialog().file().set_file_name(&suggested);
    if let Some(dir) = projects_home(&app) {
        builder = builder.set_directory(dir);
    }
    builder.save_file(move |path| {
        if tx.send(path).is_err() {
            tracing::warn!("desktop save picker result receiver was dropped");
        }
    });

    match rx.await {
        Ok(path) => {
            let Some(dest) = file_path_to_string(path) else {
                return Ok(None); // cancelled
            };
            std::fs::copy(&source_path, &dest)
                .map_err(|err| format!("could not save copy to {dest}: {err}"))?;
            Ok(Some(dest))
        }
        Err(err) => {
            tracing::warn!(%err, "desktop save picker callback was canceled");
            Ok(None)
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_wdio_webdriver::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Load stored API keys into the environment so the pipeline can use
            // them from the first command onward.
            apply_api_keys_to_env(&load_api_keys(&app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            select_config_file,
            select_csv_file,
            select_project_dir,
            get_project_status,
            get_project_paths,
            get_default_project,
            get_project_data_view,
            load_config_wallets,
            create_project_from_address,
            import_cex_csv,
            get_wallet_insights,
            plan_project_clean,
            confirm_project_clean,
            run_startup_workflow,
            run_wallet_sync,
            run_prepare_wallet,
            cancel_prepare,
            run_refresh_review,
            run_finalize_year,
            run_rebuild_ledger,
            load_review_rows,
            load_review_page,
            auto_classify_contract_calls,
            bulk_set_review,
            save_review_overrides,
            export_hmrc_questionnaire,
            open_path,
            save_file_copy,
            get_api_keys,
            save_api_keys,
        ])
        .run(tauri::generate_context!())
}

fn parse_clean_targets(targets: Vec<String>) -> Result<Vec<tinotax_app::CleanTarget>, String> {
    if targets.is_empty() {
        return Err("at least one cleanup target is required".to_string());
    }
    targets
        .into_iter()
        .map(|target| match target.trim().to_ascii_lowercase().as_str() {
            "logs" => Ok(tinotax_app::CleanTarget::Logs),
            "staging" => Ok(tinotax_app::CleanTarget::Staging),
            "out" => Ok(tinotax_app::CleanTarget::Out),
            "tax" => Ok(tinotax_app::CleanTarget::Tax),
            "evidence" => Ok(tinotax_app::CleanTarget::Evidence),
            "all-derived" => Ok(tinotax_app::CleanTarget::AllDerived),
            other => Err(format!("unknown cleanup target {other:?}")),
        })
        .collect()
}

fn emit_log(app: &AppHandle, message: &str, level: &'static str) {
    if let Err(err) = app.emit(
        "workflow-log",
        WorkflowLog {
            message: message.to_string(),
            level,
        },
    ) {
        tracing::warn!(%err, "failed to emit desktop workflow log");
    }
}

fn error_text(err: anyhow::Error) -> String {
    format!("{err:#}")
}

fn file_path_to_string(path: Option<FilePath>) -> Option<String> {
    path.and_then(|path| match path.into_path() {
        Ok(path) => Some(path.to_string_lossy().to_string()),
        Err(err) => {
            tracing::warn!(%err, "desktop picker returned a non-filesystem path");
            None
        }
    })
}
