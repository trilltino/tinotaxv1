# tinotax-connectors

External data connector crate.

## Owns

- Provider fetcher traits and provider factories.
- HTTP retry behaviour.
- Blockscout and NearBlocks wallet fetchers.
- Provider response model structs.

## Does Not Own

- Raw cache file layout beyond calling `tinotax-store`.
- Normalising provider JSON into tax events.
- Tax classification.

## Key Files

- `src/fetcher.rs` defines wallet fetcher interfaces.
- `src/http.rs` wraps HTTP calls and retry behaviour.
- `src/blockscout.rs` and `src/nearblocks.rs` implement provider fetchers.
- `src/models/` contains deserialisation models for provider JSON.
