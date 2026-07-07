//! Desktop CEX CSV import.
//!
//! The CLI's `import-cex` is config-driven: it (re)builds the CEX staging
//! files from every `[[cex_csvs]]` entry in `project.toml`. To stay
//! consistent with that, a desktop import does three things:
//!
//! 1. copies the picked file into `raw/cex/<id>/original.csv` (refusing to
//!    replace different content — evidence is immutable),
//! 2. registers the export in `project.toml`, with `path` pointing at the
//!    raw copy so later runs never depend on the user's Downloads folder,
//! 3. re-runs the shared importer over all declared sources.
//!
//! Without step 2 the next CLI run would silently drop the imported events.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::Serialize;
use tinotax_cex::ImportReport;
use tinotax_config::CexPlatform;
use tinotax_store::hash_file;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CexImportResultDto {
    pub source_id: String,
    pub platform: String,
    pub rows_read: u64,
    pub events_emitted: u64,
    pub fiat_movements_skipped: u64,
    pub zero_amount_skipped: u64,
    pub needs_review: u64,
    pub price_hints: u64,
    pub earliest: String,
    pub latest: String,
    /// Total `[[cex_csvs]]` sources now declared in the project.
    pub total_sources: usize,
}

pub fn desktop_import_cex(
    project: &str,
    source_id: &str,
    platform: &str,
    file: &str,
    mapping: Option<BTreeMap<String, String>>,
) -> Result<CexImportResultDto> {
    let (paths, config) = crate::open_project(project)?;

    let source_id = source_id.trim().to_ascii_lowercase();
    if source_id.is_empty()
        || !source_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        bail!("source id must be letters, digits, `_` or `-` (e.g. kraken_2021)");
    }
    let platform = parse_platform(platform)?;
    let mapping = mapping.filter(|m| !m.is_empty());
    if platform == CexPlatform::Generic && mapping.is_none() {
        bail!(
            "platform `generic` needs a column mapping (canonical names: \
             timestamp, type, asset, amount, fee_asset, fee_amount, note)"
        );
    }

    // Immutable evidence copy, mirroring the importer's refusal semantics so
    // the failure happens before the config is touched.
    let source_path = camino::Utf8PathBuf::from(file);
    if !source_path.exists() {
        bail!("file not found: {source_path}");
    }
    let raw_dir = paths.cex_raw_dir(&source_id);
    std::fs::create_dir_all(&raw_dir).with_context(|| format!("creating {raw_dir}"))?;
    let original = raw_dir.join("original.csv");
    let (source_hash, _) = hash_file(&source_path)?;
    if original.exists() {
        let (existing_hash, _) = hash_file(&original)?;
        if existing_hash != source_hash {
            bail!(
                "raw/cex/{source_id}/original.csv already exists with different content — \
                 raw evidence is never overwritten; import the new export under a new id"
            );
        }
    } else {
        std::fs::copy(source_path.as_std_path(), original.as_std_path())
            .with_context(|| format!("copying {source_path} to {original}"))?;
    }

    // Register in project.toml (append-only, preserving the existing text).
    if let Some(existing) = config.cex_csvs.iter().find(|entry| entry.id == source_id) {
        if existing.platform != platform {
            bail!(
                "cex source {source_id:?} is already declared as platform {:?}",
                existing.platform.as_str()
            );
        }
    } else {
        let config_path = paths.config_file();
        let before = std::fs::read_to_string(&config_path)
            .with_context(|| format!("reading {config_path}"))?;
        let mut text = before.clone();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str("\n[[cex_csvs]]\n");
        text.push_str(&format!("id = {}\n", toml_string(&source_id)));
        text.push_str(&format!("platform = {}\n", toml_string(platform.as_str())));
        // Forward slashes keep the TOML clean and work fine on Windows.
        text.push_str(&format!(
            "path = {}\n",
            toml_string(&original.as_str().replace('\\', "/"))
        ));
        if let Some(mapping) = &mapping {
            text.push_str("\n[cex_csvs.mapping]\n");
            for (canonical, header) in mapping {
                text.push_str(&format!(
                    "{} = {}\n",
                    toml_string(canonical.trim()),
                    toml_string(header.trim())
                ));
            }
        }
        std::fs::write(&config_path, &text).with_context(|| format!("writing {config_path}"))?;
        // The declared entry must round-trip through full config validation;
        // restore the previous config if it does not.
        if let Err(err) = tinotax_config::ProjectConfig::load(&config_path) {
            std::fs::write(&config_path, before)
                .with_context(|| format!("restoring {config_path}"))?;
            bail!("registering the export made the project config invalid: {err}");
        }
    }

    // Reload so the importer sees the new entry, then rebuild staging from
    // every declared source (the importer is idempotent over raw copies).
    let (_, config) = crate::open_project(project)?;
    let reports = tinotax_cex::import_all(&paths, &config)?;
    let report = reports
        .iter()
        .find(|report| report.source_id == source_id)
        .with_context(|| format!("no import report for {source_id}"))?;

    Ok(to_dto(report, config.cex_csvs.len()))
}

fn to_dto(report: &ImportReport, total_sources: usize) -> CexImportResultDto {
    CexImportResultDto {
        source_id: report.source_id.clone(),
        platform: report.platform.clone(),
        rows_read: report.rows_read,
        events_emitted: report.events_emitted,
        fiat_movements_skipped: report.fiat_movements_skipped,
        zero_amount_skipped: report.zero_amount_skipped,
        needs_review: report.needs_review,
        price_hints: report.price_hints,
        earliest: report.earliest.clone().unwrap_or_default(),
        latest: report.latest.clone().unwrap_or_default(),
        total_sources,
    }
}

