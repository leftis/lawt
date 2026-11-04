## Why
The ID token sits in `localStorage` with no expiry check or refresh. `App.tsx` and the old `Login.jsx` both handle the Cognito `?code`, which can race. A 401 clears the token and redirects through the URL hash.

## What to do
- Keep one place that exchanges the code.
- Use the refresh token to renew the ID token before it expires.
- Check token expiry before each request.
- Consider an in-memory token plus an HTTP-only cookie.

## Done when
- A session survives past the first token expiry without a new login.
- Only one code path handles `?code`.

## Triage
Blast radius 1 · Risk 1 · Complexity 1 · Size 1 = 4 → **Planned** track

_Source: TODO.md, P1 correctness_
