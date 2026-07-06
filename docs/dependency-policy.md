# Dependency Policy

TinoTax handles private wallet, CEX, pricing, and tax evidence. Dependencies are
therefore production inputs, not casual conveniences.

## Rules

- Use workspace dependencies from the root `Cargo.toml`.
- Keep `Cargo.lock` committed and run gates with `--locked`.
- Prefer established crates with active maintenance, small APIs, and compatible
  licenses.
- Do not add Git dependencies without a written reason and a fixed revision.
- Do not store API keys or wallet/source secrets in config examples.
- Run `just production-check` after dependency changes.
- Treat corrupt or unreadable project artifacts as errors, not silent cache
  resets.

## Review Checks

Every dependency change must consider:

- license accepted by `deny.toml`;
- RustSec advisories from `cargo audit`;
- duplicate major versions reported by `cargo deny`;
- duplicate versions reported by `cargo tree -d`;
- whether the crate parses untrusted input, performs HTTP/TLS, or handles money;
- whether the crate brings unsafe-heavy native, crypto, compression, or webview
  code into the transitive graph;
- whether a smaller standard-library or existing-workspace option is enough.

## Current Supply-Chain Gate

`cargo audit` checks advisories against `Cargo.lock`.

`cargo deny check` enforces source, license, advisory, and duplicate-version
policy for the Windows desktop/CLI production target
(`x86_64-pc-windows-msvc`) with all workspace features enabled. Duplicate
versions are warnings for now because transitive dependency graphs often need
upstream releases to converge.

The accepted license set includes permissive licenses plus:

- `MPL-2.0`, currently required by Tauri's CSS selector/parser transitive
  dependencies. This is accepted as file-level copyleft dependency code, not as
  a license for first-party code.
- `Apache-2.0 WITH LLVM-exception`, currently required by `target-lexicon` in
  native build stacks.

The deny policy intentionally ignores only these RustSec advisory IDs:

- `RUSTSEC-2025-0075`
- `RUSTSEC-2025-0080`
- `RUSTSEC-2025-0081`
- `RUSTSEC-2025-0098`
- `RUSTSEC-2025-0100`

All five are unmaintained `unic` crates pulled through Tauri `urlpattern`.
Cargo-deny records the reason inline in `deny.toml`. There is no safe upstream
replacement available from this workspace today, so this is tracked residual
risk, not a blanket advisory exemption. New vulnerabilities remain deny-by-
default.

Dependency unsafe is advisory-reviewed rather than globally forbidden. Tauri,
HTTP/TLS, OS, crypto, and compression crates may need unsafe internally; that is
accepted only through the dependency review process, never through first-party
unsafe code.
