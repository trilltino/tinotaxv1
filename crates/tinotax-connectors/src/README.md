# tinotax-connectors/src

Connector implementation modules.

## Invariants

- Fetchers persist raw pages before any downstream interpretation.
- Pagination state must be resumable.
- HTTP clients should be polite to public APIs and explicit about retries.
- API keys should be optional and read outside committed config values.
