# Assumptions and limitations

The per-year `tax/<year>/assumptions_and_limitations.md` is generated with
the calculation; this page is the standing list.

## Tax method assumptions

- **Tax-year boundaries use the UTC date** of each event, not UK civil
  time. Events within an hour of midnight around 5/6 April could in
  principle fall differently; flag any that matter to the accountant.
- **Crypto-to-crypto swaps** are a disposal of the sold token and an
  acquisition of the bought token, each at GBP market value.
- **Fees paid in crypto** are disposals of the fee asset at market value.
  Fiat fees from exchanges are noted on events but only enter allowable
  costs if the reviewer types them into the review CSV.
- **Airdrops received for nothing** are capital acquisitions at market
  value, not income (CRYPTO21250). Airdrops in return for a service should
  be reclassified `misc_income` during review.
- **Forks** carry the reviewer-entered apportioned base cost (default £0,
  which is conservative — it can only overstate the eventual gain).
- **Compensation** is treated as taxable income at receipt; an accountant
  should confirm whether capital treatment applies instead.
- **Transfers/bridges between the client's own wallets** have no tax
  effect; the classification is human-reviewed wherever flagged.
- **Income receipts** are valued in GBP at receipt; that value becomes the
  CGT cost basis.

## Data limitations

- Wallet coverage is what the configured providers expose; `diagnose`
  reports gaps, and disposals exceeding the pool fail loudly (usually a
  sign of an unimported source or missing `opening_pools.toml`).
- Provider price data (CoinGecko) is daily granularity; intra-day price
  moves are not captured. Nearby-day fallback (±3 days) is marked at
  reduced confidence in `pricing_audit.csv`.
- NFTs and non-fungible positions are not pooled or specially handled;
  classify them during review and discuss treatment with the accountant.
- Complex DeFi (LPs, lending, rebasing tokens) is not auto-classified;
  such events surface as `unknown`/flagged and must be reviewed.

## Product limitations

- TinoTax produces calculation support, not advice, and does not file
  anything with HMRC.
- The annual exempt amount, loss claims/elections, and interaction with
  the client's wider return are the accountant's domain — deliberately out
  of scope.
