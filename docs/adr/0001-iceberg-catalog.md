# ADR 0001 — Iceberg image catalog (revises docs/iceberg-plan.md)

Status: **Partly implemented (2026-09-25)**; see "Implementation status" at the end. Original status: **Proposed** (target UX added 2026-09-25: Google-Photos-style timeline, see section below) (owner wants Iceberg; read-path design below needs a go-ahead). Date: 2026-09-24.

## Goals (from the owner)
1. Images/galleries load fast. 2. Cheaper retrieval. 3. Selecting a gallery must not reload everything. 4. Rich EXIF search later.

## What was wrong first (fixed 2026-09-24, not Iceberg-related)
On mount the UI listed the *entire* bucket just to build the gallery dropdown, and commit `050a763` removed the client-side filter, so the saved gallery showed every image until you switched. Also the root gallery (`all images`) built prefix `all images/all images/`. Fix: `GET /images?galleries=1` (single delimiter listing), frontend loads names then only the selected gallery, stale-response guard, root gallery uses a delimiter. **Needs a Lambda + frontend deploy.**

## Gaps found in docs/iceberg-plan.md
| # | Gap | Why it matters | Resolution |
|---|---|---|---|
| 1 | **Cost math is wrong.** `ListObjectsV2` returns 1000 keys per request at $0.005/1K requests: a 10k-object load is ~$0.00005, 500 loads/mo ≈ $0.03, not $0.75–$750. | "Cheaper" is not a real driver; both options cost cents. Athena has a 10 MB/query minimum (~$0.00005) so it is not free either. | Justify Iceberg by **EXIF search, preview state, open format**, not cost. |
| 2 | **Athena is slower, not faster.** Iceberg queries take 2–5 s plus polling, and API Gateway caps at 29 s. Plan admits 1–3 s vs <500 ms today. | Would make "loads forever" worse for the main page. | Athena is **not on the gallery read path** (see design). |
| 3 | **The plan does not touch what makes images "load forever".** Per thumbnail we HEAD + presign, 800 ms debounce, 15-min non-cacheable URLs. Not measured yet; needs a profile. | Catalog swap alone would not speed up images. | Table stores `has_thumbnail/medium/full`; API returns stable CDN URLs (TODO P1). |
| 4 | **`WHERE preview_status='done'` hides images without previews.** | You could never see/generate previews for new photos (manual flow depends on seeing them). | Return all rows; status is a column, UI shows placeholder. |
| 5 | **Per-image `INSERT` from Rust via Athena.** Each DML is a multi-second query, tiny Parquet file + snapshot per image; Athena DML concurrency quotas; Iceberg optimistic-commit conflicts with parallel Lambdas. | Small-file explosion, throttling, failed commits. | Writes go S3 event → **SQS (batch window, max concurrency 1)** → one indexer Lambda → one `MERGE` per batch. Rust Lambda only emits status messages, never talks to Athena. |
| 6 | **SQL built by string concatenation** (`escapeSql`, format!). Filenames with `'`. | Injection/breakage. | Athena `ExecutionParameters` only. |
| 7 | **Sync gaps.** rclone uploads, *deletes*, and renames make the table drift. Plan only covers backfill + creates. | Ghost or missing photos. | `ObjectCreated` + `ObjectRemoved` events plus a nightly reconcile (list vs table, fix diffs). |
| 8 | **Event recursion.** Derivatives land in the same bucket/prefix and would re-trigger. | Infinite loop, runaway cost. | Filter by prefix `all images/` and skip derivative suffixes in the indexer; reserved concurrency + DLQ. |
| 9 | **Partitioning by gallery × year is wrong for a timeline.** (Superseded: partition by `month(taken_at)`, see target experience.) Personal library ≈ 10k–100k rows (a few MB): one partition per folder means many tiny files. | Slower and more files than an unpartitioned table. | Unpartitioned (or `year(exif_datetime)` only), sorted by `exif_datetime`; compact weekly. |
| 10 | **EXIF source.** `exiftool-vendored` needs a Perl runtime in Lambda (heavy, cold start); Rust `rawler` already parses EXIF. | Deploy size, latency, build pain. | Extract EXIF in the existing Rust Lambda (rawler / `kamadak-exif`), emit it in the status message. |
| 11 | **Iceberg maintenance.** Athena Iceberg needs `OPTIMIZE` and `VACUUM` (snapshot expiry, orphan files) or metadata grows. | Slowly growing cost/latency. | Weekly EventBridge job for both. |
| 12 | **Infra realities.** Needs Athena engine v3 workgroup; Iceberg DDL via Athena (custom resource or one-time CLI), not plain `CfnTable`; `<deploy-user>` lacks glue/athena/sqs perms; table/results location must be in the eu-west-1 bucket; the existing bucket is imported (S3 notifications on an imported bucket need a custom resource). | Deploy will fail or need manual steps. | Phase 1 lists the exact perms and the DDL step; owner grants IAM. |
| 13 | **Schema drift vs. code.** Plan schema has `preview_path` for `.preview.png`, but current keys are `.thumbnail/.medium/.full.jpeg`; no `updated_at`/`etag`/`catalog_version`; NULL `exif_datetime` breaks `year()` partition and sort. | Wrong rows, unstable sort. | Add `has_thumbnail/medium/full`, `etag`, `indexed_at`; sort key `coalesce(exif_datetime, uploaded_at)`. |
| 14 | **Pagination/sort contract.** Plan has `LIMIT 500`, no cursor. | Big galleries get truncated. | Keyset cursor on `(sort_ts, id)`, page size ~200. |
| 15 | **Auth model on the API is unchanged** and Lambdas need `athena:*`, `glue:Get*`, s3 on results/iceberg prefixes — least privilege. | Overbroad IAM. | Scoped policies in Phase 1. |

