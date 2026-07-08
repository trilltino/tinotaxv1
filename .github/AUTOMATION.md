# .github

This directory owns repository automation metadata for GitHub.

## Contents

- `workflows/` contains CI and repository scanning workflows.

## Invariants

- Workflows must not require live API keys or real client data.
- Any secret-scanning workflow should be able to run on public pull requests.
- Repository automation belongs here, not in crate code or project output
  folders.
