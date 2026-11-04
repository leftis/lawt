## Why
The stage sets `dataTraceEnabled: true` with `loggingLevel: INFO`. That writes request and response bodies to CloudWatch, which can include tokens and photo keys.

## What to do
- Set `dataTraceEnabled: false` and `loggingLevel: ERROR` in `deployOptions`.
- Add structured logs inside the Lambdas for the few events that matter.

## Done when
- `cdk diff` shows only this change and new calls leave no body logs.

## Triage
Blast radius 0 · Risk 1 · Complexity 0 · Size 0 = 1 → **Direct** track

_Source: TODO.md, P2 observability_
