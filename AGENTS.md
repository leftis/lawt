# Notes for AI coding agents

Short context for tools like Claude Code, Cursor or Copilot. Humans: start with [README.md](README.md).

## What this repo is
RawVault turns a bucket of camera photos (RAW, TIFF, JPEG) into a fast timeline web app on AWS.
Backend is Rust, frontend is React + TypeScript, infrastructure is AWS CDK. Read
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) before changing anything.

## Rules that matter
1. **The photo bucket holds irreplaceable originals. Never write, copy or delete there.** Code reads it
   through the `Originals` trait (`backend/crates/pipeline`), which has no write method on purpose.
   Anything generated goes to the separate derived bucket.
2. **No secrets in the repo.** Settings live in `rawvault.env` and `frontend/.env.production`, both
   git-ignored. Only the `*.example` files are committed.
3. **Pin the region and profile** on every AWS command. Deploy settings come from `rawvault.env`.
4. **Backend code is Rust, frontend code is TypeScript.** Do not add new Node or Python services.
5. **CDK:** attach the Cognito authorizer in the *method options* (`addMethod(..., options)`), not in
   `LambdaIntegration`. In the integration it is silently ignored and the route stays open.
6. **Deploying the stack without `-c enablePipeline=true` removes the pipeline function.** Pass the
   flag every time if you use the pipeline.
7. Show `cdk diff` before every deploy, and never delete the stack.

## Checks before you say "done"
```bash
cd backend && cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features
cd frontend && npm run typecheck && npm test && npm run build
cd infra/cdk && npx cdk synth --region <region>
```

## Where things live
| Path | What |
|---|---|
| `backend/crates/common` | Pure domain logic: formats, key validation, photo ids, timeline model |
| `backend/crates/processor` | Decoding and JPEG renditions |
| `backend/crates/pipeline`, `pipeline-lambda` | plan, process, publish |
| `backend/crates/api`, `api-lambda` | Timeline API |
| `backend/crates/dev-server` | Local server with a synthetic library, no AWS needed |
| `frontend/src/timeline` | Timeline UI |
| `infra/cdk` | CDK stack |
| `docs/` | Architecture, runbook, ADR, cost model, product plan |
