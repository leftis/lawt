# RawVault

Personal photo gallery (RAW + JPEG/TIFF) on AWS. A fun side project, but built to production standards: fast, cheap to run, and safe. Single owner. Photos already live in an S3 bucket (`PHOTO_BUCKET` in `rawvault.env`), synced with rclone, keys under `all images/<folder>/<file>`.

@AGENTS.md

> AGENTS.md holds the short agent rules; the AWS values for this deployment are in `rawvault.env` and `.local/LIVE.md`.

## Read first
- `docs/ARCHITECTURE.md` — how it works today, data flow, S3 key conventions, known weaknesses
- `TODO.md` — prioritized backlog. Pick work from here, update it as you go (tick items, add discoveries)
- `docs/iceberg-plan.md` — **draft, not decided**. Do not start building it without the owner's go-ahead
- `.claude/skills/` — stack skills (`rawvault-frontend`, `rawvault-aws-cdk`, `rawvault-preview-pipeline`, `rawvault-engineering`). Load the relevant one before working in that area

## Language policy (owner decision, 2026-09-25)
- **Backend = Rust, always.** New Lambdas/services are Rust (cargo-lambda, arm64). Existing Node Lambdas are legacy and get ported, not extended (bug fixes only).
- **Frontend = React + TypeScript** (strict). New frontend files are `.tsx/.ts`; existing `.jsx/.js` are migrated when touched.
- API contract is defined once in Rust and exported to TS types (see TODO), never hand-copied.
- Infra stays AWS CDK for now (currently JS); moving it to TypeScript is an open, low-priority question.

## Stack
| Layer | Tech | Path |
|---|---|---|
| Frontend | React 19 + Vite 6 (JSX today → **TypeScript strict**, no router, no tests), plain CSS | `frontend/` |
| API | API Gateway REST + Cognito authorizer; **target: one Rust API Lambda**; legacy: Node 20 Lambdas (CommonJS, SDK v3) | `infra/cdk/src/lambda/*` |
| Rust workspace (new) | `backend/`: crate `rawvault-common` (formats, keys, asset grouping, renditions, timeline model; pure, no AWS). Lambdas join here as they are ported | `backend/` |
| Preview generator | Rust Lambda (arm64, `provided.al2023`), `rawler` + `image` | `infra/cdk/src/lambda/preview-generator-rust` |
| Infra | AWS CDK v2 (JS), single stack `RawVaultStack`, eu-west-1 | `infra/cdk/lib/rawvault-stack.js` |
| Local dev | Express mock API (+ optional LocalStack) | `infra/mock-api`, `docker-compose.yml`, `scripts/local-dev.sh` |

## Commands
```bash
make help                 # all deploy targets (preflight, build-rust, build-frontend, synth, deploy, validate)
cd backend && cargo test --all-features && cargo clippy --all-targets --all-features -- -D warnings   # Rust workspace (regenerates TS types)
cd backend && cargo run -p rawvault-dev-server            # synthetic library on :3002 (no AWS)
cd frontend && npm test && npm run typecheck                    # vitest + tsc
cd frontend && VITE_UI=timeline VITE_API_BASE_URL=http://localhost:3002/api npm run dev
cd backend && cargo lambda build --release --arm64 -p rawvault-api-lambda   # needs zig (linker only): brew install zig
cd frontend && npm run dev            # Vite on :5173, proxies /api -> mock on :3001
cd infra/mock-api && node index.js    # local mock backend
cd infra/cdk && npx cdk synth --region eu-west-1
cd infra/cdk/src/lambda/preview-generator-rust && cargo lambda build --release --arm64
```
Deploy = `make deploy` (settings from `rawvault.env`). **Never deploy or change live AWS without the owner asking.**

## Working rules
0. **Never touch the photo bucket (`PHOTO_BUCKET`)**: no reads, listings, writes or deletes from the CLI/SDK unless the owner explicitly approves that specific action in the current session. Code that will run against it is written and unit-tested locally with fakes; the owner runs or approves any real run. Private live values are in `.local/LIVE.md` (git-ignored).
1. Pre-flight before acting: read the file, grep for the value, check live state. Stale IDs/URLs are the #1 failure cause (see AGENTS.md).
2. No secrets in the repo, ever. Never paste credentials into files, commits or memory.
3. Use `Write` rather than `Edit` for `.js` files in `infra/` (invisible U+00AD corruption, see AGENTS.md).
4. Keep changes small and verifiable. Frontend: `npm run build` must pass. CDK: `cdk synth` must pass. Rust: `cargo check` must pass.
5. Performance is a feature: any change touching list/preview/download paths must state its effect on request count, payload size, or S3 calls.
6. Update `TODO.md` and (if behaviour changed) `docs/ARCHITECTURE.md` in the same change.
7. Commit style follows history: `type: message` (`fix:`, `perf:`, `revert:`, `feat:`, `docs:`).

## Gotchas
- Delete `frontend/.env.local` after production builds: Vite dev picks it up and points renditions at production.
- Only `frontend/.env.production` is committed config. `VITE_RENDITION_BASE_URL` = rendition CDN; `VITE_CDN_HOST` is the legacy photo-bucket CDN and is not used by the timeline.
- API Gateway methods must get `authorizer` in the *method options*; passing it to `LambdaIntegration` is silently ignored (the whole API was open because of this until 2026-09-25).
- Deploy: `cd infra/cdk && npx cdk deploy --region eu-west-1` (default AWS profile). Pipeline function only with `-c enablePipeline=true`.

## Memory
Agent memory lives outside the repo. Save non-obvious decisions and owner preferences there, not things derivable from code or git.
