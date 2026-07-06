# tinotax-core

Shared domain model crate.

## Owns

- Event, ledger, price, review, asset, source, chain, and date primitives.
- Types shared across crates.
- Deterministic parsing helpers that are not tied to IO.

## Does Not Own

- File storage.
- External API fetching.
- Command orchestration.
- Tax engine state machines.

## Key Files

- `src/event.rs` defines normalised source events.
- `src/tax_event.rs` defines reviewed ledger event types.
- `src/review.rs` defines append-only review override records.
- `src/price.rs` defines price observations and confidence.
