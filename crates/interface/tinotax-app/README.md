# tinotax-app

Application orchestration crate for the CLI.

## Owns

- Turning parsed CLI commands into pipeline calls.
- Loading project configuration and creating `ProjectPaths`.
- Sequencing fetch, normalise, diagnose, review, ledger, pricing, tax, and
  evidence commands.

## Does Not Own

- CLI parsing.
- Provider-specific API details.
- Tax calculation rules.
- Raw storage primitives.

## Key Files

- `src/lib.rs` contains shared project-loading helpers.
- `src/pipeline.rs` contains most command workflows.
- `src/run_demo.rs` runs the demo ingestion pipeline.
- `tests/` contains application-level integration tests.
