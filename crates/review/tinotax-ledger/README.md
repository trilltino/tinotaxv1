# tinotax-ledger

Reviewed ledger construction crate.

## Owns

- Combining normalised events with latest review overrides.
- Producing reviewed `TaxLedgerEvent` rows.
- Exporting reviewed ledger CSV files.

## Does Not Own

- Spreadsheet review UI/export details.
- GBP pricing.
- UK tax calculation.

## Invariants

- Ledger IDs should be deterministic.
- Raw and normalised inputs are never mutated.
- User review decisions override machine suggestions where valid.
