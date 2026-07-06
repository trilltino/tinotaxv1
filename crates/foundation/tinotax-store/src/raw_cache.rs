//! Immutable raw endpoint cache.
//!
//! The cache writes provider pages, cursor state, and page listings for one
//! `raw/{chain}/{wallet}/{endpoint}/` directory. Page writes use create-new
//! semantics so source evidence cannot be silently overwritten.
use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};

use crate::project_dirs::ProjectPaths;

/// Resume state for one paginated endpoint. Written after every page, so a
/// crash or rate-limit abort loses at most the in-flight request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    pub schema: u32,
    /// Next page number to write (1-based).
    pub next_page: u64,
    /// Provider-specific continuation token/params for the next request.
    pub next_params: Option<serde_json::Value>,
    pub done: bool,
    pub updated_at: String,
}

impl Cursor {
    pub fn start() -> Self {
        Self {
            schema: 1,
            next_page: 1,
            next_params: None,
            done: false,
            updated_at: crate::now_rfc3339(),
        }
    }
}

/// Write-side of the immutable raw cache for one `raw/{chain}/{wallet}/{endpoint}/`.
#[derive(Debug, Clone)]
pub struct EndpointCache {
    pub chain: String,
    pub wallet: String,
    pub endpoint: String,
    dir: Utf8PathBuf,
    project_root: Utf8PathBuf,
}

impl EndpointCache {
    pub fn open(paths: &ProjectPaths, chain: &str, wallet: &str, endpoint: &str) -> Result<Self> {
        let dir = paths.wallet_raw_dir(chain, wallet).join(endpoint);
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {dir}"))?;
        Ok(Self {
            chain: chain.to_string(),
            wallet: wallet.to_string(),
            endpoint: endpoint.to_string(),
            dir,
            project_root: paths.root.clone(),
        })
    }

    pub fn dir(&self) -> &Utf8PathBuf {
        &self.dir
    }

    pub fn page_path(&self, page: u64) -> Utf8PathBuf {
        self.dir.join(format!("page_{page:06}.json"))
    }

    fn cursor_path(&self) -> Utf8PathBuf {
        self.dir.join("cursor.json")
    }

    /// Persist one fetched page exactly as received. Returns the path
    /// (relative to the project root) and the BLAKE3 hash of the bytes
    /// written.
    pub fn write_page(&self, page: u64, body: &serde_json::Value) -> Result<(String, String)> {
        let bytes = serde_json::to_vec_pretty(body).context("serialising raw page")?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let path = self.page_path(page);
        // `create_new(true)` is the evidence invariant: an already-captured
        // raw page can only be reused by resume logic, never overwritten by a
        // later provider response.
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                anyhow::bail!(
                    "raw page already exists at {path}; use --resume or create a new fetch run"
                );
            }
            Err(err) => return Err(err).with_context(|| format!("creating raw page {path}")),
        };
        file.write_all(&bytes)
            .with_context(|| format!("writing raw page {path}"))?;
        let rel = path
            .strip_prefix(&self.project_root)
            .map(|p| p.as_str().replace('\\', "/"))
            .unwrap_or_else(|_| path.as_str().replace('\\', "/"));
        // Store manifest paths relative to the project root so evidence packs
        // are relocatable across machines.
        Ok((rel, hash))
    }

    pub fn read_cursor(&self) -> Result<Option<Cursor>> {
        let path = self.cursor_path();
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
        let cursor = serde_json::from_str(&text).with_context(|| format!("parsing {path}"))?;
        Ok(Some(cursor))
    }

    pub fn write_cursor(&self, cursor: &Cursor) -> Result<()> {
        let path = self.cursor_path();
        let text = serde_json::to_string_pretty(cursor)?;
        std::fs::write(&path, text).with_context(|| format!("writing {path}"))?;
        Ok(())
    }

    /// All cached pages in page order: (page number, absolute path).
    pub fn list_pages(&self) -> Result<Vec<(u64, Utf8PathBuf)>> {
        let mut pages = Vec::new();
        for entry in
            std::fs::read_dir(&self.dir).with_context(|| format!("listing {}", self.dir))?
        {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(num) = name
                .strip_prefix("page_")
                .and_then(|s| s.strip_suffix(".json"))
                .and_then(|s| s.parse::<u64>().ok())
            {
                pages.push((num, self.dir.join(&name)));
            }
        }
        pages.sort_by_key(|(n, _)| *n);
        Ok(pages)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn roundtrips_pages_and_cursor() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
            .map_err(|path| std::io::Error::other(format!("non-UTF8 path {}", path.display())))?;
        let paths = ProjectPaths::new(root);
        paths.init()?;

        let cache = EndpointCache::open(&paths, "near", "foxboss.near", "transactions")?;
        let body = serde_json::json!({"txns": [{"transaction_hash": "abc"}]});
        let (rel, hash) = cache.write_page(1, &body)?;
        assert_eq!(rel, "raw/near/foxboss.near/transactions/page_000001.json");
        assert_eq!(hash.len(), 64);

        assert!(cache.read_cursor()?.is_none());
        let mut cursor = Cursor::start();
        cursor.next_page = 2;
        cache.write_cursor(&cursor)?;
        let saved_cursor = cache
            .read_cursor()?
            .ok_or_else(|| std::io::Error::other("expected saved cursor"))?;
        assert_eq!(saved_cursor.next_page, 2);

        let pages = cache.list_pages()?;
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].0, 1);
        Ok(())
    }

    #[test]
    fn refuses_to_overwrite_existing_page() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
            .map_err(|path| std::io::Error::other(format!("non-UTF8 path {}", path.display())))?;
        let paths = ProjectPaths::new(root);
        paths.init()?;

        let cache = EndpointCache::open(&paths, "near", "foxboss.near", "transactions")?;
        cache.write_page(1, &serde_json::json!({"first": true}))?;

        let err = match cache.write_page(1, &serde_json::json!({"first": false})) {
            Ok(_) => return Err(std::io::Error::other("expected overwrite refusal").into()),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("raw page already exists"));
        Ok(())
    }
}
