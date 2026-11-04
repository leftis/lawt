## Why
The code uses the S3 API only. Other stores speak it too (Backblaze B2, Cloudflare R2, MinIO), but the endpoint, credentials and region are not configurable yet.

## What to do
- Add settings for the storage endpoint and credentials of the photo store and the derived store.
- Pass them through the pipeline, the API Lambda and the dev server.
- Test the pipeline against a local MinIO container.

## Done when
- The pipeline runs `plan`, `run` and `publish` against MinIO in a test.

## Triage
Blast radius 2 · Risk 1 · Complexity 1 · Size 1 = 5 → **Spec** track
