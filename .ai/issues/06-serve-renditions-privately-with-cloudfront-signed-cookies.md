## Why
Renditions are reachable by anyone who has the URL. The URL contains a salted 128-bit id, so it cannot be guessed, but a copied link works forever. Signed cookies are needed before share links, and whenever more than one person uses a deployment.

## What to do
- Create a CloudFront key group and public key.
- Add an API route that sets signed cookies for `/derived/*` after login.
- Require signed cookies on the distribution behavior.
- Update the frontend to request cookies before loading images.

## Done when
- An unsigned request to a rendition URL returns 403.
- The timeline works after login on desktop and phone.
- Cookie lifetime and refresh are documented.

## Triage
Blast radius 2 · Risk 1 · Complexity 1 · Size 1 = 5 → **Spec** track

