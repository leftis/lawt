## Why
`GET /preview` is wired straight to the preview-generator Lambda, which does not understand API Gateway events. Any signed-in user can reach it. The frontend does not call it (it uses `/preview-trigger`).

## What to do
- Remove `previewResource` and its `addCorsOptions` call from the stack.
- Check with `grep` that no frontend code uses `/preview`.

## Done when
- The route returns 403/404 after deploy and the old gallery still works.

## Triage
Blast radius 1 · Risk 1 · Complexity 0 · Size 0 = 2 → **Direct** track

_Source: TODO.md, P0 security_
