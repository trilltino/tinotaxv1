# Go-Live Runbook — Closing the Last Gaps Before Handover

> End-to-end plan for the five items standing between "the tool works on the dev
> machine" and "Fox can run this and file from it." Each item: **what & why →
> steps → acceptance → owner.** Companion to [`rust-optimisation.md`](rust-optimisation.md)
> and the requirements audit.
>
> Snapshot at time of writing (`fox-project-lisk`, Lisk + IOTA): 147,500 events;
> 123 disposals; net **–£150.62 (provisional)**; **95,076 rows unpriced**;
> income **£0**; NEAR not fetched.

---

## Priority order
1. **CoinGecko price-fetch** (item 4) — unblocks real numbers; everything else is moot until this works.
2. **Income £0 confirmation** (item 1) — correctness risk; depends on pricing + review.
3. **Q5 protocol naming** (item 2) — biggest remaining *evidence* gap.
4. **Release installer** (item 5) — needed to physically hand it over.
5. **Questionnaire PDF** (item 3) — Fox's disclosures; can happen any time.

---

## 1. Income shows £0 — confirm real, not missed classification

**What & why.** `income_summary.csv` is empty. If any inbound receipt is actually
staking/airdrop/mining **income** (taxable at receipt) rather than a plain
acquisition, income tax is understated — a filing error.

**Root cause found.** There are **412 inbound receipts** (direction `in`) —
**196 Lisk + 216 IOTA** — and *all default to `acquisition`*. None have been
reviewed and reclassified as income, so income is £0 by omission, not by fact.
Distinct inbound assets include USDT, WETH, WIOTA, WBTC, ETH, LSK, IOTA, plus
ion* lending tokens and `UNI-V3-POS` (LP positions).

**Steps.**
1. In **Review**, isolate inbound receipts to triage:
   - `Type = token_transfer` and `Type = native_transfer`, per chain, and read the counterparty/method.
   - Cross-check the recurring Lisk method `0x00af0a6d` (5,184×) — if it's a *claim/reward*, the paired inbound token is **income**, not an acquisition.
2. Reclassify genuine income to `staking_reward` / `airdrop` / `mining_reward` /
   `misc_income` (bulk-select where a counterparty is a known reward contract).
3. Leave true purchases/self-transfers as `acquisition` / `transfer`.
4. **Rebuild ledger** → **Build evidence pack**; re-open `income_summary.csv`.
5. Command-line cross-check of the inbound set:
   ```bash
   grep '"direction":"in"' staging/normalised_events.jsonl \
     | grep -oE '"(event_type|asset_symbol|counterparty|method)":"[^"]*"'
   ```

**Acceptance.** Every one of the 412 inbound receipts has been reviewed; income
in the pack reflects reality; **or** a documented statement that all inbound
receipts are acquisitions/transfers with no income element (with rationale).

**Owner.** Accountant / reviewer (with tool assist). *Blocked partly on pricing —
income receipts also need a GBP value at receipt.*

---

## 2. Q5 — protocol / DEX naming

**What & why.** HMRC Q5 wants named exchanges, DEXs and protocols.
`platforms_protocols_used.csv` currently lists **only chains** (`iota-evm`,
`lisk-evm`) — not the protocols behind the contract interactions.

**Steps.**
1. Add a **counterparties report** to the evidence pack: distinct
   `to_address` / `counterparty` + `method`, with `event_count` and
   first/last-seen, per wallet. (New writer in
   [`tinotax-evidence`](../crates/output/tinotax-evidence/src/platforms.rs),
   surfaced in the Data Viewer.)
2. Maintain a **known-address → name** lookup (Lisk/IOTA routers, `ion*`
   lending, Uniswap V3 `UNI-V3-POS`, bridges). Label matches; leave the rest as
   "unknown — please name."
3. Feed named results into `platforms_protocols_used.csv` (kind = `dex` /
   `protocol` / `bridge`, not just `chain`).

**Acceptance.** The report names the DEXs/protocols used (or clearly flags the
unknown contract addresses for a human to name); Q5 is answerable from data.

**Owner.** Dev builds the report; accountant/Fox names the unknowns.

---

## 3. Questionnaire PDF — Fox's answers

**What & why.** Q1, 5, 7–13 are **taxpayer disclosures** (start date, forks,
airdrops, compensation, employment, mining/staking, spends, source of funds).
Only Fox can answer; the tool must not fabricate them. PDF not yet exported.

