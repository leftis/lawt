## Why
The new pipeline replaces them. They are still deployed, still built by `make build-rust`, and the stack needs the old Rust binary to exist before `cdk synth`, which makes first-time setup harder.

## What to do
- Remove both functions, the `/preview` and `/preview-trigger` routes and the `preview-generator-rust` crate.
- Drop the legacy build from the Makefile and README.
- Leave existing derived files next to the originals alone.

## Done when
- A fresh clone can `cdk synth` after building only the `backend/` workspace.

## Triage
Blast radius 1 · Risk 1 · Complexity 0 · Size 1 = 3 → **Planned** track

_Source: TODO.md, 'Found during T9'_
