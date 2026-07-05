# Commands

During development, prefer `just` recipes for terminal startup commands:

```bash
just --list
just doctor
just startup-demo
just run --help
```

The recipes wrap `cargo run -p tinotax-cli -- <command>`; the installed binary
is named `tinotax`.

| Command | What it does |
|---|---|
| `doctor` | Config, environment and provider reachability checks |
| `demo --config <toml> --out <dir> [--resume]` | init → fetch → normalise → diagnose → review exports → reports |
| `project init --config <toml> --out <dir>` | Create a project folder from a config |
| `fetch --project <dir> [--resume]` | Fetch wallet history into the raw cache |
| `import-cex --project <dir>` | Import `[[cex_csvs]]` exports (immutable copy + hash + normalise) |
| `normalise --project <dir>` | Raw wallet pages → `staging/normalised_events.jsonl` |
| `diagnose --project <dir>` | `out/diagnostics.json`, activity summaries |
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

## Manual price CSV format

```csv
asset_symbol,date,price_gbp,source,note
BTC,2024-05-01,54321.12,manual,closing price from exchange X
```

`source` and `note` are optional; `date` may also be a full timestamp.
