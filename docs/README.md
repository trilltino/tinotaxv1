# Documentation

This directory contains project-level documentation for users, accountants,
and maintainers.

## Reading Order

1. `architecture.md` explains the crate boundaries.
2. `commands.md` explains CLI and `just` startup commands.
3. `data_model.md` explains the core records that move through the pipeline.
4. `review_workflow.md` explains spreadsheet review and append-only edits.
5. `uk_tax_engine.md` explains the UK CGT and income calculation model.
6. `hmrc_evidence_pack.md` explains the final accountant/HMRC deliverable.
7. `analysis_and_visuals.md` explains the one-file analysis CSV and standard visuals.
8. `production_gate.md` explains the project-specific readiness gate.
9. `RUST_PRODUCTION_READINESS_CHECKLIST.md` maps the Rust production checklist
   to current repo gates and residual risks.
10. `PRODUCTION_READINESS.md` is the full general checklist this gate was
   derived from.

## Invariants

- Documentation should describe current shipped behaviour, not aspirational
  behaviour unless clearly labelled.
- Client data examples must use placeholders only.
- Tax methodology docs should point to code-owned tests where practical.
