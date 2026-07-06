# Rust Production Readiness Checklist Application

This document maps `C:\Users\isich\Downloads\RUST_PRODUCTION_READINESS_CHECKLIST.pdf`
to this repository. TinoTax is treated as a local Windows-first CLI/desktop
tool for sensitive financial evidence, not as a hosted service.

## Required Gates

Run the local production gate before merging or delivering outputs:

```bash
just production-check
```

The gate records locked metadata, runs the static policy scan, checks
formatting, Clippy, type checking, tests, docs, RustSec advisories, and cargo
deny policy. Cargo-deny is scoped to the Windows production target declared in
`deny.toml`; `cargo audit` still scans the full lockfile and reports tracked
transitive warnings. The static scan must find zero first-party matches for:

```text
.unwrap() .expect() unwrap_err() expect_err() panic!() todo!() unimplemented!() unreachable!()
unsafe { ... } unsafe fn unsafe impl
```

Run project gates before client/accountant delivery:

```bash
just preflight wallets.toml ./fox-project
just readiness ./fox-project
```

## Checklist Crosswalk

| # | Area | Status | Evidence / rule |
|---|---|---|---|
| 1 | Rust fit, product context, runtime reality | must | Local evidence tool; no hosted uptime promise; production means repeatable private project output. |
| 2 | Crate, workspace, module architecture | must | Modular workspace under `crates/foundation`, `ingest`, `review`, `valuation`, `output`, `interface`, plus `apps/desktop`. |
| 3 | Toolchain, edition, MSRV, compiler policy | must | `rust-toolchain.toml`, workspace edition, CI toolchain pin. |
| 4 | Cargo manifests, lockfiles, build inputs | must | `Cargo.lock` committed; all gates use `--locked`. |
| 5 | Dependency selection and governance | must | `docs/dependency-policy.md`, workspace dependencies, `cargo deny check`. |
| 6 | Rust supply-chain security | must | `cargo audit`, `cargo deny check`, duplicate-version review. |
| 7 | Public API and ergonomics | must | CLI names, crate package names, Tauri command names, and project formats remain stable. |
| 8 | Type design and invariants | must | Core domain types in `tinotax-core`; money uses `Decimal`, not floats. |
| 9 | Error handling and panics | must | See the deep error-handling section below; no unchecked extraction or panic shortcuts. |
| 10 | Unsafe Rust and soundness | must | Workspace `unsafe_code = "forbid"` and static unsafe scan. |
| 11 | FFI, ABI, native boundaries | risk | Tauri/webview/native-dialog/open crates are dependency boundaries; first-party code has no FFI. |
| 12 | Concurrency and shared state | must | Async used for provider/runtime orchestration; tax/domain calculations stay deterministic. |
| 13 | Async runtime and task lifecycle | must | Tokio/Tauri tasks are bounded to workflows and provider calls; blocking work is explicit. |
| 14 | Cancellation, retries, backpressure | risk | Fetchers use resumable cursors and rate pacing; live provider retry policy stays conservative. |
| 15 | Memory and resource management | must | Files are project-scoped; raw evidence immutable; generated artifacts regenerable. |
| 16 | Performance and capacity | risk | `docs/performance.md`; optimize only after measuring large project bottlenecks. |
| 17 | Build profiles and artifacts | must | Thin LTO release profile; release docs record metadata. |
| 18 | CI, formatting, linting, static analysis | must | CI mirrors the Rust production gate and policy scan. |
| 19 | Testing and verification | must | Workspace all-target tests, desktop unit/E2E, cleanup/review/tax pipeline tests. |
| 20 | Serialization and input boundaries | must | CSV/JSON/TOML parsing uses row/path context and typed domain validation. |
| 21 | Crypto, randomness, secrets | risk | No app cryptography beyond hashing; API keys read from env and never examples. |
| 22 | Networking, HTTP/TLS | risk | Provider clients are isolated in connector/pricing crates; dependency TLS risk is governed. |
| 23 | Persistence and data access | must | Filesystem-only project store; raw data hashed; human state append-only. |
| 24 | Observability and diagnostics | must | `tracing`, diagnostics outputs, readiness and preflight gates. |
| 25 | Configuration and environment | must | `tinotax-config` validates project config; env keys surfaced by preflight. |
| 26 | Containers and OS targets | n/a | No container runtime in v1; Windows-first local usage. |
| 27 | Runtime operations and shutdown | risk | CLI stages are resumable; fetch cursors limit restart loss. |
| 28 | Security hardening beyond memory safety | must | No client data in git; supply-chain gates; deletion constrained to `ProjectPaths`. |
| 29 | Multi-platform support | risk | Windows-first, Rust core cross-platform; desktop E2E is local-first. |
| 30 | WASM, embedded, no_std | n/a | Not a target. |
| 31 | Macros, code generation, build scripts | must | Tauri build is the only native app generation path; no first-party proc macros. |
| 32 | Feature flags and optional graph | must | All workspace features are checked together; no hidden production feature set. |
| 33 | Publishing, SemVer, deprecation | risk | Not currently published; public CLI/project formats are treated as stable. |
| 34 | Rustdoc and developer experience | must | `cargo doc --workspace --all-features --no-deps --locked`; crate/module READMEs. |
| 35 | Developer workflow and reproducibility | must | `just --list`, pinned toolchain, locked metadata, documented command flow. |
| 36 | Incident debugging and forensics | must | Raw manifests, audit manifests, change logs, pricing audit, evidence pack. |
| 37 | Must-answer production gate | must | `just production-check` plus `just readiness <project>`. |
| 38 | Missing iceberg items | risk | Benchmarks/geiger/vet are advisory next steps unless dependency risk changes. |
| 39 | Suggested CI gate | must | CI runs metadata, policy scan, fmt, clippy, check, test, doc, audit, deny. |
| 40 | Sources and further reading | must | PDF checklist plus project docs in `docs/`. |

