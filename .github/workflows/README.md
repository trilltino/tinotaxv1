# GitHub Workflows

This directory contains GitHub Actions workflow definitions.

## Workflows

- `ci.yml` runs formatting, clippy, tests, and docs for the Rust workspace.
- `gitleaks.yml` scans commits and pull requests for committed secrets.

## Maintenance Notes

- Normal CI must stay deterministic and fixture/local-data based.
- Live provider tests should remain opt-in through environment flags.
- Workflows must never upload raw client data, tax outputs, or evidence packs.
