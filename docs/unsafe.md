# Unsafe Rust Policy

First-party TinoTax crates use safe Rust only.

## Rule

No `unsafe` block, `unsafe fn`, or unsafe trait implementation may be added to
first-party code. The workspace enforces this with `unsafe_code = "forbid"` and
`just policy-scan`.

Changing this rule requires:

- a short design note explaining why safe Rust is insufficient;
- a reviewer who understands the invariant;
- focused tests around the boundary;
- a follow-up entry in this document.

## Current State

No first-party unsafe code is approved.

Transitive dependencies may contain unsafe code. That risk is managed through
crate selection, `cargo audit`, `cargo deny`, dependency review, and advisory
unsafe reports. It is not treated as approval to add first-party unsafe code.
