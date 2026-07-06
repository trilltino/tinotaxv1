# tinotax-app/src

This source directory contains command orchestration modules.

## Module Map

- `doctor.rs` checks environment and provider reachability.
- `fetch_project.rs` fetches configured wallets into raw cache folders.
- `normalise_project.rs` converts raw wallet data into normalised events.
- `diagnose_project.rs` writes data-quality reports.
- `export_review.rs` exports and applies review files.
- `pipeline.rs` contains reusable command bodies for ledger, pricing, tax, and
  pack workflows.
- `run_demo.rs` composes the first commercial demo flow.

## Invariants

- Keep business rules in domain crates.
- Keep IO paths flowing through `tinotax-store::ProjectPaths`.
- Prefer small orchestration functions with clear user-facing output.
