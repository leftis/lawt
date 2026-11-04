# RawVault TODO

> The open work is also written as ready-to-file GitHub issues in [`.ai/issues/`](.ai/issues/index.json). File them with `python3 .ai/issues/create_issues.py --repo OWNER/NAME`.

Convention: `[ ]` open, `[x]` done (add date + commit). Effort: S <1h, M half-day, L multi-day. Findings come from the 2026-09-24 code review; verify before fixing, some depend on live AWS state.

## P0 — Iceberg migration (see docs/adr/0001-iceberg-catalog.md)
- [x] 2026-09-24 Fix: gallery select re-listed everything / showed all images (names endpoint + scoped fetch). **Not deployed yet.**
- [ ] Owner answers ADR open questions; capture baseline load numbers. (S)
- [ ] Phase 1 infra (Glue, Athena v3, SQS+DLQ, S3 events, IAM). (M)
- [ ] Phase 2 indexer + backfill + EXIF in Rust + read model. (L)
- [ ] Phase 3 read path with fallback flag, `has_*` flags, CDN thumbnails, pagination. (L)
- [ ] Timeline UX: `timeline.json` + month files, justified virtualized grid, day headers, scrubber, blurhash, viewer prefetch, RAW+JPEG stacks (see ADR target experience). (L)
- [ ] Phase 4 search endpoint + filter UI; Phase 5 maintenance/reconcile/alarms. (M)

## NOW — work tracker (2026-09-25; owner authorised deploys, photo bucket stays untouched)
- [x] T1 Frontend TypeScript foundation (strict, typecheck in build, typed `api/` client, generated types)
- [x] T2 Rust dev server replaces the Node mock for timeline work (`backend/crates/dev-server`, port 3002)
- [x] T3 Timeline UI (TS): virtualized justified grid, sticky day headers, scrubber (label above headers), viewer + prefetch, gallery/RAW filters; browser-verified desktop + mobile
- [x] T4 Rust API: `crates/api` core, `crates/api-lambda` (deployed as `rawvault-api`), dev-server
- [x] T5 `crates/pipeline` (plan / process / publish, read-only originals port) + `crates/pipeline-lambda` (built for arm64, **not deployed**: `-c enablePipeline=true`)
- [x] T6 `crates/processor`: RAW embedded preview (rawler), TIFF 16-bit, PNG/WebP/GIF/BMP/JPEG, orientation, resize; HEIC reported unsupported. Only synthetic images tested
- [x] T7 CDK: derived bucket, salted ids secret, rendition CDN (`/derived/*` only), Rust API + routes, gated pipeline function with read-only photo-bucket access
- [x] T9 Deployed 2026-09-25 (stack UPDATE_COMPLETE): API now requires Cognito (was open), CORS on 401s, list-images gallery fix, timeline API, rendition CDN, frontend
- [x] T10 Pipeline enabled and run on the whole library 2026-09-25 (owner approved, read-only): 7,826 photos, 277.8 GB originals, 15.1 GB derived, 1 failed (`100_FUJI/DSCF0025.RAF`, no embedded preview). RAW orientation and dates verified on real DNG/JPG; TIFF colour (16-bit/ICC) still unverified. Costs in docs/COSTS.md

## P0 — Protect the originals
- [ ] Remove write/delete on the photo bucket from every Lambda role (`grantReadWrite` on preview generator → read-only); write derivatives to a new `rawvault-derived-<account>` bucket. Needs owner approval before any deploy. (M)
- [ ] Owner decision: backup strategy for the photo bucket (versioning / replication / Deep Archive copy). We do not enable it ourselves. (S)
- [ ] Replace S3-event design with scheduled read-only reconcile (no bucket config changes). (M)

## P0 — Language migration (owner policy: Rust backend, React+TS frontend)
- [x] 2026-09-25 `backend/` workspace + `rawvault-common` crate (formats, key validation, asset grouping, rendition keys, timeline model, TS type generation), 27 tests, clippy pedantic clean.
- [ ] Cargo workspace for lambdas (`common`, `api`, `indexer`, `preview-generator`); port `list-images`, `download-signer`, `batch-download-signer`, `preview-trigger`, `auth` to Rust (one `api` binary); delete Node Lambdas and `api/`. (L)
- [x] TS types generated from Rust into `frontend/src/api/generated/` (`cargo test --all-features` in `backend/`; commit the output). Frontend still to consume them. (S)
- [ ] Frontend → TypeScript: `tsconfig` strict, `typecheck` script, migrate `api.js` first, then components as touched. (M)

## P0 — Security (do first)
- [ ] Lambdas: replace `Access-Control-Allow-Origin: *` fallback (and `*` on error paths) with a single allow-listed origin from env var; drop `Allow-Credentials` if not using cookies. Shared helper module. (S)
- [ ] Validate `key` in download-signer / batch signer / preview-trigger: must start with `all images/`, no `..`, length cap, batch size cap (e.g. ≤100). Currently any key in the bucket can be signed and batch is unbounded. (S)
- [ ] Verify `/preview` route (API GW → Rust Lambda directly) is used; if not, remove it. Rust Lambda has no API Gateway event handling and is reachable by any authed user. (S)

