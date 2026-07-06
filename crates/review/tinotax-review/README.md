# tinotax-review

Human review workflow crate.

## Owns

- Exporting all normalised events for spreadsheet review.
- Exporting uncertain/manual-review rows.
- Validating edited review CSVs.
- Appending review overrides.
- Loading current review state.

## Does Not Own

- Raw source data.
- Ledger pricing.
- UK tax calculations.

## Invariants

- Review changes are append-only.
- Bad edits should fail with row-level context.
- Raw and normalised records are never rewritten by review commands.
