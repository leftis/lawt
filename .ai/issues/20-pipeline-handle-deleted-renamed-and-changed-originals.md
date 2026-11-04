## Why
`plan` only adds photos that have no record. If an original is deleted or renamed (rclone does this), its record and renditions stay and the photo remains in the timeline. If an original changes, nothing re-renders it unless `force` is set.

## What to do
- In `plan`, compare the listing with the records.
- Mark records whose original is gone (hide them from the timeline, keep the files for now).
- Re-render when size or last-modified differs from the record.
- Never delete anything in the photo bucket.

## Done when
- Tests with the in-memory ports cover delete, rename and edit.

## Triage
Blast radius 1 · Risk 1 · Complexity 1 · Size 1 = 4 → **Planned** track

_Source: docs/adr/0001-iceberg-catalog.md, gap 7_
