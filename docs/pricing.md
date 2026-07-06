# Pricing

UK tax needs GBP values at (or near) each event. TinoTax builds a per-day
**price book** from three observation sources and values the reviewed
ledger against it.

## Sources, in precedence order

1. **Reviewer-typed GBP values** (`user_*_gbp` in the review CSV) — already
   on the ledger row after `ledger build`; never overwritten.
2. **Price observations** (`staging/price_observations.jsonl`) — from
   `prices import` (manual CSV) and `prices fetch` (CoinGecko daily
   history; built-in symbol→id table, unknown symbols reported, gentle
   rate limiting, resumable).

   For production runs, set one CoinGecko key before fetching:

   ```bash
   export COINGECKO_API_KEY=your_demo_or_public_paid_key
   # or, for Pro:
   export COINGECKO_PRO_API_KEY=your_pro_key
   ```

   PowerShell:

   ```powershell
   $env:COINGECKO_API_KEY="your_demo_or_public_paid_key"
   # or, for Pro:
   $env:COINGECKO_PRO_API_KEY="your_pro_key"
   ```
3. **CEX price hints** (`staging/cex_price_hints.jsonl`) — GBP spot prices
   stated in exchange exports, captured during `import-cex`.

The book keeps the best observation per (asset, day): higher confidence
wins, later fetch breaks ties. Lookups try the exact day, then ±1 day
(confidence `medium`), then ±2/±3 days (`low`).

## What gets valued

| Row type | Field filled |
|---|---|
| disposals (incl. swaps out, spends, fees) | `proceeds_gbp` |
| acquisitions (incl. swaps in, airdrops) | `cost_gbp` |
| income receipts | `income_gbp` |
| forks | **never auto-priced** — base cost is apportioned, enter it in review |
| transfers / bridges / ignored | nothing to value |

`out/pricing_audit.csv` records every derived value: quantity × price,
which day's observation was used, source and confidence.

## The rule

Tax calculation cannot run while required GBP values are missing, unless
`--allow-unpriced` is passed — then unresolved rows are excluded from the
numbers and reported in `tax/<year>/unresolved_tax_items.csv`. Nothing is
ever silently valued at zero.

Workflow: `ledger build` → `prices missing` → `prices import` / `prices
fetch` → `ledger price` → repeat `prices missing` until clean.
