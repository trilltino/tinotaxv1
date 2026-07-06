# TinoTax Production Gate

This is the project-specific application of the general
[production readiness checklist](PRODUCTION_READINESS.md). TinoTax is a local
CLI for sensitive financial evidence, not a hosted service, so the production
gate is mostly about integrity, repeatability, privacy, and human review.

## Production Definition

A TinoTax project is production-ready for accountant/client delivery only when:

- source wallet and CEX data has been fetched or imported into `raw/`;
- first-party Rust contains no unsafe code, unchecked unwrap/expect extraction,
  or direct panic-style shortcuts;
- raw files are hashed and verified against raw manifests;
- fetch cursors are complete, or the incomplete source is explicitly excluded;
- normalisation has completed with rejected raw items inspected;
- every transaction has been exported for review;
- human review overrides and manual prices are append-only and auditable;
- reports and evidence packs have an `out/audit_manifest.json`;
- secrets, real wallet configs, raw data, reports, and evidence packs are not committed.
- paid pricing credentials are supplied through environment variables, not config files.

## Required Command Flow

```bash
just production-check
just preflight wallets.toml ./fox-project
just startup wallets.toml ./fox-project
```

After human review and pricing:

```bash
just review-apply ./fox-project/out/review_all_transactions_edited.csv ./fox-project
just ledger-build ./fox-project
just prices-missing ./fox-project
export COINGECKO_API_KEY=your_key
just prices-import ./manual_prices.csv ./fox-project
just ledger-price ./fox-project
just calculate 2024-2025 ./fox-project
just pack 2024-2025 ./fox-project
just readiness ./fox-project
```

## Automated Gate

`tinotax readiness --project <dir>` checks:

- project config parses and validates;
- required project directories exist;
- raw manifest entries point to files that still match their BLAKE3 hashes;
- fetch cursors are complete;
- `out/audit_manifest.json` output hashes still match current files;
- normalised events and review CSV exist;
- rejected raw items and warnings are surfaced before delivery.

`tinotax preflight --config <wallets.toml> --project <dir>` checks:

- config file exists and validates;
- project parent path exists;
- base currency is GBP;
- provider URLs are HTTP(S);
- `NEARBLOCKS_API_KEY` is set when NearBlocks wallets are configured;
- declared CEX CSV input files exist;
- CoinGecko pricing key status is surfaced before pricing.

Failures block the gate. Warnings do not block because they may be acceptable
with accountant/client sign-off, but they must be reviewed and noted.

`just production-check` also runs `policy-scan`, which fails on first-party
`.unwrap()`, `.expect()`, `.unwrap_err()`, `.expect_err()`, direct `panic!`, `todo!`,
`unimplemented!`, `unreachable!`, or unsafe Rust. Optional parse-to-`Option`
uses are reviewed separately and documented in the Rust readiness checklist.

The supply-chain portion runs `cargo audit` against the full lockfile and
`cargo deny check` against the Windows production target configured in
`deny.toml`. Accepted license exceptions and tracked transitive advisory
ignores are documented in [dependency policy](dependency-policy.md); new
vulnerabilities remain release blockers.

## Local Data Rule

Generated project folders and real wallet configs are private client data.
They must stay gitignored. Use `wallets.example.toml` for public examples and
keep real files such as `wallets.toml`, `wallets.lisk.toml`, `fox-project/`,
`fox-project-lisk/`, imports, tax outputs, and evidence packs out of git.

## Residual Manual Gates

These remain human responsibilities:

- tax advice and final HMRC treatment decisions;
- ambiguous DeFi classification;
- source-of-funds and HMRC questionnaire answers;
- review of rejected raw items;
- manual GBP prices where provider pricing is incomplete;
- selection of paid API plans and cost limits for large wallets;
- incident response if a secret or client data file is accidentally exposed.
