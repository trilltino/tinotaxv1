# tinotax-cli

Thin command-line crate.

## Owns

- Clap command definitions.
- Logging/tracing setup.
- Dispatching parsed commands into `tinotax-app`.

## Does Not Own

- Business logic.
- Project file layout.
- Provider, pricing, review, ledger, tax, or evidence behaviour.

## Key Files

- `src/cli.rs` defines the public command surface.
- `src/main.rs` performs command dispatch.
