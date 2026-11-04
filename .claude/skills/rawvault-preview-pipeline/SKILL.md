---
name: rawvault-preview-pipeline
description: Use when working on RawVault image preview generation — the Rust Lambda (rawler/image), derivative key naming (.thumbnail/.medium/.full), RAW/TIFF/JPEG decoding, batch generation, or list/sign logic that depends on derivatives.
---

# RawVault preview pipeline

Rust Lambda at `infra/cdk/src/lambda/preview-generator-rust/` (crate `rawvault-preview-generator`, binary `bootstrap`, arm64, `provided.al2023`, 1024 MB, 15 min). Also load `rust-engineer` / `rust-best-practices` (and `ms-rust`, which is mandatory before touching any `.rs` file).

## Contract (keep these in sync everywhere)
Derivatives are written next to the original: `<key>.thumbnail.jpeg` (~400px), `<key>.medium.jpeg` (~1920px), `<key>.full.jpeg`. Legacy `.preview.png/.jpg` are read-only fallbacks.
A suffix change or addition touches all of: Rust (`thumbnail_key`, `medium_preview_key`, `full_jpeg_key`, `is_preview_key`), `list-images` (skip list), `download-signer`, `batch-download-signer` (`TYPE_MAP`), frontend `api.js`. The RAW extension list exists in both `list-images` and Rust `is_raw_extension`.

Invocation: `{ "key": "all images/..." }` (async, from `preview-trigger`) or an S3 event. The handler must never process a derivative key (recursion → infinite loop and cost). No S3 trigger exists today; if added, filter by prefix and suffix and set reserved concurrency + DLQ.

## Rules for changes
- Idempotent: re-running for the same key overwrites the same three outputs.
- Failures must be visible: return an error status and do not write a placeholder under a real derivative key (today's placeholder-under-`.jpeg` behaviour makes failures look like success; see TODO). Do not swallow S3 upload errors.
- Memory: originals are 25–60 MB. Avoid extra full-frame copies (`to_rgba8()` on the full image doubles/quadruples memory); resize from the decoded buffer, encode straight to JPEG.
- Quality/size targets: thumbnail 20–50 KB, medium 200–400 KB, full q≈92. Set `Content-Type` and `Cache-Control: public, max-age=31536000, immutable` on uploads (keys are content-stable per original).
- Orientation: apply EXIF orientation when decoding; verify with a portrait RAF/NEF/DNG.
- Bucket comes from env (`STORAGE_BUCKET`), not the const.

## Build & test
```bash
cd infra/cdk/src/lambda/preview-generator-rust
cargo check && cargo clippy -- -D warnings && cargo test
cargo lambda build --release --arm64      # or: make build-rust (normalises output path used by CDK)
```
History: cross-compiling was blocked by `aws-lc-sys` sysroot headers (see `.pi/progress.md`, `Dockerfile`, `scripts/build-preview-generator.sh`). If the build fails on that, use the Docker path rather than downgrading TLS silently; consider `rustls` with the `ring` provider.
Add unit tests for pure helpers (key builders, extension checks, resize dimensions) before refactoring. Keep sample fixtures tiny (do not commit real RAW files).

## Performance ideas backlog
Parallel derivative uploads (`tokio::join!`), decode once and derive all sizes, WebP/AVIF variants, embedded-JPEG fast path for thumbnails (skip full demosaic when the embedded preview is large enough). Measure duration and peak memory in CloudWatch before and after.
