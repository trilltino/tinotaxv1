# tinotax-pricing

GBP valuation crate.

## Owns

- Price-book loading and lookup.
- Missing-price reporting.
- Manual price imports.
- CoinGecko price fetching.
- Producing priced ledger and pricing audit files.

## Does Not Own

- Review decisions.
- Tax calculations.
- Asset identity policy beyond lookup keys currently supplied by ledger rows.

## Invariants

- Missing prices must be reported instead of guessed.
- Price observations should retain source, date, confidence, and notes.
- Manual/accountant prices take precedence over provider prices.
