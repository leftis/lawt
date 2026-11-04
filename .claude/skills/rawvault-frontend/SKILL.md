---
name: rawvault-frontend
description: Use when editing anything under frontend/ in RawVault (React 19 + Vite gallery UI, api.js, Gallery/ImageCard components, Cognito login flow, image loading performance).
---

# RawVault frontend

React 19 + Vite 6, **TypeScript (strict) is the standard**; existing files are still JSX and are migrated when touched. No router or test runner yet. Source: `frontend/src/`. Read `docs/ARCHITECTURE.md` (Frontend section) first.

## Rules
- New code is `.ts/.tsx`, `strict: true`, no `any` (use `unknown` + narrowing), no `@ts-ignore` without a reason comment. Add `tsconfig.json` + `npm run typecheck` (`tsc --noEmit`) if missing and run it with the build.
- API response types come from the Rust backend (generated with `ts-rs` or OpenAPI) into `src/api/types.gen.ts`; never hand-write a type that mirrors a Rust struct. Validate at the boundary (e.g. zod) only where the server is not trusted to match.
- Migration order when touching a file: rename to `.tsx`, type props/state/hooks, remove `any`, then change behaviour. Keep renames in their own commit.
- All network calls go through `src/api.js`. Do not `fetch` API endpoints from components (App.jsx's token exchange is a known exception to remove, see TODO).
- Config comes from `import.meta.env.VITE_*`. Never hard-code URLs, pool IDs or client IDs. After changing `.env.production`, rebuild (values are baked in).
- Never store secrets. The access token in localStorage is a known weakness; do not spread token handling further.
- Verify with `cd frontend && npm run build`. For UI changes, run `npm run dev` and check in the browser (mock API on :3001 via `infra/mock-api`), including a narrow mobile viewport.
- React 19: function components + hooks only. Type/idiom help: invoke the `react-dev` skill.

## Performance checklist (gallery is image-heavy; this is where quality is won)
- Images: `loading="lazy"`, `decoding="async"`, explicit `width/height` or `aspect-ratio` to avoid layout shift; never load `medium`/`full` in grid cards, only `thumbnail`.
- Fetch URLs in batches (`batchGetDownloadUrls`), never one request per card. Prefer IntersectionObserver so off-screen cards do not request anything.
- Large lists: window/virtualize or use `content-visibility: auto`; do not render thousands of cards at once.
- Memoize: `React.memo` on `ImageCard`, stable callbacks (`useCallback`), no inline object/array props in hot lists. Measure with React Profiler before and after; do not memoize blindly.
- State: keep high-churn state (selection, hover, batch progress) out of components that re-render the whole grid; colocate or use refs.
- Cache: presigned URLs expire in 900 s; a cache must respect expiry. Do not cache `null`/not-yet-generated results permanently.
- Cancel/ignore stale requests on gallery switch (AbortController or request-id guard) to avoid old results overwriting new ones.
- Bundle: watch dependency weight (`aws-amplify` is installed but unused for auth). Check `vite build` output sizes when adding a dependency.
- Batch preview jobs are throttled (concurrency 3, 500 ms stagger) because each invocation costs money; do not raise without reason.

## Accessibility & UX
- Interactive cards are buttons/links with labels; viewer is keyboard-operable (Esc, arrows) and traps focus; alt text from image name.
- Respect `prefers-reduced-motion`. Keep contrast in both themes if theming is added.

## Style
- Prefer `styles.css` classes/CSS variables (`var(--bg-primary)` etc.) over new inline styles. When touching a component with heavy inline styles, migrate the parts you touch.
- Match existing naming; components are PascalCase files in `src/components/`.
