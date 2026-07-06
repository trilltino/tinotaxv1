//! The human questionnaire: HMRC questions that blockchain data cannot
//! answer (source of funds, employment/PAYE, compensation, goods and
//! services, when activity began). Answers live in `questionnaire.toml`
//! at the project root and flow into the evidence pack.

use anyhow::{Context, Result};
use camino::Utf8Path;
use tinotax_store::ProjectPaths;

pub const QUESTIONNAIRE_TEMPLATE: &str = r#"# Answers HMRC asks for that on-chain data cannot provide.
# Fill in what applies and re-run `tinotax pack hmrc`.

[activity]
# When did cryptoasset activity begin? (YYYY-MM-DD or free text)
began_on = ""
notes = ""

[source_of_funds]
# Where did the money that bought the first crypto come from?
summary = ""
bank_statement_refs = []

[forks]
received_forks = false
notes = ""

[compensation]
received_compensation = false
notes = ""

[employment]
received_crypto_from_employment = false
# "yes" | "no" | "unknown"
paye_operated = "unknown"
notes = ""

[goods_services]
used_crypto_to_buy_goods_or_services = false
notes = ""
"#;

/// Create `questionnaire.toml` from the template if it does not exist.
/// Returns true if the file was just created (i.e. answers are pending).
pub fn ensure_questionnaire(paths: &ProjectPaths) -> Result<bool> {
    let path = paths.questionnaire_file();
    if path.exists() {
        return Ok(false);
    }
    std::fs::write(&path, QUESTIONNAIRE_TEMPLATE).with_context(|| format!("writing {path}"))?;
    Ok(true)
}

/// Parsed loosely so users can add their own keys without breaking the pack.
pub fn load_questionnaire(paths: &ProjectPaths) -> Result<toml::Value> {
    let path = paths.questionnaire_file();
    if !path.exists() {
        return Ok(toml::Value::Table(Default::default()));
    }
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
    toml::from_str(&text).with_context(|| format!("parsing {path}"))
}

pub fn q_str<'v>(q: &'v toml::Value, section: &str, key: &str) -> Option<&'v str> {
    q.get(section)?
        .get(key)?
        .as_str()
        .filter(|s| !s.trim().is_empty())
}

pub fn q_bool(q: &toml::Value, section: &str, key: &str) -> Option<bool> {
    q.get(section)?.get(key)?.as_bool()
}

/// `source_of_funds_notes.md` from the questionnaire (HMRC question 13).
pub fn source_of_funds_notes(q: &toml::Value, dir: &Utf8Path) -> Result<()> {
    let summary = q_str(q, "source_of_funds", "summary")
        .unwrap_or("*Not yet answered — fill in `questionnaire.toml` and re-run `pack hmrc`.*");
    let refs = q
        .get("source_of_funds")
        .and_then(|s| s.get("bank_statement_refs"))
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|i| i.as_str())
                .map(|s| format!("- {s}"))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "*(none listed)*".to_string());
    let text = format!(
        "# Source of funds\n\n{summary}\n\n## Supporting bank statement references\n\n{refs}\n"
    );
    std::fs::write(dir.join("source_of_funds_notes.md"), text)?;
    Ok(())
}
