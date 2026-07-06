# Security Policy

## Reporting vulnerabilities

Please do not open public issues for security vulnerabilities or accidental
exposure of client data. Email the maintainers privately with:

- the affected commit or release,
- the command or workflow involved,
- any relevant logs with secrets and wallet addresses redacted.

## Client data handling

Never commit real client data to this repository. This includes:

- `wallets.toml` or other files containing real wallet addresses,
- CEX exports and imported CSVs,
- raw API responses under project `raw/` folders,
- tax calculations, evidence packs, and review spreadsheets,
- API keys, `.env` files, and provider credentials.

Use `wallets.example.toml` for placeholders only. Real project folders such as
`demo-data/`, `fox-project/`, and `imports/` are gitignored by design.

## Secrets

Provider keys must be supplied through environment variables such as
`NEARBLOCKS_API_KEY`, `BLOCKSCOUT_API_KEY`, `PIKESPEAK_API_KEY`, and
`COINGECKO_API_KEY` / `COINGECKO_PRO_API_KEY`. Do not place keys in TOML
files committed to git.
