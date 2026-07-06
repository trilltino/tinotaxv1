# tinotax-cex/src

Source modules for CEX import handling.

## Module Map

- `binance.rs`, `coinbase.rs`, `kraken.rs`, and `awaken.rs` parse known export
  shapes.
- `generic_csv.rs` parses user-mapped CSV files.
- `column_mapping.rs` validates generic mappings.
- `record.rs` stores exchange-neutral intermediate rows.
- `importer.rs` copies raw CSVs, hashes them, and writes normalised events.
- `report.rs` writes import diagnostics.

## Invariants

- Preserve original CSV row numbers when possible.
- Never edit source CSVs in place.
- Unknown columns or ambiguous rows should be reported, not guessed away.
