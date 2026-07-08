# Rust Optimisation — Scaling TinoTax to 100k+ Events

> Companion to [`performance.md`](performance.md). That doc states the *principles*
> (measure first, stream large consumers, no silent defaults). This one is the
> *deep dive*: concrete hot paths, file/line references, ordered fixes, and
> implementation sketches — triggered by the IOTA wallet blowing the project up
> to ~147k events.

---

## 0. Why this doc exists now

Fetching the IOTA EVM wallet took the `fox-project-lisk` project from one
13k-event wallet to three:

| Wallet | Chain | Normalised events |
|---|---:|---:|
| `lisk_main` | lisk-evm | 13,447 |
| `iota_main` | iota-evm | **134,053** |
| `near_foxboss` | near | 0 (needs paid key) |
| **Total** | | **147,500** |

`staging/normalised_events.jsonl` is now **147 MB**. IOTA alone is **10× Lisk**,
and it is dominated by noise:

| IOTA event type | Count | Tax relevance |
|---|---:|---|
| `fee` | 94,561 | native-token disposals, priced per **day** not per event |
| `contract_call` (all `amount:"0"`) | 39,057 | **non-taxable → bulk-ignore** |
| `token_transfer` | 420 | potentially taxable |
| `native_transfer` | 15 | potentially taxable |

The engine is *correct* at this scale (it computed fine via the CLI). The
problem is **the desktop path**: several commands each re-read and re-parse the
entire 147 MB file, and `load_review_rows` returns **all 147,500 rows over the
Tauri IPC bridge in one payload**. That is the wall we just hit.

---

## 1. TL;DR — priority order

| # | Change | Impact | Effort | Status |
|---|---|---|---|---|
| **1** | **Server-side pagination/filtering in `load_review_rows`** | 🔴 removes the IPC wall | M | **do first** |
| 2 | Backend parsed-event cache (mtime-keyed) shared across commands | 🔴 kills repeated 147 MB re-parse | M | |
| 3 | Faster JSONL read (streaming + capacity + `from_slice`) | 🟠 ~2–4× parse | S | |
| 4 | Binary/columnar staging cache derived at normalise time | 🟠 5–20× load | L | |
| 5 | Single "project snapshot" command to collapse the refresh fan-out | 🟠 4 parses → 1 | M | |
| 6 | Borrowed/`Cow` DTOs to cut clone+alloc on the hot page | 🟡 | S | |
| 7 | Ship a **release** desktop binary for real use | 🔴 (cheap) | S | |

> **Guardrail (from `performance.md`):** none of these may replace contextual
> errors with silent defaults, break append-only review/pricing state, drop raw
> evidence hashes, or introduce `unwrap`/`expect`/`panic`/`unsafe` (the justfile
> `policy-scan` enforces this). Every sketch below keeps `Result`/`?` and path
> context.

---

## 2. Measured baseline

- **Data:** 147,500 events / 147 MB JSONL; IOTA 134k of it.
- **Build:** dev/e2e runs the **debug** Tauri binary (`justfile` line 69,
  `run-tauri.mjs build --debug`). `Cargo.toml` has `[profile.release] lto = "thin"`
  but the app we exercise is unoptimised. Debug amplifies every parse/alloc cost
  below by roughly 5–50×.
- **Repeated work:** a single desktop "refresh" fans out to
  `get_project_status`, `get_project_paths`, `get_project_data_view`, and
  `get_wallet_insights`; then "Load rows" calls `load_review_rows`; then
  auto-classify/save reload again. **Each** of those that touches events calls
  `load_all_events` → a fresh 147 MB read + parse. No caching.

---

## 3. Hot-path analysis