## Decision: Iceberg as the source of truth, **materialized read model for the UI**
```
S3 events (create/remove) ─► SQS (batch, concurrency 1, DLQ) ─► indexer Lambda ─► Athena MERGE ─► Iceberg table (truth, EXIF, history)
                                                                      │
                                                                      └─► regenerate read model: s3://…/index/galleries.json
                                                                                                  s3://…/index/g/<gallery>.json.gz   (immutable, versioned)
Browser ─► GET /images?gallery=X ─► one S3 GET (or CloudFront) of the read model : tens of ms, ~0 cost
Browser ─► GET /images/search?camera=&from=&iso= ─► Athena (parameterized, cached 5 min) : 2–5 s, acceptable for search
```
- Keeps the owner's Iceberg goal (open table, EXIF queries, time travel, preview-state visibility) without paying Athena latency on every gallery click.
- Read model is derived, rebuildable from the table at any time; rollback = `USE_ICEBERG=false` falls back to `ListObjectsV2` (already prefix-scoped).
- Alternative considered: DynamoDB (single-digit-ms reads, simpler writes, but not open format and worse ad-hoc analytics). Rejected only because the owner wants Iceberg; still the simplest option if Athena ops become annoying.

## Target experience: "Google Photos, but for every RAW format" (owner, 2026-09-25)
One library-wide **timeline by capture day**, newest first, folders merged. Folder/camera/RAW-vs-JPEG are filters, not the primary navigation.

### Data model changes
- `taken_at = coalesce(EXIF DateTimeOriginal, S3 LastModified)` plus `taken_at_source` (`exif|mtime`). EXIF has no timezone, so group by camera wall-clock day.
- **Partition by `month(taken_at)`, not by folder and not by day.** Day partitions mean ~365 tiny files/year (the small-file problem); month = 12/yr. A "show 15 June 2024" query prunes to one month partition and then to the right row groups via Parquet min/max stats because rows are written sorted by `taken_at`. Folder is just the `gallery` column: filterable, still cheap.
- Extra columns: `width`, `height` (justified layout with no layout shift), `orientation`, `blurhash`/dominant color, `has_thumbnail/medium/full`, GPS (later: map view).

### Asset + renditions (owner decision 2026-09-25: ~10k photos, camera folders)
One table row = one **asset** (photo), keyed by `gallery + stem` (so `R0001234.RAF` and `R0001234.JPG` in the same folder are the same asset). Each asset has renditions with distinct jobs:
`original` is **whatever file was uploaded** (RAF, NEF, ARW, DNG, TIFF, JPEG, PNG, HEIC...). It is archived untouched; `original_ext` and `is_raw` are columns. When a folder+stem has several files (RAW+JPEG, RAF+TIFF), the indexer picks the primary original by priority **RAW > TIFF > JPEG/PNG/HEIC** and records the rest as `companions` (array of keys).
| Rendition | Purpose | How it is produced, by original type | Size |
|---|---|---|---|
| `original` | archive, never modified | as uploaded | any |
| `full` | full-res viewing/export, always browser-renderable | RAW: companion JPEG if present, else embedded JPEG, else demosaic. TIFF/PNG/HEIC: decode → JPEG q92. **JPEG: the original itself, no copy** (`full_key = original_key`; saves storage and the current server-side copy) | full |
| `medium` | viewer | resize of full | 1920 px |
| `thumbnail` | timeline grid, loads instantly | resize of full | ~400 px (add ~200 px + WebP later) |
Rules for all types: apply EXIF orientation once when producing `full`; store `width/height` of the oriented image; 16-bit TIFF is tone-mapped to 8-bit sRGB (check colour-profile handling on samples).

