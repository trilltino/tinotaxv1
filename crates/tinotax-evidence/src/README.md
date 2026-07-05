# tinotax-evidence/src

Source modules for evidence-pack generation.

## Module Map

- `pack.rs` orchestrates pack assembly.
- `markdown.rs` writes human-readable pack documentation.
- `hmrc_questions.rs` drafts HMRC-facing answers.
- `raw_index.rs` indexes source evidence files.
- `platforms.rs` lists platforms and protocols used.
- `assumptions.rs` maintains human-supplied questionnaire data.

## Invariants

- Evidence packs should be understandable without reading source code.
- Unresolved items must be disclosed, not hidden.
- Pack generation should copy outputs, not mutate calculation inputs.
