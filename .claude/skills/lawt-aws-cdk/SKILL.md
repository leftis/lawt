---
name: lawt-aws-cdk
description: Use when touching lawt AWS infrastructure or Node Lambdas: CDK stack, API Gateway, Cognito, S3, CloudFront, IAM, CORS, deploys, or anything under infra/cdk/.
---

# lawt AWS / CDK / Lambdas (Rust target, Node legacy)

Region, account and resource ids come from `lawt.env` (see `lawt.env.example`). Single stack `LawtStack` (`infra/cdk/lib/lawt-stack.js`). Runbook facts live in `AGENTS.md`; architecture in `docs/ARCHITECTURE.md`.

## Hard rules
0. **Never touch the photo bucket `<photo-bucket>`** (no list/read/write/delete via CLI or SDK) without explicit owner approval for that specific action. Read-only checks of other resources (CloudFormation, Cognito, API Gateway) are fine. Profile is `default`, region eu-west-1.
1. **Do not deploy, delete or mutate live AWS unless the owner asks in this session.** Read-only AWS CLI queries to verify state are fine.
2. Never delete `LawtStack`. The photo bucket and Cognito pool are imported (not owned by CDK): never add code that could replace/remove them.
3. Always `--region eu-west-1`. Confirm with `aws configure get region`.
4. No secrets in code, env files, docs or commits. Use IAM roles / profiles.
5. Write `.js` files in `infra/` with `Write`, not `Edit` (U+00AD corruption).

## Workflow for an infra/Lambda change
1. Read the stack and the affected Lambda; grep for the asset path in `lawt-stack.js` before editing (Lambda dirs: `auth`, `list-images`, `download-signer`, `batch-download-signer`, `preview-trigger`, `preview-generator-rust`; `api/` is unused).
2. Make the change; `cd infra/cdk && npx cdk synth --region eu-west-1`.
3. `grep` synthesized output for stale values: old CloudFront domains (`<dead-cdn-id>` is dead), old client id `<old-client-id>`, `prodauth`.
4. Show the `cdk diff` to the owner before deploy. Deploy via `make deploy` (builds Rust + frontend first, because `BucketDeployment` ships `frontend/dist`).
5. Post-deploy: `make validate` (frontend 200, CORS header, Cognito client id, Rust Lambda invoke).
6. If CDK does not pick up Lambda code changes (asset hash cache), fall back to `aws lambda update-function-code`.

## Language policy
Backend is **Rust**. Do not add new Node Lambdas; port them. Target layout: a Cargo workspace at `backend/` (already exists; `crates/common` first, Lambda crates added as they are ported; the legacy `preview-generator-rust` moves in later) with a shared `common` crate (models, key/derivative naming, CORS, input validation, error type), one `api` binary (lambda_http router for `/images`, `/images/download`, batch, preview-trigger, search), plus `indexer` and `preview-generator`. Build with `cargo lambda build --release --arm64`; runtime `provided.al2023`, arm64. Load `ms-rust`, `rust-engineer`, `rust-async-patterns` before writing Rust. Use `aws-sdk-*` crates, `tracing` JSON logs, `thiserror`, no `unwrap` in handlers, `cargo clippy -D warnings`, tests with `aws-smithy-mocks` or trait-based fakes.

## Legacy Node Lambda conventions (bug fixes only, until ported)
- Create SDK clients outside the handler. Read bucket/pool from `process.env`, not literals.
- CORS: echo an allow-listed origin only; never `*` with credentials; same headers on error paths. Centralize in a helper rather than copying (today it is copied 4x).
- Validate all inputs: key must start with `all images/`, reject `..`, cap batch size, cap lengths. Return 400 with a stable `{error}` shape.
- Don't log tokens or full events. Log key + outcome + duration.
- Keep responses small; set `Cache-Control` where the response is cacheable.
- IAM: least privilege (`grantRead` on signer/list, write only for the Rust Lambda). New permission = justify in the PR.

## API Gateway / CORS
- `defaultCorsPreflight` propagates to children; manual `addCorsOptions` currently coexists. Do not add a third mechanism. Preflight max-age should be ≥ 1 h.
- `dataTraceEnabled: true` logs request/response bodies; do not add sensitive fields to bodies while it is on (and see TODO to reduce it).
- Presigned URLs: 900 s expiry. If moving to CloudFront delivery, keep originals private and only expose derivative suffixes.

## Cost awareness
Single-user gallery: prefer serverless pay-per-use, avoid always-on resources (NAT, OpenSearch, provisioned concurrency). Estimate the monthly cost of any new resource in the PR description.
