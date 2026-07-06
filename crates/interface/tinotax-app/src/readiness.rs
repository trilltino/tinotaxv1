//! Project production-readiness checks.
//!
//! These checks are deliberately local and evidence-focused. TinoTax is a CLI,
//! so production readiness mostly means: can the project be rebuilt, audited,
//! and trusted without silent data loss or mutated source evidence?

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use tinotax_store::manifest::{collect_raw_manifests, hash_file};
use tinotax_store::{AuditManifest, Cursor};

use crate::open_project;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone)]
struct Check {
    severity: Severity,
    message: String,
}

impl Check {
    fn pass(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Pass,
            message: message.into(),
        }
    }

    fn warn(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warn,
            message: message.into(),
        }
    }

    fn fail(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Fail,
            message: message.into(),
        }
    }
}

/// Run production-readiness checks for one initialized project.
pub fn readiness(project: &str) -> Result<()> {
    let (paths, config) = open_project(project)?;
    let mut checks = Vec::new();

    checks.push(Check::pass(format!(
        "project config valid: {:?} ({} wallet(s), {} provider(s))",
        config.project.name,
        config.wallets.len(),
        config.providers.len()
    )));

    for dir in [paths.raw(), paths.staging(), paths.out(), paths.logs()] {
        if dir.exists() {
            checks.push(Check::pass(format!("required directory exists: {dir}")));
        } else {
            checks.push(Check::fail(format!("required directory missing: {dir}")));
        }
    }

    check_raw_manifests(&paths.root, &paths.raw(), &mut checks)?;
    check_cursors(&paths.raw(), &mut checks)?;
    check_audit_manifest(
        &paths.root,
        &paths.out().join("audit_manifest.json"),
        &mut checks,
    )?;
    check_review_surface(&paths.staging(), &paths.out(), &mut checks)?;

    println!("production readiness: {}", paths.root);
    for check in &checks {
        let label = match check.severity {
            Severity::Pass => "PASS",
            Severity::Warn => "WARN",
            Severity::Fail => "FAIL",
        };
        println!("[{label}] {}", check.message);
    }

    let failures = checks
        .iter()
        .filter(|c| c.severity == Severity::Fail)
        .count();
    let warnings = checks
        .iter()
        .filter(|c| c.severity == Severity::Warn)
        .count();

    if failures > 0 {
        anyhow::bail!("readiness gate failed: {failures} failure(s), {warnings} warning(s)");
    }
    println!("readiness gate passed: {warnings} warning(s)");
    Ok(())
}

fn check_raw_manifests(
    project_root: &Utf8Path,
    raw_dir: &Utf8Path,
    checks: &mut Vec<Check>,
) -> Result<()> {
    let manifests = collect_raw_manifests(raw_dir)?;
    if manifests.is_empty() {
        checks.push(Check::warn(
            "no raw manifests found yet; fetch at least one source before client delivery",
        ));
        return Ok(());
    }

    let mut entries = 0usize;
    let mut items = 0u64;
    for manifest in manifests {
        for entry in manifest.entries {
            entries += 1;
            items += entry.item_count;
            let path = project_root.join(&entry.path);
            match hash_file(&path) {
                Ok((actual, _)) if actual == entry.blake3 => {}
                Ok((actual, _)) => checks.push(Check::fail(format!(
                    "raw hash mismatch for {}: manifest {}, actual {}",
                    entry.path, entry.blake3, actual
                ))),
                Err(err) => checks.push(Check::fail(format!(
                    "raw manifest entry unreadable: {} ({err})",
                    entry.path
                ))),
            }
        }
    }

    checks.push(Check::pass(format!(
        "raw evidence verified: {entries} file(s), {items} item(s)"
    )));
    Ok(())
}

