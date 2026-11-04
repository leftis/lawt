## Why
The timeline is the default. The old gallery (`Gallery.jsx` 693 lines, `ImageCard.jsx`, `Header.jsx`), the legacy API client and the unused `aws-amplify` dependency still ship in the bundle (74 KB gzip).

## What to do
- Wait until the owner has used the timeline for a while and says go.
- Delete the old components, `api/legacy.ts` and `VITE_UI=legacy`.
- Port `Login.jsx` to `Login.tsx`.
- Remove `aws-amplify`.
- Record the bundle size before and after.

## Done when
- No `.jsx` file is left in `frontend/src`.
- Bundle gzip size is lower than before and noted in the PR.

## Triage
Blast radius 1 · Risk 0 · Complexity 1 · Size 1 = 3 → **Planned** track

_Source: TODO.md, P1 performance_
