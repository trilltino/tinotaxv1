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
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [API keys](#api-keys-environment-variables)
- [Config file (wallets.toml)](#config-file-walletstoml)
- [Quick start](#quick-start)
- [Full command reference](#full-command-reference)
- [Manual step-by-step run](#manual-step-by-step-run)
- [Troubleshooting](#troubleshooting)
- [Desktop app](#desktop-app)
- [Project layout & outputs](#project-layout--outputs)
- [Review workflow](#review-workflow)
- [Pricing](#pricing)
- [CEX imports](#cex-imports)
- [Tax calculation](#tax-calculation)
- [Evidence pack](#evidence-pack)
- [Workspace / crates](#workspace--crates)
- [Development](#development)
- [Contributing](#contributing)
- [Docs](#docs)
- [License](#license)

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
- **No `f64` for money, anywhere.** Every GBP and asset amount is
  `rust_decimal::Decimal`; raw integer chain amounts are preserved
  alongside human-readable quantities.
- **Unsafe Rust is forbidden workspace-wide** (`unsafe_code = "forbid"`),
  and unchecked `.unwrap()`/`.expect()`/panics are denied by lint and
  enforced by `just policy-scan` before every merge.

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
- Ships fail-fast diagnostics at every stage: `doctor` (environment/provider
  reachability), `preflight` (config sanity before a run starts), and
  `readiness` (evidence integrity and delivery-risk gate) so problems
  surface before they cost you a re-run.

### What it doesn't do

- It is not tax advice — review the output with an accountant.
- It does not classify ambiguous DeFi activity for you; uncertain rows wait
  for a human decision instead of a best guess.
- It does not file anything with HMRC.

## Prerequisites

- **Rust toolchain (stable) with cargo.** Install from
  [rustup.rs](https://rustup.rs) — this repo pins a toolchain via
  `rust-toolchain.toml`, so `rustup` fetches the right version
  automatically.
- **Git.**
- **Internet access** (for fetching wallet history and prices).
- Windows, macOS, or Linux.
- For the desktop app only: **Node.js + npm** (LTS), and on Windows the
  "Desktop development with C++" workload from the Visual Studio Build
  Tools installer (needed to compile the Tauri Rust binary); see
  [Desktop app](#desktop-app).

Verify Rust is installed:

```bash
rustc --version
cargo --version
```

## Installation

```bash
git clone https://github.com/trilltino/tinotax
cd tinotax
```

The CLI binary is named `tinotax` and lives in the `tinotax-cli` crate.
Three ways to run it:

**A — build a release binary (recommended):**

```bash
cargo build --release -p tinotax-cli
```

The binary is then at `target/release/tinotax` (Linux/macOS) or
`target\release\tinotax.exe` (Windows).

**B — install it onto your PATH:**

```bash
cargo install --path crates/interface/tinotax-cli
```

**C — run without installing, via cargo:**

```bash
cargo run -q -p tinotax-cli -- <command> [args...]
```

Everything after `--` is passed to the CLI, e.g.
`cargo run -q -p tinotax-cli -- doctor`.

> Throughout this document and the docs, commands are written as
> `tinotax <command>`. If you didn't `cargo install`, substitute
> `cargo run -q -p tinotax-cli -- <command>`, or use the `just` recipes
> below, which do this for you.

## API keys (environment variables)

The tool works without keys but is slow/limited. For real runs set:

| Variable | Purpose |
|---|---|
| `NEARBLOCKS_API_KEY` | Required for NEAR wallets on a paid plan. Without it, NEAR fetching falls back to the slow anonymous tier. |
| `NEARBLOCKS_CREDITS_PER_MINUTE` | Optional rate override for the NearBlocks plan. |
| `COINGECKO_API_KEY` | CoinGecko demo/public paid key (price fetching). |
| `COINGECKO_DEMO_API_KEY` | CoinGecko demo key (alternative to the above). |
| `COINGECKO_PRO_API_KEY` | CoinGecko Pro key (full history, preferred if set). |
| `RUST_LOG` | Optional logging control, defaults to `tinotax=info,warn` (e.g. `tinotax=debug`). |

PowerShell:

```powershell
$env:NEARBLOCKS_API_KEY = "your-key"
$env:COINGECKO_API_KEY  = "your-key"
```

bash/zsh:

```bash
export NEARBLOCKS_API_KEY="your-key"
export COINGECKO_API_KEY="your-key"
```

## Config file (wallets.toml)

A project is created from a wallet/source config file. Copy the example and
fill in real addresses — `wallets.toml` is gitignored, so client addresses
never go into git:

```bash
cp wallets.example.toml wallets.toml          # Linux/macOS
Copy-Item wallets.example.toml wallets.toml   # PowerShell
```

The config declares:

- `[project]` — name, `base_currency` (GBP), `period_start`, `period_end`
- `[[wallets]]` — one block per wallet: id, name, chain, address, provider
- `[[cex_csvs]]` — optional CEX CSV exports to import (`binance`/`coinbase`/
  `kraken`/`awaken`/`generic`, with column mapping for `generic`)
- `[providers.*]` — provider definitions (Blockscout / NearBlocks base URLs)

See `wallets.example.toml` in the repo root for a fully commented template.

## Quick start

> ⚠️ **Never commit client data.** Real `wallets.toml` files, CEX exports,
> imported raw API responses, tax outputs, and evidence packs contain
> private financial data. Keep them in gitignored project folders only —
> `wallets.toml` and typical project directories are already gitignored.

The fastest path uses the `prepare` workflow, which chains fetch →
normalise → auto-classify → build → (price fetch) → price → calculate:

```bash
just doctor                            # config + provider connectivity checks

tinotax project workflow prepare \
    --config ./wallets.toml \
    --project ./fox-project \
    --tax-year 2024-2025 \
    --fetch-prices

tinotax pack hmrc --project ./fox-project --tax-year 2024-2025
```

Or run a scripted demo of the whole ingestion pipeline in one shot:

```bash
just startup-demo                      # demo ingestion with --resume
```

See [docs/installation.md](docs/installation.md) for the complete
installation and command walkthrough, including the desktop app.

## Full command reference

During development, prefer `just` recipes — see `just --list` for the full
set. They wrap `cargo run -p tinotax-cli -- <command>`; the installed
binary is named `tinotax`.

| Command | What it does |
|---|---|
| `doctor` | Config, environment and provider reachability checks |
| `preflight --config <toml> --project <dir>` | Fail-fast startup checks for config, inputs, project path and required API keys |
| `demo --config <toml> --out <dir> [--resume]` | init → fetch → normalise → diagnose → review exports → reports |
| `project init --config <toml> --out <dir>` | Create a project folder from a config |
| `project status --project <dir>` | Summarise source counts, project folders, and human/audit state |
| `project paths --project <dir> [--tax-year 2024-2025]` | Print canonical project paths as `key=value` lines |
| `project clean --project <dir> --target logs [--confirm]` | Dry-run-first cleanup of generated project artifacts |
| `project workflow startup --config <toml> --project <dir> [--resume]` | preflight → init → fetch → import → normalise → diagnose → review exports → reports → readiness |
| `project workflow refresh-review --project <dir>` | Rebuild normalised data, diagnostics, review exports, reports, readiness |
| `project workflow finalize-year --project <dir> --tax-year 2024-2025 [--allow-unpriced]` | ledger build → missing prices → price ledger → calculate → pack → readiness |
| `project workflow prepare --config <toml> --project <dir> [--wallet <id>]... --tax-year <label> [--resume] [--fetch-prices]` | One-click: fetch → normalise → auto-ignore contract calls → build → (price fetch) → price → calculate |
| `fetch --project <dir> [--resume] [--wallet <id>]...` | Fetch wallet history into the raw cache |
| `import-cex --project <dir>` | Import `[[cex_csvs]]` exports (immutable copy + hash + normalise) |
| `normalise --project <dir>` | Raw wallet pages → `staging/normalised_events.jsonl` |
| `diagnose --project <dir>` | `out/diagnostics.json`, activity summaries |
| `readiness --project <dir>` | Evidence integrity, cursor completion and delivery-risk gate |
| `report --project <dir>` | `out/normalised_transactions.csv` + `out/audit_manifest.json` |
| `review export-all --project <dir>` | Every event → `out/review_all_transactions.csv` |
| `review export-uncertain --project <dir>` | Flagged events only → `out/manual_review.csv` |
| `review apply --project <dir> --file <csv>` | Validate + record the edited CSV as overrides |
| `review auto-classify --project <dir>` | Bulk-classify zero-value contract calls as `ignore` |
| `ledger build --project <dir>` | Events + overrides → reviewed ledger (JSONL + CSV) |
| `prices missing --project <dir>` | (asset, day) pairs still needing GBP → `out/missing_prices.csv` |
| `prices import --project <dir> --file <csv>` | Manual daily prices → price observations |
| `prices fetch --project <dir> [--provider coingecko]` | Fetch missing daily GBP prices |
| `ledger price --project <dir>` | Value the ledger in GBP + `out/pricing_audit.csv` |
| `calculate uk --project <dir> --tax-year 2024-2025 [--allow-unpriced]` | UK CGT + income → `tax/<year>/` |
| `pack hmrc --project <dir> --tax-year 2024-2025` | Evidence pack → `evidence_pack/<year>/` |

Global flags: `--project <path>`, `--config <path>`, `--tax-year <label>`
(e.g. `2024-2025`), `--resume` (reuse already-fetched raw pages instead of
refetching).

### Exit behaviour worth knowing

- `review apply` fails the whole run on any invalid cell (a typo can't
  silently drop a decision); untouched rows are skipped.
- `calculate uk` refuses when disposal/income rows are unclassified or
  unpriced. `--allow-unpriced` excludes them and reports them in
  `unresolved_tax_items.csv` instead. Disposals exceeding the pool always
  fail, listing every shortfall.
- `import-cex` refuses to overwrite a differing `raw/cex/<id>/original.csv`;
  declare a new export as a new id.
- `readiness` fails on broken or incomplete evidence, such as raw hash
  mismatches, unreadable manifest entries, incomplete fetch cursors, or
  stale output hashes in `out/audit_manifest.json`. It reports rejected raw
  items and normalisation warnings as warnings so they can be signed off
  explicitly.
- `preflight` fails before project startup if the wallet config is missing
  or invalid, declared CEX CSV files are absent, provider URLs are
  malformed, GBP is not the base currency, or a NearBlocks project is
  missing `NEARBLOCKS_API_KEY`.
- `project clean` is a dry run unless `--confirm` is passed. It never
  removes `raw/`, `project.toml`, `questionnaire.toml`,
  `opening_pools.toml`, `staging/review_overrides.jsonl`, or
  `staging/price_observations.jsonl`. `--target all-derived` expands to
  logs, generated staging files, `out/`, tax outputs, and evidence packs;
  `--tax-year` limits tax/evidence cleanup.

See [docs/commands.md](docs/commands.md) and
[docs/installation.md](docs/installation.md) for every flag and full
help-text-level detail.

## Manual step-by-step run

If you'd rather drive each stage yourself instead of `prepare`/`workflow`:

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

## Troubleshooting

- `tinotax doctor` reports which keys are set and whether providers are
  reachable — always start here.
- NEAR fetching is slow / rate-limited: set `NEARBLOCKS_API_KEY` (paid
  plan).
- Many rows unpriced: set a CoinGecko key and re-run `prices fetch`; price
  the remaining unlistable tokens (LP NFTs, obscure tokens) via
  `prices import`.
- `calculate uk` refuses to run: either resolve unpriced rows, or pass
  `--allow-unpriced` to proceed with those rows excluded (and reported).
- Want to reset generated artifacts:
  `tinotax project clean --project <path> --target all-derived` (add
  `--confirm` to actually delete).
- Verbose logs: set `RUST_LOG=tinotax=debug` before running.

## Desktop app

`apps/desktop/` is a Windows-first Tauri v2 cockpit over local projects. It
keeps the CLI as the canonical automation surface, but calls the same Rust
`tinotax-app` orchestration directly instead of shelling out — same
pipeline, same guarantees, a UI on top.

```bash
cd apps/desktop
npm install --no-audit --fund=false   # or: just desktop-install
npm run tauri:dev                     # or: just dev (install + dev in one)
```

`just dev` runs `desktop-install` then `desktop-dev` (`npm run tauri:dev`),
which starts the Vite dev server on `127.0.0.1:1420` and compiles the Rust
`src-tauri` binary (`tinotax-desktop`) into a desktop window pointed at it.

The app opens local projects, loads wallets from the selected config,
drives the Lisk Blockscout API sync path, prepares and edits review rows
(append-only overrides), imports CEX CSVs per wallet, shows a per-wallet
data dashboard (monthly activity, per-asset GBP movement, pricing coverage,
review progress, tax-year headline numbers), and exports HMRC questionnaire
responses. GBP pricing is driven by CoinGecko in the project pipeline.
Additional chain API buttons stay disabled until those providers are signed
off for the desktop flow. Set the same [API key environment
variables](#api-keys-environment-variables) before launching so
price/chain fetching works.

```bash
just desktop-test    # React/Vitest unit tests
just e2e             # hermetic seeded WebdriverIO/Tauri end-to-end flow
```

To build a distributable installer:

```bash
cd apps/desktop
npm run build
npx tauri build      # target/release/bundle/ — e.g. Windows .msi / NSIS .exe
```

Full walkthrough, prerequisites, and installer packaging in
[docs/installation.md § 11](docs/installation.md#11-desktop-app-appsdesktop--tauri-v2-gui)
and the e2e suite structure in
[apps/desktop/e2e/README.md](apps/desktop/e2e/README.md).

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
4. `review auto-classify` optionally bulk-classifies zero-value contract
   calls as `ignore` so review time goes to transactions that matter.
5. `ledger build` re-derives the reviewed ledger from events + overrides.

The invariant: **raw data and normalised events are never mutated; every
human change is an override; the ledger is always re-derivable.** See
[docs/review_workflow.md](docs/review_workflow.md).

## Pricing

UK tax needs GBP values at (or near) each event. TinoTax builds a per-day
**price book** from three observation sources, in precedence order:

1. **Reviewer-typed GBP values** (`user_*_gbp` in the review CSV) — already
   on the ledger row after `ledger build`; never overwritten.
2. **Price observations** (`staging/price_observations.jsonl`) — from
   `prices import` (manual CSV) and `prices fetch` (CoinGecko daily
   history; built-in symbol→id table, unknown symbols reported, gentle
   rate limiting, resumable).
3. **CEX price hints** (`staging/cex_price_hints.jsonl`) — GBP spot prices
   stated in exchange exports, captured during `import-cex`.

The book keeps the best observation per (asset, day): higher confidence
wins, later fetch breaks ties. Lookups try the exact day, then ±1 day
(confidence `medium`), then ±2/±3 days (`low`).

`out/pricing_audit.csv` records every derived value: quantity × price,
which day's observation was used, source and confidence. Tax calculation
cannot run while required GBP values are missing unless `--allow-unpriced`
is passed — nothing is ever silently valued at zero. Full detail in
[docs/pricing.md](docs/pricing.md).

## CEX imports

HMRC's standard request includes *full, unedited trading data files*, so
the original export is the evidence: `import-cex` copies each file to
`raw/cex/<id>/original.csv`, writes a BLAKE3 `original_hash.txt`, and
refuses to overwrite an existing copy with different content.

| Platform | Expected export |
|---|---|
| `binance` | Transaction History: `User_ID,UTC_Time,Account,Operation,Coin,Change,Remark` |
| `coinbase` | Transaction report (preamble tolerated) |
| `kraken` | Ledgers export (legacy codes XXBT/ZGBP translated) |
| `awaken` | Universal sent/received format |
| `generic` | Anything single-row-per-movement, via `[cex_csvs.mapping]` |

Fiat movements are skipped as events (the crypto legs carry the tax story)
and counted in `out/cex_import_diagnostics.csv`; unrecognised operations
import as `unknown` + `needs_review` rather than being dropped. Full
configuration and behaviour in [docs/cex_imports.md](docs/cex_imports.md).

## Tax calculation

`calculate uk` consumes the priced ledger and refuses to run if any
disposal/income row is unclassified or unpriced (use `--allow-unpriced` to
exclude-and-report instead). Disposals exceeding the available pool always
fail with instructions to add `opening_pools.toml` or fix the data. The
engine implements HMRC's same-day → 30-day → Section 104 pool matching
order plus income-at-receipt, per tax year, always processing the full
event history so pool balances are correct at the requested year's
boundaries (only the report is filtered to the year).

Outputs land in `tax/<year>/`:

- `disposals_calculation.csv` — per disposal: quantity, proceeds, matched
  same-day/30-day/S104 quantities and costs, allowable cost, gain/loss,
  source ledger event ids, matching notes
- `s104_pool_movements.csv` / `s104_pool_opening_closing.csv` — pool
  changes and per-asset balances at the boundaries
- `income_summary.csv` — per-receipt income rows by category
- `self_assessment_crypto_summary.csv` — headline figures
- `unresolved_tax_items.csv` — blockers and warnings
- `assumptions_and_limitations.md` — method + assumptions, regenerated per
  run

See [docs/uk_tax_engine.md](docs/uk_tax_engine.md) for the full matching
rules, opening-pool config, and unit test coverage.

## Evidence pack

`pack hmrc` copies the year's calculations and provenance into
`evidence_pack/<year>/`, generates `hmrc_questions_draft.md` mapped to
HMRC's standard 13 cryptoasset questions (activity start date, full CGT
calculations with S104, platforms/exchanges used, full unedited trading
data, forks, airdrops, compensation, employment income, mining/staking,
goods/services spend, source of funds), and creates `questionnaire.toml`
for the answers only a human has. Fill it in and re-run — everything in the
pack is a copy, regenerated in full each time. See
[docs/hmrc_evidence_pack.md](docs/hmrc_evidence_pack.md) for the complete
question-by-question mapping.

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

Within the pipeline crates: `ledger` depends on `review` (it consumes the
override log), `pricing` depends on `ledger` (it values the reviewed
ledger), `evidence` depends on `review` + `ledger`. See
[crates/README.md](crates/README.md) and
[docs/architecture.md](docs/architecture.md) for the full architecture,
principles, and dependency graph.

## Development

```bash
just fmt              # format the workspace
just policy-scan      # forbid unwrap/expect/panic!/unsafe in first-party code
just clippy           # clippy --workspace --all-targets --all-features -D warnings
just test             # cargo test --workspace --all-targets --all-features
just doc              # cargo doc --workspace --no-deps
just check            # metadata + policy-scan + fmt-check + clippy + check-build + test + doc
```

Use `just --list` to see all terminal startup and development shortcuts. On
Windows the recipes fall back to `%USERPROFILE%\.cargo\bin\cargo.exe`; set
`CARGO` if you need a different cargo executable.

`just production-check` (`check` + `audit` + `deny`) is the full local gate.
It forbids first-party unsafe Rust, unchecked
`.unwrap()`/`.expect()`/`.unwrap_err()`/`.expect_err()`, and direct
panic-style shortcuts (`panic!`/`todo!`/`unimplemented!`/`unreachable!`)
before running fmt, clippy, check, tests, docs, `cargo-audit`, and
`cargo-deny`. Install `cargo-audit`/`cargo-deny` first if either is
reported missing. The deny gate is Windows-targeted for the v1 desktop/CLI
release surface; see [docs/dependency-policy.md](docs/dependency-policy.md)
for accepted license exceptions and tracked Tauri transitive advisories.

## Contributing

TinoTax is designed around reviewed, immutable source data. Changes should
preserve these boundaries:

- CLI code parses arguments only.
- Connector code fetches source data only.
- Raw data is append-only and must not be silently overwritten.
- Review code records human decisions as overrides.
- Pricing code records source, confidence, and audit information.
- UK tax code consumes reviewed, priced ledger events only.

Run `just check` before opening a pull request. Never commit live client
data, wallet addresses, exchange exports, tax outputs, evidence packs,
`.env` files, or API keys. See [CONTRIBUTING.md](CONTRIBUTING.md) and
[SECURITY.md](SECURITY.md).

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
[Production readiness](docs/PRODUCTION_READINESS.md) ·
[Rust readiness checklist](docs/RUST_PRODUCTION_READINESS_CHECKLIST.md) ·
[Assumptions & limitations](docs/assumptions_and_limitations.md) ·
[Accountant review](docs/accountant_review.md) ·
[Go-live runbook](docs/go-live-runbook.md) ·
[Runbook](docs/runbook.md) ·
[Release process](docs/release.md) ·
[Dependency policy](docs/dependency-policy.md) ·
[Unsafe code policy](docs/unsafe.md) ·
[Security](SECURITY.md) ·
[Contributing](CONTRIBUTING.md)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option.