**Steps.**
1. Fox fills the **HMRC Questionnaire** tab (13 questions; Y/N + free text).
2. Click **Export PDF** → `out/hmrc_questionnaire_responses.pdf` (+ `questionnaire.toml`).
3. **Build evidence pack** so `source_of_funds_notes.md` (Q13) picks up the answer.
4. Accountant reviews the answers for consistency with the data.

**Acceptance.** `hmrc_questionnaire_responses.pdf` exists; Data Viewer
"Questionnaire PDF" = **Ready**; answers reviewed.

**Owner.** Fox (answers) + accountant (review).

---

## 4. Prepare price-fetch — verify end-to-end against CoinGecko

**What & why.** `Prepare` calls `prices_fetch(project, "coingecko")` when a key
is set, but this has **never been run against a real key**. Provider string,
GBP currency, historical-range limits, and rate limiting are all unverified.
Until it works, ~95k rows stay unpriced and the CGT number is provisional.

**Steps.**
1. Obtain a CoinGecko key (Demo free, or Basic ~$35/mo for full history).
2. **Settings → API keys** → paste CoinGecko key → Save (sets `COINGECKO_API_KEY`).
3. Run it headless first for a clean signal:
   ```bash
   cargo run -q -p tinotax-cli -- prices fetch --project ./fox-project-lisk --provider coingecko
   cargo run -q -p tinotax-cli -- ledger price --project ./fox-project-lisk
   cargo run -q -p tinotax-cli -- calculate uk --project ./fox-project-lisk --tax-year 2024-2025
   ```
4. Then verify **Prepare** in-app does the same (fetchPrices auto-on with a key).
5. **Check:**
   - `provider = "coingecko"` matches what `fetch_missing_prices` expects
     ([provider.rs](../crates/valuation/tinotax-pricing/src/provider.rs)).
   - Prices are **GBP** (not USD).
   - Demo tier's **365-day** history limit vs the 2024-25 data age (Basic removes it).
   - Rate-limit/backoff behaviour on a few hundred (asset, day) pairs.

**Acceptance.** After fetch: unpriced rows drop to only genuinely unlistable
tokens (~240 ion*/LP/`UNI-V3-POS`); `pricing_audit.csv` shows `coingecko`
sources; `calculate uk` completes and the net **changes from –£150.62** to a
fully-priced figure; readiness gate no longer fails on price blockers.

**Risks.** IOTA-ecosystem tokens (`ion*`, `WIOTA`, `DEEPR`) and LP NFTs may not
be on CoinGecko at all → **manual prices** required regardless of the key.

**Owner.** Dev (verify), then accountant (manual-price the unlistable tail).

---

## 5. Release installer (`tauri build`)

**What & why.** Only a `--debug --no-bundle` binary has ever been built. To put
TinoTax on Fox's PC you need a real installer.

**Steps.**
1. Confirm bundle config in `apps/desktop/src-tauri/tauri.conf.json` (identifier,
   product name, icons, version).
2. Build the release bundle:
   ```bash
   cd apps/desktop && npx tauri build
   # → target/release/bundle/  (Windows .msi and/or NSIS .exe)
   ```
3. Install on a **clean Windows profile** (not the dev tree) and verify:
   - app launches (WebView2 present on Win10/11);
   - **API keys persist** across relaunch (`settings.toml` in app config dir);
   - **Prepare** runs a wallet end-to-end;
   - Data Viewer opens/downloads artifacts.
4. **Code signing** — an unsigned installer triggers SmartScreen. Decide whether
   to sign (recommended for a client) or document the "More info → Run anyway".

**Acceptance.** A single installer that installs on a clean machine, persists
keys, and completes a Prepare run; SmartScreen behaviour understood/handled.

**Owner.** Dev.

---

## Definition of done (handover-ready)
- [ ] CoinGecko price-fetch verified; calc fully priced (item 4)
- [ ] All 412 inbound receipts reviewed; income confirmed/justified (item 1)
- [ ] Protocols/DEXs named for Q5 (item 2)
- [ ] Questionnaire answered + PDF exported (item 3)
- [ ] Signed/tested release installer produced (item 5)
- [ ] Evidence pack rebuilt after the above; readiness gate passes (or residual
      gaps are documented and accepted)
- [ ] NEAR wallet fetched (needs NearBlocks key) — *separate from these five but
      required for a complete return*
