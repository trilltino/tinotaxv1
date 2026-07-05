# tinotax-config/src

Source modules for typed project configuration.

## Module Map

- `project_config.rs` contains the TOML-facing schema and validation logic.
- `lib.rs` exposes the crate API.

## Invariants

- Config validation should fail early with actionable errors.
- Example config values should remain placeholders.
- Provider secrets should be referenced by environment variable names, not
  committed values.
