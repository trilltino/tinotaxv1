# tinotax

Crypto transaction reconciliation CLI — the ingestion and audit-trail foundation for a UK
crypto tax engine.

```text
raw wallet/API data
    ↓
immutable raw cache          (hashed, never overwritten by later stages)
    ↓
normalised event model       (JSONL, deterministic event IDs, source-traceable)
    ↓
diagnostics + review CSVs    (what we know, what needs a human)
    ↓
later: CEX imports + UK tax/S104 engine + HMRC evidence pack
```

## Architecture principles

1. **Raw data is immutable.** API responses are cached to disk and hashed (BLAKE3). Never overwritten.
2. **Normalised data is derived.** Delete `staging/` and `out/` and regenerate them from `raw/`.
3. **Every output row traces back to raw evidence** — source file, page, tx hash, log index, wallet.
4. **Don't guess tax treatment too early.** Uncertain rows are marked `needs_review`.
5. **Never `f64`.** Raw integer strings for chain amounts, `rust_decimal::Decimal` for reporting.
6. **Tax logic is separate from ingestion.** Each pipeline stage is its own crate.
7. **Fetching is resumable.** Cursors are persisted per endpoint; re-run with `--resume`.

## Workspace layout

```text
crates/
├── tinotax-core         pure domain types (no I/O, no HTTP, no CLI)
├── tinotax-config       wallets.toml / project.toml parsing
├── tinotax-connectors   Blockscout + NearBlocks fetchers (pagination, retry, resume)
├── tinotax-store        project folders, raw page cache, hashing, manifests, JSONL
├── tinotax-normalise    raw JSON → NormalisedEvent (EVM + NEAR), dedupe, event IDs
├── tinotax-diagnostics  diagnostics.json + wallet_activity_summary.csv
├── tinotax-review       manual_review.csv export / apply
├── tinotax-report       normalised_transactions.csv + audit_manifest.json
├── tinotax-tax-uk       (stub) UK CGT: same-day, 30-day, S104 pooling — milestone 2
├── tinotax-app          orchestration (the pipeline)
└── tinotax-cli          thin clap CLI over tinotax-app
```

Dependency direction is strictly one-way: `cli → app → {connectors, store, normalise,
diagnostics, review, report, tax-uk} → core`. `tinotax-core` has no HTTP, CSV, or CLI
dependencies, so the same engine can later power a Tauri app, an Axum backend, or a SaaS.

## Quick start

```bash
# sanity checks (connectivity to configured providers)
tinotax doctor

# whole pipeline in one shot — the demo command
cargo run -p tinotax-cli -- demo --config wallets.toml --out ./demo-data

# or stage by stage
cargo run -p tinotax-cli -- project init --config wallets.toml --out ./demo-data
cargo run -p tinotax-cli -- fetch --project ./demo-data
cargo run -p tinotax-cli -- normalise --project ./demo-data
cargo run -p tinotax-cli -- diagnose --project ./demo-data
cargo run -p tinotax-cli -- review export --project ./demo-data
```

Edit `wallets.toml` first — the two EVM addresses are placeholders.

`NEARBLOCKS_API_KEY` is read from the environment if set (recommended: the free
anonymous tier is heavily rate-limited).

## Runtime project folder

```text
demo-data/
├── project.toml                     copy of the config the project was created from
├── raw/{chain}/{wallet}/{endpoint}/page_000001.json   immutable, hashed API pages
│                                   .../cursor.json    resume state
│                        /raw_manifest.json            per-wallet hash manifest
├── staging/
│   ├── normalised_events.jsonl     one event per line, regenerable
│   ├── rejected_raw_items.jsonl    raw items we could not parse (kept, not dropped)
│   └── warnings.jsonl
├── out/
│   ├── normalised_transactions.csv
│   ├── wallet_activity_summary.csv
│   ├── manual_review.csv           only uncertain rows; accountant fills user_action
│   ├── diagnostics.json
│   └── audit_manifest.json         which raw files, which hashes, which outputs
└── logs/
```

## Demo acceptance criteria (milestone 1)

- [x] CLI accepts `wallets.toml`
- [x] Fetches the three specified wallets with pagination
- [x] Resumes after a partial fetch (`cursor.json` per endpoint)
- [x] Raw JSON pages saved and BLAKE3-hashed into `raw_manifest.json`
- [x] EVM token transfers and native fees extracted
- [x] NEAR account activity extracted as far as the public API allows
- [x] `normalised_transactions.csv`, `manual_review.csv`, `diagnostics.json`,
      `wallet_activity_summary.csv` generated
- [x] Every output row has an `event_id` and a source reference

Deliberately **not** in milestone 1: UK S104 engine, CEX importers, historical GBP
pricing, DeFi decoding, Polars/SQLite/Tauri/Axum. The `tinotax-tax-uk` crate exists as a
stub so the workspace shape doesn't change when that work starts.
