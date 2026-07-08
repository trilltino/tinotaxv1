# Installation & CLI Command Reference

Reviewed-ledger UK crypto tax tool.
Pipeline: fetch → normalise → review → ledger → price → UK CGT → HMRC pack

The command-line binary is called `tinotax`. There is also a Tauri v2 desktop
app (`apps/desktop`) that wraps the same Rust core — see
[Desktop app](#11-desktop-app-appsdesktop--tauri-v2-gui) for how to install
and run it. Everything else in this document covers the end-to-end CLI.

## Contents

1. [Prerequisites](#1-prerequisites)
2. [Get the code](#2-get-the-code)
3. [Build & install the CLI](#3-build--install-the-cli)
4. [API keys (environment variables)](#4-api-keys-environment-variables)
5. [Config file (wallets.toml)](#5-config-file-walletstoml)
6. [End-to-end quickstart](#6-end-to-end-quickstart)
7. [Global usage](#7-global-usage)
8. [Full command reference](#8-full-command-reference)
9. [Typical step-by-step run (manual pipeline)](#9-typical-step-by-step-run-manual-pipeline)
10. [Troubleshooting](#10-troubleshooting)
11. [Desktop app (apps/desktop) — Tauri v2 GUI](#11-desktop-app-appsdesktop--tauri-v2-gui)

## 1. Prerequisites

- Rust toolchain (stable) with cargo. Install from
  [rustup.rs](https://rustup.rs) — this repo pins a toolchain via
  `rust-toolchain.toml`, so `rustup` will fetch the right version
  automatically.
- Git.
- Internet access (for fetching wallet history and prices).
- Windows, macOS, or Linux.

Verify Rust is installed:

```bash
rustc --version
cargo --version
```

## 2. Get the code

```bash
git clone https://github.com/trilltino/tinotax
cd tinotax
```

## 3. Build & install the CLI

The CLI binary is named `tinotax` and lives in the `tinotax-cli` crate.

**Option A — build a release binary (recommended):**

```bash
cargo build --release -p tinotax-cli
```

The binary is then at:

```text
target/release/tinotax          (Linux/macOS)
target\release\tinotax.exe      (Windows)
```

**Option B — install it onto your PATH:**

```bash
cargo install --path crates/interface/tinotax-cli
```

After this you can call `tinotax` from anywhere.

**Option C — run without installing (via cargo):**

```bash
cargo run -q -p tinotax-cli -- <command> [args...]
```

Everything after `--` is passed to the CLI. Example:

```bash
cargo run -q -p tinotax-cli -- doctor
```

> Throughout this document commands are written as `tinotax <command>`. If
> you did not `cargo install`, substitute:
> `cargo run -q -p tinotax-cli -- <command>`

## 4. API keys (environment variables)

The tool works without keys but is slow/limited. For real runs set:

| Variable | Purpose |
|---|---|
| `NEARBLOCKS_API_KEY` | Required for NEAR wallets on a paid plan. Without it, NEAR fetching falls back to the slow anonymous tier. |
| `NEARBLOCKS_CREDITS_PER_MINUTE` | Optional rate override for the NearBlocks plan. |
| `COINGECKO_API_KEY` | CoinGecko demo/public paid key (price fetching). |
| `COINGECKO_DEMO_API_KEY` | CoinGecko demo key (alternative to the above). |
| `COINGECKO_PRO_API_KEY` | CoinGecko Pro key (full history, preferred if set). |

Set them in your shell before running.

PowerShell (Windows):

```powershell
$env:NEARBLOCKS_API_KEY = "your-key"
$env:COINGECKO_API_KEY  = "your-key"
```

bash/zsh (Linux/macOS):

```bash
export NEARBLOCKS_API_KEY="your-key"
export COINGECKO_API_KEY="your-key"
```

Optional logging control (defaults to `tinotax=info,warn`):

```bash
RUST_LOG=tinotax=debug
```

## 5. Config file (wallets.toml)

A project is created from a wallet/source config file. Copy the example and
fill in real addresses (`wallets.toml` is gitignored — client addresses never
go into git):

```bash
cp wallets.example.toml wallets.toml          # Linux/macOS
Copy-Item wallets.example.toml wallets.toml   # PowerShell
```

The config declares:

- `[project]` — name, base_currency (GBP), period_start, period_end
- `[[wallets]]` — one block per wallet: id, name, chain, address, provider
- `[[cex_csvs]]` — optional CEX CSV exports to import (binance/coinbase/
  kraken/awaken/generic, with column mapping for "generic")
- `[providers.*]` — provider definitions (blockscout / nearblocks base URLs)

See `wallets.example.toml` in the repo root for a fully commented template.

## 6. End-to-end quickstart

The fastest path uses the "prepare" workflow, which chains fetch → normalise
→ auto-classify → build → (price fetch) → price → calculate:

```bash
# 1. Sanity-check environment and provider reachability
tinotax doctor

# 2. One-click prepare for a tax year (creates/reuses the project folder)
tinotax project workflow prepare \
    --config ./wallets.toml \
    --project ./fox-project \
    --tax-year 2024-2025 \
    --fetch-prices

# 3. Build the client HMRC evidence pack
tinotax pack hmrc --project ./fox-project --tax-year 2024-2025
```

If you prefer to run the pipeline step by step, use the individual commands
in [section 8](#8-full-command-reference).

For a scripted demo of the whole ingestion pipeline in one shot:

```bash
tinotax demo --config ./wallets.toml --out ./demo-project
```

## 7. Global usage

```bash
tinotax --help              # Show all commands
tinotax <command> --help    # Show help for a specific command
tinotax --version           # Show version
```

Common flags:

| Flag | Meaning |
|---|---|
| `--project <path>` | Path to the project folder (created by `project init`). |
| `--config <path>` | Path to a wallet/source config file (e.g. `wallets.toml`). |
| `--tax-year <label>` | UK tax year label, e.g. `2024-2025`. |
| `--resume` | Reuse already-fetched raw pages instead of refetching. |

## 8. Full command reference

### Top-level commands

- **`tinotax doctor`** — Check config, environment, and provider
  reachability. Reports which API keys are set. Run this first.
- **`tinotax preflight --config <cfg> --project <path>`** — Fail-fast checks
  for a local startup run (validates config and required keys before doing
  real work).
- **`tinotax demo --config <cfg> --out <path> [--resume]`** — Run the whole
  ingestion pipeline in one shot: init → fetch → normalise → diagnose →
  review exports → reports. `--resume` reuses already-fetched raw pages.
- **`tinotax fetch --project <path> [--resume] [--wallet <id>]...`** — Fetch
  wallet history from chain APIs into the immutable raw cache. `--wallet`
  fetches only the named wallet id(s) (repeat for several; omit for all).
  `--resume` reuses already-fetched raw pages.
- **`tinotax import-cex --project <path>`** — Import the CEX CSV exports
  declared as `[[cex_csvs]]` entries in `project.toml`. Original files are
  stored unedited (immutable hashed copy under `raw/cex/`).
- **`tinotax normalise --project <path>`** — Normalise raw wallet pages into
  `staging/normalised_events.jsonl`.
- **`tinotax diagnose --project <path>`** — Data quality and completeness
  diagnostics.
- **`tinotax readiness --project <path>`** — Verify project evidence,
  outputs, and unresolved production risks (the go-live readiness gate).
- **`tinotax report --project <path>`** — Export the normalised transactions
  CSV + audit manifest.

### Project — folder management

- **`tinotax project init --config <cfg> --out <path>`** — Create a project
  folder from a config file.
- **`tinotax project status --project <path>`** — Summarise project folders,
  sources, and human/audit state.
- **`tinotax project paths --project <path> [--tax-year <label>]`** — Print
  canonical project paths (optionally for a given tax year).
- **`tinotax project clean --project <path> --target <t>[,<t>...] [--tax-year <label>] [--confirm]`**
  — Clean generated project artifacts. Dry-run (prints the plan) unless
  `--confirm` is passed.
  - `--target` — one or more of: `logs`, `staging`, `out`, `tax`, `evidence`,
    `all-derived` (repeat `--target` or comma-separate).
  - `--tax-year` — limit tax/evidence cleanup to one tax year.
  - `--confirm` — actually delete files.

### Project workflow — multi-step orchestration

- **`tinotax project workflow startup --config <cfg> --project <path> [--resume]`**
  — Run: preflight, init, fetch, import, normalise, diagnose, review exports,
  reports, readiness.
- **`tinotax project workflow refresh-review --project <path>`** — Rebuild
  review surfaces and reports from current raw/project state.
- **`tinotax project workflow finalize-year --project <path> --tax-year <label> [--allow-unpriced]`**
  — Build ledger, price, calculate one tax year, pack evidence, and run
  readiness. `--allow-unpriced` excludes unpriced/unresolved rows (reported,
  not silently dropped) instead of refusing.
- **`tinotax project workflow prepare --config <cfg> --project <path> [--wallet <id>]... --tax-year <label> [--resume] [--fetch-prices]`**
  — One-click: fetch → normalise → auto-ignore contract calls → build →
  (price fetch) → price → calculate, for the selected wallet(s).
  - `--wallet` — prepare only the named wallet id(s); repeat for several
    (omit for all).
  - `--resume` — reuse already-fetched raw pages.
  - `--fetch-prices` — also fetch GBP prices from CoinGecko (needs an API
    key).

### Review — human decisions (never mutates raw/normalised data)

All human changes are recorded as overrides; raw and normalised data are
never mutated.

- **`tinotax review export-all --project <path>`** — Export every event to
  `out/review_all_transactions.csv` for full review.
- **`tinotax review export-uncertain --project <path>`** — Export only rows
  flagged as uncertain to `out/manual_review.csv`.
- **`tinotax review apply --project <path> --file <csv>`** — Apply an edited
  review CSV (records decisions, never mutates data).
- **`tinotax review auto-classify --project <path>`** — Bulk-classify every
  zero-value contract call as `ignore` (non-taxable).

### Ledger — build & value the reviewed tax ledger

- **`tinotax ledger build --project <path>`** — Derive
  `staging/reviewed_ledger.jsonl` from events + review overrides.
- **`tinotax ledger price --project <path>`** — Value the reviewed ledger in
  GBP using the price book.

### Prices — historical GBP prices

- **`tinotax prices missing --project <path>`** — List (asset, date) pairs
  that still need a GBP price.
- **`tinotax prices import --project <path> --file <csv>`** — Import a manual
  price CSV. CSV columns: `asset_symbol,date,price_gbp` (optionally
  `source,note`).
- **`tinotax prices fetch --project <path> [--provider <name>]`** — Fetch
  missing daily GBP prices from a provider. `--provider` — price provider
  (default: `coingecko`).

### Calculate — tax calculations

- **`tinotax calculate uk --project <path> --tax-year <label> [--allow-unpriced]`**
  — UK CGT (same-day, 30-day, Section 104) + income for one tax year.
  `--allow-unpriced` excludes unpriced/unresolved rows (reported, not
  silently dropped) instead of refusing to calculate.

### Pack — client-facing deliverables

- **`tinotax pack hmrc --project <path> --tax-year <label>`** — Build the
  HMRC / Self Assessment evidence pack for one tax year.

## 9. Typical step-by-step run (manual pipeline)

```bash
# 0. Preflight
tinotax doctor
tinotax preflight --config ./wallets.toml --project ./fox-project

# 1. Create the project
tinotax project init --config ./wallets.toml --out ./fox-project

# 2. Ingest
tinotax fetch --project ./fox-project
tinotax import-cex --project ./fox-project          # if CEX CSVs declared
tinotax normalise --project ./fox-project
tinotax diagnose --project ./fox-project

# 3. Review
tinotax review export-all --project ./fox-project
#   ...edit the CSV in a spreadsheet...
tinotax review apply --project ./fox-project --file ./out/review_all_transactions.csv
tinotax review auto-classify --project ./fox-project   # optional bulk ignore

# 4. Ledger + pricing
tinotax ledger build --project ./fox-project
tinotax prices fetch --project ./fox-project --provider coingecko
tinotax prices missing --project ./fox-project
#   ...manually price any gaps...
tinotax prices import --project ./fox-project --file ./manual_prices.csv
tinotax ledger price --project ./fox-project

# 5. Calculate + pack
tinotax calculate uk --project ./fox-project --tax-year 2024-2025
tinotax pack hmrc --project ./fox-project --tax-year 2024-2025

# 6. Confirm readiness
tinotax readiness --project ./fox-project
```

## 10. Troubleshooting

- `tinotax doctor` reports which keys are set and whether providers are
  reachable — always start here.
- NEAR fetching is slow / rate-limited: set `NEARBLOCKS_API_KEY` (paid plan).
- Many rows unpriced: set a CoinGecko key and re-run `prices fetch`; price
  the remaining unlistable tokens (LP NFTs, obscure tokens) via
  `prices import`.
- `calculate uk` refuses to run: either resolve unpriced rows, or pass
  `--allow-unpriced` to proceed with those rows excluded (and reported).
- Want to reset generated artifacts:
  `tinotax project clean --project <path> --target all-derived` (add
  `--confirm` to actually delete).
- Verbose logs: set `RUST_LOG=tinotax=debug` before running.

## 11. Desktop app (apps/desktop) — Tauri v2 GUI

The desktop app is a local operations cockpit that wraps the same Rust core
(`tinotax-app`) used by the CLI. It does not replace the CLI — it calls the
same project/fetch/normalise/review/ledger/price/calculate/pack logic from a
GUI, and is Windows-first (dev-build quality; the CLI is the canonical
automation surface).

### 11.1 Prerequisites (in addition to section 1)

- Node.js + npm (LTS recommended). Verify:

  ```bash
  node --version
  npm --version
  ```

- Rust MSVC build tools on Windows: "Desktop development with C++" workload
  from the Visual Studio Build Tools installer (needed to compile the Tauri
  Rust binary). On Linux you also need the usual Tauri system deps
  (webkit2gtk, libssl-dev, etc. — see
  [v2.tauri.app/start/prerequisites](https://v2.tauri.app/start/prerequisites/)).
- WebView2 runtime on Windows (pre-installed on most modern Windows 10/11).

### 11.2 Install dependencies

From the repo root:

```bash
cd apps/desktop
npm install --no-audit --fund=false
```

This installs the React/Vite frontend deps and the `@tauri-apps/cli` used to
drive the Tauri Rust build.

If you have `just` installed (`cargo install just`), the equivalent is:

```bash
just desktop-install
```

### 11.3 Run in dev mode

From `apps/desktop`:

```bash
npm run tauri:dev
```

Or from the repo root with `just`:

```bash
just dev
```

This runs `npm run tauri:dev`, which is `node scripts/run-tauri.mjs dev`. It:

1. Starts the Vite dev server on `127.0.0.1:1420` (`npm run dev`).
2. Compiles the Rust `src-tauri` binary (`tinotax-desktop`, first run takes a
   few minutes) and opens the Tauri desktop window pointed at the dev server.

`just dev` is `just desktop-install` followed by `just desktop-dev` (the same
`npm run tauri:dev` call), so it takes you from a fresh clone straight to a
running desktop window in one command (after Node/Rust prerequisites are in
place).

### 11.4 Config

The desktop app reads/writes the same project folders and `wallets.toml` as
the CLI. Point it at an existing `wallets.toml` and project directory (or
create a new project) from the Wallets tab; the app calls the same Rust
project-init/fetch/normalise code paths as `tinotax project init` / `fetch`
etc. Set the same API key environment variables (section 4) before launching
the app so price/chain fetching works.

### 11.5 Tests

```bash
npm test               # React/Vitest unit tests (apps/desktop)
just desktop-test       # same, via just

just e2e                # hermetic WebdriverIO/Tauri end-to-end flow
                        #  == just desktop-e2e ==
                        #  == npm run e2e (builds a debug, no-bundle
                        #     Tauri binary, then runs wdio)
```

See [apps/desktop/e2e/README.md](../apps/desktop/e2e/README.md) for details
on how the end-to-end suite is structured and how to update its fixture.

### 11.6 Build a distributable installer

From `apps/desktop`:

```bash
npm run build          # tsc + vite build (frontend)
npx tauri build         # bundles the release Tauri binary + installer
```

Output installer(s) land under:

```text
target/release/bundle/     (e.g. Windows .msi and/or NSIS .exe)
```

For a debug binary without installer packaging (used by the e2e test), use:

```bash
just desktop-build
  # == npm run build && npm run tauri -- build --debug --no-bundle
```

### 11.7 Summary: clone → running desktop app

```bash
git clone https://github.com/trilltino/tinotax
cd tinotax
Copy-Item wallets.example.toml wallets.toml   # edit with real wallets
$env:NEARBLOCKS_API_KEY = "your-key"          # optional but recommended
$env:COINGECKO_API_KEY  = "your-key"          # optional but recommended
cd apps/desktop
npm install --no-audit --fund=false
npm run tauri:dev
```

(or, from the repo root, with `just` installed: `just dev`)
