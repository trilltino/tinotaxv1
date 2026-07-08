# TinoTax

**A reviewed-ledger UK crypto tax engine — Rust core, CLI, and a Tauri v2
desktop app.**

TinoTax turns wallet and exchange history into an HMRC-ready UK Capital
Gains Tax / income evidence pack, without ever guessing on your behalf.
Every transaction is surfaced for human review, every human decision is
recorded as an append-only override, and the tax calculation refuses to run
on anything it can't classify or price.

```text
raw data → normalised events → review ALL of it → apply edits
        → reviewed ledger → GBP-priced ledger → UK CGT + income → HMRC pack
```

## Contents

- [Why TinoTax](#why-tinotax)
- [What it does / doesn't do](#what-it-does)
- [Quick start](#quick-start)
- [Full command flow](#full-command-flow)
- [Desktop app](#desktop-app)
- [Project layout & outputs](#project-layout--outputs)
- [Review workflow](#review-workflow)
- [Tax calculation](#tax-calculation)
- [Evidence pack](#evidence-pack)
- [Workspace / crates](#workspace--crates)
- [Development](#development)
- [Docs](#docs)

## Why TinoTax

Most crypto tax tools force a choice between "trust the black box" and
"build your own spreadsheet from scratch." TinoTax takes a third path:

- **Nothing is silently classified.** Every row the engine is unsure about
  is flagged `needs_review` and blocks the tax calculation until a human
  (you, or your accountant) makes a call.
- **Raw data never changes.** Wallet API responses and CEX CSV exports are
  copied into an immutable, BLAKE3-hashed cache the moment they're fetched.
  Fixing a mistake means adding an override, never editing history.
- **Everything downstream is re-derivable.** Delete `staging/` or `out/` and
  rebuild it from the raw cache + your review overrides — the pipeline is a
  pure function of (raw data, human decisions).
- **The tax engine is auditable.** `tinotax-tax-uk` implements HMRC's
  Cryptoassets Manual share-matching rules (same-day → 30-day "bed and
  breakfast" → Section 104 pool) as a deterministic function with full unit
  test coverage — see [docs/uk_tax_engine.md](docs/uk_tax_engine.md).

## What it does

- Fetches wallet history for NEAR and any Blockscout-compatible EVM chain
  into the immutable raw cache; imports CEX CSV exports (Binance, Coinbase,
  Kraken, Awaken-style, or a generic column-mapped format) the same way.
- Normalises everything into one canonical event model, then exports
  **every single transaction** to a reviewable CSV — not just the uncertain
  ones — so nothing is hidden from review.
- Applies human corrections as append-only overrides and rebuilds a
  reviewed tax ledger from events + overrides, never by editing source data.
- Values the ledger in GBP using manual prices, prices captured directly
  from exchange exports, or CoinGecko, with full price provenance and a
  confidence rating on every value.
- Calculates UK CGT (same-day / 30-day / Section 104 pool) and income at
  receipt, per tax year, with opening-pool support for holdings acquired
  before your data window.
- Assembles a Self Assessment / HMRC evidence pack mapped to HMRC's
  standard 13 cryptoasset questions, including raw-data hashes, a full
  pricing audit trail, and the complete review change log.

### What it doesn't do

- It is not tax advice — review the output with an accountant.
- It does not classify ambiguous DeFi activity for you; uncertain rows wait
  for a human decision instead of a best guess.
- It does not file anything with HMRC.

## Quick start

> ⚠️ **Never commit client data.** Real `wallets.toml` files, CEX exports,
> imported raw API responses, tax outputs, and evidence packs contain
> private financial data. Keep them in gitignored project folders only —
> `wallets.toml` and typical project directories are already gitignored.

```bash
cp wallets.example.toml wallets.toml   # fill in real wallet addresses
just doctor                            # config + provider connectivity checks
just startup-demo                      # demo ingestion with --resume
```

See [docs/installation.md](docs/installation.md) for prerequisites, API
keys, building the CLI, and installing the desktop app from scratch.

## Full command flow

```bash
just init
just project-status
just project-paths
just fetch-resume
just import-cex
just normalise
just diagnose
just readiness

just review-export
# … edit out/review_all_transactions.csv in a spreadsheet …
just review-apply ./fox-project/out/review_all_transactions_edited.csv

just ledger-build
just prices-missing
export COINGECKO_API_KEY=your_key   # PowerShell: $env:COINGECKO_API_KEY="your_key"
just prices-import ./manual_prices.csv
just prices-fetch        # optional, defaults to coingecko
just ledger-price

just calculate 2024-2025
just pack 2024-2025
just readiness
```

The same flow can be driven from grouped project helpers:

```bash
tinotax project workflow startup --config wallets.toml --project ./fox-project --resume
tinotax project workflow refresh-review --project ./fox-project
tinotax project workflow finalize-year --project ./fox-project --tax-year 2024-2025
tinotax project clean --project ./fox-project --target logs
tinotax project clean --project ./fox-project --target logs --confirm
```

Full reference of every subcommand, flags, and exit/refusal behaviour is in
[docs/commands.md](docs/commands.md).

## Desktop app

`apps/desktop/` is a Windows-first Tauri v2 cockpit over local projects. It
keeps the CLI as the canonical automation surface, but calls the same Rust
`tinotax-app` orchestration directly instead of shelling out — same
pipeline, same guarantees, a UI on top.

```bash
just dev            # run the desktop app in dev mode
just desktop-test    # local frontend/unit checks
just e2e             # seeded WebDriver end-to-end flow
```

The app opens local projects, loads wallets from the selected config,
drives the Lisk Blockscout API sync path, prepares and edits review rows
(append-only overrides), imports CEX CSVs per wallet, shows a per-wallet
data dashboard (monthly activity, per-asset GBP movement, pricing coverage,
review progress, tax-year headline numbers), and exports HMRC questionnaire
responses. GBP pricing is driven by CoinGecko in the project pipeline.
Additional chain API buttons stay disabled until those providers are signed
off for the desktop flow.

## Project layout & outputs

```text
fox-project/
├── project.toml            the config the project was created from
├── questionnaire.toml      HMRC questions the data can't answer (human input)
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

Full class-by-class data model (what's raw, what's derived, what's human
input, and what may ever be edited) is in
[docs/data_model.md](docs/data_model.md).

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
human change is an override; the ledger is always re-derivable.** See
[docs/review_workflow.md](docs/review_workflow.md).

## Tax calculation

`calculate uk` consumes the priced ledger and refuses to run if any
disposal/income row is unclassified or unpriced (use `--allow-unpriced` to
exclude-and-report instead). Disposals exceeding the available pool always
fail with instructions to add `opening_pools.toml` or fix the data. The
engine implements HMRC's same-day → 30-day → Section 104 pool matching
order plus income-at-receipt, per tax year. Outputs land in `tax/<year>/`
— see [docs/uk_tax_engine.md](docs/uk_tax_engine.md) for the full method,
matching order, and test coverage.

## Evidence pack

`pack hmrc` copies the year's calculations and provenance into
`evidence_pack/<year>/`, generates `hmrc_questions_draft.md` against HMRC's
standard 13 questions, and creates `questionnaire.toml` for the answers only
a human has (source of funds, employment, forks, compensation). Fill it in
and re-run. See [docs/hmrc_evidence_pack.md](docs/hmrc_evidence_pack.md).

## Workspace / crates

The workspace is organised into five layers with a strict one-way
dependency direction: `tinotax-cli` → `tinotax-app` → pipeline crates →
`tinotax-core` (+ `tinotax-store` for IO). `tinotax-core` has no HTTP, CSV,
or CLI dependencies, so the engine can power a GUI or service unchanged.

| Layer | Crate | Responsibility |
|---|---|---|
| foundation | `tinotax-core` | Pure domain types — events, tax ledger, prices, dates |
| foundation | `tinotax-config` | `wallets.toml` / `project.toml` (+ `[[cex_csvs]]`) |
| foundation | `tinotax-store` | Project folders, raw cache, hashing, JSONL, manifests |
| ingest | `tinotax-connectors` | Blockscout + NearBlocks fetchers (resumable) |
| ingest | `tinotax-cex` | CEX CSV importers (Binance/Coinbase/Kraken/Awaken/generic) |
| ingest | `tinotax-normalise` | Raw JSON → `NormalisedEvent` (EVM + NEAR) |
| review | `tinotax-diagnostics` | Data quality / completeness reports |
| review | `tinotax-review` | export-all / export-uncertain / apply, override log |
| review | `tinotax-ledger` | Normalised events + overrides → reviewed ledger |
| valuation | `tinotax-pricing` | Price book, manual import, CoinGecko fetch, valuation |
| valuation | `tinotax-tax-uk` | Same-day / 30-day / S104 engine + income + reports |
| output | `tinotax-report` | Normalised transactions CSV + audit manifest |
| output | `tinotax-evidence` | HMRC evidence pack generator |
| interface | `tinotax-app` | Orchestration — the only crate the CLI/desktop app calls |
| interface | `tinotax-cli` | Thin clap binary |

See [crates/README.md](crates/README.md) and
[docs/architecture.md](docs/architecture.md) for the full architecture,
principles, and dependency graph.

## Development

```bash
just fmt
just policy-scan
just clippy
just test
just doc
```

Use `just --list` to see all terminal startup and development shortcuts. On
Windows the recipes fall back to `%USERPROFILE%\.cargo\bin\cargo.exe`; set
`CARGO` if you need a different cargo executable.

`just production-check` is the full local gate. It forbids first-party unsafe
Rust, unchecked `.unwrap()`/`.expect()`/`.unwrap_err()`/`.expect_err()`, and
direct panic-style shortcuts before running fmt, clippy, check, tests, docs,
audit, and deny. The deny gate is Windows-targeted for the v1 desktop/CLI
release surface; see [docs/dependency-policy.md](docs/dependency-policy.md)
for accepted license exceptions and tracked Tauri transitive advisories.

## Docs

[Installation](docs/installation.md) ·
[Architecture](docs/architecture.md) ·
[Commands](docs/commands.md) ·
[Data model](docs/data_model.md) ·
[Review workflow](docs/review_workflow.md) ·
[CEX imports](docs/cex_imports.md) ·
[Pricing](docs/pricing.md) ·
[UK tax engine](docs/uk_tax_engine.md) ·
[Evidence pack](docs/hmrc_evidence_pack.md) ·
[Analysis & visuals](docs/analysis_and_visuals.md) ·
[Production gate](docs/production_gate.md) ·
[Rust readiness checklist](docs/RUST_PRODUCTION_READINESS_CHECKLIST.md) ·
[Assumptions & limitations](docs/assumptions_and_limitations.md) ·
[Accountant review](docs/accountant_review.md) ·
[Go-live runbook](docs/go-live-runbook.md) ·
[Runbook](docs/runbook.md) ·
[Release process](docs/release.md) ·
[Security](SECURITY.md) ·
[Contributing](CONTRIBUTING.md)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option.
