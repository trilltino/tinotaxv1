use anyhow::{Context, Result};
use tinotax_store::manifest::collect_raw_manifests;
use tinotax_store::{hash_file, AuditManifest, OutputFileEntry, ProjectPaths};

/// Write `out/audit_manifest.json`: every raw page (with the hash recorded
/// at fetch time) plus every generated output, hashed now. This is the
/// document that answers "where did this data come from and what proves it
/// was not changed?".
///
/// Must run last — it hashes whatever is in `out/` at the time.
pub fn write_audit_manifest(paths: &ProjectPaths, project_name: &str) -> Result<()> {
    let raw_files = collect_raw_manifests(&paths.raw())?
        .into_iter()
        .flat_map(|m| m.entries)
        .collect();

    let mut outputs = Vec::new();
    let out_dir = paths.out();
    if out_dir.exists() {
        let mut names: Vec<_> = std::fs::read_dir(&out_dir)
            .with_context(|| format!("listing {out_dir}"))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| name != "audit_manifest.json")
            .collect();
        names.sort();
        for name in names {
            let path = out_dir.join(&name);
            let (blake3, bytes) = hash_file(&path)?;
            outputs.push(OutputFileEntry {
                path: paths.relative(&path),
                blake3,
                bytes,
            });
        }
    }

    let manifest = AuditManifest {
        project: project_name.to_string(),
        tool: "tinotax".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at: tinotax_store::now_rfc3339(),
        raw_files,
        outputs,
    };

    let path = out_dir.join("audit_manifest.json");
    std::fs::write(&path, serde_json::to_string_pretty(&manifest)?)
        .with_context(|| format!("writing {path}"))?;
    Ok(())
}
