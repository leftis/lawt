## Why
Users want to keep sets of photos. This needs a place to store user choices that is not the rebuilt read model.

## What to do
- Define storage for user metadata (an object per album or a small table).
- Add favourite and album routes and UI.
- Make `publish` leave user metadata alone.

## Done when
- Favourites survive a full rebuild of the timeline.

## Triage
Blast radius 2 · Risk 1 · Complexity 1 · Size 1 = 5 → **Spec** track

_Source: TODO.md, P3 features_
