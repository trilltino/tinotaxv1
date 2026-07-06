# tinotax-cex

Centralised exchange CSV ingestion crate.

## Owns

- Parsing exchange export CSVs.
- Normalising rows into shared event records.
- Copying and hashing source CSVs as raw evidence.
- Writing CEX import diagnostics.

## Does Not Own

- Wallet API fetching.
- Ledger pricing or tax classification beyond safe import suggestions.

## Key Files

- `src/importer.rs` orchestrates configured imports.
- `src/record.rs` defines intermediate CEX row records.
- Platform modules parse Binance, Coinbase, Kraken, Awaken-style, and generic
  CSVs.
