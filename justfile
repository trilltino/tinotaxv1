set dotenv-load
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

windows_cargo := env_var_or_default("CARGO", env_var("USERPROFILE") + "\\.cargo\\bin\\cargo.exe")
unix_cargo := env_var_or_default("CARGO", "cargo")
cargo := if os_family() == "windows" { windows_cargo } else { unix_cargo }
bin := cargo + " run -p tinotax-cli --"

default:
    @just --list

# Run the development quality gate.
check: fmt-check clippy test doc

# Format the workspace in place.
fmt:
    {{cargo}} fmt --all

# Check workspace formatting.
fmt-check:
    {{cargo}} fmt --all -- --check

# Run clippy with release-gate warnings.
clippy:
    {{cargo}} clippy --workspace --all-targets -- -D warnings

# Run all workspace tests.
test:
    {{cargo}} test --workspace

# Build workspace docs without dependencies.
doc:
    {{cargo}} doc --workspace --no-deps

# Run the CLI through cargo. Example: just run --help
run +args:
    {{bin}} {{args}}

# Check config, environment, and provider reachability.
doctor:
    {{bin}} doctor

# Run the three-wallet demo ingestion pipeline.
demo config="wallets.toml" out="./demo-data" *flags:
    {{bin}} demo --config {{config}} --out {{out}} {{flags}}

# Create a project folder from a wallet/source config.
init config="wallets.toml" out="./fox-project":
    {{bin}} project init --config {{config}} --out {{out}}

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

# Run the standard full project flow up to review export.
startup config="wallets.toml" project="./fox-project":
    {{just_executable()}} init {{config}} {{project}}
    {{just_executable()}} fetch-resume {{project}}
    {{just_executable()}} import-cex {{project}}
    {{just_executable()}} normalise {{project}}
    {{just_executable()}} diagnose {{project}}
    {{just_executable()}} review-export {{project}}

# Run the demo pipeline with resume enabled.
startup-demo config="wallets.toml" out="./demo-data":
    {{just_executable()}} demo {{config}} {{out}} --resume
