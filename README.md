# TinoTax

Reviewed-ledger UK crypto tax CLI: fetch wallet and exchange history, review
**all** of it in a spreadsheet, apply corrections without ever mutating the
source data, value everything in GBP, calculate UK CGT (same-day / 30-day /
Section 104) plus income, and produce a Self Assessment / HMRC evidence pack.

```text
raw data → normalised events → review all data → apply edits
        → reviewed ledger → GBP-priced ledger → UK CGT + income → HMRC pack
```

## What TinoTax does

- Fetches wallet history (NEAR, Blockscout-compatible EVM chains) into an
  immutable, hashed raw cache; imports CEX CSV exports (Binance, Coinbase,
  Kraken, Awaken-style, generic) the same way.
- Exports **every** transaction to one reviewable CSV; human edits are
  recorded as append-only overrides — raw and normalised data are never
  changed.
- Builds a reviewed tax ledger, values it in GBP (manual prices, prices
  captured from exchange exports, or CoinGecko), and refuses to calculate
  tax on unresolved rows instead of guessing.
- Calculates UK CGT per HMRC's Cryptoassets Manual (same-day → 30-day →
  Section 104) and income at receipt, per tax year, with opening-pool
  support for pre-history holdings.
- Assembles an evidence pack mapped to HMRC's standard 13 cryptoasset
  questions, including raw-data hashes, a pricing audit, and the full
  review change log.

## What TinoTax does not do

- It is not tax advice, and its output should be reviewed by an accountant.
- It does not classify ambiguous DeFi activity for you — uncertain rows are
  flagged and wait for a human decision.
- It does not file anything with HMRC.

## Quick start

```bash
cp wallets.example.toml wallets.toml   # then fill in real addresses (gitignored)
cargo run -p tinotax-cli -- doctor     # config + provider connectivity checks
cargo run -p tinotax-cli -- demo --config wallets.toml --out ./demo-data
```

## Full command flow

```bash
tinotax project init --config wallets.toml --out ./fox-project
tinotax fetch --project ./fox-project --resume
tinotax import-cex --project ./fox-project
tinotax normalise --project ./fox-project
tinotax diagnose --project ./fox-project

tinotax review export-all --project ./fox-project
# … edit out/review_all_transactions.csv in a spreadsheet …
tinotax review apply --project ./fox-project \
  --file ./fox-project/out/review_all_transactions_edited.csv

tinotax ledger build --project ./fox-project
tinotax prices missing --project ./fox-project
tinotax prices import --project ./fox-project --file ./manual_prices.csv
tinotax prices fetch --project ./fox-project --provider coingecko   # optional
tinotax ledger price --project ./fox-project

tinotax calculate uk --project ./fox-project --tax-year 2024-2025
tinotax pack hmrc --project ./fox-project --tax-year 2024-2025
```

## Outputs

```text
fox-project/
├── project.toml            the config the project was created from
├── questionnaire.toml      HMRC questions data can't answer (human input)
├── opening_pools.toml      holdings acquired before the data window (optional)
├── raw/                    evidence — hashed, never edited
├── staging/                machine-derived intermediates (JSONL, regenerable)
├── out/                    reviewable working files (CSV/JSON)
├── tax/<year>/             disposals, pool movements, income, SA summary
└── evidence_pack/<year>/   the client/accountant deliverable
```

Key files: `out/review_all_transactions.csv` (the editable review surface),
`out/reviewed_ledger.csv`, `out/priced_ledger.csv` + `out/pricing_audit.csv`,
`tax/<year>/disposals_calculation.csv`, and
`evidence_pack/<year>/hmrc_questions_draft.md`.

## Review workflow

1. `review export-all` writes every event with the machine's suggested tax
   type and empty `user_*` columns.
2. Edit in any spreadsheet: set `user_tax_type` (e.g. `transfer_in`,
   `swap_disposal`, `staking_reward`, `ignore`), correct quantities or GBP
   values, add notes.
3. `review apply` validates every edit and appends it to
   `staging/review_overrides.jsonl`; `out/change_log.csv` is the full
   history. Re-export any time — current decisions are pre-filled.
4. `ledger build` re-derives the reviewed ledger from events + overrides.

The invariant: **raw data and normalised events are never mutated; every
human change is an override; the ledger is always re-derivable.**

## Tax calculation workflow

`calculate uk` consumes the priced ledger and refuses to run if any
disposal/income row is unclassified or unpriced (use `--allow-unpriced` to
exclude-and-report instead). Disposals exceeding the available pool always
fail with instructions to add `opening_pools.toml` or fix the data.
Outputs land in `tax/<year>/`; see
[docs/uk_tax_engine.md](docs/uk_tax_engine.md) for the method.

## Evidence pack workflow

`pack hmrc` copies the year's calculations and provenance into
`evidence_pack/<year>/`, generates `hmrc_questions_draft.md` against HMRC's
standard 13 questions, and creates `questionnaire.toml` for the answers only
a human has (source of funds, employment, forks, compensation). Fill it in
and re-run.

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

Docs: [architecture](docs/architecture.md) · [commands](docs/commands.md) ·
[data model](docs/data_model.md) · [review workflow](docs/review_workflow.md) ·
[CEX imports](docs/cex_imports.md) · [pricing](docs/pricing.md) ·
[UK tax engine](docs/uk_tax_engine.md) ·
[evidence pack](docs/hmrc_evidence_pack.md) ·
[assumptions](docs/assumptions_and_limitations.md) ·
[accountant review](docs/accountant_review.md)
