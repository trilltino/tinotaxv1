# Data model

Every public struct belongs to exactly one of these classes; the class
dictates whether it may ever be edited.

| Class | Examples | May be edited? |
|---|---|---|
| **Raw evidence** | files under `raw/`, `RawManifest` | Never — copied, hashed, kept |
| **Normalised derived data** | `NormalisedEvent` (`staging/normalised_events.jsonl`, `staging/cex_normalised_events.jsonl`) | Never by hand — regenerate |
| **Human override** | `ReviewOverride` (`staging/review_overrides.jsonl`), `questionnaire.toml`, `opening_pools.toml`, manual price CSVs | Yes — this *is* the human input |
| **Reviewed ledger** | `TaxLedgerEvent` (`staging/reviewed_ledger.jsonl`) | Never — derived from events + overrides |
| **Priced ledger** | `TaxLedgerEvent` with GBP values (`staging/priced_ledger.jsonl`), `PriceObservation` | Never — derived from ledger + price book |
| **Tax output** | `UkTaxCalculation` and everything in `tax/<year>/`, `evidence_pack/<year>/` | Never — derived |

## NormalisedEvent (tinotax-core)

One source-traceable movement of value affecting one wallet or exchange
account. Amounts are always positive; `direction` carries in/out; the raw
chain amount survives in `raw_amount`. `event_id` is deterministic
(BLAKE3 of the identifying fields), so re-imports converge.

## ReviewOverride (tinotax-core)

The latest decision per `event_id` wins; the file is append-only so the
full history is preserved for `change_log.csv`. Fields: coarse
`user_action` (milestone-1 vocabulary), precise `user_tax_type`, corrected
asset/quantity, `user_proceeds_gbp` / `user_cost_gbp` / `user_income_gbp` /
`user_fee_gbp`, `user_price_source`, `user_note`.

## TaxLedgerEvent (tinotax-core)

The tax engine's input row: timestamp, tax year, platform/chain/wallet,
`tax_event_type`, asset, quantity, the four GBP value fields, price
provenance (`price_source`, `price_confidence`), `review_status`, and
back-references (`source_event_ids`, `source_refs`).

### TaxEventType

```text
acquisition, disposal, swap_disposal, swap_acquisition,
transfer_in, transfer_out, bridge_in, bridge_out, fee,
staking_reward, mining_reward, airdrop, fork,
employment_income, self_employment_income, misc_income, compensation,
goods_or_services_spend, ignore, unknown
```

Grouped by effect:

- **pool entries**: acquisition, swap_acquisition, airdrop, fork + all income
- **disposals**: disposal, swap_disposal, goods_or_services_spend, fee
- **income at receipt**: staking/mining/employment/self-employment/misc/compensation
- **no tax effect**: transfer_in/out, bridge_in/out, ignore
- **blocks calculation**: unknown

## PriceObservation (tinotax-core)

`(asset, day) → GBP price` with source (`manual`, `cex`, `coingecko`,
`user_provided`) and confidence (`high`/`medium`/`low`/`missing`). The
price book merges observation files; higher confidence wins, later fetch
breaks ties; lookups fall back ±3 days at reduced confidence.
