# HMRC evidence pack

`pack hmrc --tax-year <year>` assembles `evidence_pack/<year>/` — the
folder that goes to the client/accountant, mapped to HMRC's standard 13
cryptoasset questions in `hmrc_questions_draft.md`:

| HMRC question | Answered by |
|---|---|
| 1. When did activities begin? | questionnaire + `platforms_protocols_used.csv` first_seen |
| 2. Full CGT calculations with S104 | `disposals_calculation.csv`, `s104_pool_movements.csv`, `s104_pool_opening_closing.csv` |
| 3. If S104 not applied, why | `assumptions_and_limitations.md` (it *is* applied) |
| 4. Commercial calculator used | `calculator_statement.md` |
| 5. Platforms/exchanges/protocols | `platforms_protocols_used.csv` |
| 6. Full unedited trading data | `raw_data_index.csv` + hashed originals under `raw/` |
| 7. Forks | questionnaire + `income_summary.csv` (category `fork`) |
| 8. Airdrops received and sold | `income_summary.csv` (`airdrop`) + `disposals_calculation.csv` |
| 9. Compensation for lost crypto | questionnaire + `income_summary.csv` (`compensation`) |
| 10. Employment/self-employment crypto | questionnaire + `income_summary.csv` |
| 11. Mining/staking | `income_summary.csv` (`staking_reward`, `mining_reward`, `misc_income`) |
| 12. Goods/services bought with crypto | questionnaire + `goods_or_services_spend` disposals |
| 13. Source of funds | `source_of_funds_notes.md` (from the questionnaire) |

## The questionnaire

Some questions cannot be answered from chain data. `questionnaire.toml`
(created at the project root on first `pack hmrc`) collects: when activity
began, source of funds + bank statement references, forks, compensation,
employment/PAYE, and goods/services spending. Fill it in and re-run the
pack; the README inside the pack flags when answers are still pending.

## Also in the pack

- `pricing_audit.csv` — where every GBP number came from
- `manual_review_decisions.csv` / `change_log.csv` — human decisions,
  latest and full history
- `unresolved_review_items.csv` — anything still open (read first)
- `wallet_addresses.csv` — declared wallets and exchange sources
- `assumptions_and_limitations.md` — method and known limits

Everything is a copy; re-running `pack hmrc` after new review/pricing/
calculation work regenerates the folder.
