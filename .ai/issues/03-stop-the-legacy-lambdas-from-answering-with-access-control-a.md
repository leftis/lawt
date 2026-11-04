## Why
The Node Lambdas echo the request `Origin` or fall back to `*`, and error paths always send `*`, together with `Access-Control-Allow-Credentials: true`. API Gateway now blocks unauthenticated calls, which lowers the risk, but the pattern is wrong. `rawvault-api` already does it right with `allowed_origin`.

## What to do
- Best: remove the Lambdas (Node removal issue).
- Otherwise add one shared helper that allows exactly one configured origin and use it on every response, errors included.

## Done when
- A request with a foreign `Origin` gets no `Access-Control-Allow-Origin` header from any route.

## Triage
Blast radius 1 · Risk 1 · Complexity 0 · Size 0 = 2 → **Direct** track

_Source: TODO.md, 'Found during T9'_
