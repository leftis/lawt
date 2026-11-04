## Why
Only the layout math and filters are tested. `useMonths` (concurrency, retries, abort), `TimelineView` (scroll anchoring, filters, viewer navigation across months) and `Viewer` (keyboard, preload) are verified by hand only.

## What to do
- Add Vitest with Testing Library and a mocked API.
- Cover: month queue limit of 4, retry after failure, abort on unmount, viewer next/previous across an unloaded month, empty and error states.

## Done when
- New tests fail when each of those behaviours is broken on purpose.

## Triage
Blast radius 0 · Risk 0 · Complexity 1 · Size 1 = 2 → **Direct** track

_Source: TODO.md, P2 foundations_
