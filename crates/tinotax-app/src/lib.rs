//! Orchestration layer: the CLI calls these; they wire the pipeline crates
//! together. No business logic lives here or in the CLI.

pub mod diagnose_project;
pub mod doctor;
pub mod export_review;
pub mod fetch_project;
pub mod normalise_project;
pub mod run_demo;

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use tinotax_config::ProjectConfig;
use tinotax_store::ProjectPaths;

pub use diagnose_project::diagnose_project;
pub use doctor::doctor;
pub use export_review::{apply_review, export_review};
pub use fetch_project::fetch_project;
pub use normalise_project::normalise_project;
pub use run_demo::run_demo;

/// Create a project folder from a config file (`project init`).
pub fn project_init(config: &str, out: &str) -> Result<ProjectPaths> {
    let config_path = Utf8PathBuf::from(config);
    // Validate before creating anything.
    ProjectConfig::load(&config_path)?;
    let paths = ProjectPaths::init_from_config(Utf8PathBuf::from(out), &config_path)?;
    println!("initialised project at {}", paths.root);
    Ok(paths)
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
    tinotax_report::write_audit_manifest(&paths, &config.project.name)?;
    println!("wrote normalised_transactions.csv ({rows} rows) and audit_manifest.json");
    Ok(())
}
