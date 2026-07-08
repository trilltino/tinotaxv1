//! Process-global cache of parsed normalised events.
//!
//! The desktop app fires several commands per refresh (status, insights, review
//! page, save), and each previously re-read and re-parsed the whole
//! `staging/normalised_events.jsonl` — 147 MB on large projects. This cache
//! parses once per file version and hands every caller a shared `Arc`, keyed by
//! the wallet/CEX file paths and their (mtime, len) so a `normalise`/`import`
//! that rewrites staging is picked up automatically on the next load.
//!
//! Correctness note: this only caches the *derived* staging events, keyed by the
//! files' own modification signature. It never serves data past an mtime/len
//! change and never suppresses a parse error — a miss falls straight through to
//! [`tinotax_review::load_all_events`], preserving its path/line context.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use tinotax_core::NormalisedEvent;
use tinotax_store::ProjectPaths;

type FileSig = Option<(SystemTime, u64)>;

struct CacheEntry {
    wallet_path: Utf8PathBuf,
    wallet_sig: FileSig,
    cex_sig: FileSig,
    events: Arc<Vec<NormalisedEvent>>,
}

fn cache() -> &'static Mutex<Option<CacheEntry>> {
    static CACHE: OnceLock<Mutex<Option<CacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn file_sig(path: &Utf8Path) -> FileSig {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// All normalised events for the project, reusing the last parse when the
/// staging files are byte-for-byte unchanged. The returned `Arc` lets callers
/// share a single parse for the duration of a refresh.
pub fn load_events_cached(paths: &ProjectPaths) -> Result<Arc<Vec<NormalisedEvent>>> {
    let wallet_path = paths.events_jsonl();
    let wallet_sig = file_sig(&wallet_path);
    let cex_sig = file_sig(&paths.cex_events_jsonl());

    {
        // A poisoned lock only means a prior holder panicked mid-update; recover
        // the guard rather than propagating, then treat it as a normal miss.
        let guard = cache().lock().unwrap_or_else(|poison| poison.into_inner());
        if let Some(entry) = guard.as_ref() {
            if entry.wallet_path == wallet_path
                && entry.wallet_sig == wallet_sig
                && entry.cex_sig == cex_sig
            {
                return Ok(Arc::clone(&entry.events));
            }
        }
    }

    // Miss: parse outside the lock so IO does not serialise other readers.
    let events = Arc::new(tinotax_review::load_all_events(paths)?);
    let mut guard = cache().lock().unwrap_or_else(|poison| poison.into_inner());
    *guard = Some(CacheEntry {
        wallet_path,
        wallet_sig,
        cex_sig,
        events: Arc::clone(&events),
    });
    Ok(events)
}
