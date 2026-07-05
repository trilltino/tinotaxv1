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

## Invariants

- Documentation should describe current shipped behaviour, not aspirational
  behaviour unless clearly labelled.
- Client data examples must use placeholders only.
- Tax methodology docs should point to code-owned tests where practical.
