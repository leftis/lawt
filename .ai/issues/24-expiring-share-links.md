## Why
Share one photo, a selection or a day with someone who has no account, and let the link expire or be revoked.

## What to do
- A record `shares/{token}.json` in the derived bucket: 128-bit random token, photo ids, expiry, created time, download allowed or not.
- Public Rust route `GET /s/{token}` (no Cognito) that checks expiry and returns the photo list.
- Renditions for shared photos through CloudFront signed URLs or a short-lived signed cookie (depends on the signed-cookies issue).
- Expiry choices: 1 hour, 1 day, 7 days, 30 days, custom. Revoke from a list in the app.
- Originals are never exposed unless the owner chooses it for that link.

## Done when
- Expired and revoked links return 410.
- A link lists only its own photos.
- Tests cover expiry boundaries and revocation.

## Triage
Blast radius 2 · Risk 1 · Complexity 1 · Size 1 = 5 → **Spec** track

_Source: TODO.md, 'Next features'_
