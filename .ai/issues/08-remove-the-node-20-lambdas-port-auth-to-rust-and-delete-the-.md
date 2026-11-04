## Why
Backend code is Rust by policy. Five Node 20 Lambdas remain (`auth`, `list-images`, `download-signer`, `batch-download-signer`, `preview-trigger`). CDK warns that Node 20 is deprecated and updates stop in 2027.

## What to do
- Port `auth` (code-for-token exchange using the client secret) to a Rust Lambda in `backend/`, or add it as a route of `rawvault-api`.
- Delete the other four together with the old gallery (see the retire-legacy-gallery issue) and the dead `infra/cdk/src/lambda/api/index.js`.
- Remove their stack resources and routes.

## Done when
- No `nodejs20.x` function remains in `cdk synth`.
- Login still works end to end.
- The test for the auth exchange covers a bad code and a missing secret.

## Triage
Blast radius 2 · Risk 1 · Complexity 1 · Size 1 = 5 → **Spec** track

_Source: TODO.md, P0 language migration_
