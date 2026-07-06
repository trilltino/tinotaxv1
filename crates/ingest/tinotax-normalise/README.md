# tinotax-normalise

Raw-provider-data normalisation crate.

## Owns

- Reading immutable raw wallet pages.
- Converting provider-specific JSON into `NormalisedEvent` records.
- Assigning deterministic event IDs.
- Dedupe and conservative classification hints.

## Does Not Own

- Fetching raw API pages.
- Applying human review decisions.
- Calculating tax.

## Invariants

- Every normalised event should link back to source evidence.
- Low-confidence activity must remain reviewable.
- Unknown DeFi should not disappear or be silently reclassified.
