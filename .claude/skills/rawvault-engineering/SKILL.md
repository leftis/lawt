---
name: rawvault-engineering
description: Use for any non-trivial RawVault change or design decision — architecture, performance budgets, security review, testing, and definition of done. Load alongside the stack skill for the area you are touching.
---

# RawVault engineering standards

Companion skills: `software-architecture`, `engineering:system-design`, `engineering:code-review`, `engineering:testing-strategy`, `engineering:architecture` (ADRs), `performance-profiler`, `security-review`. Stack skills: `rawvault-frontend`, `rawvault-aws-cdk`, `rawvault-preview-pipeline`.

## Principles
0. **Stack:** backend Rust, frontend React + TypeScript (strict). Types flow Rust → TS, not the other way.
1. **Measure first.** No performance change without a before/after number (requests per gallery load, bytes transferred, Lambda duration/memory, LCP, bundle size).
2. **Smallest change that solves it.** This is a personal project: prefer boring, serverless, cheap. Reject always-on infrastructure and heavy frameworks unless justified in an ADR.
3. **One source of truth.** Config (bucket, region, IDs, origins) and shared logic (CORS, key validation, extension lists) live in one place. Duplication is tracked debt.
4. **Fail loudly, degrade gracefully.** Errors carry a stable shape and are logged; the UI shows a usable fallback, never a blank screen.
5. **Security by default.** Least-privilege IAM, allow-listed CORS, validated inputs, no secrets in repo, originals never publicly readable.
6. **Reversible and reviewable.** Small commits (`type: message`), no destructive AWS actions without the owner, infra changes shown as `cdk diff` first.

## Performance budgets (targets; adjust with evidence)
| Area | Budget |
|---|---|
| Gallery first meaningful paint | < 1.5 s on broadband, first 30 thumbs visible |
| Thumbnail | ≤ 50 KB, lazy, cacheable for a year |
| API calls per gallery load | ≤ 3 (list + 1 batch), not O(images) |
| `/images` response | paginated, ≤ ~200 items/page |
| Fullscreen viewer | medium visible < 500 ms after open; next image preloaded |
| Lambda list/sign p95 | < 500 ms warm |
| Frontend JS bundle | track in PR; justify > +30 KB gzip |

## Definition of done
- [ ] Builds/synths clean: `npm run build`, `cdk synth`, `cargo check` (whichever apply)
- [ ] Tests added or updated for logic changes (see TODO P2 for setup status); manual browser check for UI (desktop + mobile width)
- [ ] Perf impact stated (numbers or "none")
- [ ] Security check: inputs validated, CORS/IAM not widened, no secrets or tokens logged
- [ ] `TODO.md` updated; `docs/ARCHITECTURE.md` updated if behaviour/contract changed
- [ ] Non-obvious decision → ADR in `docs/adr/NNNN-title.md` (context, decision, consequences) and/or a memory entry

## Decision records
Larger choices (Iceberg vs DynamoDB for metadata, CDN delivery model, auto-generation trigger) get a short ADR in `docs/adr/` before implementation. `docs/iceberg-plan.md` is a proposal, not an accepted decision.

## Review lens (use for self-review)
Correctness → security → data safety (never overwrite/delete originals; the pipeline only writes derivatives) → performance/cost → simplicity → tests → docs.
