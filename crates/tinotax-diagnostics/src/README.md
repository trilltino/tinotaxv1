# tinotax-diagnostics/src

Diagnostic report modules.

## Module Map

- `summary.rs` writes wallet/activity summaries.
- `assets.rs` reports asset movement coverage.
- `duplicates.rs` identifies duplicate event IDs.
- `review_flags.rs` counts rows requiring human review.

## Invariants

- Diagnostics should explain data risk without mutating source data.
- Reports should be deterministic from the current normalised event set.
