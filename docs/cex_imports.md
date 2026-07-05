# CEX imports

HMRC's standard request includes *full, unedited trading data files*, so
the original export is the evidence: `import-cex` copies each file to
`raw/cex/<id>/original.csv`, writes `original_hash.txt` (BLAKE3), and
refuses to overwrite an existing copy with different content — a new
export needs a new `[[cex_csvs]]` id.

## Configuration

```toml
[[cex_csvs]]
id = "binance_2017_2025"
platform = "binance"            # binance | coinbase | kraken | awaken | generic
path = "./imports/binance.csv"
```

For `generic`, map your CSV's headers to the canonical columns
(`timestamp`, `type`, `asset`, `amount` required; `fee_asset`,
`fee_amount`, `note` optional; `amount` must be signed):

```toml
[cex_csvs.mapping]
timestamp = "Date"
type = "Operation"
asset = "Coin"
amount = "Change"
```

## Supported formats

| Platform | Expected export |
|---|---|
| `binance` | Transaction History: `User_ID,UTC_Time,Account,Operation,Coin,Change,Remark` |
| `coinbase` | Transaction report (preamble tolerated): `Timestamp,Transaction Type,Asset,Quantity Transacted,…` |
| `kraken` | Ledgers export: `txid,refid,time,type,…,asset,wallet,amount,fee,balance` (legacy codes XXBT/ZGBP translated) |
| `awaken` | Universal sent/received format: `Date,Type,Sent Quantity,Sent Currency,Received Quantity,…` |
| `generic` | Anything single-row-per-movement, via `[cex_csvs.mapping]` |

## Behaviour worth knowing

- Fiat movements (GBP/USD/EUR/…) are skipped as events — the crypto legs
  carry the tax story — and counted in `out/cex_import_diagnostics.csv`.
- Coinbase GBP spot prices are captured into
  `staging/cex_price_hints.jsonl` and feed the price book for free.
- Fiat fees (e.g. Coinbase "Fees and/or Spread") are noted on the event
  text for the reviewer; crypto fees become their own `fee` events.
- Unrecognised operations import as `unknown` + `needs_review` — they are
  never silently dropped, and they block tax calculation until classified.
- Events merge with wallet events in review exports and the ledger;
  `staging/cex_normalised_events.jsonl` is rebuilt in full on every run.
