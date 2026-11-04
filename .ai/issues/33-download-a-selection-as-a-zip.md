## Why
Photographers want a set of files, sometimes originals. Originals mean download cost and big files.

## What to do
- Build zips from selected photo ids in a Lambda that streams to the derived bucket, then return a signed link.
- Offer full JPEG or original, with a size warning.
- Cap zip size.

## Done when
- A selection of 50 photos downloads as one zip.

## Triage
Blast radius 1 · Risk 1 · Complexity 1 · Size 1 = 4 → **Planned** track

_Source: TODO.md, P3 features_
