# tinotax-cli/src

Source files for the command-line entry point.

## Invariants

- CLI code should parse and dispatch only.
- Adding a command here should usually require adding orchestration in
  `tinotax-app`.
- User-facing help text should stay short and operational.
