# Accountant review guide

What to check, in order, when a TinoTax evidence pack lands on your desk.

## 1. Open items first

`unresolved_review_items.csv` — anything `blocker` was **excluded** from
the numbers (only possible when the preparer used `--allow-unpriced`);
anything `warning` is included but was flagged. An empty file is the goal.

## 2. The judgement calls

`manual_review_decisions.csv` lists every human classification, with notes;
`change_log.csv` is the full history. Points that deserve attention:

- rows classified `transfer_in`/`transfer_out` (own-wallet moves — the
  claim is "not taxable"; the addresses are in the ledger CSVs)
- `ignore` rows (spam tokens, dust, failed transactions)
- income category choices (`staking_reward` vs `misc_income` vs
  employment) — see the assumptions doc for the defaults
- any `user_*_gbp` values typed by the reviewer — sources should be in
  `user_note` or `pricing_audit.csv`

## 3. The valuations

`pricing_audit.csv`: every derived GBP value with its price, source
(`manual` / `cex` / `coingecko` / `user_provided`), the day the price was
observed, and confidence. Filter `confidence != high` for stand-in prices.

## 4. The computation

`disposals_calculation.csv` shows, per disposal, exactly how much was
matched same-day, 30-day and from the pool, at what cost, with notes.
`s104_pool_movements.csv` re-derives every pool balance step by step;
opening/closing balances are in `s104_pool_opening_closing.csv`. Spot
checks are practical: each row cites its ledger event ids, which cite the
raw evidence file (see `raw_data_index.csv` for hashes).

## 5. The human answers

`hmrc_questions_draft.md` marks anything still **pending** from
`questionnaire.toml` (source of funds, employment/PAYE, forks,
compensation, goods/services). These need the client, not the data.

## 6. What TinoTax deliberately did not do

No annual exempt amount, no loss elections, no treatment decisions on
NFTs/DeFi beyond what the reviewer classified — see
[assumptions_and_limitations.md](assumptions_and_limitations.md).
