# Contributing

TinoTax is designed around reviewed, immutable source data. Changes should
preserve these boundaries:

- CLI code parses arguments only.
- Connector code fetches source data only.
- Raw data is append-only and must not be silently overwritten.
- Review code records human decisions as overrides.
- Pricing code records source, confidence, and audit information.
- UK tax code consumes reviewed, priced ledger events only.

Before opening a pull request, run:

```bash
just check
```

Do not include live client data, wallet addresses, exchange exports, tax
outputs, evidence packs, `.env` files, or API keys in commits.
