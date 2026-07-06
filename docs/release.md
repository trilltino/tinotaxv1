# Release Process

TinoTax releases should be reproducible enough that a client/project output can
be traced back to the exact binary, source revision, and dependency graph.

## Local Release Gate

```bash
just production-check
```

This includes the first-party no-unsafe/no-unwrap policy scan.

For a project deliverable:

```bash
just startup wallets.toml ./fox-project
just readiness ./fox-project
```

After review/pricing/tax outputs are complete, run `just readiness` again.

## Release Metadata To Record

- TinoTax crate version.
- Git commit.
- Rust toolchain from `rust-toolchain.toml`.
- Cargo version and target triple.
- Hash of `Cargo.lock`.
- `target/cargo-metadata.json` from the locked dependency graph.
- Enabled feature set, currently all default workspace features.
- Project `out/audit_manifest.json`.
- Result of `cargo audit`, `cargo deny check`, and duplicate dependency review.

## Artifact Policy

Generated project folders contain client data and stay out of git. Release
artifacts may include binaries, checksums, locked metadata, and docs, but not
raw wallet data, CEX CSVs, API keys, or evidence packs unless they are delivered
through the approved client channel.