## P1 — Performance (biggest wins for the gallery feel)
- [ ] Serve thumbnails/medium via the existing CloudFront download distribution instead of per-image presigned URLs: removes HEAD+sign calls, gets edge caching and stable URLs. Needs signed URLs/cookies or OAC + restrict to derivative suffixes; auth decision required. Removes most of `batchGetDownloadUrls` traffic. (M–L)
- [ ] Add `Cache-Control: public, max-age=31536000, immutable` on derivative uploads in Rust `upload_file`. (S)
- [ ] Paginate `/images` (cursor = S3 continuation token) and virtualize the grid (windowing or `content-visibility` + IntersectionObserver lazy loading). Today all objects load and sort in Lambda. (M)
- [ ] Stop HEAD-ing per image: return `has_thumbnail/has_medium/has_full` in the list response (derive from the same `ListObjectsV2` result, it already contains the derivative keys). Then signer only signs. (M)
- [ ] Cache `/images` per gallery (ETag or short `Cache-Control` + client stale-while-revalidate). (S)
- [ ] Split `Gallery.jsx` (693 lines) and `ImageCard.jsx`: extract viewer, batch-job hook, cost modal; memoize cards (`React.memo`), avoid inline-object props. Measure with React Profiler first. (M)
- [ ] Modern formats: WebP/AVIF thumbnails from Rust (`image` supports WebP encode); `srcset` for card sizes; blurhash/dominant-color placeholder instead of hash-color boxes. (M)
- [ ] Vite: check bundle size (`aws-amplify` is a dependency but Hosted UI is used by hand; remove if unused). (S)

## P1 — Correctness / reliability
- [ ] Rust: handle every listed format explicitly (WebP, GIF, BMP, HEIC/HEIF currently fall into the RAW path and yield a placeholder); support or drop each; for JPEG originals stop copying to `.full.jpeg` and reference the original. (M)
- [ ] Rust: stop writing a placeholder PNG under `.jpeg` keys on decode failure (looks like success, blocks retry); write nothing or a distinct marker and return an error status. (S)
- [ ] Rust: propagate upload errors (`.ok()` swallows them); process only the S3 object needed rather than `to_rgba8()` copies of full images (memory). (M)
- [ ] Automatic preview generation: S3 `ObjectCreated` event (prefix `all images/`, exclude derivative suffixes) → Rust Lambda (or SQS in between with DLQ + reserved concurrency) instead of manual batch UI + cost modal. Beware recursion: derivatives are written into the same bucket/prefix. (M)
- [ ] Remove hard-coded bucket in Rust (`STORAGE_BUCKET` const) — read env var already set by CDK. (S)
- [ ] Auth: token refresh (use `refresh_token`), don't keep long-lived token in localStorage without expiry check; handle `?code` in one place (App.jsx and Login.jsx both do). (M)
- [ ] `infra/cdk/src/lambda/api/index.js` uses ESM `import` in a CJS package and is not in the stack — delete if dead. (S)
- [ ] `.env.example` says `us-east-1`; project is `eu-west-1`. Fix defaults. (S)
- [ ] `read.20631.1.lock` is a tracked stray file in `infra/cdk/`; remove and ignore. (S)

## P2 — Engineering foundations
- [ ] Tests: Vitest for `api.js` and pure helpers; node:test for Lambda handlers with mocked S3 (aws-sdk-client-mock); `cargo test` for key/extension/resize helpers in Rust. (M)
- [ ] ESLint + Prettier config (the `lint` script exists but nothing is installed); `cargo clippy` + `fmt` in Makefile. (S)
- [ ] CI (GitHub Actions): frontend build, lint, tests, `cdk synth`, `cargo check`. No deploy from CI until decided. (M)
- [ ] Config single source of truth: stack outputs → generated `frontend/.env.production` (script) instead of hand-copied IDs; CDK context for account/region/bucket/pool. (M)
- [ ] Shared Lambda layer/module for CORS, key validation, extension lists (currently duplicated 4×). (M)
- [ ] Update AGENTS.md: remove `iac/` refs and the Cognito client ID contradiction; convert stale-URL warnings into a `make validate` check. (S)
- [ ] Observability: reduce API GW `dataTraceEnabled` (logs full payloads, incl. tokens) to errors only; Lambda structured logs; CloudWatch alarm on Rust Lambda errors. (S)

