# 03. How people get in: access, upload, architecture and roadmap

## Bottom line
**Upload is the make-or-break experience.** A RAW library is hundreds of gigabytes; the first upload takes days on a home connection. The product wins if the user sees their library within minutes. So: a **Rust desktop uploader that extracts previews and metadata locally and uploads those first**, originals following in the background. Web app first for viewing and sharing, desktop uploader second, mobile after that.

## The upload problem in numbers
| | Value |
|---|---|
| Library | 10,000 RAW photos = about 355 GB |
| Upload over a 20 Mbps home uplink | about **39 hours** (14 seconds per photo) |
| Upload over 100 Mbps | about 8 hours |
| Previews and metadata only (medium + thumbnail about 0.32 MB per photo, 3.2 GB) | about **20 minutes** at 20 Mbps |
Both figures are from the measured RawVault sizes. 🟡 (uplink speeds assumed)

**Consequence:** ingest in two stages. Stage 1 (minutes): previews plus EXIF from the user's machine; the timeline appears and is fully browsable. Stage 2 (hours or days): originals upload in the background with resume, throttling and scheduling. Show honest progress ("1,240 of 10,000 originals safe").

## Access channels
| Channel | What it does | Build | Impact | Notes |
|---|---|---|---|---|
| **Web app** (PWA, installable) | Browse, search, view, share, manage bin; drag-and-drop upload | done (view) / M (upload) | High | Resumable multipart upload with presigned URLs, checksums, hash dedupe. Works everywhere, no store fees |
| **Desktop uploader** (Tauri: Rust core, React + TypeScript UI; macOS and Windows first) | Watch folders and SD cards, local preview and EXIF extraction using our own `rawvault-processor` crate, resumable background upload, bandwidth limits and schedules, never deletes local files | L | **Highest** | The differentiator. Our Rust processing code runs natively on the desktop |
| **rclone / S3-compatible** | Power users upload with the tools they already use (the founder does) | M | Medium | Per-tenant scoped credentials; scheduled or event scan on the product-owned bucket |
| **Mobile: PWA first, native later** | View, search, share, capture phone photos and ProRAW DNG | M (PWA) / L (native) | Medium | Cameras send JPEG to phones far more often than RAW; native app mainly for auto-upload of the camera roll and camera-transfer albums. Native billing adds store fees (keep billing on the web) |
| **Importers** (Google Takeout, Amazon Photos, Flickr) | Bring an existing library in | M | High for acquisition | Turns competitors' unhappy users into ours |
| **Lightroom Classic publish plugin, Capture One** | Fits pro workflows | L | Low until Studio grows | Later |
| **Public API** | Integrations | M | Low | Later |

## Target architecture (multi-tenant)
```
Desktop app / web / rclone
   │ 1. previews + EXIF (small, first)      2. originals (large, resumable, hashed)
   ▼
API (Rust, Cognito or similar)  ──►  metering, quotas, authorization on EVERY route by tenant id
   ▼
Object storage (S3-compatible; B2/R2 for originals, R2/S3 + CDN for renditions)
   t/{tenant}/orig/...     (never modified, exportable)
   t/{tenant}/derived/...  (thumbnails, medium, full, records, read model)
   ▼ object-created events (bucket is ours, so events are fine here)
Queue (SQS) ─► pipeline workers (Rust Lambda) ─► records + renditions ─► read model
   ▼
Delivery: CDN with signed cookies (private), share-link service (expiring tokens)
```
Key design decisions:
- **Tenant isolation everywhere:** tenant id in every key and every query; per-tenant id salt derived from a master secret; automated isolation tests; security review before beta.
- **Storage abstraction:** the Rust code already uses the S3 API; B2 and R2 are S3-compatible, so the backend becomes configuration, not a rewrite.
- **Private renditions:** replace today's "unlisted link" model with CloudFront signed cookies (or R2 signed URLs).
- **Quotas and metering:** bytes per tenant computed from records; hard cap per plan; alarms on cost per tenant.
- **Idempotent, retryable processing:** dead-letter queue, poison-file handling (one bad RAF must never block a library).
- **Never delete originals on the user's behalf** except through the bin after 30 days, with versioning as the safety net.

## Roadmap (durations are 🔴 estimates for one full-time developer)
| Phase | Weeks | Goal | Exit criteria |
|---|---|---|---|
| 0. Personal instance | done | Prove the pipeline and timeline on the founder's real library | 7,826 photos, 22 minutes, 1 failure |
| 1. Concierge beta | 1 to 8 | 25 photographers using it, hand-onboarded, unpaid or founding-member | Multi-tenant foundations, web upload, private delivery; 25 users; measured cost per GB and fill |
| 2. Paid beta | 9 to 16 | First revenue, product safe for strangers | Billing, quotas, share links with expiry, bin, desktop uploader alpha; 10+ paying users |
| 3. Public launch | 17 to 26 | Self-serve signup | Desktop uploader GA, PWA, importers, marketing site, support tooling; 280 paying users is the break-even target |
| 4. Growth | later | Mobile, Studio depth, teams | Native mobile, client proofing, team seats, API |

## Epics (copy into GitHub issues; details of the first two are in `TODO.md`)
1. **Multi-tenancy foundation:** tenant prefixes, authorization on every route, per-tenant salts, isolation tests.
2. **Storage backend abstraction and migration** to B2/R2 for originals; measured cost per GB.
3. **Web uploader:** resumable multipart, checksums, hash dedupe, folder upload, progress UI.
4. **Event-driven processing** with queue, retries, dead-letter, per-tenant concurrency limits.
5. **Desktop uploader (Tauri + Rust):** folder watch, card detection, local preview and EXIF extraction, resumable upload, throttling, verified-upload status.
6. **Private delivery:** signed cookies for renditions.
7. **Expiring share links** and client galleries (Studio).
8. **Bin (30 days)** with restore; purge only touches derived data unless originals live in a tenant-owned bucket with versioning.
9. **Billing, plans, quotas, metering, dunning, VAT** (Merchant of Record decision first).
10. **Account lifecycle:** full export of originals (zip or rclone), delete account, data retention.
11. **Abuse and safety:** content reporting, takedown workflow, hash-based detection hooks (needs legal advice).
12. **Search:** camera, lens, ISO, date range from the records already captured.
13. **PWA and mobile:** installable app, then native auto-upload.
14. **Importers:** Google Takeout, Amazon Photos, Flickr.
15. **Observability and cost control:** SLOs, alarms, per-tenant cost dashboard, budget alerts.
16. **Marketing site, pricing page, waitlist, docs.**
17. **Support tooling:** admin console, impersonation with audit log, status page.
