//! Assemble `evidence_pack/<year>/` from the tax outputs, provenance files
//! and questionnaire answers.

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use rust_decimal::Decimal;
use tinotax_config::ProjectConfig;
use tinotax_store::ProjectPaths;

use crate::assumptions::{ensure_questionnaire, load_questionnaire, source_of_funds_notes};
use crate::counterparties::write_counterparties;
use crate::hmrc_questions::{hmrc_questions_draft, load_summary_csv};
use crate::markdown::{calculator_statement, pack_readme, write_md};
use crate::platforms::{write_platforms, write_wallet_addresses};
use crate::raw_index::write_raw_index;

fn copy_into(from: &Utf8Path, dir: &Utf8Path, name: &str) -> Result<()> {
    std::fs::copy(from.as_std_path(), dir.join(name).as_std_path())
        .with_context(|| format!("copying {from}"))?;
    Ok(())
}

/// Build the pack. Returns its folder.
pub fn build_pack(
    paths: &ProjectPaths,
    config: &ProjectConfig,
    tax_year: &str,
) -> Result<Utf8PathBuf> {
    let tax_dir = paths.tax_dir(tax_year);
    if !tax_dir.exists() {
        bail!("no tax outputs at {tax_dir} — run `calculate uk --tax-year {tax_year}` first");
    }
    let dir = paths.evidence_dir(tax_year);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {dir}"))?;

    // 1. Copy the calculation outputs.
    for name in [
        "self_assessment_crypto_summary.csv",
        "disposals_calculation.csv",
        "s104_pool_movements.csv",
        "s104_pool_opening_closing.csv",
        "income_summary.csv",
        "assumptions_and_limitations.md",
    ] {
        copy_into(&tax_dir.join(name), &dir, name)?;
    }
    copy_into(
        &tax_dir.join("unresolved_tax_items.csv"),
        &dir,
        "unresolved_review_items.csv",
    )?;

    // 2. Provenance from out/ (present once the relevant stage has run).
    for (source, name) in [
        (paths.out().join("pricing_audit.csv"), "pricing_audit.csv"),
        (paths.out().join("change_log.csv"), "change_log.csv"),
    ] {
        if source.exists() {
            copy_into(&source, &dir, name)?;
        } else {
            // An empty placeholder beats a missing file in a client pack.
            std::fs::write(
                dir.join(name),
                "no_data\nstage has not produced this file yet\n",
            )?;
        }
    }

    // 3. Latest review decisions (one row per event).
    write_manual_review_decisions(paths, &dir)?;

    // 4. Derived listings.
    let ledger = tinotax_ledger::load_reviewed_ledger(paths)
        .context("the evidence pack needs the reviewed ledger — run `ledger build` first")?;
    write_platforms(&ledger, &dir)?;
    write_wallet_addresses(config, &dir)?;
    write_raw_index(paths, &dir)?;
    // Q5 needs named protocols/DEXs, not just chains — list every contract the
    // wallets touched, from the normalised events.
    let events = tinotax_review::load_all_events(paths)
        .context("the evidence pack needs normalised events — run `normalise` first")?;
    write_counterparties(&events, &dir)?;

    // 5. Markdown: questionnaire-driven and generated statements.
    let pending = ensure_questionnaire(paths)?;
    let questionnaire = load_questionnaire(paths)?;
    let summary = load_summary_csv(&tax_dir.join("self_assessment_crypto_summary.csv"))?;
    write_md(
        &dir,
        "hmrc_questions_draft.md",
        &hmrc_questions_draft(tax_year, &summary, &questionnaire),
    )?;
    source_of_funds_notes(&questionnaire, &dir)?;
    write_md(
        &dir,
        "calculator_statement.md",
        &calculator_statement(tax_year),
    )?;
    write_md(&dir, "README.md", &pack_readme(tax_year, pending))?;

    Ok(dir)
}

/// `manual_review_decisions.csv`: the latest human decision per event (the
/// full history is `change_log.csv`).
fn write_manual_review_decisions(paths: &ProjectPaths, dir: &Utf8Path) -> Result<()> {
    let latest = tinotax_review::load_latest_overrides(paths)?;
    let path = dir.join("manual_review_decisions.csv");
    let mut writer = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
    writer.write_record([
        "event_id",
        "user_action",
        "user_tax_type",
        "user_asset_symbol",
        "user_quantity",
        "user_proceeds_gbp",
        "user_cost_gbp",
        "user_income_gbp",
        "user_fee_gbp",
        "user_note",
        "applied_at",
    ])?;
    let opt_dec = |d: Option<Decimal>| d.map(|v| v.to_string()).unwrap_or_default();
    for o in latest.values() {
        writer.write_record([
            o.event_id.as_str(),
            o.user_action.map(|a| a.as_str()).unwrap_or(""),
            o.user_tax_type.map(|t| t.as_str()).unwrap_or(""),
            o.user_asset_symbol.as_deref().unwrap_or(""),
            &opt_dec(o.user_quantity),
            &opt_dec(o.user_proceeds_gbp),
            &opt_dec(o.user_cost_gbp),
            &opt_dec(o.user_income_gbp),
            &opt_dec(o.user_fee_gbp),
            o.user_note.as_deref().unwrap_or(""),
            o.applied_at.as_str(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}
