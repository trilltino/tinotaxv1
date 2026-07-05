use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};

/// Canonical layout of a runtime project folder.
///
/// ```text
/// <root>/project.toml   copy of the config the project was created from
/// <root>/raw/           immutable fetched pages (never regenerated)
/// <root>/staging/       derived JSONL (safe to delete)
/// <root>/out/           derived CSV/JSON outputs (safe to delete)
/// <root>/logs/
/// ```
#[derive(Debug, Clone)]
pub struct ProjectPaths {
    pub root: Utf8PathBuf,
}

impl ProjectPaths {
    pub fn new(root: impl Into<Utf8PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn config_file(&self) -> Utf8PathBuf {
        self.root.join("project.toml")
    }

    pub fn raw(&self) -> Utf8PathBuf {
        self.root.join("raw")
    }

    pub fn staging(&self) -> Utf8PathBuf {
        self.root.join("staging")
    }

    pub fn out(&self) -> Utf8PathBuf {
        self.root.join("out")
    }

    pub fn logs(&self) -> Utf8PathBuf {
        self.root.join("logs")
    }

    pub fn wallet_raw_dir(&self, chain: &str, wallet: &str) -> Utf8PathBuf {
        self.raw().join(chain).join(wallet)
    }

    pub fn events_jsonl(&self) -> Utf8PathBuf {
        self.staging().join("normalised_events.jsonl")
    }

    pub fn rejected_jsonl(&self) -> Utf8PathBuf {
        self.staging().join("rejected_raw_items.jsonl")
    }

    pub fn warnings_jsonl(&self) -> Utf8PathBuf {
        self.staging().join("warnings.jsonl")
    }

    /// Normalised events derived from CEX CSV imports (kept separate from
    /// wallet events so re-running either import never clobbers the other).
    pub fn cex_events_jsonl(&self) -> Utf8PathBuf {
        self.staging().join("cex_normalised_events.jsonl")
    }

    /// Append-only log of human review decisions. Never truncated: `ledger
    /// build` takes the latest decision per event.
    pub fn overrides_jsonl(&self) -> Utf8PathBuf {
        self.staging().join("review_overrides.jsonl")
    }

    pub fn reviewed_ledger_jsonl(&self) -> Utf8PathBuf {
        self.staging().join("reviewed_ledger.jsonl")
    }

    pub fn price_observations_jsonl(&self) -> Utf8PathBuf {
        self.staging().join("price_observations.jsonl")
    }

    /// GBP spot prices captured incidentally from CEX exports.
    pub fn price_hints_jsonl(&self) -> Utf8PathBuf {
        self.staging().join("cex_price_hints.jsonl")
    }

    pub fn priced_ledger_jsonl(&self) -> Utf8PathBuf {
        self.staging().join("priced_ledger.jsonl")
    }

    /// Immutable copy of one imported CEX CSV, e.g. `raw/cex/binance_2017/`.
    pub fn cex_raw_dir(&self, source_id: &str) -> Utf8PathBuf {
        self.raw().join("cex").join(source_id)
    }

    /// Tax calculation outputs for one tax year, e.g. `tax/2024-2025/`.
    pub fn tax_dir(&self, tax_year: &str) -> Utf8PathBuf {
        self.root.join("tax").join(tax_year)
    }

    /// Final client-facing deliverable for one tax year.
    pub fn evidence_dir(&self, tax_year: &str) -> Utf8PathBuf {
        self.root.join("evidence_pack").join(tax_year)
    }

    /// Human-answered questions HMRC asks that data alone cannot answer.
    pub fn questionnaire_file(&self) -> Utf8PathBuf {
        self.root.join("questionnaire.toml")
    }

    /// Holdings acquired before the data window (seeds the S104 pools).
    pub fn opening_pools_file(&self) -> Utf8PathBuf {
        self.root.join("opening_pools.toml")
    }

    /// Create the folder skeleton. Idempotent.
    pub fn init(&self) -> Result<()> {
        for dir in [
            self.root.clone(),
            self.raw(),
            self.staging(),
            self.out(),
            self.logs(),
        ] {
            std::fs::create_dir_all(&dir).with_context(|| format!("creating {dir}"))?;
        }
        Ok(())
    }

    /// Create the skeleton and install the config as `project.toml`.
    pub fn init_from_config(root: impl Into<Utf8PathBuf>, config_path: &Utf8Path) -> Result<Self> {
        let paths = Self::new(root);
        paths.init()?;
        let text = std::fs::read_to_string(config_path)
            .with_context(|| format!("reading config {config_path}"))?;
        std::fs::write(paths.config_file(), text)
            .with_context(|| format!("writing {}", paths.config_file()))?;
        Ok(paths)
    }

    /// Path relative to the project root, for storing in source refs and
    /// manifests (portable across machines). Falls back to the absolute
    /// path if `path` is not under the root.
    pub fn relative(&self, path: &Utf8Path) -> String {
        path.strip_prefix(&self.root)
            .map(|p| p.as_str().replace('\\', "/"))
            .unwrap_or_else(|_| path.as_str().replace('\\', "/"))
    }
}
