# Review workflow

The client brief in one line: *review all the data, change what's wrong,
without ever corrupting the evidence.*

## The rule

```text
Never mutate raw data.
Never mutate normalised_events.jsonl.
All human changes become review_overrides.jsonl (append-only).
reviewed_ledger.jsonl is derived from normalised_events + review_overrides.
```

## Round trip

1. **Export.** `review export-all` writes `out/review_all_transactions.csv`:
   every event, its detected type/direction, confidence, review reasons, a
   `suggested_tax_type`, and empty `user_*` columns. Existing decisions are
   pre-filled, so the file always shows the current review state.
   (`review export-uncertain` gives the short flagged-only file.)
2. **Edit** in any spreadsheet. Editable columns:
   - `user_tax_type` — the precise classification (see
     [data_model.md](data_model.md) for the vocabulary)
   - `user_asset_symbol`, `user_quantity` — corrections
   - `user_proceeds_gbp`, `user_cost_gbp`, `user_income_gbp`, `user_fee_gbp`
     — GBP values you know better than any price feed
   - `user_price_source`, `user_note`
   Leave a row untouched to accept the machine's suggestion.
3. **Apply.** `review apply --file <edited.csv>` validates every filled
   cell (unknown types, bad numbers, unknown event ids → the whole run
   fails, nothing is half-recorded), appends accepted decisions to the
   override log, and regenerates `out/change_log.csv`.
4. **Rebuild.** `ledger build` re-derives the reviewed ledger. Precedence
   per row: `user_tax_type` > `user_action` > machine suggestion.

## Review status on every ledger row

- `auto` — machine classification, was never flagged
- `needs_review` — flagged, no human decision yet (warns in tax calc)
- `reviewed` — a human decision was applied

## Auditability

- `staging/review_overrides.jsonl` — every decision ever, timestamped
- `out/change_log.csv` — the same history for humans
- `evidence_pack/<year>/manual_review_decisions.csv` — latest per event
