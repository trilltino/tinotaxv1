//! Project-level inspection, cleanup, and workflow helpers.
//!
//! These commands are operational wrappers around the existing pipeline. They
//! do not own business logic; they make project state easier to inspect and
//! repeat safely from the CLI.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use tinotax_config::{ProjectConfig, ProviderKind};
use tinotax_store::ProjectPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CleanTarget {
    Logs,
    Staging,
    Out,
    Tax,
    Evidence,
    AllDerived,
}

impl CleanTarget {
    fn as_str(self) -> &'static str {
        match self {
            Self::Logs => "logs",
            Self::Staging => "staging",
            Self::Out => "out",
            Self::Tax => "tax",
            Self::Evidence => "evidence",
            Self::AllDerived => "all-derived",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CleanupAction {
    ClearDirContents,
    RemoveDir,
    RemoveFile,
}

impl CleanupAction {
    fn label(self) -> &'static str {
        match self {
            Self::ClearDirContents => "clear directory contents",
            Self::RemoveDir => "remove directory",
            Self::RemoveFile => "remove file",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CleanupItem {
    target: CleanTarget,
    action: CleanupAction,
    path: Utf8PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CleanupPlan {
    items: Vec<CleanupItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanPlanEntry {
    pub target: String,
    pub action: String,
    pub path: String,
    pub exists: bool,
}

pub fn project_status(project: &str) -> Result<()> {
    let (paths, config) = crate::open_project(project)?;

    println!("project: {}", paths.root);
    println!("name: {}", config.project.name);
    println!(
        "period: {} to {} ({})",
        config.project.period_start, config.project.period_end, config.project.base_currency
    );
    println!(
        "sources: {} wallet(s), {} CEX import(s), {} provider(s)",
        config.wallets.len(),
        config.cex_csvs.len(),
        config.providers.len()
    );

    println!("\nfolders:");
    for (label, path) in [
        ("raw", paths.raw()),
        ("staging", paths.staging()),
        ("out", paths.out()),
        ("logs", paths.logs()),
        ("tax", paths.root.join("tax")),
        ("evidence_pack", paths.root.join("evidence_pack")),
    ] {
        match dir_stats(&path)? {
            Some((files, bytes)) => println!("  {label}: {path} ({files} file(s), {bytes} bytes)"),
            None => println!("  {label}: {path} (missing)"),
        }
    }

    println!("\nhuman/audit state:");
    println!(
        "  review overrides: {} line(s) in {}",
        count_non_empty_lines(&paths.overrides_jsonl())?,
        paths.overrides_jsonl()
    );
    println!(
        "  price observations: {} line(s) in {}",
        count_non_empty_lines(&paths.price_observations_jsonl())?,
        paths.price_observations_jsonl()
    );
    print_file_presence("questionnaire", &paths.questionnaire_file());
    print_file_presence("opening pools", &paths.opening_pools_file());
    Ok(())
}

pub fn project_paths(project: &str, tax_year: Option<&str>) -> Result<()> {
    let paths = ProjectPaths::new(Utf8PathBuf::from(project));
    println!("root={}", paths.root);
    println!("config={}", paths.config_file());
    println!("raw={}", paths.raw());
    println!("staging={}", paths.staging());
    println!("out={}", paths.out());
    println!("logs={}", paths.logs());
    println!("questionnaire={}", paths.questionnaire_file());
    println!("opening_pools={}", paths.opening_pools_file());
    match tax_year {
        Some(year) => {
            println!("tax={}", paths.tax_dir(year));
            println!("evidence_pack={}", paths.evidence_dir(year));
        }
        None => {
            println!("tax={}", paths.root.join("tax"));
            println!("evidence_pack={}", paths.root.join("evidence_pack"));
        }
    }
    Ok(())
}

pub fn project_clean(
    project: &str,
    targets: &[CleanTarget],
    tax_year: Option<&str>,
    confirm: bool,
) -> Result<()> {
    let (paths, _) = crate::open_project(project)?;
    let plan = cleanup_plan(&paths, targets, tax_year)?;
    print_cleanup_plan(&plan, confirm);
    if confirm {
        apply_cleanup_plan(&paths, &plan)?;
    } else {
        println!("dry run only; re-run with --confirm to delete these paths");
    }
    Ok(())
}

pub fn project_clean_plan(
    project: &str,
    targets: &[CleanTarget],
    tax_year: Option<&str>,
) -> Result<Vec<CleanPlanEntry>> {
    let (paths, _) = crate::open_project(project)?;
    let plan = cleanup_plan(&paths, targets, tax_year)?;
    Ok(clean_plan_entries(&plan))
}

pub fn project_clean_confirm(
    project: &str,
    targets: &[CleanTarget],
    tax_year: Option<&str>,
) -> Result<Vec<CleanPlanEntry>> {
    let (paths, _) = crate::open_project(project)?;
    let plan = cleanup_plan(&paths, targets, tax_year)?;
    let entries = clean_plan_entries(&plan);
    apply_cleanup_plan(&paths, &plan)?;
    Ok(entries)
}

pub async fn workflow_startup(config: &str, project: &str, resume: bool) -> Result<()> {
    let config_path = crate::resolve_config_path(config)?;

    println!("== 1/9 preflight ==");
    crate::preflight(config_path.as_str(), project)?;

    println!("\n== 2/9 project init ==");
    crate::project_init(config_path.as_str(), project)?;

    println!("\n== 3/9 fetch ==");
    crate::fetch_project(project, resume).await?;

    println!("\n== 4/9 import CEX ==");
    crate::import_cex_if_declared(project)?;

    println!("\n== 5/9 normalise ==");
    crate::normalise_project(project)?;

    println!("\n== 6/9 diagnose ==");
    crate::diagnose_project(project)?;

    println!("\n== 7/9 review exports ==");
    crate::export_review(project)?;
    crate::export_review_all(project)?;

    println!("\n== 8/9 reports ==");
    crate::export_reports(project)?;

    println!("\n== 9/9 readiness ==");
    crate::readiness(project)?;

    println!("\nstartup workflow complete: {project}");
    Ok(())
}

pub async fn workflow_sync_wallets(
    config: &str,
    project: &str,
    wallet_ids: &[String],
    resume: bool,
    hooks: crate::FetchHooks<'_>,
) -> Result<()> {
    println!("== 1/8 load selected wallets ==");
    let config_path = crate::resolve_config_path(config)?;
    let source_config = ProjectConfig::load(&config_path)
        .with_context(|| format!("loading wallet config {config_path}"))?;
    // Validate the selection (this also enforces the Lisk-only sync gate) and
    // capture which wallet ids to fetch. We deliberately do NOT persist this
    // filtered config: trimming project.toml to one wallet would delete the
    // other wallets from the project.
    let selected = selected_lisk_config(source_config.clone(), wallet_ids)?;
    let fetch_ids: Vec<String> = selected.wallets.iter().map(|w| w.id.clone()).collect();

    let paths = ProjectPaths::new(Utf8PathBuf::from(project));
    paths.init()?;
    // Seed a brand-new project with the full multi-wallet config, but never
    // clobber an existing project.toml — syncing one wallet must not drop the
    // rest of the project's wallets.
    if !paths.config_file().exists() {
        let full_text = toml::to_string_pretty(&source_config)
            .context("serialising wallet config")?;
        fs::write(paths.config_file(), full_text)
            .with_context(|| format!("writing {}", paths.config_file()))?;
    }

    println!("\n== 2/8 preflight ==");
    crate::preflight(paths.config_file().as_str(), project)?;

    println!("\n== 3/8 fetch selected wallet API ==");
    crate::fetch_project_wallets(project, resume, Some(&fetch_ids), hooks).await?;

    println!("\n== 4/8 import CEX ==");
    crate::import_cex_if_declared(project)?;

    println!("\n== 5/8 normalise ==");
    crate::normalise_project(project)?;

    println!("\n== 6/8 diagnose ==");
    crate::diagnose_project(project)?;

    println!("\n== 7/8 review exports ==");
    crate::export_review(project)?;
    crate::export_review_all(project)?;

    println!("\n== 8/8 reports ==");
    crate::export_reports(project)?;

    println!("\nwallet sync complete: {project}");
    Ok(())
}

fn selected_lisk_config(mut config: ProjectConfig, wallet_ids: &[String]) -> Result<ProjectConfig> {
    if wallet_ids.is_empty() {
        bail!("select at least one wallet");
    }

    let requested = wallet_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if requested.is_empty() {
        bail!("select at least one wallet");
    }

    let known = config
        .wallets
        .iter()
        .map(|wallet| wallet.id.clone())
        .collect::<BTreeSet<_>>();
    let unknown = requested
        .difference(&known)
        .cloned()
        .collect::<Vec<String>>();
    if !unknown.is_empty() {
        bail!("unknown selected wallet id(s): {}", unknown.join(", "));
    }

    // A wallet is syncable when its data source is reachable: keyless Blockscout
    // explorers (Lisk, IOTA EVM) always are; NearBlocks needs its paid API key
    // present. Compute this up front to avoid borrowing `config` inside retain.
    let nearblocks_ready = std::env::var("NEARBLOCKS_API_KEY")
        .ok()
        .filter(|key| !key.is_empty())
        .is_some();
    let mut syncable = BTreeSet::new();
    let mut rejected = Vec::new();
    for wallet in &config.wallets {
        if !requested.contains(&wallet.id) {
            continue;
        }
        let ok = match config.provider_for(wallet).kind {
            ProviderKind::Blockscout => true,
            ProviderKind::Nearblocks => nearblocks_ready,
        };
        if ok {
            syncable.insert(wallet.id.clone());
        } else {
            rejected.push(format!("{} ({})", wallet.id, wallet.chain));
        }
    }
    config.wallets.retain(|wallet| syncable.contains(&wallet.id));
    if !rejected.is_empty() {
        bail!(
            "these selected wallets need a paid API key before they can sync (set NEARBLOCKS_API_KEY): {}",
            rejected.join(", ")
        );
    }
    if config.wallets.is_empty() {
        bail!("no syncable wallet selected");
    }

    let used_providers = config
        .wallets
        .iter()
        .map(|wallet| wallet.provider.clone())
        .collect::<BTreeSet<_>>();
    config
        .providers
        .retain(|name, _provider| used_providers.contains(name));
    config.validate()?;
    Ok(config)
}

pub fn workflow_refresh_review(project: &str) -> Result<()> {
    println!("== 1/5 normalise ==");
    crate::normalise_project(project)?;

    println!("\n== 2/5 diagnose ==");
    crate::diagnose_project(project)?;

    println!("\n== 3/5 review exports ==");
    crate::export_review(project)?;
    crate::export_review_all(project)?;

    println!("\n== 4/5 reports ==");
    crate::export_reports(project)?;

    println!("\n== 5/5 readiness ==");
    crate::readiness(project)?;

    println!("\nreview refresh complete: {project}");
    Ok(())
}

/// One-click "add a wallet and prepare it for tax": fetch → normalise → review
/// exports → auto-ignore zero-value contract calls → build → (optional price
/// fetch) → price → calculate. Each step reports through `progress` so the UI
/// can show where it is. This is the self-service path — every step is an
/// existing pipeline function; this only sequences them.
#[allow(clippy::too_many_arguments)]
pub async fn workflow_prepare(
    config: &str,
    project: &str,
    wallet_ids: &[String],
    tax_year: &str,
    resume: bool,
    fetch_prices: bool,
    allow_unpriced: bool,
    progress: &(dyn Fn(&str) + Sync),
    hooks: crate::FetchHooks<'_>,
) -> Result<()> {
    progress("fetching + normalising wallet data");
    workflow_sync_wallets(config, project, wallet_ids, resume, hooks).await?;

    progress("classifying zero-value contract calls");
    crate::auto_classify_contract_calls(project)?;

    progress("building reviewed ledger");
    crate::ledger_build(project)?;

    if fetch_prices {
        progress("fetching GBP prices");
        crate::prices_fetch(project, "coingecko").await?;
    }

    progress("pricing ledger");
    crate::ledger_price(project)?;

    progress(&format!("calculating UK tax for {tax_year}"));
    crate::calculate_uk(project, tax_year, allow_unpriced)?;

    progress("prepare complete");
    Ok(())
}

/// Rebuild the reviewed + priced ledger from the current normalised events and
/// review overrides, without running the UK calculation or evidence pack. This
/// is the light "apply my review decisions" step: after bulk-classifying rows,
/// it refreshes the ledger the Wallet Data insights read from, so counts like
/// "outstanding" and pricing coverage update.
pub fn workflow_rebuild_ledger(project: &str) -> Result<()> {
    println!("== 1/2 ledger build ==");
    crate::ledger_build(project)?;

    println!("\n== 2/2 ledger price ==");
    crate::ledger_price(project)?;

    println!("\nledger rebuild complete: {project}");
    Ok(())
}

pub fn workflow_finalize_year(project: &str, tax_year: &str, allow_unpriced: bool) -> Result<()> {
    println!("== 1/6 ledger build ==");
    crate::ledger_build(project)?;

    println!("\n== 2/6 missing prices ==");
    crate::prices_missing(project)?;

    println!("\n== 3/6 ledger price ==");
    crate::ledger_price(project)?;

    println!("\n== 4/6 calculate UK ==");
    crate::calculate_uk(project, tax_year, allow_unpriced)?;

    println!("\n== 5/6 HMRC pack ==");
    crate::pack_hmrc(project, tax_year)?;

    println!("\n== 6/6 readiness ==");
    crate::readiness(project)?;

    println!("\nfinalize-year workflow complete: {project} {tax_year}");
    Ok(())
}

fn cleanup_plan(
    paths: &ProjectPaths,
    targets: &[CleanTarget],
    tax_year: Option<&str>,
) -> Result<CleanupPlan> {
    if targets.is_empty() {
        bail!("at least one --target is required");
    }

    let mut items = BTreeMap::<(Utf8PathBuf, CleanupAction), CleanupItem>::new();
    for target in targets {
        if *target == CleanTarget::AllDerived {
            for expanded in [
                CleanTarget::Logs,
                CleanTarget::Staging,
                CleanTarget::Out,
                CleanTarget::Tax,
                CleanTarget::Evidence,
            ] {
                add_cleanup_target(paths, expanded, tax_year, &mut items);
            }
        } else {
            add_cleanup_target(paths, *target, tax_year, &mut items);
        }
    }

    Ok(CleanupPlan {
        items: items.into_values().collect(),
    })
}

fn add_cleanup_target(
    paths: &ProjectPaths,
    target: CleanTarget,
    tax_year: Option<&str>,
    items: &mut BTreeMap<(Utf8PathBuf, CleanupAction), CleanupItem>,
) {
    match target {
        CleanTarget::Logs => {
            add_item(items, target, CleanupAction::ClearDirContents, paths.logs());
        }
        CleanTarget::Staging => {
            for path in [
                paths.events_jsonl(),
                paths.cex_events_jsonl(),
                paths.rejected_jsonl(),
                paths.warnings_jsonl(),
                paths.reviewed_ledger_jsonl(),
                paths.priced_ledger_jsonl(),
                paths.price_hints_jsonl(),
            ] {
                add_item(items, target, CleanupAction::RemoveFile, path);
            }
        }
        CleanTarget::Out => {
            add_item(items, target, CleanupAction::ClearDirContents, paths.out());
        }
        CleanTarget::Tax => match tax_year {
            Some(year) => add_item(items, target, CleanupAction::RemoveDir, paths.tax_dir(year)),
            None => add_item(
                items,
                target,
                CleanupAction::ClearDirContents,
                paths.root.join("tax"),
            ),
        },
        CleanTarget::Evidence => match tax_year {
            Some(year) => add_item(
                items,
                target,
                CleanupAction::RemoveDir,
                paths.evidence_dir(year),
            ),
            None => add_item(
                items,
                target,
                CleanupAction::ClearDirContents,
                paths.root.join("evidence_pack"),
            ),
        },
        CleanTarget::AllDerived => {}
    }
}

fn add_item(
    items: &mut BTreeMap<(Utf8PathBuf, CleanupAction), CleanupItem>,
    target: CleanTarget,
    action: CleanupAction,
    path: Utf8PathBuf,
) {
    items.insert(
        (path.clone(), action),
        CleanupItem {
            target,
            action,
            path,
        },
    );
}

fn print_cleanup_plan(plan: &CleanupPlan, confirm: bool) {
    let verb = if confirm {
        "will delete"
    } else {
        "would delete"
    };
    println!("cleanup plan: {} item(s)", plan.items.len());
    for item in &plan.items {
        println!(
            "  {verb}: [{}] {} -> {}",
            item.target.as_str(),
            item.action.label(),
            item.path
        );
    }
}

fn clean_plan_entries(plan: &CleanupPlan) -> Vec<CleanPlanEntry> {
    plan.items
        .iter()
        .map(|item| CleanPlanEntry {
            target: item.target.as_str().to_string(),
            action: item.action.label().to_string(),
            exists: item.path.exists(),
            path: item.path.to_string(),
        })
        .collect()
}

fn apply_cleanup_plan(paths: &ProjectPaths, plan: &CleanupPlan) -> Result<()> {
    for item in &plan.items {
        match item.action {
            CleanupAction::ClearDirContents => clear_dir_contents(paths, &item.path)?,
            CleanupAction::RemoveDir => remove_dir(paths, &item.path)?,
            CleanupAction::RemoveFile => remove_file(paths, &item.path)?,
        }
    }
    Ok(())
}

fn clear_dir_contents(paths: &ProjectPaths, dir: &Utf8Path) -> Result<()> {
    if !dir.exists() {
        println!("skipped missing directory: {dir}");
        return Ok(());
    }
    ensure_project_child(paths, dir)?;
    for entry in fs::read_dir(dir).with_context(|| format!("reading {dir}"))? {
        let entry = entry.with_context(|| format!("reading entry in {dir}"))?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|path| anyhow::anyhow!("non-UTF8 path {}", path.display()))?;
        if entry
            .file_type()
            .with_context(|| format!("reading file type for {path}"))?
            .is_dir()
        {
            remove_dir(paths, &path)?;
        } else {
            remove_file(paths, &path)?;
        }
    }
    println!("cleared directory contents: {dir}");
    Ok(())
}

fn remove_dir(paths: &ProjectPaths, dir: &Utf8Path) -> Result<()> {
    if !dir.exists() {
        println!("skipped missing directory: {dir}");
        return Ok(());
    }
    ensure_project_child(paths, dir)?;
    fs::remove_dir_all(dir).with_context(|| format!("removing directory {dir}"))?;
    println!("removed directory: {dir}");
    Ok(())
}

fn remove_file(paths: &ProjectPaths, file: &Utf8Path) -> Result<()> {
    if !file.exists() {
        println!("skipped missing file: {file}");
        return Ok(());
    }
    ensure_project_child(paths, file)?;
    fs::remove_file(file).with_context(|| format!("removing file {file}"))?;
    println!("removed file: {file}");
    Ok(())
}

fn ensure_project_child(paths: &ProjectPaths, path: &Utf8Path) -> Result<()> {
    let root = fs::canonicalize(&paths.root)
        .with_context(|| format!("canonicalising project root {}", paths.root))?;
    let target = fs::canonicalize(path).with_context(|| format!("canonicalising {path}"))?;
    if target == root || !target.starts_with(&root) {
        bail!("refusing to delete path outside project root: {path}");
    }
    Ok(())
}

fn dir_stats(dir: &Utf8Path) -> Result<Option<(u64, u64)>> {
    if !dir.exists() {
        return Ok(None);
    }
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
    Ok(Some((files, bytes)))
}

fn count_non_empty_lines(path: &Utf8Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let text = fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    Ok(text.lines().filter(|line| !line.trim().is_empty()).count() as u64)
}

fn print_file_presence(label: &str, path: &Utf8Path) {
    let state = if path.exists() { "present" } else { "missing" };
    println!("  {label}: {state} ({path})");
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    const PROJECT_TOML: &str = r#"
[project]
name = "ops-test"
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

    const MULTI_WALLET_TOML: &str = r#"
[project]
name = "wallet-select-test"
base_currency = "GBP"
period_start = "2017-01-01T00:00:00Z"
period_end = "2025-04-05T23:59:59Z"

[[wallets]]
id = "lisk_main"
name = "Lisk wallet"
chain = "lisk-evm"
address = "0x1111111111111111111111111111111111111111"
provider = "lisk_blockscout"

[[wallets]]
id = "iota_main"
name = "IOTA wallet"
chain = "iota-evm"
address = "0x1111111111111111111111111111111111111111"
provider = "iota_blockscout"

[[wallets]]
id = "near_main"
name = "NEAR wallet"
chain = "near"
address = "test.near"
provider = "nearblocks"

[providers.lisk_blockscout]
kind = "blockscout"
base_url = "https://blockscout.lisk.com/api/v2"

[providers.iota_blockscout]
kind = "blockscout"
base_url = "https://explorer.evm.iota.org/api/v2"

[providers.nearblocks]
kind = "nearblocks"
base_url = "https://api.nearblocks.io/v1"
"#;

    fn project_paths() -> Result<(tempfile::TempDir, ProjectPaths), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let root = Utf8PathBuf::from_path_buf(tmp.path().join("project"))
            .map_err(|path| std::io::Error::other(format!("non-UTF8 path {}", path.display())))?;
        let paths = ProjectPaths::new(root);
        paths.init()?;
        Ok((tmp, paths))
    }

    fn write(path: &Utf8Path, text: &str) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, text)?;
        Ok(())
    }

    #[test]
    fn dry_run_deletes_nothing() -> Result<(), Box<dyn Error>> {
        let (_tmp, paths) = project_paths()?;
        let log = paths.logs().join("run.log");
        write(&log, "hello")?;
        write(&paths.config_file(), PROJECT_TOML)?;

        project_clean(paths.root.as_str(), &[CleanTarget::Logs], None, false)?;

        assert!(log.exists());
        Ok(())
    }

    #[test]
    fn confirm_removes_selected_target() -> Result<(), Box<dyn Error>> {
        let (_tmp, paths) = project_paths()?;
        let log = paths.logs().join("run.log");
        write(&log, "hello")?;

        let plan = cleanup_plan(&paths, &[CleanTarget::Logs], None)?;
        apply_cleanup_plan(&paths, &plan)?;

        assert!(paths.logs().exists());
        assert!(!log.exists());
        Ok(())
    }

    #[test]
    fn all_derived_preserves_evidence_and_human_state() -> Result<(), Box<dyn Error>> {
        let (_tmp, paths) = project_paths()?;
        let raw = paths.raw().join("source/page.json");
        write(&raw, "{}")?;
        write(&paths.config_file(), "config")?;
        write(&paths.questionnaire_file(), "answers")?;
        write(&paths.opening_pools_file(), "pools")?;
        write(&paths.overrides_jsonl(), "{}\n")?;
        write(&paths.price_observations_jsonl(), "{}\n")?;

        write(&paths.logs().join("run.log"), "log")?;
        write(&paths.events_jsonl(), "{}\n")?;
        write(&paths.cex_events_jsonl(), "{}\n")?;
        write(&paths.rejected_jsonl(), "{}\n")?;
        write(&paths.warnings_jsonl(), "{}\n")?;
        write(&paths.reviewed_ledger_jsonl(), "{}\n")?;
        write(&paths.priced_ledger_jsonl(), "{}\n")?;
        write(&paths.price_hints_jsonl(), "{}\n")?;
        write(&paths.out().join("report.csv"), "x")?;
        write(&paths.tax_dir("2024-2025").join("summary.csv"), "x")?;
        write(&paths.evidence_dir("2024-2025").join("README.md"), "x")?;

        let plan = cleanup_plan(&paths, &[CleanTarget::AllDerived], None)?;
        apply_cleanup_plan(&paths, &plan)?;

        for protected in [
            raw,
            paths.config_file(),
            paths.questionnaire_file(),
            paths.opening_pools_file(),
            paths.overrides_jsonl(),
            paths.price_observations_jsonl(),
        ] {
            assert!(
                protected.exists(),
                "protected path was removed: {protected}"
            );
        }

        assert!(!paths.logs().join("run.log").exists());
        assert!(!paths.events_jsonl().exists());
        assert!(!paths.cex_events_jsonl().exists());
        assert!(!paths.rejected_jsonl().exists());
        assert!(!paths.warnings_jsonl().exists());
        assert!(!paths.reviewed_ledger_jsonl().exists());
        assert!(!paths.priced_ledger_jsonl().exists());
        assert!(!paths.price_hints_jsonl().exists());
        assert!(!paths.out().join("report.csv").exists());
        assert!(!paths.tax_dir("2024-2025").join("summary.csv").exists());
        assert!(!paths.evidence_dir("2024-2025").join("README.md").exists());
        Ok(())
    }

    #[test]
    fn tax_year_limits_tax_and_evidence_cleanup() -> Result<(), Box<dyn Error>> {
        let (_tmp, paths) = project_paths()?;
        let selected_tax = paths.tax_dir("2024-2025").join("summary.csv");
        let other_tax = paths.tax_dir("2023-2024").join("summary.csv");
        let selected_pack = paths.evidence_dir("2024-2025").join("README.md");
        let other_pack = paths.evidence_dir("2023-2024").join("README.md");

        for path in [&selected_tax, &other_tax, &selected_pack, &other_pack] {
            write(path, "x")?;
        }

        let plan = cleanup_plan(&paths, &[CleanTarget::AllDerived], Some("2024-2025"))?;
        apply_cleanup_plan(&paths, &plan)?;

        assert!(!selected_tax.exists());
        assert!(other_tax.exists());
        assert!(!selected_pack.exists());
        assert!(other_pack.exists());
        Ok(())
    }

    #[test]
    fn selected_config_keeps_keyless_blockscout_wallets_and_their_providers() -> Result<(), Box<dyn Error>> {
        // Lisk and IOTA are both keyless Blockscout — both should sync.
        let config: ProjectConfig = toml::from_str(MULTI_WALLET_TOML)?;
        let filtered =
            selected_lisk_config(config, &["lisk_main".to_string(), "iota_main".to_string()])?;

        assert_eq!(filtered.wallets.len(), 2);
        assert!(filtered.wallets.iter().any(|w| w.id == "lisk_main"));
        assert!(filtered.wallets.iter().any(|w| w.id == "iota_main"));
        assert!(filtered.providers.contains_key("lisk_blockscout"));
        assert!(filtered.providers.contains_key("iota_blockscout"));
        assert!(!filtered.providers.contains_key("nearblocks"));
        Ok(())
    }

    #[test]
    fn selected_config_rejects_nearblocks_without_a_key() -> Result<(), Box<dyn Error>> {
        // NearBlocks needs a paid key; without NEARBLOCKS_API_KEY it is rejected.
        if std::env::var("NEARBLOCKS_API_KEY").is_ok_and(|k| !k.is_empty()) {
            return Ok(()); // key present in this environment — gate would allow it
        }
        let config: ProjectConfig = toml::from_str(MULTI_WALLET_TOML)?;
        let err = match selected_lisk_config(config, &["near_main".to_string()]) {
            Ok(_) => return Err(std::io::Error::other("expected gated-wallet error").into()),
            Err(err) => err,
        };

        assert!(err.to_string().contains("NEARBLOCKS_API_KEY"));
        Ok(())
    }
}