**Gap found in the current Rust Lambda:** it only handles RAW, TIFF, JPG/JPEG and PNG. `list-images` also lists `webp`, `gif`, `bmp`, `heic`, `heif`, but the Lambda sends those down the RAW path, where rawler fails and a placeholder is written. HEIC also needs a decoder such as libheif (not in the `image` crate). Decision needed per format: support (HEIC, WebP) or drop from the list.

Only one tile appears in the timeline per asset; the viewer offers "download original / full JPEG" (original is whatever type it was). Today the indexer would see two S3 objects for a pair; grouping by stem happens in the indexer, never in the UI. Correction: the legacy Lambda's `rawler full_image` call already returns the camera's embedded preview (no demosaic), and the new processor keeps that approach (embedded preview first, embedded-JPEG scan as fallback). Still to verify on real files.

### Sizing at ~10k photos
Everything is small: `timeline.json` ≈ 30–60 KB, whole-library row data ≈ 2–4 MB gzipped, month files ≈ tens of KB. Backfill (EXIF + thumbnails) is a one-off of roughly a dollar or two of Lambda time; Athena/S3 running costs stay in cents. At this size Iceberg is chosen for openness/EXIF search, not necessity (DynamoDB or a single Parquet would also work).

### Read model (replaces per-gallery JSON above)
```
index/timeline.json            [{day:"2024-06-15", n:42}, ...]   whole library, a few KB for 10 years
index/months/2024-06.json.gz   rows for that month (id,gallery,stem,w,h,taken_at,ext,is_raw,has_*,blurhash,companion)
```
The client loads `timeline.json` once → knows every day header, photo count and section height and can draw the whole scroll bar and year/month scrubber immediately. It then fetches month files only as their section nears the viewport. Filters (folder, camera, RAW only, date jump) apply client-side to loaded months or go to Athena search for cross-month queries.

### UI (frontend work)
Virtualized justified grid with sticky day headers; scrubber with year/month labels and jump-to-date; IntersectionObserver-driven thumbnails from CDN with blurhash placeholders; viewer with arrow-key/swipe navigation and next/prev prefetch (medium first, full/RAW download on demand); multi-select by day header; keyboard shortcuts. Later: search by camera/lens/ISO, favorites, map from GPS.

### Costs / backfill facts
- EXIF backfill should use **ranged GETs of the file header** (~1–2 MB; verify per format) rather than downloading whole 25–60 MB RAWs.
- Thumbnails/medium must exist for every photo for this to feel like Google Photos: a one-off full generation run (roughly a few seconds of Lambda per RAW at 1 GB; order of a few dollars per 50k photos, to be measured on a sample) with reserved concurrency to keep it cheap and bounded.
- Rust `rawler` already gives dimensions and orientation, so `width/height` come from the same pass.

## Originals are irreplaceable: safety constraints (owner, 2026-09-25: assume no backup exists)
1. **No tool, script or session touches `<photo-bucket>` without explicit per-action owner approval**, not even listing.
2. **Photo bucket becomes read-only for our code.** Today `storageBucket.grantReadWrite(previewGeneratorLambda)` also grants `DeleteObject`/overwrite on every original. Target: Lambdas get `s3:GetObject` (+ `ListBucket` for the indexer) on `all images/*` only; nothing can write or delete there.
3. **All generated data goes to a separate new bucket** (`lawt-derived-<account>`): thumbnails, medium, full JPEGs, Iceberg data/metadata, read-model JSON, Athena results. This also removes derivative clutter from the photo folders and the suffix/recursion problems. Rule: only one code path may read originals, and none may write to that bucket.
4. **No changes to the photo bucket's configuration** (notifications, versioning, policy, lifecycle, CORS). CDK only *imports* it. Consequence: no S3 event notifications on it (they rewrite bucket notification config). New photos are found by a scheduled read-only reconcile Lambda (list `all images/` by prefix, diff against the table) that the owner approves; rclone uploads appear within the schedule interval.
5. **Backup is the owner's call, not ours.** Strongly recommended independently of this project (an rclone-synced bucket with no versioning/replica means one bad sync, or `rclone sync` with a wrong source, deletes originals permanently). Options for the owner to decide: S3 Versioning + noncurrent-version lifecycle, cross-region replication, or a periodic copy to another bucket/Glacier Deep Archive (~$1/TB/month). We will not enable any of these on the owner's behalf.
6. First real read of originals (backfill) is run by/with the owner: dry-run mode that only prints what it would do, then a small sample folder, then everything; the code path uses only Get/List.

