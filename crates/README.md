# Workspace Crates

This directory contains the Rust workspace crates that implement TinoTax.

## Boundary Map

```text
foundation/
  tinotax-core         pure domain types (events, tax ledger, prices, dates)
  tinotax-config       wallets.toml / project.toml (+ [[cex_csvs]])
  tinotax-store        project folders, raw cache, hashing, JSONL IO

ingest/
  tinotax-connectors   Blockscout + NearBlocks fetchers
  tinotax-cex          centralised exchange CSV importers
  tinotax-normalise    raw provider data -> normalised events

review/
  tinotax-diagnostics  data quality and review-risk reports
  tinotax-review       spreadsheet exports and append-only overrides
  tinotax-ledger       normalised events + overrides -> reviewed ledger

valuation/
  tinotax-pricing      GBP price book, manual import, fetch, valuation
  tinotax-tax-uk       UK CGT and income calculation

output/
  tinotax-report       normalised reports and audit manifests
  tinotax-evidence     HMRC evidence pack assembly

interface/
  tinotax-app          command workflow orchestration
  tinotax-cli          command-line parsing and dispatch
```

## Invariants

- No crate should mutate raw data after it has been cached.
- Tax calculation crates should consume reviewed, priced ledger data only.
- API/provider logic should not leak into tax logic.
- Human review decisions should remain append-only.
