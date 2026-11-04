## Why
Setup needs two deploys because the frontend needs the API and CDN URLs that only exist after the first deploy, and people copy values by hand.

## What to do
- Add `scripts/gen-frontend-env` that reads the stack outputs and `lawt.env` and writes `frontend/.env.production`.
- Add a Makefile target that runs it between the two deploys.
- Document it in the README and drop the manual copy steps.

## Done when
- One command sequence deploys a working frontend from `lawt.env` only.

## Triage
Blast radius 1 · Risk 0 · Complexity 1 · Size 1 = 3 → **Planned** track

_Source: TODO.md, P2 foundations_