## Language constraints (owner, 2026-09-25)
All new backend code is **Rust**: indexer, read-model builder, search/list API, backfill, EXIF and preview generation (one workspace, shared `common` crate). Node `list-images`/signers are ported as part of Phase 3 rather than extended (the interim `?galleries=1` fix stays as-is until then). Athena is driven from Rust via `aws-sdk-athena` with `ExecutionParameters`; native `iceberg-rust` writes (removing Athena from the write path) become a realistic later option instead of a "future" Phase 5. Frontend is React + TypeScript with API types generated from the Rust structs (`ts-rs`).

## Format policy (owner answers, 2026-09-25)
No HEIC/WebP/GIF/BMP files today but possible later; TIFFs are probably 16-bit.
- WebP, GIF, BMP: supported via the `image` crate (cheap). **HEIC/HEIF: recognised but reported `unsupported` (explicit status, no placeholder)** until a decoder is chosen; needs libheif (native build, cross-compile risk) so it is a separate task.
- 16-bit TIFF: decode to 16-bit, convert to 8-bit sRGB honouring the embedded ICC profile (pure-Rust `moxcms`), then JPEG. Budget Lambda memory for the decoded frame (60 MP × 16-bit RGB ≈ 360 MB, plus copies): plan ~3 GB and verify on a sample. Only uncompressed/LZW/Deflate TIFF is supported by the `tiff` crate; JPEG-in-TIFF would be reported unsupported.

## Phases (revised)
0. **Done in this branch:** prefix/gallery bug fix. Measure baseline (see below).
1. Infra: Glue db, Athena v3 workgroup + results bucket (1-day lifecycle), Iceberg table via DDL, SQS+DLQ, scoped IAM, S3 notifications (prefix filter, create+remove).
2. Indexer Lambda + backfill (same code path: list bucket → SQS messages), EXIF from Rust Lambda; generate read model.
3. Read path: `list-images` serves read model with `USE_ICEBERG` flag + fallback; add `has_*` flags and CDN thumbnail URLs (TODO P1); pagination.
4. Search endpoint + filter UI (gallery, date range, camera; later ISO/lens).
5. Maintenance (weekly OPTIMIZE/VACUUM, nightly reconcile), alarms on DLQ depth.

## Baseline to capture before Phase 1 (needed to prove "faster")
Gallery load waterfall (DevTools): time to first thumbnail, number of API calls, `/images` duration and payload size, HEAD/sign time in batch signer (CloudWatch). Store the numbers in this ADR.

## Open questions for the owner
1. Approve the read-model design (Athena only for search)?
2. Approx. number of objects and galleries in the bucket (sizes the design)?
3. OK to grant `<deploy-user>` the extra IAM (glue, athena, sqs, s3-notifications) and to create resources in eu-west-1?
4. Do you want EXIF search in v1, or only fast galleries + preview state first?


## Implementation status (2026-09-25)
Decided while building, deviating from the plan above:
- **No Iceberg/Athena yet.** For ~10k photos, per-photo JSON records plus a rebuilt read model give the same UX with no Athena latency, IAM sprawl or maintenance. The record format (`Photo` + status) is the natural input for a later Iceberg table (records -> Parquet) if EXIF search needs it; that remains open and cheap to add.
- **Manual pipeline, not S3 events.** The photo bucket is never reconfigured (no notifications). New photos are picked up by re-running `plan`/`run` (idempotent), later on a schedule if the owner approves.
- **Originals are read-only for our code**, enforced by the `Originals` trait having no write method and by IAM (`GetObject` + prefix-scoped `ListBucket` only).
- **Delivery model:** CloudFront over the derived bucket, `/derived/*` only, salted-HMAC ids (unlisted links). Strictly private alternative (signed cookies) not built.
- **Companions:** a RAW's renditions come from the RAW's embedded preview; if that cannot be decoded, the camera JPEG next to it is used. JPEG originals get no derived "full" (`has_full=false`); the viewer offers "Download original" instead.
- **Failed/unsupported files** (e.g. HEIC) still appear in the timeline with an extension tile, so nothing silently disappears.
- **Security finding fixed:** every API method was unauthenticated because the Cognito authorizer had been passed to the wrong CDK option. Now enforced, frontend sends the ID token.

- **Camera metadata captured in the first pass:** each record stores make, model, lens, ISO, aperture, shutter, focal length (and 35 mm equivalent) and GPS, so search or an Iceberg table can be built later from the records without re-reading any original.

Still open: real-file verification (RAW orientation, TIFF ICC colour), HEIC decoder, signed cookies, Iceberg for EXIF search, backup of the photo bucket (owner).
