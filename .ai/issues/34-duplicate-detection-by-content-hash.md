## Why
The same photo can exist under several folders after card imports and rclone syncs.

## What to do
- Store a hash of the original (or of the embedded preview) in the record.
- Report duplicates; do not delete anything.

## Done when
- A report lists duplicate groups with their keys.

## Triage
Blast radius 1 · Risk 0 · Complexity 1 · Size 1 = 3 → **Planned** track

_Source: docs/product/02_
