# TinoTax

A reviewed-ledger UK crypto tax engine: Rust core, CLI, and a Tauri v2
desktop app.

TinoTax turns wallet and exchange history into an HMRC-ready UK CGT /
income evidence pack without guessing on your behalf. Every transaction is
surfaced for human review, every decision is recorded as an append-only
override, and tax calculation refuses to run on anything unclassified or
unpriced.

```text
raw data → normalised events → review → apply edits
        → reviewed ledger → GBP-priced ledger → UK CGT + income → HMRC pack
```

## Why

- Nothing is silently classified — uncertain rows block calculation until
  reviewed.
- Raw data is immutable and BLAKE3-hashed; corrections are overrides, never
  edits.
- Everything downstream re-derives from raw data + overrides.
- The UK tax engine (same-day → 30-day → Section 104 pool) is deterministic
  and unit-tested — see [docs/uk_tax_engine.md](docs/uk_tax_engine.md).
- No `f64` for money — `rust_decimal::Decimal` throughout.
- `unsafe` is forbidden workspace-wide; unchecked `.unwrap()`/`.expect()`/
  panics are denied by lint.

## What it does

Fetches NEAR + Blockscout-EVM wallet history and CEX CSVs (Binance,
Coinbase, Kraken, Awaken, generic) into an immutable raw cache; normalises
everything into one event model; exports every transaction for review;
applies human edits as overrides; values the ledger in GBP (manual, CEX
hints, or CoinGecko, with confidence + audit trail); calculates UK CGT and
income per tax year; assembles an HMRC evidence pack. Ships `doctor`,
`preflight`, and `readiness` diagnostics to fail fast.

Not tax advice, doesn't auto-classify ambiguous DeFi activity, doesn't file
with HMRC.

## Install

Requires Rust (stable, via [rustup.rs](https://rustup.rs)) and Git.

```bash
git clone https://github.com/trilltino/tinotax
cd tinotax
cargo build --release -p tinotax-cli   # binary: target/release/tinotax
```

Or run without installing: `cargo run -q -p tinotax-cli -- <command>`.

## Quick start

```bash
cp wallets.example.toml wallets.toml   # fill in real addresses (gitignored)

tinotax doctor

tinotax project workflow prepare \
    --config ./wallets.toml --project ./my-project \
    --tax-year 2024-2025 --fetch-prices

tinotax pack hmrc --project ./my-project --tax-year 2024-2025
```

Set `NEARBLOCKS_API_KEY` and `COINGECKO_API_KEY` first for real runs (see
`tinotax doctor`). Full walkthrough, every command, and the desktop app in
[docs/installation.md](docs/installation.md) and
[docs/commands.md](docs/commands.md).

## Desktop app

```bash
cd apps/desktop
npm install
npm run tauri:dev
```

A Windows-first Tauri v2 cockpit calling the same `tinotax-app`
orchestration as the CLI. Details in
[docs/installation.md](docs/installation.md#11-desktop-app-appsdesktop--tauri-v2-gui).

## Project layout

```text
my-project/
├── project.toml, questionnaire.toml, opening_pools.toml
├── raw/            immutable, hashed evidence
├── staging/        machine-derived intermediates
├── out/            reviewable CSV/JSON
├── tax/<year>/     disposals, pool movements, income, SA summary
└── evidence_pack/<year>/
```

See [docs/data_model.md](docs/data_model.md) for what's raw, derived, or
human input.

## Workspace

Five layers, one-way dependency: `tinotax-cli` → `tinotax-app` → pipeline
crates → `tinotax-core` (+ `tinotax-store`).

| Layer | Crate | Role |
|---|---|---|
| foundation | `tinotax-core`, `tinotax-config`, `tinotax-store` | domain types, config, project/raw storage |
| ingest | `tinotax-connectors`, `tinotax-cex`, `tinotax-normalise` | wallet + CEX fetch, normalisation |
| review | `tinotax-diagnostics`, `tinotax-review`, `tinotax-ledger` | quality reports, overrides, reviewed ledger |
| valuation | `tinotax-pricing`, `tinotax-tax-uk` | GBP pricing, UK CGT/income engine |
| output | `tinotax-report`, `tinotax-evidence` | reports, HMRC evidence pack |
| interface | `tinotax-app`, `tinotax-cli` | orchestration, CLI |

See [crates/README.md](crates/README.md) and
[docs/architecture.md](docs/architecture.md).

## Development

```bash
just check              # fmt-check + clippy + build + test + doc + policy-scan
just production-check   # check + cargo-audit + cargo-deny
```

Run `just check` before opening a PR. Never commit real wallet addresses,
exchange exports, tax outputs, evidence packs, or API keys — see
[CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

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
[Production readiness](docs/PRODUCTION_READINESS.md) ·
[Full docs index](docs/README.md)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option.
