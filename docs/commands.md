# Commands

During development, prefer `just` recipes for terminal startup commands:

```bash
just --list
just doctor
just preflight wallets.toml ./fox-project
just production-check
just startup-demo
just run --help
```

The recipes wrap `cargo run -p tinotax-cli -- <command>`; the installed binary
is named `tinotax`.

## Desktop operations cockpit

`apps/desktop/` is a dev-build Tauri v2 app for the same local project
operations. It does not replace the CLI; it calls Rust commands that reuse the
same `tinotax-app` orchestration:

```bash
just desktop-install
just desktop-dev
just desktop-test
just desktop-e2e
```

The desktop app covers project open/create, typed status and path inspection,
append-only review override saves, startup/refresh/finalize workflow buttons,
workflow logs, and dry-run-first cleanup.

| Command | What it does |
|---|---|
| `doctor` | Config, environment and provider reachability checks |
| `preflight --config <toml> --project <dir>` | Fail-fast startup checks for config, inputs, project path and required API keys |
| `demo --config <toml> --out <dir> [--resume]` | init → fetch → normalise → diagnose → review exports → reports |
| `project init --config <toml> --out <dir>` | Create a project folder from a config |
| `project status --project <dir>` | Summarise source counts, project folders, and human/audit state |
| `project paths --project <dir> [--tax-year 2024-2025]` | Print canonical project paths as `key=value` lines |
| `project clean --project <dir> --target logs [--confirm]` | Dry-run-first cleanup of generated project artifacts |
| `project workflow startup --config <toml> --project <dir> [--resume]` | preflight -> init -> fetch -> import -> normalise -> diagnose -> review exports -> reports -> readiness |
| `project workflow refresh-review --project <dir>` | Rebuild normalised data, diagnostics, review exports, reports, readiness |
| `project workflow finalize-year --project <dir> --tax-year 2024-2025 [--allow-unpriced]` | ledger build -> missing prices -> price ledger -> calculate -> pack -> readiness |
| `fetch --project <dir> [--resume]` | Fetch wallet history into the raw cache |
| `import-cex --project <dir>` | Import `[[cex_csvs]]` exports (immutable copy + hash + normalise) |
| `normalise --project <dir>` | Raw wallet pages → `staging/normalised_events.jsonl` |
| `diagnose --project <dir>` | `out/diagnostics.json`, activity summaries |
| `readiness --project <dir>` | Evidence integrity, cursor completion and delivery-risk gate |
| `report --project <dir>` | `out/normalised_transactions.csv` + `out/audit_manifest.json` |
| `review export-all --project <dir>` | Every event → `out/review_all_transactions.csv` |
| `review export-uncertain --project <dir>` | Flagged events only → `out/manual_review.csv` |
| `review apply --project <dir> --file <csv>` | Validate + record the edited CSV as overrides |
| `ledger build --project <dir>` | Events + overrides → reviewed ledger (JSONL + CSV) |
| `prices missing --project <dir>` | (asset, day) pairs still needing GBP → `out/missing_prices.csv` |
| `prices import --project <dir> --file <csv>` | Manual daily prices → price observations |
| `prices fetch --project <dir> [--provider coingecko]` | Fetch missing daily GBP prices |
| `ledger price --project <dir>` | Value the ledger in GBP + `out/pricing_audit.csv` |
| `calculate uk --project <dir> --tax-year 2024-2025 [--allow-unpriced]` | UK CGT + income → `tax/<year>/` |
| `pack hmrc --project <dir> --tax-year 2024-2025` | Evidence pack → `evidence_pack/<year>/` |

## Exit behaviour worth knowing

- `review apply` fails the whole run on any invalid cell (a typo can't
  silently drop a decision); untouched rows are skipped.
- `calculate uk` refuses when disposal/income rows are unclassified or
  unpriced. `--allow-unpriced` excludes them and reports them in
  `unresolved_tax_items.csv` instead. Disposals exceeding the pool always
  fail, listing every shortfall.
- `import-cex` refuses to overwrite a differing `raw/cex/<id>/original.csv`;
  declare a new export as a new id.
- `readiness` fails on broken or incomplete evidence, such as raw hash
  mismatches, unreadable manifest entries, incomplete fetch cursors, or stale
  output hashes in `out/audit_manifest.json`. It reports rejected raw items and
  normalisation warnings as warnings so they can be signed off explicitly.
- `preflight` fails before project startup if the wallet config is missing or
  invalid, declared CEX CSV files are absent, provider URLs are malformed,
  GBP is not the base currency, or a NearBlocks project is missing
  `NEARBLOCKS_API_KEY`.
- `project clean` is a dry run unless `--confirm` is passed. It never removes
  `raw/`, `project.toml`, `questionnaire.toml`, `opening_pools.toml`,
  `staging/review_overrides.jsonl`, or `staging/price_observations.jsonl`.
  `--target all-derived` expands to logs, generated staging files, `out/`,
  tax outputs, and evidence packs; `--tax-year` limits tax/evidence cleanup.

## Manual price CSV format

```csv
asset_symbol,date,price_gbp,source,note
BTC,2024-05-01,54321.12,manual,closing price from exchange X
```

`source` and `note` are optional; `date` may also be a full timestamp.
