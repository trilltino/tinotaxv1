# Performance And Startup Optimization

TinoTax optimizes for correctness first, then measured throughput on large
wallets.

## Current Fast Path

- Raw API pages and JSONL outputs use buffered IO.
- Fetches are resumable and rate-limit aware.
- Money calculations use `rust_decimal::Decimal`.
- Pipeline stages are rebuildable, so expensive stages can be rerun from cached
  raw evidence without refetching.

## Measurement Plan

For large wallets such as NEAR accounts with hundreds of thousands of
transactions:

```bash
just startup wallets.toml ./fox-project
```

Record:

- fetch pages/items and provider rate limits;
- normalise duration and rejected/warning counts;
- review export row count and duration;
- ledger/pricing duration;
- peak memory where available.

## Refactor Priority

Only optimize after measuring:

1. Stream raw page and JSONL consumers that currently materialize large vectors.
2. Replace repeated price/review lookups with prebuilt maps where profiling
   shows repeated scans.
3. Tune provider page size, paid API rate limits, and retry pacing.
4. Add targeted benchmarks for tax matching and price-book lookup.

## Error-Safe Optimization Rule

Performance work must not replace contextual errors with silent defaults.
Streaming refactors must preserve row/path/event context, raw evidence hashes,
append-only review/pricing state, and the no-unwrap/no-unsafe policy.
