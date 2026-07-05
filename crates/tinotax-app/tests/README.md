# tinotax-app/tests

Integration tests for the application orchestration layer.

## Owns

- End-to-end checks that cross crate boundaries.
- Regression coverage for the public command flow.

## Invariants

- Tests must use temporary directories and synthetic data.
- Tests must not call live provider APIs.
- Test fixtures should preserve auditability expectations such as hashes and
  stable IDs.
