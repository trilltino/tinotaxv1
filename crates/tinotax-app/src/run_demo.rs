use anyhow::Result;

/// The Friday command: whole pipeline in one shot.
///
/// 1. create the project folder from the config
/// 2. fetch all wallets into the raw cache (resumable)
/// 3. normalise raw pages to `staging/normalised_events.jsonl`
/// 4. diagnostics (`diagnostics.json`, `wallet_activity_summary.csv`)
/// 5. review export (`manual_review.csv`)
/// 6. reports (`normalised_transactions.csv`) + `audit_manifest.json`
pub async fn run_demo(config: &str, out: &str, resume: bool) -> Result<()> {
    println!("== 1/5 project init ==");
    crate::project_init(config, out)?;

    println!("\n== 2/5 fetch ==");
    crate::fetch_project(out, resume).await?;

    println!("\n== 3/5 normalise ==");
    crate::normalise_project(out)?;

    println!("\n== 4/5 diagnose ==");
    crate::diagnose_project(out)?;

    println!("\n== 5/5 review + reports ==");
    crate::export_review(out)?;
    crate::export_review_all(out)?;
    crate::export_reports(out)?;

    println!("\ndemo complete — outputs in {out}/out:");
    for name in [
        "normalised_transactions.csv",
        "wallet_activity_summary.csv",
        "review_all_transactions.csv",
        "manual_review.csv",
        "diagnostics.json",
        "audit_manifest.json",
    ] {
        println!("  {out}/out/{name}");
    }
    println!("\nnext: edit review_all_transactions.csv, then `review apply`, `ledger build`,");
    println!("`prices import`/`prices fetch`, `ledger price`, `calculate uk`, `pack hmrc`.");
    Ok(())
}
