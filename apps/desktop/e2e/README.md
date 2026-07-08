# Desktop E2E Tests

Hermetic WebdriverIO + Tauri end-to-end flow for the desktop app. It drives a
real, debug-built Tauri binary against a seeded project fixture and exercises
the full user journey: opening a project, selecting a wallet, importing a CEX
CSV, reviewing rows, and viewing the Data/Wallet/HMRC tabs.

## What's in this folder

| Path | Purpose |
|---|---|
| `desktop.e2e.ts` | The single WDIO/Mocha spec — the seeded desktop workflow test. |
| `wdio.conf.ts` | WebdriverIO Testrunner config; launches the Tauri binary via `@wdio/tauri-service`. |
| `fixtures/seeded-project/` | A pre-built project folder (config, staging events, price observations) copied into a temp dir before each run. |

## How it runs

`desktop.e2e.ts` copies `fixtures/seeded-project` into a fresh OS temp
directory (`os.tmpdir()/tinotax-desktop-e2e-<timestamp>`) so the test never
touches a real project or mutates the checked-in fixture. It also writes a
throwaway `logs/run.log` and a small Kraken CSV export into that temp copy for
the CEX-import step, then removes the whole temp directory in `after()`.

The spec drives the UI purely through `document.querySelector` /
`textContent` assertions executed in-browser (via `browser.execute`), plus a
few small helpers at the bottom of the file:

- `setProject` / `setInputValue` — set a controlled input's value and fire a
  React-compatible `input` event.
- `setSelectValue` — same, for `<select>` elements.
- `clickButton` — finds a `<button>` by its trimmed text content and clicks it
  once it's enabled (polls up to 30s).
- `clickTestId` — same, but by `data-testid`.
- `waitForText` — polls `document.body.innerText` for a substring (up to
  120s), and on failure dumps the last seen body text to make failures
  debuggable without opening a browser.

`wdio.conf.ts` points `@wdio/tauri-service` at a debug Tauri binary:

- Binary path: `TINOTAX_DESKTOP_BIN` env var if set, otherwise
  `../../target/debug/tinotax-desktop.exe` (relative to `apps/desktop`).
- `driverProvider: "embedded"` — no separate `tauri-driver` process to manage.
- `WEBVIEW2_USER_DATA_FOLDER` is redirected to a throwaway temp directory
  before the runner starts, so the test's WebView2 profile (localStorage,
  cache, "last project" state) never shares or pollutes your real desktop
  app's user-data dir. That temp dir is best-effort cleaned up in
  `onComplete()`.

## Running the suite

From `apps/desktop`:

```bash
npm run e2e
```

This is `node scripts/run-tauri.mjs build --debug --no-bundle && wdio run
e2e/wdio.conf.ts` — it first builds a debug, unbundled Tauri binary, then runs
this WDIO suite against it.

Or from the repo root with `just`:

```bash
just e2e
```

(`just e2e` → `just desktop-e2e` → `cd apps/desktop; npm run e2e`.)

## Requirements

- Everything in the main [installation guide](../../../docs/installation.md)
  for building the desktop app (Node.js, Rust MSVC build tools, WebView2 on
  Windows).
- No chain/CEX API keys are required — the fixture project is fully seeded
  and the test only imports a local CSV file.

## Updating the fixture

`fixtures/seeded-project/` is a real (checked-in) project folder: a
`project.toml`, `staging/normalised_events.jsonl`, and
`staging/price_observations.jsonl`. It contains one Lisk Blockscout wallet
(`lisk_test`) and pre-normalised events matching the assertions in
`desktop.e2e.ts` (e.g. an `ETH` row and a taxable event referenced as
`e2e_sell`). If you change the spec's assertions, keep the fixture data and
`data-testid`/text expectations in sync — the fixture contains no real client
data and must stay that way.
