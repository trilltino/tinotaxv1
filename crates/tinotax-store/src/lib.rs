//! Project storage primitives for TinoTax.
//!
//! This crate owns project paths, raw cache writes, manifests, hashing, and
//! JSONL helpers. It deliberately avoids domain interpretation.
pub mod jsonl;
pub mod manifest;
pub mod project_dirs;
pub mod raw_cache;

pub use jsonl::{read_jsonl, JsonlWriter};
pub use manifest::{hash_file, AuditManifest, OutputFileEntry, RawFileManifestEntry, RawManifest};
pub use project_dirs::ProjectPaths;
pub use raw_cache::{Cursor, EndpointCache};

/// RFC 3339 timestamp for "now", used in manifests and cursors.
pub fn now_rfc3339() -> String {
    jiff::Timestamp::now().to_string()
}
