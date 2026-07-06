# tinotax-store

Project storage and provenance crate.

## Owns

- Project folder paths.
- Immutable raw page cache writes.
- JSONL readers and writers.
- Raw manifest records and file hashing.

## Does Not Own

- Provider API calls.
- Tax classification.
- Pricing or evidence-pack interpretation.

## Invariants

- Raw pages are append-only and should not be overwritten.
- Project paths should be derived in one place.
- Hashes should be stable and auditable.
