## Why
`download-signer`, `batch-download-signer` and `preview-trigger` accept any key and sign it. Any signed-in user can ask for a link to any object in the bucket, and the batch endpoint has no size limit.

## What to do
- Accept only keys that start with `all images/`, have no `..` or empty segments, and are at most 1024 bytes. `OriginalKey::parse` in `backend/crates/common/src/key.rs` already does this.
- Cap batch size at 100 requests.
- Return 400 with `{ "error": ... }`.
- If these Lambdas are retired instead (see the Node removal issue), close this one.

## Done when
- Tests show `../x`, keys outside the prefix, empty keys and 101-item batches are rejected.

## Triage
Blast radius 1 · Risk 1 · Complexity 0 · Size 0 = 2 → **Direct** track

_Source: TODO.md, P0 security_
