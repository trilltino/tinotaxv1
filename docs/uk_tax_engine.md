# UK tax engine

`tinotax-tax-uk` implements the CGT share-matching rules HMRC applies to
cryptoassets (Cryptoassets Manual CRYPTO22200 and CRYPTO22251–22256,
mirroring TCGA92 s104/s105/s106A) plus income at receipt.

## Matching order

For each asset, for each disposal:

1. **Same-day** — matched against acquisitions of the same asset on the
   same day. All disposals on a day are one disposal, all acquisitions one
   acquisition (TCGA92 s105).
2. **30-day ("bed & breakfast")** — the remainder is matched against
   acquisitions in the 30 days *following* the disposal, earliest first
   (TCGA92 s106A). Matched acquisitions never enter the pool.
3. **Section 104 pool** — the remainder draws cost at the pool's average.
   Each asset has one pool carrying total quantity and total allowable
   cost across the entire timeline (all tax years).

The engine always processes the full event history so pools are correct at
the requested year's boundaries; only the report is filtered to the year.

## What enters the pool at what cost

| Event | Pool effect | Cost basis |
|---|---|---|
| acquisition / swap_acquisition | add | GBP paid / market value |
| airdrop | add | market value at receipt (not income by default) |
| staking / mining / employment / misc income / compensation | add | the taxed GBP value at receipt |
| fork | add | reviewer-entered apportioned cost (default £0, warned) |
| disposal / swap_disposal / goods_or_services_spend | remove | matched per the order above |
| fee | remove | a small disposal of the fee asset at market value |
| transfer_in/out, bridge_in/out, ignore | none | — |

## Refusals (by design)

- **Unresolved rows** (`unknown` type, or missing GBP on a taxable row)
  stop the calculation with a list and instructions. `--allow-unpriced`
  excludes and reports them instead (`unresolved_tax_items.csv`).
- **Pool shortfalls** (disposal exceeds available pool) always fail,
  listing every shortfall and pointing at `opening_pools.toml` — holdings
  acquired before the data window are declared there:

```toml
[[pools]]
asset = "BTC"
quantity = "0.25"
allowable_cost_gbp = "1200.00"
as_of = "2017-01-01T00:00:00Z"
```

## Outputs (`tax/<year>/`)

- `disposals_calculation.csv` — per disposal: quantity, proceeds, matched
  same-day/30-day/S104 quantities and costs, allowable cost, gain/loss,
  source ledger event ids, matching notes
- `s104_pool_movements.csv` — every pool change in the year
- `s104_pool_opening_closing.csv` — per-asset balances at the boundaries
- `income_summary.csv` — per-receipt income rows by category
- `self_assessment_crypto_summary.csv` — headline figures
- `unresolved_tax_items.csv` — blockers and warnings
- `assumptions_and_limitations.md` — method + assumptions, regenerated per run

## Tests

`crates/tinotax-tax-uk/src/lib.rs` covers: buy/sell, partial disposal,
same-day, 30-day (inside and outside the window), pool averaging, swaps,
staking-then-sale, airdrop-then-sale, fee-only transactions, disposal
before acquisition, opening pools, negative-pool prevention, unresolved
blocking/allow-unpriced, transfers/ignores, and an HMRC-style
same-day + 30-day + pool interaction example.
