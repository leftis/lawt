## Why
`storageBucket.grantReadWrite(previewGeneratorLambda)` in `infra/cdk/lib/lawt-stack.js` gives the legacy Lambda put, overwrite and delete on every original. The new pipeline only needs read, and nothing in the legacy code deletes, so the permission is risk without a use.

## What to do
- Change the grant to read-only, or delete the function (see the issue that retires the legacy preview generator).
- Run `cdk diff` and check that only s3 write and delete actions on the photo bucket disappear.
- Deploy only after the owner approves the diff.

## Done when
- `cdk diff` shows only the removed actions.
- After deploy, the old gallery still loads and the timeline is unaffected.

## Triage
Blast radius 1 · Risk 1 · Complexity 0 · Size 0 = 2 → **Direct** track

_Source: TODO.md, 'Found during T9'_