fn parse_platform(text: &str) -> Result<CexPlatform> {
    Ok(match text.trim().to_ascii_lowercase().as_str() {
        "binance" => CexPlatform::Binance,
        "coinbase" => CexPlatform::Coinbase,
        "kraken" => CexPlatform::Kraken,
        "awaken" => CexPlatform::Awaken,
        "generic" => CexPlatform::Generic,
        other => bail!(
            "unknown platform {other:?} (supported: binance, coinbase, kraken, awaken, generic)"
        ),
    })
}

fn toml_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use camino::Utf8PathBuf;
    use tinotax_store::ProjectPaths;

    use super::*;

    const CONFIG: &str = r#"
[project]
name = "cex-import-test"
base_currency = "GBP"
period_start = "2017-01-01T00:00:00Z"
period_end = "2025-04-05T23:59:59Z"

[[wallets]]
id = "lisk_main"
name = "Lisk wallet"
chain = "lisk-evm"
address = "0xAAAA000000000000000000000000000000000001"
provider = "lisk_blockscout"

[providers.lisk_blockscout]
kind = "blockscout"
base_url = "https://blockscout.lisk.com/api/v2"
"#;

    const KRAKEN_CSV: &str = "\
txid,refid,time,type,subtype,aclass,asset,wallet,amount,fee,balance\n\
L1,R1,2024-05-01 10:00:00,deposit,,currency,XXBT,spot,0.5,0,0.5\n\
L2,R2,2024-06-01 11:00:00,trade,,currency,XXBT,spot,-0.2,0.0001,0.2999\n\
L3,R3,2024-06-01 11:00:00,trade,,currency,ZGBP,spot,9000,0,9000\n";

    fn seed_project() -> Result<(tempfile::TempDir, String, Utf8PathBuf), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
            .map_err(|p| std::io::Error::other(format!("non-UTF8 path {}", p.display())))?;
        let paths = ProjectPaths::new(root.clone());
        paths.init()?;
        std::fs::write(paths.config_file(), CONFIG)?;
        let csv = root.join("kraken_export.csv");
        std::fs::write(&csv, KRAKEN_CSV)?;
        Ok((tmp, root.to_string(), csv))
    }

    #[test]
    fn imports_registers_and_reimports_idempotently() -> Result<(), Box<dyn Error>> {
        let (_tmp, project, csv) = seed_project()?;
        let result = desktop_import_cex(&project, "Kraken_2024", "kraken", csv.as_str(), None)?;

        assert_eq!(result.source_id, "kraken_2024");
        assert_eq!(result.rows_read, 3);
        // BTC deposit + BTC trade + BTC fee; the GBP legs are fiat and skipped.
        assert_eq!(result.events_emitted, 3);
        assert_eq!(result.fiat_movements_skipped, 1);
        assert_eq!(result.total_sources, 1);

        let paths = ProjectPaths::new(Utf8PathBuf::from(&project));
        assert!(paths.cex_raw_dir("kraken_2024").join("original.csv").exists());
        let config = tinotax_config::ProjectConfig::load(&paths.config_file())?;
        assert_eq!(config.cex_csvs.len(), 1);
        assert_eq!(config.cex_csvs[0].id, "kraken_2024");
        let events = std::fs::read_to_string(paths.cex_events_jsonl())?;
        assert_eq!(events.lines().count(), 3);

        // Same file again under the same id: no duplicate config entry.
        let again = desktop_import_cex(&project, "kraken_2024", "kraken", csv.as_str(), None)?;
        assert_eq!(again.total_sources, 1);
        let config = tinotax_config::ProjectConfig::load(&paths.config_file())?;
        assert_eq!(config.cex_csvs.len(), 1);
        Ok(())
    }

    #[test]
    fn refuses_different_content_under_same_id() -> Result<(), Box<dyn Error>> {
        let (_tmp, project, csv) = seed_project()?;
        desktop_import_cex(&project, "kraken_2024", "kraken", csv.as_str(), None)?;

        let other = Utf8PathBuf::from(&project).join("other.csv");
        std::fs::write(&other, KRAKEN_CSV.replace("0.5", "0.7"))?;
        let err = match desktop_import_cex(&project, "kraken_2024", "kraken", other.as_str(), None)
        {
            Ok(_) => return Err(std::io::Error::other("expected overwrite refusal").into()),
            Err(err) => err,
        };
        assert!(err.to_string().contains("never overwritten"));
        Ok(())
    }

    #[test]
    fn generic_requires_mapping_and_bad_platform_fails() -> Result<(), Box<dyn Error>> {
        let (_tmp, project, csv) = seed_project()?;
        let err = match desktop_import_cex(&project, "x", "generic", csv.as_str(), None) {
            Ok(_) => return Err(std::io::Error::other("expected mapping error").into()),
            Err(err) => err,
        };
        assert!(err.to_string().contains("column mapping"));

        let err = match desktop_import_cex(&project, "x", "ftx", csv.as_str(), None) {
            Ok(_) => return Err(std::io::Error::other("expected platform error").into()),
            Err(err) => err,
        };
        assert!(err.to_string().contains("unknown platform"));
        Ok(())
    }
}
