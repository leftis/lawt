# Runbook: bring your photos into the timeline

State on 2026-09-25: everything below is **deployed except the pipeline function**, which is deliberately off. Until you run it, the timeline is empty and the site keeps showing the legacy gallery.

**Nothing here touches the photo bucket until you decide to run step 2.** Step 1 only creates a function with read-only access. Steps 2+ are yours to run (or to tell me to run).

## 0. What is already live
| Piece | Where |
|---|---|
| Frontend (legacy gallery by default; timeline with `?ui=timeline`) | https://<frontend-domain> |
| API (all routes now require a Cognito ID token) | https://<api-id>.execute-api.eu-west-1.amazonaws.com/prod/ |
| Timeline API (Rust Lambda `rawvault-api`) | `/timeline`, `/timeline/months/{YYYY-MM}` |
| Derived bucket (renditions, records, timeline files) | `<derived-bucket>` |
| Rendition CDN (only `/derived/*`) | https://<rendition-cdn-domain> |

First login after this deploy: your old session token no longer works (the API now wants the ID token). You will be sent through the Cognito login once; that is expected.

## 1. Create the pipeline function (read-only access to the photo bucket)
Needs the Rust cross-build, which uses Zig as the linker (`brew install zig`, or `pip3 install ziglang`).
```bash
cd backend && cargo lambda build --release --arm64 -p rawvault-pipeline-lambda
cd ../infra/cdk && npx cdk deploy -c enablePipeline=true --require-approval never --region eu-west-1
```
The function's role may only `ListBucket` (prefix `all images/`) and `GetObject` (`all images/*`) on the photo bucket. It cannot write or delete there. Review with `npx cdk diff -c enablePipeline=true` first if you like.

## 2. Plan (lists keys only, downloads no photo)
```bash
aws lambda invoke --function-name rawvault-pipeline --region eu-west-1 \
  --cli-binary-format raw-in-base64-out \
  --payload '{"action":"plan"}' /dev/stdout
```
Check `photos_in_scope` matches what you expect (~10k), `unsupported_files` and the `sample` keys.

## 3. Try one small folder first
```bash
aws lambda invoke --function-name rawvault-pipeline --region eu-west-1 \
  --cli-binary-format raw-in-base64-out --cli-read-timeout 900 \
  --payload '{"action":"run","folder":"100RICOH","limit":5}' /dev/stdout
```
Open https://<frontend-domain>/?ui=timeline and look at those 5 photos. **Check on real files:** orientation of portrait RAW previews, colour of your 16-bit TIFFs (embedded ICC profiles are not applied), and the capture dates.

## 4. Everything
Repeat with a larger `limit` (default 25; 100-200 is realistic) and no `folder` until `remaining_after_batch` is `0`. Each `run` also republishes the timeline, so photos appear as you go. `{"action":"publish"}` rebuilds the timeline alone.

## 5. Make the timeline the default
Set `VITE_UI=timeline` in `frontend/.env.production`, rebuild and redeploy the frontend. Use `?ui=legacy` to go back on one browser.

## Undo
`cdk deploy` without the flag removes the pipeline function. Renditions and records stay in the derived bucket (safe to delete; they are regenerated from your photos).

## Known limits to verify in step 3
- RAW support reads the camera's embedded preview (via `rawler`). Never run on real RAW files yet; only synthetic images are tested.
- HEIC/HEIF is listed with a placeholder tile, not rendered.
- Renditions are reachable by anyone who has their (unguessable, salted) URL. Signed cookies would make them strictly private.
- The salt is in Secrets Manager (`IdSalt`). Do not rotate it once photos are processed; every id and URL would change.
