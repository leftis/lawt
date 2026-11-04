## Why
Tiles show a flat colour until the thumbnail arrives. Thumbnails are 400 px JPEG at about 13 KB.

## What to do
- Store a blurhash or dominant colour in the record.
- Produce a 200 px and a 400 px WebP.
- Use `srcset` in tiles.
- Measure bytes per 100 tiles before and after.

## Done when
- The measured bytes drop and the placeholder matches the photo.

## Triage
Blast radius 1 · Risk 0 · Complexity 1 · Size 1 = 3 → **Planned** track

_Source: TODO.md, P1 performance_
