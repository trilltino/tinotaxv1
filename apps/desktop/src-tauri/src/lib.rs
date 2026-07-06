use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowLog {
    message: String,
    level: &'static str,
}

#[tauri::command]
async fn select_config_file(app: AppHandle) -> Option<String> {
    let (tx, rx) = oneshot::channel();
    app.dialog()
        .file()
        .add_filter("TOML", &["toml"])
        .pick_file(move |path| {
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
    app.dialog().file().pick_folder(move |path| {
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
fn load_review_rows(project: String) -> Result<tinotax_app::ReviewRowsResult, String> {
    tinotax_app::load_review_rows(&project).map_err(error_text)
}

#[tauri::command]
fn save_review_overrides(
    project: String,
    drafts: Vec<tinotax_app::ReviewOverrideDraft>,
) -> Result<tinotax_app::SaveReviewResult, String> {
    tinotax_app::save_review_overrides(&project, drafts).map_err(error_text)
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    open::that_detached(path).map_err(|err| err.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_wdio_webdriver::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            select_config_file,
            select_project_dir,
            get_project_status,
            get_project_paths,
            plan_project_clean,
            confirm_project_clean,
            run_startup_workflow,
            run_refresh_review,
            run_finalize_year,
            load_review_rows,
            save_review_overrides,
            open_path,
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
