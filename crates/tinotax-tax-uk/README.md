# tinotax-tax-uk

UK cryptoasset tax calculation crate.

## Owns

- UK tax-year parsing.
- Same-day matching.
- 30-day matching.
- Section 104 pooling.
- Income at receipt and later CGT basis.
- Tax report CSV and summary generation.

## Does Not Own

- Raw ingestion.
- Human review workflow.
- Price fetching.

## Invariants

- Consume reviewed, priced ledger events only.
- Unknown/unresolved rows should block calculation unless explicitly allowed.
- Every disposal should show its matching and pool working.
