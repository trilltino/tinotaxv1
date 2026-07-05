//! Orchestration for the post-ingestion pipeline: CEX import, full review,
//! ledger build/price, pricing, UK calculation, and the evidence pack.

use anyhow::Result;

/// `import-cex`: copy + hash the original CSVs, normalise them into events.
pub fn import_cex(project: &str) -> Result<()> {
    let (paths, config) = crate::open_project(project)?;
    let reports = tinotax_cex::import_all(&paths, &config)?;
    for r in &reports {
        println!(
            "{} ({}): {} rows → {} events ({} flagged, {} fiat movements skipped, {} price hints)",
            r.source_id,
            r.platform,
            r.rows_read,
            r.events_emitted,
            r.needs_review,
            r.fiat_movements_skipped,
            r.price_hints
        );
    }
    println!(
        "wrote {} and {}",
        paths.cex_events_jsonl(),
        paths.out().join("cex_import_diagnostics.csv")
    );
    Ok(())
}

/// `review export-all`: every event, reviewable and editable.
pub fn export_review_all(project: &str) -> Result<u64> {
    let (paths, _) = crate::open_project(project)?;
    let rows = tinotax_review::export_review_all(&paths)?;
    println!(
        "exported all {rows} rows to {}",
        paths.out().join("review_all_transactions.csv")
    );
    Ok(rows)
}

/// `ledger build`: normalised events + review overrides → reviewed ledger.
pub fn ledger_build(project: &str) -> Result<()> {
    let (paths, _) = crate::open_project(project)?;
    let summary = tinotax_ledger::build_ledger(&paths)?;
    println!(
        "built reviewed ledger: {} events ({} reviewed, {} still flagged, {} ignored, {} unknown)",
        summary.total, summary.reviewed, summary.needs_review, summary.ignored, summary.unknown
    );
    println!(
        "  {}\n  {}",
        paths.reviewed_ledger_jsonl(),
        paths.out().join("reviewed_ledger.csv")
    );
    if summary.unknown > 0 {
        println!(
            "note: {} events are still `unknown` — classify them via `review export-all` + `review apply` before `calculate uk`",
            summary.unknown
        );
    }
    Ok(())
}

/// `ledger price`: reviewed ledger + price book → priced ledger.
pub fn ledger_price(project: &str) -> Result<()> {
    let (paths, _) = crate::open_project(project)?;
    let summary = tinotax_pricing::price_ledger(&paths)?;
    println!(
        "priced ledger: {} rows — {} valued from the price book, {} already valued by review, {} nothing to price, {} still missing",
        summary.total,
        summary.valued_from_book,
        summary.already_valued,
        summary.nothing_to_price,
        summary.still_missing
    );
    println!(
        "  {}\n  {}\n  {}",
        paths.priced_ledger_jsonl(),
        paths.out().join("priced_ledger.csv"),
        paths.out().join("pricing_audit.csv")
    );
    if summary.still_missing > 0 {
        println!("note: run `prices missing` to see what to import/fetch before `calculate uk`");
    }
    Ok(())
}

/// `prices missing`: what still needs a GBP value.
pub fn prices_missing(project: &str) -> Result<()> {
    let (paths, _) = crate::open_project(project)?;
    let rows = tinotax_pricing::export_missing_prices(&paths)?;
    println!(
        "{rows} (asset, day) pairs still need a GBP price — {}",
        paths.out().join("missing_prices.csv")
    );
    Ok(())
}

/// `prices import`: manual price CSV → price observations.
pub fn prices_import(project: &str, file: &str) -> Result<()> {
    let (paths, _) = crate::open_project(project)?;
    let count = tinotax_pricing::import_manual_prices(&paths, &camino::Utf8PathBuf::from(file))?;
    println!(
        "imported {count} price observations into {}",
        paths.price_observations_jsonl()
    );
    Ok(())
}

/// `prices fetch`: pull missing daily GBP prices from a provider.
pub async fn prices_fetch(project: &str, provider: &str) -> Result<()> {
    let (paths, _) = crate::open_project(project)?;
    let count = tinotax_pricing::fetch_missing_prices(&paths, provider).await?;
    println!(
        "fetched {count} price observations into {}",
        paths.price_observations_jsonl()
    );
    Ok(())
}

/// `calculate uk`: same-day / 30-day / S104 CGT + income for one tax year.
pub fn calculate_uk(project: &str, tax_year: &str, allow_unpriced: bool) -> Result<()> {
    let (paths, _) = crate::open_project(project)?;
    let year = tinotax_tax_uk::TaxYear::parse(tax_year)?;
    let events = tinotax_pricing::load_priced_ledger(&paths)?;
    let opening_pools = tinotax_tax_uk::load_opening_pools(&paths.opening_pools_file())?;
    let calc = tinotax_tax_uk::calculate(&events, &opening_pools, year, allow_unpriced)?;
    let dir = tinotax_tax_uk::write_reports(&paths, &calc)?;

    let s = &calc.summary;
    println!("UK tax calculation for {}:", s.tax_year);
    println!(
        "  disposals: {} — proceeds £{}, allowable costs £{}, gains £{}, losses £{}, net £{}",
        s.disposal_count,
        s.total_proceeds_gbp.round_dp(2),
        s.total_allowable_costs_gbp.round_dp(2),
        s.total_gains_gbp.round_dp(2),
        s.total_losses_gbp.round_dp(2),
        s.net_gain_or_loss_gbp.round_dp(2)
    );
    println!("  income: £{}", s.total_income_gbp.round_dp(2));
    if s.unresolved_blockers > 0 || s.unresolved_warnings > 0 {
        println!(
            "  unresolved: {} excluded blockers, {} warnings — see unresolved_tax_items.csv",
            s.unresolved_blockers, s.unresolved_warnings
        );
    }
    println!("outputs in {dir}");
    Ok(())
}

/// `pack hmrc`: the client-facing evidence pack for one tax year.
pub fn pack_hmrc(project: &str, tax_year: &str) -> Result<()> {
    let (paths, config) = crate::open_project(project)?;
    let year = tinotax_tax_uk::TaxYear::parse(tax_year)?;
    let dir = tinotax_evidence::build_pack(&paths, &config, &year.label())?;
    println!("evidence pack ready: {dir}");
    println!("review hmrc_questions_draft.md and questionnaire.toml before sending anything on.");
    Ok(())
}
