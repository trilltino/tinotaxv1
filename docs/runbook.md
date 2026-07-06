# Runbook

This runbook is for local CLI operation.

## Startup

```bash
just preflight wallets.toml ./fox-project
just startup wallets.toml ./fox-project
```

`startup` validates inputs, creates the project, fetches with resume enabled,
imports CEX CSVs, normalises, diagnoses, exports review CSVs, writes reports,
and runs readiness.

## Resume A Failed Fetch

```bash
just fetch-resume ./fox-project
just normalise ./fox-project
just diagnose ./fox-project
just review-export ./fox-project
just report ./fox-project
just readiness ./fox-project
```

Fetchers persist raw pages and cursors. If a provider rate-limits or the process
is interrupted, rerun with resume rather than deleting raw evidence.

## Pricing

Set one of:

```bash
COINGECKO_PRO_API_KEY=...
COINGECKO_DEMO_API_KEY=...
COINGECKO_API_KEY=...
```

Then run:

```bash
just ledger-build ./fox-project
just prices-missing ./fox-project
just prices-fetch ./fox-project coingecko
just ledger-price ./fox-project
```

Unknown tokens or provider gaps are handled through `prices import`.

## Delivery Gate

Before sending anything to a client or accountant:

```bash
just readiness ./fox-project
```

Warnings require human inspection and notes. Failures block delivery.
