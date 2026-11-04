## Why
New photos appear only when someone runs the pipeline by hand. When two runs overlap (a folder run plus a full sweep), the same photos are rendered twice. About 1,100 were rendered twice in the first run.

## What to do
- Add an EventBridge schedule (daily) that invokes `{"action":"run"}` until nothing remains. It must be opt-in and off by default.
- Add a lease object in the derived bucket (`locks/run.json` with an expiry) so a second run exits early.
- Keep reads of the photo bucket limited to list and get.

## Done when
- Two concurrent invocations render each photo once.
- With the schedule on, a new photo shows up within a day without manual steps.

## Triage
Blast radius 1 · Risk 1 · Complexity 1 · Size 1 = 4 → **Planned** track

_Source: TODO.md, 'After the first full run'_
