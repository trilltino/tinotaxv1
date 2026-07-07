set dotenv-load
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

windows_cargo := env_var_or_default("CARGO", env_var("USERPROFILE") + "\\.cargo\\bin\\cargo.exe")
unix_cargo := env_var_or_default("CARGO", "cargo")
cargo := if os_family() == "windows" { windows_cargo } else { unix_cargo }
bin := cargo + " run -p tinotax-cli --"

default:
    @just --list

# Install frontend dependencies and load the full desktop app.
dev:
    {{just_executable()}} desktop-install
    {{just_executable()}} desktop-dev

# Run the full seeded desktop end-to-end flow.
e2e:
    {{just_executable()}} desktop-e2e

# Run the development quality gate.
check: metadata policy-scan fmt-check clippy check-build test doc

# Run the full local production gate. Install cargo-audit and cargo-deny first
# if Cargo reports that either subcommand is missing.
production-check: check audit deny

# Record locked Cargo metadata for release/debug evidence.
metadata:
    {{cargo}} metadata --locked --format-version 1 > target/cargo-metadata.json

# Fail if first-party Rust contains unchecked extraction, panic shortcuts, or unsafe blocks.
policy-scan:
    $banned = rg -n "\.unwrap\(\)|\.expect\(|panic!\(|todo!\(|unimplemented!\(|unreachable!\(|unwrap_err\(|expect_err\(|unwrap_none\(|expect_none\(" crates apps/desktop/src-tauri -g "*.rs"; $code = $LASTEXITCODE; if ($code -eq 0) { $banned; exit 1 } elseif ($code -ne 1) { exit $code }; exit 0
    $unsafe = rg -n "unsafe\s*\{|unsafe fn|unsafe impl" crates apps/desktop/src-tauri -g "*.rs"; $code = $LASTEXITCODE; if ($code -eq 0) { $unsafe; exit 1 } elseif ($code -ne 1) { exit $code }; exit 0

# Format the workspace in place.
fmt:
    {{cargo}} fmt --all

# Check workspace formatting.
fmt-check:
    {{cargo}} fmt --all -- --check

# Run clippy with release-gate warnings.
clippy:
    {{cargo}} clippy --workspace --all-targets --all-features --locked -- -D warnings

# Type-check every target/feature combination used by CI.
check-build:
    {{cargo}} check --workspace --all-targets --all-features --locked

# Run all workspace tests.
test:
    {{cargo}} test --workspace --all-targets --all-features --locked

# Build workspace docs without dependencies.
doc:
    {{cargo}} doc --workspace --all-features --no-deps --locked

# Install desktop frontend dependencies.
desktop-install:
    cd apps/desktop; npm install --no-audit --fund=false

# Run the Tauri desktop app in development mode.
desktop-dev:
    cd apps/desktop; npm run tauri:dev

# Build the desktop frontend and debug Tauri binary without bundling installers.
desktop-build:
    cd apps/desktop; npm run build
    cd apps/desktop; npm run tauri -- build --debug --no-bundle

# Run React/Vitest desktop tests.
desktop-test:
    cd apps/desktop; npm test

# Run the hermetic WebdriverIO/Tauri desktop flow.
desktop-e2e:
    cd apps/desktop; npm run e2e

# Check RustSec advisories for locked dependencies.
audit:
    {{cargo}} audit

# Check dependency sources, licenses, advisories, and duplicate versions.
deny:
    {{cargo}} deny check

# Run the CLI through cargo. Example: just run --help
run +args:
    {{bin}} {{args}}

# Check config, environment, and provider reachability.
doctor:
    {{bin}} doctor

# Fail-fast startup checks before creating/fetching a project.
preflight config="wallets.toml" project="./fox-project":
    {{bin}} preflight --config {{config}} --project {{project}}

# Run the three-wallet demo ingestion pipeline.
demo config="wallets.toml" out="./demo-data" *flags:
    {{bin}} demo --config {{config}} --out {{out}} {{flags}}

