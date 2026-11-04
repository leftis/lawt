## Why
Small things that confuse newcomers.

## What to do
- Delete `infra/cdk/read.20631.1.lock`.
- Fix `.env.example` (`us-east-1`, LocalStack values) or remove it.
- Remove the Node mock `infra/mock-api`, the `mock-api` service in `docker-compose.yml` and the matching parts of `scripts/local-dev.sh`. The Rust `dev-server` replaces them.
- Remove `mise.toml` entries that are not used.

## Done when
- `git grep mock-api` finds nothing outside history.
- README quickstart still works.

## Triage
Blast radius 0 · Risk 0 · Complexity 0 · Size 0 = 0 → **Direct** track

_Source: TODO.md, P1 correctness_
