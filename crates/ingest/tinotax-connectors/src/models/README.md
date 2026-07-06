# Provider Models

This directory contains structs used to deserialize provider API responses.

## Invariants

- Models should mirror provider JSON closely.
- Optional fields are preferred when public APIs are inconsistent.
- Domain interpretation belongs in `tinotax-normalise`, not in these models.
