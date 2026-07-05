//! `hmrc_questions_draft.md`: HMRC's standard cryptoasset information
//! request, question by question, each answered by files in this pack (or
//! flagged as needing the questionnaire).

use std::collections::BTreeMap;

use camino::Utf8Path;

use crate::assumptions::{q_bool, q_str};

/// `summary` is the parsed self_assessment_crypto_summary.csv (item → value).
pub fn hmrc_questions_draft(
    tax_year: &str,
    summary: &BTreeMap<String, String>,
    questionnaire: &toml::Value,
) -> String {
    let value = |key: &str| summary.get(key).cloned().unwrap_or_else(|| "n/a".into());
    let began = q_str(questionnaire, "activity", "began_on")
        .map(str::to_string)
        .unwrap_or_else(|| "**pending — answer in questionnaire.toml**".into());
    let yn =
        |section: &str, key: &str, question_note: &str| match q_bool(questionnaire, section, key) {
            Some(true) => format!("Yes — see notes in questionnaire.toml. {question_note}"),
            Some(false) => "No, per the client's questionnaire answers.".to_string(),
            None => "**pending — answer in questionnaire.toml**".to_string(),
        };

    format!(
        "# HMRC cryptoasset questions — draft responses ({tax_year})\n\n\
         Draft only: an accountant should review before anything is sent to HMRC.\n\n\
         ## 1. When did cryptoasset activities begin?\n\n\
         {began}\n\n\
         The earliest activity visible in the data is in `platforms_protocols_used.csv`\n\
         (first_seen column).\n\n\
         ## 2. Full capital gains calculations with Section 104 matching/pooling\n\n\
         - `disposals_calculation.csv` — every disposal, matched same-day → 30-day → S104\n\
         - `s104_pool_movements.csv` — every pool change\n\
         - `s104_pool_opening_closing.csv` — pool balances at the year boundaries\n\n\
         Headline figures: {count} disposals, total proceeds £{proceeds}, total\n\
         allowable costs £{costs}, net gain/loss £{net}.\n\n\
         ## 3. If Section 104 was not applied, why\n\n\
         Section 104 pooling **was** applied, with the statutory same-day and 30-day\n\
         exceptions. See `assumptions_and_limitations.md`.\n\n\
         ## 4. Commercial calculator used\n\n\
         See `calculator_statement.md` (TinoTax, version, method, audit trail).\n\n\
         ## 5. Platforms, exchanges and protocols used\n\n\
         `platforms_protocols_used.csv`.\n\n\
         ## 6. Full, unedited trading data files\n\n\
         `raw_data_index.csv` lists every raw file with its BLAKE3 hash. Exchange\n\
         exports are stored unedited under the project's `raw/cex/<id>/original.csv`;\n\
         wallet API pages under `raw/<chain>/<wallet>/`.\n\n\
         ## 7. Forks\n\n\
         {forks}\n\
         Fork receipts, if any, appear in `income_summary.csv` with category `fork`.\n\n\
         ## 8. Airdrops received and sold\n\n\
         `income_summary.csv` (category `airdrop`) lists receipts; later sales appear\n\
         in `disposals_calculation.csv` under the same asset.\n\n\
         ## 9. Compensation for lost cryptoassets\n\n\
         {compensation}\n\n\
         ## 10. Employment / self-employment crypto\n\n\
         {employment}\n\
         Any rows classified as employment income are in `income_summary.csv`\n\
         (categories `employment_income`, `self_employment_income`).\n\n\
         ## 11. Mining / staking\n\n\
         `income_summary.csv` (categories `staking_reward`, `mining_reward`,\n\
         `misc_income`) — total income this year £{income}.\n\n\
         ## 12. Crypto used to acquire goods, services or property\n\n\
         {goods}\n\
         Rows classified `goods_or_services_spend` appear in\n\
         `disposals_calculation.csv` as disposals.\n\n\
         ## 13. Source of funds\n\n\
         See `source_of_funds_notes.md`.\n",
        count = value("number_of_disposals"),
        proceeds = value("total_proceeds_gbp"),
        costs = value("total_allowable_costs_gbp"),
        net = value("net_gain_or_loss_gbp"),
        income = value("total_income_gbp"),
        forks = yn("forks", "received_forks", ""),
        compensation = yn("compensation", "received_compensation", ""),
        employment = yn(
            "employment",
            "received_crypto_from_employment",
            "Confirm whether PAYE was operated."
        ),
        goods = yn("goods_services", "used_crypto_to_buy_goods_or_services", ""),
    )
}

/// Parse `self_assessment_crypto_summary.csv` back into item → value.
pub fn load_summary_csv(path: &Utf8Path) -> anyhow::Result<BTreeMap<String, String>> {
    use anyhow::Context;
    let mut reader = csv::Reader::from_path(path)
        .with_context(|| format!("reading {path} — run `calculate uk` first"))?;
    let mut map = BTreeMap::new();
    for record in reader.records() {
        let record = record?;
        if let (Some(item), Some(value)) = (record.get(0), record.get(1)) {
            map.insert(item.to_string(), value.to_string());
        }
    }
    Ok(map)
}
