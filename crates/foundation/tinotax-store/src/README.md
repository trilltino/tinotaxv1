# tinotax-store/src

Storage implementation modules.

## Module Map

- `project_dirs.rs` defines the project folder contract.
- `raw_cache.rs` writes immutable raw provider pages and cursors.
- `jsonl.rs` reads and writes append-style JSONL records.
- `manifest.rs` indexes raw files and computes hashes.

## Invariants

- Keep path construction centralised.
- Preserve source evidence exactly as received.
- Treat overwrite attempts as data-integrity errors.