fn check_cursors(raw_dir: &Utf8Path, checks: &mut Vec<Check>) -> Result<()> {
    if !raw_dir.exists() {
        return Ok(());
    }

    let mut cursors = 0usize;
    let mut unfinished = Vec::new();
    for entry in walkdir::WalkDir::new(raw_dir) {
        let entry = entry.with_context(|| format!("walking {raw_dir}"))?;
        if !entry.file_type().is_file() || entry.file_name() != "cursor.json" {
            continue;
        }
        cursors += 1;
        let path = Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
            .map_err(|p| anyhow::anyhow!("non-UTF8 path {}", p.display()))?;
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
        let cursor: Cursor =
            serde_json::from_str(&text).with_context(|| format!("parsing {path}"))?;
        if !cursor.done {
            unfinished.push(path);
        }
    }

    if cursors == 0 {
        checks.push(Check::warn("no fetch cursors found yet"));
    } else if unfinished.is_empty() {
        checks.push(Check::pass(format!(
            "fetch cursors complete: {cursors} endpoint(s)"
        )));
    } else {
        for path in unfinished {
            checks.push(Check::fail(format!(
                "fetch cursor not complete; re-run fetch --resume: {path}"
            )));
        }
    }
    Ok(())
}

fn check_audit_manifest(
    project_root: &Utf8Path,
    manifest_path: &Utf8Path,
    checks: &mut Vec<Check>,
) -> Result<()> {
    if !manifest_path.exists() {
        checks.push(Check::warn(
            "out/audit_manifest.json missing; run report before client delivery",
        ));
        return Ok(());
    }

    let text = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("reading {manifest_path}"))?;
    let manifest: AuditManifest =
        serde_json::from_str(&text).with_context(|| format!("parsing {manifest_path}"))?;

    let mut verified_outputs = 0usize;
    for output in &manifest.outputs {
        let path = project_root.join(&output.path);
        match hash_file(&path) {
            Ok((actual, bytes)) if actual == output.blake3 && bytes == output.bytes => {
                verified_outputs += 1;
            }
            Ok((actual, bytes)) => checks.push(Check::fail(format!(
                "output hash/size mismatch for {}: manifest {} / {} bytes, actual {} / {} bytes",
                output.path, output.blake3, output.bytes, actual, bytes
            ))),
            Err(err) => checks.push(Check::fail(format!(
                "audit output unreadable: {} ({err})",
                output.path
            ))),
        }
    }

    for raw in &manifest.raw_files {
        let path = project_root.join(&raw.path);
        match hash_file(&path) {
            Ok((actual, _)) if actual == raw.blake3 => {}
            Ok((actual, _)) => checks.push(Check::fail(format!(
                "audit raw hash mismatch for {}: manifest {}, actual {}",
                raw.path, raw.blake3, actual
            ))),
            Err(err) => checks.push(Check::fail(format!(
                "audit raw file unreadable: {} ({err})",
                raw.path
            ))),
        }
    }

    checks.push(Check::pass(format!(
        "audit manifest verified: {} raw file(s), {verified_outputs} output(s)",
        manifest.raw_files.len()
    )));
    Ok(())
}

fn check_review_surface(
    staging_dir: &Utf8Path,
    out_dir: &Utf8Path,
    checks: &mut Vec<Check>,
) -> Result<()> {
    let events = staging_dir.join("normalised_events.jsonl");
    if events.exists() {
        checks.push(Check::pass(format!("normalised events present: {events}")));
    } else {
        checks.push(Check::warn(
            "normalised_events.jsonl missing; run normalise before review/export",
        ));
    }

    let review = out_dir.join("review_all_transactions.csv");
    if review.exists() {
        checks.push(Check::pass(format!("review surface present: {review}")));
    } else {
        checks.push(Check::warn(
            "review_all_transactions.csv missing; run review export-all before accountant review",
        ));
    }

    let rejected = staging_dir.join("rejected_raw_items.jsonl");
    let rejected_count = count_non_empty_lines(&rejected)?;
    if rejected_count > 0 {
        checks.push(Check::warn(format!(
            "{rejected_count} rejected raw item(s) require inspection: {rejected}"
        )));
    } else if rejected.exists() {
        checks.push(Check::pass("no rejected raw items"));
    }

    let warnings = staging_dir.join("warnings.jsonl");
    let warning_count = count_non_empty_lines(&warnings)?;
    if warning_count > 0 {
        checks.push(Check::warn(format!(
            "{warning_count} normalisation warning(s) require inspection: {warnings}"
        )));
    } else if warnings.exists() {
        checks.push(Check::pass("no normalisation warnings"));
    }

    Ok(())
}

fn count_non_empty_lines(path: &Utf8Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    Ok(text.lines().filter(|line| !line.trim().is_empty()).count() as u64)
}