## P3 — Features / ideas
- [ ] EXIF-driven metadata (camera, lens, ISO, date taken) and search/filter — this is the motivation behind `docs/iceberg-plan.md`. Consider a cheaper first step (JSON sidecar or DynamoDB) before Iceberg; discuss with owner. (L)
- [ ] Sort by capture date not upload date (needs EXIF). (M)
- [ ] Albums/favourites/ratings (needs a metadata store). (L)
- [ ] Share links with expiry for a single image/gallery. (M)
- [ ] Keyboard navigation + preloading next image in the viewer; swipe on mobile. (S)
- [ ] Download original as zip for a selection. (M)

## Productization (plan in docs/product/, written 2026-09-26)
Order matters: validate demand first, then build the safety foundations, then billing.
- [ ] **Validate:** 30 ICP interviews, landing page + pre-orders, Van Westendorp on the beta cohort (docs/product/02, 04).
- [ ] **Decide** (founder): storage backend (B2/R2 vs AWS tiered), first market/currency, Merchant of Record vs Stripe Tax, free tier and founding offer, name/trademark, legal counsel (docs/product/04, bottom).
- [ ] Epics from docs/product/03: multi-tenancy foundation; storage backend abstraction + migration; web uploader (resumable, hashed); event-driven processing with DLQ; desktop uploader (Tauri + Rust, previews-first ingest); private delivery (signed cookies); expiring share links; bin; billing/quotas/metering/VAT; account export and deletion; abuse handling; EXIF search; PWA then mobile; importers (Takeout, Amazon, Flickr); observability and per-tenant cost; marketing site; support tooling.

## Next features (drafted 2026-09-26; copy into GitHub issues)
- [ ] **Expiring share links.** Share one photo, a selection or a day via a link that expires (1 h / 1 day / 7 days / 30 days / custom) and can be revoked. Design: a `shares/{token}.json` record in the derived bucket (token = 128-bit random, photo ids, expiry, created-by); a public Rust route `GET /s/{token}` (no Cognito) that checks expiry and returns the photo list; renditions for shared photos served through CloudFront **signed URLs or a short-lived signed cookie** (the current "unlisted link" renditions are not private enough for shares). Options: view-only vs allow download of full JPEG; never expose originals unless the owner opts in. Acceptance: expired and revoked links return 410, nothing else is listed, links are unguessable, list/revoke UI in the app, tests for expiry and revocation.
- [ ] **Bin: mark for deletion, keep 30 days.** Select photos, "Move to bin"; they disappear from the timeline immediately and can be restored for 30 days; after that they are purged. Design: `deleted_at` in the record; `publish` excludes binned photos and writes `index/bin.json`; a scheduled purge job. **Hard constraint: our code never deletes originals.** Decision needed before building: what "purge" means. Proposed: purge deletes only derived data (renditions and record) and leaves the original in the photo bucket, or lists it in a report for the owner to delete by hand. Deleting originals is only acceptable in a product where the user's originals live in a bucket we own, with versioning on. Acceptance: bin view, restore, 30-day countdown, tests for restore and purge boundaries.
- [ ] **What else (candidates to triage):** favourites and albums; camera/lens/ISO search (data already in records); RAW+JPEG stacks in the grid; map view (needs GPS, none in this library); duplicate detection; download a selection as a zip; scheduled daily pipeline run; private renditions via signed cookies; multi-tenant groundwork if this becomes a product (see docs/COSTS.md).

## After the first full run
- [ ] Investigate `100_FUJI/DSCF0025.RAF` (likely truncated; placeholder tile shows).
- [ ] Schedule `run` (e.g. daily EventBridge) so new uploads appear automatically, if the owner approves.
- [ ] Parallel workers overlapped (sweep + per-folder) and re-rendered ~1,100 photos: add a lock or shard by folder only.
- [ ] Remove the pipeline function's role if not needed between runs (no cost while idle, so optional).

## Found during T9 (fix soon)
- [ ] Legacy Lambdas still echo `Access-Control-Allow-Origin: *`; the API is now authenticated, but tighten it (shared helper) while porting to Rust.
- [ ] `rawvault-preview-generator` role has read/WRITE/DELETE on the photo bucket (`grantReadWrite`); the new pipeline replaces it, then remove that grant.
- [ ] `DownloadCloudFront` (photo bucket, OAI) cannot read the imported bucket and is unused; delete it after confirming.
- [ ] Node 20 runtime is deprecated (update blocked 2027-03); port the remaining Node Lambdas to Rust (list-images, signers, auth, preview-trigger) and remove `/preview` route.
- [ ] Remove the old Node mock (`infra/mock-api`, docker-compose `mock-api`) once nothing uses it (Docker container on :3001 still runs it).
- [ ] Real-file verification: RAW orientation heuristic, ICC profiles for TIFF, HEIC support decision.
- [ ] Optional: CloudFront signed cookies for strictly private renditions.

## Done
- [x] 2026-09-24 Project scaffolding: CLAUDE.md, docs/ARCHITECTURE.md, TODO.md, project skills, memory; redacted secrets in `.pi/progress.md`.