## Error Handling And Fallibility Policy

TinoTax treats error handling as part of the tax evidence model. A failure must
either block the unsafe output, produce an auditable warning, or be an explicit
best-effort UI/logging operation.

Hard rules:

- First-party Rust must not use unchecked `.unwrap()`, `.expect()`,
  `.unwrap_err()`, `.expect_err()`, direct `panic!`, `todo!`, `unimplemented!`, or
  `unreachable!`.
- Domain crates expose typed errors when callers need to distinguish causes:
  `CoreError`, `ConfigError`, and `TaxError`.
- App, CLI, connector, report, and desktop orchestration may use `anyhow`, but
  every IO/parse/network boundary must add operation context such as path, row,
  event id, tax year, provider, or workflow step.
- Tests return `Result` and use `?` for setup failures. Assertion macros remain
  allowed because they are the idiomatic way to express expected test outcomes.
- Directory walks, raw manifests, audit manifests, cleanup plans, and evidence
  indexes must not skip IO errors silently. They propagate errors with context.
- Optional parsing may use `.ok()` only when absence is the intended data model:
  optional env vars, optional provider numeric fields, optional token decimals,
  lookup misses, or CSV cells where missing value is explicitly allowed.
- Display/export defaults such as empty CSV cells are allowed only after the
  upstream data has already been validated or intentionally modelled as
  optional.
- Tauri commands return full error-chain text to the frontend. Workflow helpers
  stop on the first Rust error and emit a failure log event.
- Cleanup operations only delete paths derived from `ProjectPaths`; protected
  human/audit state remains outside delete plans.

User-facing error text should answer four questions when possible:

1. What operation failed?
2. Which file, row, event id, provider, tax year, or command step was involved?
3. Whether the output was blocked, skipped with warning, or left unchanged.
4. Which command or human action should happen next.

## Residual Risk Register

- Transitive dependencies, especially Tauri/webview, HTTP/TLS, OS, crypto, and
  compression crates, may contain unsafe internals. This is governed through
  dependency review, `cargo audit`, `cargo deny`, and advisory unsafe reports.
- The Windows cargo-deny gate allows `MPL-2.0` and
  `Apache-2.0 WITH LLVM-exception` because they are required by current Tauri
  transitive crates. First-party code remains `MIT OR Apache-2.0`.
- Tauri currently pulls unmaintained `unic` crates through `urlpattern`; the
  affected RustSec IDs are documented in `deny.toml` and
  `docs/dependency-policy.md` until an upstream replacement exists.
- Duplicate dependency versions are currently reviewed rather than denied
  because the desktop and HTTP stacks depend on upstream convergence.
- Live provider behavior can change. Hermetic E2E is required; live-provider
  tests remain manual or release-only.
- Final tax treatment, DeFi classification, source-of-funds answers, and manual
  GBP prices require human/accountant review.
