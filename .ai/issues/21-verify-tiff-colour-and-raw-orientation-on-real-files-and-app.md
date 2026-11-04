## Why
The 28 TIFFs in the library rendered, but colour is unchecked: embedded ICC profiles are ignored, so wide-gamut exports may look flat. RAW orientation was checked on Ricoh DNG and Fujifilm JPEG only. The RAW orientation rule is a heuristic.

## What to do
- Compare thumbnails with the originals for several TIFFs.
- Apply the embedded profile to sRGB with `moxcms` when present.
- Check portrait shots from RAF, NEF and ARW cameras.

## Done when
- Side-by-side checks are recorded in the PR and unit tests cover the profile conversion.

## Triage
Blast radius 0 · Risk 0 · Complexity 2 · Size 1 = 3 → **Spec** track

_Source: TODO.md, 'Found during T9'_