# Create a project folder from a wallet/source config.
init config="wallets.toml" out="./fox-project":
    {{bin}} project init --config {{config}} --out {{out}}

# Summarise project folders, sources, and human/audit state.
project-status project="./fox-project":
    {{bin}} project status --project {{project}}

# Print canonical project paths. Example: just project-paths ./fox-project --tax-year 2024-2025
project-paths project="./fox-project" *args:
    {{bin}} project paths --project {{project}} {{args}}

# Plan or run safe cleanup. Example: just project-clean ./fox-project --target logs --confirm
project-clean project="./fox-project" *args:
    {{bin}} project clean --project {{project}} {{args}}

# Fetch wallet history into the immutable raw cache.
fetch project="./fox-project" *flags:
    {{bin}} fetch --project {{project}} {{flags}}

# Fetch wallet history and resume from saved cursors.
fetch-resume project="./fox-project":
    {{bin}} fetch --project {{project}} --resume

# Import configured CEX CSV exports.
import-cex project="./fox-project":
    {{bin}} import-cex --project {{project}}

# Normalise raw wallet pages into staging events.
normalise project="./fox-project":
    {{bin}} normalise --project {{project}}

# Run project diagnostics.
diagnose project="./fox-project":
    {{bin}} diagnose --project {{project}}

# Verify evidence integrity and unresolved production risks.
readiness project="./fox-project":
    {{bin}} readiness --project {{project}}

# Export normalised reports and the audit manifest.
report project="./fox-project":
    {{bin}} report --project {{project}}

# Export every event for spreadsheet review.
review-export project="./fox-project":
    {{bin}} review export-all --project {{project}}

# Apply an edited review CSV.
review-apply file project="./fox-project":
    {{bin}} review apply --project {{project}} --file {{file}}

# Build the reviewed ledger.
ledger-build project="./fox-project":
    {{bin}} ledger build --project {{project}}

# Export missing price rows.
prices-missing project="./fox-project":
    {{bin}} prices missing --project {{project}}

# Fetch missing prices from a provider.
prices-fetch project="./fox-project" provider="coingecko":
    {{bin}} prices fetch --project {{project}} --provider {{provider}}

# Import manual price CSV rows.
prices-import file project="./fox-project":
    {{bin}} prices import --project {{project}} --file {{file}}

# Value the reviewed ledger in GBP.
ledger-price project="./fox-project":
    {{bin}} ledger price --project {{project}}

# Calculate UK tax for one tax year.
calculate tax_year project="./fox-project" *flags:
    {{bin}} calculate uk --project {{project}} --tax-year {{tax_year}} {{flags}}

# Build the HMRC evidence pack for one tax year.
pack tax_year project="./fox-project":
    {{bin}} pack hmrc --project {{project}} --tax-year {{tax_year}}

# Run the binary's grouped startup workflow.
workflow-startup config="wallets.toml" project="./fox-project" *flags:
    {{bin}} project workflow startup --config {{config}} --project {{project}} {{flags}}

# Rebuild review surfaces and reports from current project state.
workflow-refresh-review project="./fox-project":
    {{bin}} project workflow refresh-review --project {{project}}

# Build, price, calculate, pack, and readiness-check one tax year.
workflow-finalize-year tax_year project="./fox-project" *flags:
    {{bin}} project workflow finalize-year --project {{project}} --tax-year {{tax_year}} {{flags}}

# Run the standard full project flow up to review export.
startup config="wallets.toml" project="./fox-project":
    {{just_executable()}} preflight {{config}} {{project}}
    {{just_executable()}} init {{config}} {{project}}
    {{just_executable()}} fetch-resume {{project}}
    {{just_executable()}} import-cex {{project}}
    {{just_executable()}} normalise {{project}}
    {{just_executable()}} diagnose {{project}}
    {{just_executable()}} review-export {{project}}
    {{just_executable()}} report {{project}}
    {{just_executable()}} readiness {{project}}

# Run the demo pipeline with resume enabled.
startup-demo config="wallets.toml" out="./demo-data":
    {{just_executable()}} demo {{config}} {{out}} --resume
