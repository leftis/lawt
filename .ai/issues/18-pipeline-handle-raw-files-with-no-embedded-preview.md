## Why
One of 7,826 photos failed: `100_FUJI/DSCF0025.RAF` has no readable embedded preview (probably truncated). It shows a placeholder tile.

## What to do
- Check whether the file is truncated or just unusual.
- If a real RAF can lack a preview, add a fallback that develops the raw data with `rawler` at reduced size.
- Keep the `failed` status and the reason in the record when nothing works.

## Done when
- A RAW with no preview either renders through the fallback or is clearly marked as failed with a reason.

## Triage
Blast radius 0 · Risk 0 · Complexity 1 · Size 0 = 1 → **Direct** track

_Source: TODO.md, 'After the first full run'_
