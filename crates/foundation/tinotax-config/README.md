# tinotax-config

Configuration parsing and validation crate.

## Owns

- Reading `wallets.toml` and `project.toml`.
- Validating wallet, provider, and CEX CSV entries.
- Turning config files into typed records used by app workflows.

## Does Not Own

- Fetching provider data.
- Creating project folders.
- Reading secrets directly except through environment-variable names declared
  in config.
