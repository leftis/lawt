## Why
There is no automated check. The repo has 77 Rust tests, 17 frontend tests, strict TypeScript and clippy at `-D warnings`, so a pull request can break any of them unnoticed.

## What to do
- Job 1: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`.
- Job 2: `npm ci`, `npm run typecheck`, `npm test`, `npm run build`.
- Job 3: `cdk synth` with a dummy `rawvault.env` and stub Lambda assets.
- Fail if regenerating the TypeScript types changes any file in `frontend/src/api/generated/`.
- No deploy from CI.

## Done when
- A pull request shows the three jobs and fails when a check fails.

## Triage
Blast radius 1 · Risk 0 · Complexity 1 · Size 1 = 3 → **Planned** track

_Source: TODO.md, P2 foundations_
