# Architecture

## Pipeline

```text
raw data → normalised events → review all data → apply user/accountant edits
        → reviewed tax ledger → GBP-priced ledger → UK CGT + income engine
        → Self Assessment / HMRC evidence pack
```

Each arrow is a derivation: later stages can always be deleted and rebuilt
from earlier ones. Only two things are ever authored by humans — review
overrides and price imports — and both are recorded, never applied by
editing upstream files.

## Principles

1. **Raw data is immutable.** API pages and CEX CSVs are copied into `raw/`,
   hashed (BLAKE3), and never overwritten. A changed re-import under the
   same id is an error.
2. **Everything else is derived.** `staging/` and `out/` are regenerable.
3. **Every output row traces to evidence** — file, page/row, tx hash.
4. **Don't guess.** Uncertain classification → `needs_review`; missing GBP →
   `missing`; the tax engine refuses rather than assumes.
5. **No `f64` for money.** `rust_decimal::Decimal` for human amounts; raw
   integer strings preserved for chain amounts.
6. **Pure tax logic.** `tinotax-tax-uk::calculate` is a deterministic
   function of (events, opening pools, tax year); IO lives at the edges.

## Workspace

```text
crates/
├── tinotax-core         pure domain types (events, tax ledger, prices, dates)
├── tinotax-config       wallets.toml / project.toml (+ [[cex_csvs]])
├── tinotax-connectors   Blockscout + NearBlocks fetchers (resumable)
├── tinotax-store        project folders, raw cache, hashing, JSONL, manifests
├── tinotax-normalise    raw JSON → NormalisedEvent (EVM + NEAR)
├── tinotax-diagnostics  data quality/completeness reports
├── tinotax-review       export-all / export-uncertain / apply, override log
├── tinotax-ledger       normalised events + overrides → reviewed ledger
├── tinotax-cex          CEX CSV importers (binance/coinbase/kraken/awaken/generic)
├── tinotax-pricing      price book, manual import, CoinGecko fetch, valuation
├── tinotax-tax-uk       same-day / 30-day / S104 engine + income + reports
├── tinotax-report       normalised transactions CSV + audit manifest
├── tinotax-evidence     HMRC evidence pack generator
├── tinotax-app          orchestration — the only crate the CLI calls
└── tinotax-cli          thin clap binary
```

Dependency direction is one-way:

```text
tinotax-cli → tinotax-app
            → {connectors, cex, normalise, review, ledger, pricing, tax-uk, evidence, report}
            → tinotax-core (+ tinotax-store for IO)
```

Within the pipeline crates: `ledger` depends on `review` (it consumes the
override log), `pricing` depends on `ledger` (it values the reviewed
ledger), `evidence` depends on `review` + `ledger`. `tinotax-core` has no
HTTP, CSV or CLI dependencies, so the engine can later power a GUI or
service unchanged.
