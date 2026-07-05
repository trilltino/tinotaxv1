# Workspace Crates

This directory contains the Rust workspace crates that implement TinoTax.

## Boundary Map

- `tinotax-cli` parses command-line arguments only.
- `tinotax-app` orchestrates command workflows.
- `tinotax-config` parses project and provider configuration.
- `tinotax-connectors` fetches external source data.
- `tinotax-store` owns project folders, raw cache, hashing, and JSONL IO.
- `tinotax-normalise` converts raw provider data into normalised events.
- `tinotax-diagnostics` reports data quality and review risk.
- `tinotax-review` owns spreadsheet review exports and append-only overrides.
- `tinotax-ledger` builds reviewed ledger events from normalised data.
- `tinotax-cex` imports centralised exchange CSV files.
- `tinotax-pricing` values reviewed ledger events in GBP.
- `tinotax-tax-uk` calculates UK CGT and income.
- `tinotax-report` exports normalised reports and audit manifests.
- `tinotax-evidence` assembles the HMRC evidence pack.

## Invariants

- No crate should mutate raw data after it has been cached.
- Tax calculation crates should consume reviewed, priced ledger data only.
- API/provider logic should not leak into tax logic.
- Human review decisions should remain append-only.
