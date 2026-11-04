# RawVault Architecture (as built)

Last updated 2026-09-25 after the timeline deploy. Live state was checked read-only (CloudFormation, API Gateway, Lambda config); the photo bucket was never touched.

## Timeline architecture (new, deployed; pipeline function not yet enabled)
```
Browser (React + TypeScript SPA)
  ├─ GET /timeline, /timeline/months/{YYYY-MM}  ─► API Gateway (Cognito ID token) ─► rawvault-api (Rust) ─► derived bucket index/*.json
  └─ <img src=CDN/derived/{id}/thumbnail|medium|full.jpeg> ─► rendition CDN (CloudFront, OAC, /derived/* only) ─► derived bucket

rawvault-pipeline (Rust, manual invoke, NOT deployed until -c enablePipeline=true)
  plan:    list `all images/` (read-only)  ─► which photos lack a record
  run:     GET original ─► rawvault-processor ─► derived/{id}/{thumbnail,medium,full}.jpeg + records/{id}.json ─► publish
  publish: records/* ─► index/timeline.json + index/months/YYYY-MM.json
Photo bucket <photo-bucket>: read-only for the pipeline role; nothing else references it except the legacy Lambdas.
```
- `backend/` Rust workspace: `common` (formats, key validation, salted asset ids, renditions, timeline model, read model), `processor` (decode/orient/resize/encode), `pipeline` (orchestration over read-only `Originals` and read-write `Derived` ports), `api` (framework-agnostic routes), `api-lambda`, `pipeline-lambda`, `dev-server` (synthetic library).
- Photo ids are HMAC(salt, folder, stem); the salt lives in Secrets Manager (`IdSalt`). Rendition URLs are unguessable "unlisted links".
- Read model is derived data: delete `index/` and run `publish` to rebuild. Records are the source of truth, renditions are regenerable from originals.
- RAW: the camera's largest embedded JPEG preview is read (rawler `full_image`, else scan for an embedded JPEG). No demosaicing. Orientation of RAW previews uses a heuristic that must be verified on real files.
- The frontend picks the UI: legacy gallery by default, `?ui=timeline` (remembered) or `VITE_UI=timeline` for the timeline.

## Legacy architecture (still live; being replaced)


## Overview
```
Browser (React SPA, CloudFront + S3 frontend bucket)
  │  Cognito Hosted UI (code flow)  ──►  POST /auth/token (auth Lambda) ──► access_token in localStorage
  │  Authorization: Bearer <access_token>
  ▼
API Gateway REST "prod" + Cognito authorizer
  ├─ GET  /images?gallery=X          list-images Lambda      ListObjectsV2 on `all images/[X/]`
  ├─ GET  /images/download?key&type  download-signer Lambda  HEAD derivative + presign GET (15 min)
  ├─ POST /images/download/batch     batch-download-signer   same, N keys per call (10 parallel)
  ├─ POST /preview-trigger?raw_path  preview-trigger Lambda  async-invokes the Rust Lambda
  └─ GET  /preview                   wired straight to the Rust Lambda (see TODO: likely broken)
S3 bucket <photo-bucket> (pre-existing, imported, not managed by CDK)
  originals + derivatives side by side; Rust Lambda reads/writes it
```
A second CloudFront distribution (`DownloadCloudFront`, OAI) fronts the photo bucket, and `VITE_CDN_HOST` / `toCloudFrontUrl` exist in the frontend, but the gallery currently uses **presigned S3 URLs**, not the CDN.

## S3 key conventions
- Originals: `all images/<gallery>/<file>.<ext>`; gallery = first path segment, files directly under `all images/` → gallery `all images`.
- Derivatives are siblings of the original, suffix appended to the full filename:
  | Suffix | What | Size |
  |---|---|---|
  | `.thumbnail.jpeg` | gallery card | ~400px, 20–50 KB |
  | `.medium.jpeg` | fullscreen viewer | ~1920px, 200–400 KB |
  | `.full.jpeg` | full-res JPEG (RAW/TIFF: encoded q92; JPEG/PNG: S3 copy) | full |
  | `.preview.png` / `.preview.jpg` | legacy, read-only fallback | — |
- list-images filters derivatives out by suffix; **adding a new suffix requires updating list-images, download-signer, batch signer and Rust `is_preview_key`**.
- RAW extension list is duplicated in list-images and Rust (`is_raw_extension`).

## Preview pipeline
1. User opens gallery → `GET /images` (whole gallery or all objects).
2. Cards call batch signer for `thumbnail`; `preview_exists=false` → placeholder + "generate" affordance.
3. Generation is **manual**: per-image or multi-select batch (Gallery.jsx: concurrency 3, 500ms stagger, job persisted in localStorage `rawvaultBatchJob`, resumes on reload) → `/preview-trigger` → async invoke Rust Lambda `{key}`.
4. Rust: download original → decode (rawler full decode, fallback embedded JPEG; `image` for TIFF/JPEG/PNG) → upload thumbnail, medium, full. On failure a placeholder PNG is written **under the `.jpeg` keys** (so a failed decode looks like "exists").
5. No S3 event trigger is configured, though the Rust handler supports S3 events.

## Frontend
- `App.jsx`: hash-based auth gate (`#login` / `#gallery`), handles `?code=` exchange. `Login.jsx` builds Hosted UI redirect. `Gallery.jsx` (693 lines) owns view modes (masonry/grid/list), gallery filter (localStorage), multi-select, batch jobs, fullscreen viewer, downloads. `ImageCard.jsx` (417) renders cards. `api.js` is the only API client (in-memory URL cache, 500 entries).
- Inline styles are widespread alongside `styles.css` (690 lines).
- Build-time config via `VITE_*` in `frontend/.env.production` (copied to `.env.local` by `make build-frontend`).

## Infra
- One CDK stack; frontend deployed by `BucketDeployment` from `frontend/dist` (so build **before** synth/deploy).
- Cognito pool/client and photo bucket are **imported by hard-coded ID**, not created.
- Frontend distribution: caching disabled. CORS: `defaultCorsPreflight` for the CloudFront origin + manual `addCorsOptions` on each resource (AGENTS.md warns manual OPTIONS can collide; it currently synths).

## Known weaknesses (detail + priority in TODO.md)
- ~~API methods open~~ fixed 2026-09-25: authorizer now on every method except `/auth/token`.
- list-images does a full `ListObjectsV2` scan per request; no pagination to the client, sorted in Lambda.
- Presigned-URL fan-out: HEAD + sign per image per size; URLs expire in 15 min and are not CDN-cacheable.
- Lambda CORS falls back to `Access-Control-Allow-Origin: *` with credentials header; error paths always `*`.
- Auth token in localStorage; no refresh-token flow; 401 handling redirects via hash.
- Bucket name, region, account and Cognito IDs hard-coded across stack, Rust, Lambdas.
- Zero automated tests, no linter config (`npm run lint` has no eslint dep), no CI.
- Rust Lambda swallows upload errors (`.ok()`), holds whole originals in memory (1024 MB, 50 MB+ RAFs).
