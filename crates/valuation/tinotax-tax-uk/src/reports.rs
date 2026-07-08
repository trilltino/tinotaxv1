//! Write one tax year's calculation to `tax/<year>/` as the CSVs an
//! accountant reviews and the evidence pack repackages.

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use tinotax_store::ProjectPaths;

use crate::domain::UkTaxCalculation;

fn gbp(value: Decimal) -> String {
    value.round_dp(2).to_string()
}

/// Quantities carry up to 18 dp of chain precision. Round to 12 dp for the
/// human-facing reports — far below materiality for any token — and trim
/// trailing zeros. The pool math itself always uses full-precision Decimals;
/// this is display only.
fn qty(value: Decimal) -> String {
    value.round_dp(12).normalize().to_string()
}

/// A pool movement is dust — a sub-atomic rounding artifact from the source
/// data (e.g. a 3e-14 token "acquisition") — when both its quantity and cost
/// round away to nothing. These add noise rows without changing the pool.
fn is_dust_movement(quantity_delta: Decimal, cost_delta: Decimal) -> bool {
    quantity_delta.round_dp(12).is_zero() && cost_delta.round_dp(2).is_zero()
}

/// Returns the folder everything was written to.
pub fn write_reports(paths: &ProjectPaths, calc: &UkTaxCalculation) -> Result<camino::Utf8PathBuf> {
    let dir = paths.tax_dir(&calc.tax_year.label());
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {dir}"))?;

    // disposals_calculation.csv
    {
        let path = dir.join("disposals_calculation.csv");
        let mut w = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
        w.write_record([
            "disposal_id",
            "asset",
            "date",
            "tax_year",
            "quantity",
            "proceeds_gbp",
            "matched_same_day_quantity",
            "matched_same_day_cost_gbp",
            "matched_30_day_quantity",
            "matched_30_day_cost_gbp",
            "matched_s104_quantity",
            "matched_s104_cost_gbp",
            "allowable_cost_gbp",
            "gain_or_loss_gbp",
            "source_ledger_event_ids",
            "matching_notes",
        ])?;
        for d in &calc.disposals {
            w.write_record([
                d.disposal_id.as_str(),
                d.asset.as_str(),
                d.date.as_str(),
                d.tax_year.as_str(),
                &qty(d.quantity),
                &gbp(d.proceeds_gbp),
                &qty(d.matched_same_day_quantity),
                &gbp(d.matched_same_day_cost_gbp),
                &qty(d.matched_30_day_quantity),
                &gbp(d.matched_30_day_cost_gbp),
                &qty(d.matched_s104_quantity),
                &gbp(d.matched_s104_cost_gbp),
                &gbp(d.allowable_cost_gbp),
                &gbp(d.gain_or_loss_gbp),
                &d.source_ledger_event_ids.join("; "),
                &d.matching_notes.join(" | "),
            ])?;
        }
        w.flush()?;
    }

    // s104_pool_movements.csv
    {
        let path = dir.join("s104_pool_movements.csv");
        let mut w = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
        w.write_record([
            "asset",
            "date",
            "tax_year",
            "kind",
            "quantity_delta",
            "cost_delta_gbp",
            "quantity_after",
            "cost_after_gbp",
            "note",
        ])?;
        for m in &calc.pool_movements {
            // Skip pure-dust movements (sub-atomic source-data artifacts) so the
            // ledger reads cleanly; the pool math already accounts for them.
            if is_dust_movement(m.quantity_delta, m.cost_delta_gbp) {
                continue;
            }
            w.write_record([
                m.asset.as_str(),
                m.date.as_str(),
                m.tax_year.as_str(),
                m.kind.as_str(),
                &qty(m.quantity_delta),
                &gbp(m.cost_delta_gbp),
                &qty(m.quantity_after),
                &gbp(m.cost_after_gbp),
                m.note.as_str(),
            ])?;
        }
        w.flush()?;
    }

    // s104_pool_opening_closing.csv
    {
        let path = dir.join("s104_pool_opening_closing.csv");
        let mut w = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
        w.write_record([
            "asset",
            "opening_quantity",
            "opening_cost_gbp",
            "closing_quantity",
            "closing_cost_gbp",
        ])?;
        for s in &calc.pool_year_states {
            w.write_record([
                s.asset.as_str(),
                &qty(s.opening_quantity),
                &gbp(s.opening_cost_gbp),
                &qty(s.closing_quantity),
                &gbp(s.closing_cost_gbp),
            ])?;
        }
        w.flush()?;
    }

    // income_summary.csv (per-event rows; totals live in the SA summary)
    {
        let path = dir.join("income_summary.csv");
        let mut w = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
        w.write_record([
            "ledger_event_id",
            "date",
            "tax_year",
            "category",
            "asset",
            "quantity",
            "income_gbp",
            "note",
        ])?;
        for i in &calc.income {
            w.write_record([
                i.ledger_event_id.as_str(),
                i.date.as_str(),
                i.tax_year.as_str(),
                i.category.as_str(),
                i.asset.as_str(),
                &qty(i.quantity),
                &gbp(i.income_gbp),
                i.note.as_deref().unwrap_or(""),
            ])?;
        }
        w.flush()?;
    }

    // self_assessment_crypto_summary.csv
    {
        let path = dir.join("self_assessment_crypto_summary.csv");
        let mut w = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
        w.write_record(["item", "value"])?;
        let s = &calc.summary;
        let rows: Vec<(String, String)> = [
            ("tax_year".to_string(), s.tax_year.clone()),
            (
                "number_of_disposals".to_string(),
                s.disposal_count.to_string(),
            ),
            ("total_proceeds_gbp".to_string(), gbp(s.total_proceeds_gbp)),
            (
                "total_allowable_costs_gbp".to_string(),
                gbp(s.total_allowable_costs_gbp),
            ),
            ("total_gains_gbp".to_string(), gbp(s.total_gains_gbp)),
            ("total_losses_gbp".to_string(), gbp(s.total_losses_gbp)),
            (
                "net_gain_or_loss_gbp".to_string(),
                gbp(s.net_gain_or_loss_gbp),
            ),
            ("total_income_gbp".to_string(), gbp(s.total_income_gbp)),
            (
                "crypto_fees_disposed_gbp".to_string(),
                gbp(s.crypto_fees_disposed_gbp),
            ),
            (
                "unresolved_blockers_excluded".to_string(),
                s.unresolved_blockers.to_string(),
            ),
            (
                "unresolved_warnings".to_string(),
                s.unresolved_warnings.to_string(),
            ),
        ]
        .into_iter()
        .chain(
            s.income_by_category_gbp
                .iter()
                .map(|(category, total)| (format!("income_{category}_gbp"), gbp(*total))),
        )
        .collect();
        for (item, value) in rows {
            w.write_record([item.as_str(), value.as_str()])?;
        }
        w.flush()?;
    }

    // unresolved_tax_items.csv
    {
        let path = dir.join("unresolved_tax_items.csv");
        let mut w = csv::Writer::from_path(&path).with_context(|| format!("creating {path}"))?;
        w.write_record(["ledger_event_id", "asset", "date", "severity", "reason"])?;
        for u in &calc.unresolved {
            w.write_record([
                u.ledger_event_id.as_str(),
                u.asset.as_str(),
                u.date.as_str(),
                u.severity.as_str(),
                u.reason.as_str(),
            ])?;
        }
        w.flush()?;
    }

    // assumptions_and_limitations.md
    {
        let s = &calc.summary;
        let text = format!(
            "# Assumptions and limitations — {year}\n\n\
             Generated by TinoTax v{version}. This is calculation support for a Self\n\
             Assessment return, not tax advice; it should be reviewed by an accountant.\n\n\
             ## Method\n\n\
             - Disposals are matched same-day, then against acquisitions in the following\n\
               30 days (earliest first), then against the Section 104 pool, following\n\
               HMRC's Cryptoassets Manual (CRYPTO22200, CRYPTO22251-22256).\n\
             - All disposals of an asset on one day are treated as a single disposal and\n\
               all acquisitions on one day as a single acquisition (TCGA92 s105).\n\
             - The full event timeline is processed so pools carry across tax years;\n\
               this report covers {year} only.\n\n\
             ## Assumptions\n\n\
             - Tax-year boundaries use the UTC date of each event.\n\
             - Crypto-to-crypto swaps are a disposal of the sold token and an acquisition\n\
               of the bought token, each at GBP market value.\n\
             - Network/exchange fees paid in crypto are disposals of the fee asset at\n\
               market value. Fiat fees stated by exchanges are noted on events but not\n\
               added to allowable costs unless entered during review.\n\
             - Transfers and bridge movements between the client's own wallets have no\n\
               tax effect; classification was human-reviewed where flagged.\n\
             - Airdrops received for nothing are capital acquisitions at market value,\n\
               not income (HMRC CRYPTO21250); airdrops received in return for a service\n\
               were classified as income during review.\n\
             - Staking/mining/other income is valued in GBP at receipt and that value\n\
               becomes the CGT cost basis.\n\
             - Fork receipts carry the base cost entered during review (default GBP 0 —\n\
               conservative; HMRC expects an apportionment of the original asset's cost).\n\
             - Compensation receipts are treated as taxable income at receipt; an\n\
               accountant should confirm capital treatment is not more appropriate.\n\n\
             ## Data quality in this year\n\n\
             - Unresolved blockers excluded from the calculation: {blockers}\n\
             - Included rows still flagged as warnings: {warnings}\n\
             - See `unresolved_tax_items.csv` for every item.\n",
            year = s.tax_year,
            version = env!("CARGO_PKG_VERSION"),
            blockers = s.unresolved_blockers,
            warnings = s.unresolved_warnings,
        );
        std::fs::write(dir.join("assumptions_and_limitations.md"), text)?;
    }

    Ok(dir)
}
