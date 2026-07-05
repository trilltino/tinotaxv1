//! Raw and output manifest structures.
//!
//! Manifests record paths, hashes, source IDs, and timestamps so source data
//! and generated outputs can be audited later.
use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

/// One hashed raw page. Together these prove where the data came from,
/// when it was fetched, and that it has not been altered since.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFileManifestEntry {
    pub source_id: String,
    pub chain: String,
    pub wallet: String,
    pub endpoint: String,
    pub page: u64,
    /// Relative to the project root.
    pub path: String,
    pub blake3: String,
    pub fetched_at: String,
    pub item_count: u64,
}

/// Per-wallet manifest stored at `raw/{chain}/{wallet}/raw_manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawManifest {
    pub source_id: String,
    pub chain: String,
    pub wallet: String,
    pub generated_at: String,
    pub entries: Vec<RawFileManifestEntry>,
}

impl RawManifest {
    pub fn new(source_id: &str, chain: &str, wallet: &str) -> Self {
        Self {
            source_id: source_id.to_string(),
            chain: chain.to_string(),
            wallet: wallet.to_string(),
            generated_at: crate::now_rfc3339(),
            entries: Vec::new(),
        }
    }

    pub fn load_or_new(path: &Utf8Path, source_id: &str, chain: &str, wallet: &str) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(|| Self::new(source_id, chain, wallet))
    }

    /// Insert or replace the entry for (endpoint, page).
    pub fn upsert(&mut self, entry: RawFileManifestEntry) {
        self.entries
            .retain(|e| !(e.endpoint == entry.endpoint && e.page == entry.page));
        self.entries.push(entry);
        self.entries
            .sort_by(|a, b| (&a.endpoint, a.page).cmp(&(&b.endpoint, b.page)));
        self.generated_at = crate::now_rfc3339();
    }

    pub fn save(&self, path: &Utf8Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text).with_context(|| format!("writing {path}"))?;
        Ok(())
    }
}

/// A hashed derived output in `out/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFileEntry {
    /// Relative to the project root.
    pub path: String,
    pub blake3: String,
    pub bytes: u64,
}

/// Top-level `out/audit_manifest.json`: every raw file and every output,
/// hashed. Answers "where did this data come from and what proves it was
/// not changed?".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditManifest {
    pub project: String,
    pub tool: String,
    pub tool_version: String,
    pub generated_at: String,
    pub raw_files: Vec<RawFileManifestEntry>,
    pub outputs: Vec<OutputFileEntry>,
}

pub fn hash_file(path: &Utf8Path) -> Result<(String, u64)> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {path}"))?;
    Ok((
        blake3::hash(&bytes).to_hex().to_string(),
        bytes.len() as u64,
    ))
}

/// Collect every `raw_manifest.json` under `raw/`.
pub fn collect_raw_manifests(raw_dir: &Utf8Path) -> Result<Vec<RawManifest>> {
    let mut manifests = Vec::new();
    if !raw_dir.exists() {
        return Ok(manifests);
    }
    for entry in walkdir::WalkDir::new(raw_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() && entry.file_name() == "raw_manifest.json" {
            let path = Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
                .map_err(|p| anyhow::anyhow!("non-UTF8 path {}", p.display()))?;
            let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
            let manifest: RawManifest =
                serde_json::from_str(&text).with_context(|| format!("parsing {path}"))?;
            manifests.push(manifest);
        }
    }
    manifests.sort_by(|a, b| (&a.chain, &a.wallet).cmp(&(&b.chain, &b.wallet)));
    Ok(manifests)
}