### 3.1 `load_review_rows` — the bottleneck 🔴
[`crates/interface/tinotax-app/src/desktop_api.rs:580`](../crates/interface/tinotax-app/src/desktop_api.rs#L580)

```rust
let events = tinotax_review::load_all_events(&paths)?;      // parse ALL 147k
let overrides = tinotax_review::load_latest_overrides(&paths)?;
let mut rows = Vec::with_capacity(events.len());
for event in &events { rows.push(ReviewRowDto { /* ~30 owned String fields */ }); }
Ok(ReviewRowsResult { rows, /* … */ })
```

Every call:
1. reads + parses the whole 147 MB file,
2. builds **147,500 DTOs**, each ~30 `String`s (heap alloc per field, many `clone`s),
3. serialises the lot to JSON (~tens of MB),
4. ships it across IPC, where JS must `JSON.parse` and React must hold it.

The frontend already caps *rendering* at 250 rows and builds auto-classify
drafts by scanning the full `review.rows` in JS — so the cost lands anyway. This
is the single change with the highest payoff.

### 3.2 `read_jsonl` — per-line String + validate 🟠
[`crates/foundation/tinotax-store/src/jsonl.rs:65`](../crates/foundation/tinotax-store/src/jsonl.rs#L65)

```rust
for (i, line) in reader.lines().enumerate() {
    let line = line?;                       // allocates a String per line
    let record = serde_json::from_str(&line)?;
    records.push(record);                    // Vec grows without pre-reserve
}
```

At 147k lines: 147k transient `String` allocations, per-line UTF-8 validation,
and a `Vec` that reallocates as it grows. `serde_json::from_str` on borrowed
`&str` is fine, but the surrounding overhead is avoidable.

### 3.3 `load_all_events` — full materialise + re-sort, called everywhere 🔴
[`crates/review/tinotax-review/src/load.rs:12`](../crates/review/tinotax-review/src/load.rs#L12)

Loads wallet + CEX events into one `Vec` and `sort_by` on
`(timestamp, event_id)` string tuples over 147k items. The sort is O(n log n)
string compares; tolerable. The real cost is that **many commands call this**
(`load_review_rows`, `desktop_wallet_insights`, ledger build, diagnostics),
each paying the full read+parse with no shared result.

### 3.4 `desktop_wallet_insights` — another full load 🟠
[`crates/interface/tinotax-app/src/wallet_insights.rs:132`](../crates/interface/tinotax-app/src/wallet_insights.rs#L132)

Loads all events + overrides, builds an `event_counts` map, filters by
`source_id`, then loads the reviewed/priced ledger too. Fired on every refresh
and on every wallet-card click. With IOTA present it now parses 147 MB per click.

### 3.5 Tax `aggregate` — fine, but allocation-heavy 🟡
[`crates/valuation/tinotax-tax-uk/src/matching.rs:79`](../crates/valuation/tinotax-tax-uk/src/matching.rs#L79)

Single pass over all events into per-asset `BTreeMap<day, …>` with
`Vec<String> event_ids` per bucket. Correct and roughly O(n); the cost is many
small `String` clones. Only worth touching once loads are fixed and if the tax
run itself is profiled hot.

### 3.6 Data model is `String`-dense
`NormalisedEvent` and the DTOs own `String` for chain, symbol, addresses, hashes.
147k × ~10 strings = heavy allocation and cache pressure on every load. Interning
or `Arc<str>`/enums for low-cardinality fields (chain, event_type, direction)
would shrink footprint materially.

---

## 4. Detailed designs (top items)

### 4.1 Server-side pagination + filtering for review rows 🔴 (do first)

**Goal:** the IPC payload is one *page* (e.g. ≤500 rows) plus facet counts, not
147k rows.

Add a request type and change the command to filter/slice **before** building
DTOs:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewQuery {
    pub offset: usize,
    pub limit: usize,                 // clamp server-side, e.g. 1..=1000
    #[serde(default)] pub needs_review_only: bool,
    #[serde(default)] pub unknown_only: bool,
    #[serde(default)] pub asset: Option<String>,
    #[serde(default)] pub tax_year: Option<String>,
    #[serde(default)] pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPage {
    pub rows: Vec<ReviewRowDto>,      // only this page is materialised
    pub total: usize,                 // rows matching the filter
    pub grand_total: usize,           // rows in project
    pub needs_review_count: usize,    // facet for the UI badges
    pub assets: Vec<String>,          // distinct assets (for the dropdown)
    pub tax_years: Vec<String>,
    pub tax_event_types: Vec<String>,
    pub price_sources: Vec<String>,
}

pub fn load_review_page(project: &str, query: &ReviewQuery) -> Result<ReviewPage> {
    let (paths, _) = crate::open_project(project)?;
    let events = load_events_cached(&paths)?;        // §4.2
    let overrides = tinotax_review::load_latest_overrides(&paths)?;

    // 1) compute facets in one pass (cheap: counts + small sets)
    // 2) filter → collect matching indices
    // 3) slice [offset, offset+limit) and build DTOs for ONLY those
    // … full error/path context preserved, no unwrap …
}
```

Frontend: `App.tsx` moves filter state into the request, drops the 250-row
render cap in favour of real paging (Prev/Next or infinite scroll), and the
auto-classify action stops scanning 147k rows in JS — see §5.

**Why first:** it removes the IPC/JSON/React wall directly and unblocks using
Review on IOTA at all. Everything else is throughput polish on top.

### 4.2 Backend parsed-event cache (mtime-keyed) 🔴

One parse per file version, shared by every command in the session:

```rust
struct EventCache {
    path: Utf8PathBuf,
    modified: SystemTime,
    len: u64,
    events: Arc<Vec<NormalisedEvent>>,
}
// held in Tauri State<Mutex<Option<EventCache>>>, or a small LRU keyed by project

fn load_events_cached(paths: &ProjectPaths) -> Result<Arc<Vec<NormalisedEvent>>> {
    let meta = std::fs::metadata(paths.events_jsonl())
        .with_context(|| format!("stat {}", paths.events_jsonl()))?;
    // if cached.modified == meta.modified() && cached.len == meta.len() → reuse
    // else parse once, store Arc, return clone of Arc (cheap)
}
```

Invalidate on `normalise`, `import-cex`, and `save_review_overrides` (bump mtime
or clear). Turns the refresh→insights→load-rows→save sequence from ~4 full
parses into **one**. Must stat both wallet and CEX files; never serve stale data
past an mtime/len change.

### 4.3 Faster JSONL reading 🟠
[`jsonl.rs:65`](../crates/foundation/tinotax-store/src/jsonl.rs#L65)

- Pre-reserve: estimate capacity from `file_len / avg_line_bytes`.
- Prefer a **streaming byte parser** over `.lines()`:
  `serde_json::Deserializer::from_reader(reader).into_iter::<T>()` — avoids the
  per-line `String` and re-validation, keeps line context via the stream error.
- Or `read_to_string` once then `from_slice`/split on `\n` (one big alloc beats
  147k small ones).
- Evaluate **`simd-json`** for the parse itself (biggest single lever on parse
  time; must stay within the no-unsafe policy — `simd-json` uses `unsafe`
  internally, so gate it behind the store crate and keep the workspace rule via
  an allow-list, or benchmark `serde_json` with the above tweaks first).

Keep the error context (`parsing {path} line {n}`) — non-negotiable per the
error-safe rule.

### 4.4 Binary/columnar staging cache 🟠 (bigger lift)

JSONL stays as the human-readable, hashable evidence format. Add a **derived**
fast cache written during `normalise`:

- `staging/normalised_events.bin` via `bincode`/`postcard` (5–20× faster to load
  than JSON), **or**
- Arrow/Parquet columnar if we also want cheap column scans for filtering
  (filter on `needs_review`/`event_type` without touching the wide row).

Rule: the binary cache is *rebuildable from JSONL* and never the source of
truth. Guard with a schema-version header; fall back to JSONL on mismatch.

### 4.5 Collapse the refresh fan-out 🟠

One `get_project_snapshot(project, tax_year, review_query)` returning
`{ status, paths, data_view, insights, review_page }`, computed from a **single**
cached event load. Cuts the per-refresh parses and round-trips from ~4 to 1.

### 4.6 Trim the hot data model 🟡

For the paginated DTO path, serialise **borrowed** rows (`&str`/`Cow<str>`) to
avoid cloning ~30 fields × page. For the in-memory model, represent
`chain`/`event_type`/`direction` as `enum`s (already exist as enums upstream —
keep them enums end-to-end) and consider `Arc<str>` for repeated addresses.

---

## 5. Frontend implications (so the Rust work lands)

- `App.tsx`: replace `REVIEW_ROW_LIMIT` render-capping with real paging state
  (`offset`/`limit`) sent to `load_review_page`; move `filters` into the request.
- **Auto-classify at scale:** don't build 44k ignore-drafts in JS from a
  147k-row array. Add a backend `auto_classify_contract_calls(project)` that
  scans server-side and appends the `ignore` overrides in one call (auditable,
  same change-log path). This also fixes a latent memory spike in the current
  client-side approach once IOTA is loaded.
- Insights wallet-switch should hit the cache (§4.2), not re-parse.

---

## 6. Action plan (the immediate three)

1. **Server-side pagination/filtering for `load_review_rows`** (§4.1) — *return
   needs-review rows / a page at a time*. **This is the real bottleneck; do it
   first.** Pair with §4.2 (event cache) since the paginator wants a cached load.
2. **Auto-ignore the ~44k zero-value contract calls → rebuild ledger → read the
   2-wallet (Lisk + IOTA) numbers.** Prefer the backend auto-classify (§5) now
   that the client-side scan is 147k rows. Then `ledger build → price` to refresh
   the Wallet-Data meters, and inspect `disposals_calculation.csv` /
   `self_assessment_crypto_summary.csv` for the combined figures.
3. **Pricing: confirm how many IOTA assets CoinGecko actually lists** before
   committing to the paid key. Enumerate the distinct `asset_symbol` /
   `asset_contract` on `chain:"iota-evm"` disposal-eligible rows, check each
   against CoinGecko's coin list, and split into *auto-priceable* vs
   *needs-manual*. That tells us exactly what the CoinGecko Basic ($35/mo) key
   buys vs. what still needs manual prices.

---

## 7. Benchmarking & guardrails

- **Always benchmark in `--release`.** Add `criterion` benches for: `read_jsonl`
  (147k), `load_review_page` (cold vs cached), `calculate` (147k timeline),
  price-book lookup. Record before/after in `performance.md`'s measurement table.
- Track **peak RSS** on a full refresh with IOTA loaded (the number that matters
  for the desktop webview host).
- Every change must pass `just policy-scan` (no `unwrap`/`expect`/`panic`/
  `unsafe`) and preserve row/path/event error context and append-only review
  state. If `simd-json` is adopted, isolate its `unsafe` behind the store crate
  and document the exception in [`unsafe.md`](unsafe.md).

---

## 8. Appendix — current data profile

```
project fox-project-lisk
  lisk-evm   13,447 events   (fee-heavy, ~5,252 zero-value contract_calls)
  iota-evm  134,053 events   (94,561 fee, 39,057 zero-value contract_call,
                              420 token_transfer, 15 native_transfer)
  near           0 events    (blocked: NEARBLOCKS_API_KEY)
  total    147,500 events    staging/normalised_events.jsonl = 147 MB
```

Real taxable surface after ignoring zero-value contract calls: on the order of
**hundreds** of movements across both chains — the volume is almost entirely
gas-fee and contract-call noise, which is exactly why filtering/paging (not
raw throughput) is the priority.
