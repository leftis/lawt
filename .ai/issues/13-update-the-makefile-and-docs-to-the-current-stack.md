## Why
`docs/ARCHITECTURE.md` still describes the legacy design first, and `docs/RUNBOOK-timeline.md` mixes private and public steps. The Makefile `validate` target probes `/images`, which the timeline does not use.

## What to do
- Move the legacy section of `ARCHITECTURE.md` to the end.
- Make `validate` check `/timeline` returns 401 without a token, the rendition CDN returns 403 for non-derived paths, and the frontend returns 200.
- Align the runbook with the README steps.

## Done when
- `make validate` passes on a live deployment and fails when the authorizer is missing.

## Triage
Blast radius 1 · Risk 0 · Complexity 0 · Size 1 = 2 → **Direct** track

_Source: TODO.md, P2 foundations_
