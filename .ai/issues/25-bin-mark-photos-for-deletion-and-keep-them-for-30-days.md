## Why
Users need a safe way to remove photos from the timeline and get them back within 30 days. Open decision: what does purge delete? The pipeline must never delete originals.

## What to do
- Add `deleted_at` to the record. `publish` leaves binned photos out of the timeline and writes `index/bin.json`.
- Bin view with restore and a 30-day countdown.
- A scheduled purge after 30 days.
- **Decide first**: purge removes only generated files and the record and leaves the original untouched (proposed), or lists originals in a report for the owner to delete by hand. Deleting originals is acceptable only where the originals live in a bucket we own, with versioning on.

## Done when
- Decision recorded.
- Tests cover restore on day 29 and purge on day 30.

## Triage
Blast radius 2 · Risk 2 · Complexity 1 · Size 1 = 6 → **Spec** track

_Source: TODO.md, 'Next features'_
