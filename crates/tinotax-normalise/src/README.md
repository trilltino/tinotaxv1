# tinotax-normalise/src

Normalisation implementation modules.

## Module Map

- `evm.rs` handles Blockscout-compatible EVM raw pages.
- `near.rs` handles NearBlocks raw pages.
- `event_id.rs` creates deterministic IDs.
- `classify.rs` provides conservative event confidence hints.
- `dedupe.rs` removes duplicate event IDs while preserving determinism.
- `lib.rs` orchestrates configured wallet normalisation.
