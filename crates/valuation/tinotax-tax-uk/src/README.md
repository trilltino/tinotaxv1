# tinotax-tax-uk/src

UK tax engine implementation modules.

## Module Map

- `tax_year.rs` parses and compares UK tax years.
- `validation.rs` rejects unresolved or unsafe input rows.
- `matching.rs` coordinates same-day, 30-day, and pool matching.
- `same_day.rs` handles same-day acquisition matching.
- `thirty_day.rs` handles 30-day acquisition matching.
- `s104_pool.rs` maintains Section 104 pooled cost state.
- `income.rs` summarises income receipts.
- `fees.rs` handles fee-only disposals.
- `disposals.rs` identifies disposal event types.
- `reports.rs` writes accountant-facing CSV outputs.
- `domain.rs` contains calculation structs.
- `lib.rs` exposes the calculation API and regression tests.
