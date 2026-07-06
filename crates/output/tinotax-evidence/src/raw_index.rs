//! `raw_data_index.csv`: every file under `raw/`, hashed. HMRC asks for
//! full unedited data files; this proves what they are and that they have
//! not changed.

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use tinotax_store::{hash_file, ProjectPaths};

pub fn write_raw_index(paths: &ProjectPaths, dir: &Utf8Path) -> Result<u64> {
    let out = dir.join("raw_data_index.csv");
    let mut writer = csv::Writer::from_path(&out).with_context(|| format!("creating {out}"))?;
    writer.write_record(["path", "bytes", "blake3"])?;

    let mut rows = 0u64;
    let raw = paths.raw();
    if raw.exists() {
        let mut files = Vec::new();
        for entry in walkdir::WalkDir::new(&raw) {
            let entry = entry.with_context(|| format!("walking {raw}"))?;
            if entry.file_type().is_file() {
                let path = Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
                    .map_err(|p| anyhow::anyhow!("non-UTF8 path {}", p.display()))?;
                files.push(path);
            }
        }
        files.sort();
        for file in files {
            let (hash, bytes) = hash_file(&file)?;
            writer.write_record([
                paths.relative(&file).as_str(),
                &bytes.to_string(),
                &format!("blake3:{hash}"),
            ])?;
            rows += 1;
        }
    }
    writer.flush()?;
    Ok(rows)
}
