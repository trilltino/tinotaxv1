# Analysis CSV And Visuals

`report` writes `out/analysis_export.csv`: one wide, analysis-ready CSV for
spreadsheet calculations, BI tools, and visual dashboards.

It joins:

- raw/normalised event context;
- review status and tax classification;
- ledger identifiers;
- GBP pricing fields when `ledger price` has run;
- source evidence pointers.

## Recommended Visuals

Use these industry-standard patterns:

- **Monthly activity bars:** count of events by month, split by `visual_bucket`.
- **Asset flow stacked bars:** signed quantity by `asset_symbol` and month.
- **GBP taxable value timeline:** `gross_value_gbp` by month for taxable rows.
- **Tax classification breakdown:** count or GBP value by `tax_event_type`.
- **Review backlog:** rows where `review_status = needs_review` or
  `tax_event_type = unknown`, grouped by `review_reasons`.
- **Pricing coverage:** count of rows by `price_confidence` and `price_source`.
- **Evidence coverage:** count of rows by `source_id`, `chain`, and wallet.

Useful pivot fields:

- `date`, `month`, `year`, `tax_year`
- `asset_symbol`
- `tax_event_type`
- `activity_class`
- `visual_bucket`
- `gross_value_gbp`
- `signed_quantity`
- `review_status`
- `price_confidence`
- `price_source`

## Why This Exists

The review CSV is for editing. The priced ledger is for tax calculation.
`analysis_export.csv` is for analysis: it keeps the important fields from both
worlds in one flat table so charts and pivots do not need to join JSONL files
or cross-reference multiple CSVs manually.
