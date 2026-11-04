## Why
Each record already stores make, model, lens, ISO, aperture, shutter, focal length and GPS. Nothing uses it yet.

## What to do
- Add the fields needed for filtering to the month files, or a small search index file.
- Filter bar: camera, lens, ISO range, focal length range, year.
- Decide if client-side filtering over loaded months is enough, or if a query service is needed (see the Iceberg/Athena decision issue).

## Done when
- Filtering by lens and year returns the same photos as a query over the records.

## Triage
Blast radius 1 · Risk 0 · Complexity 1 · Size 1 = 3 → **Planned** track

_Source: TODO.md, P3 features_
