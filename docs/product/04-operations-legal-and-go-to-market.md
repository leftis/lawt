# 04. Operations, legal and go-to-market (COO and marketing view)

## Bottom line
This is a **trust business**: customers hand over the only copy of irreplaceable photos. Operations, durability promises and legal basics matter more than features. Run it as a solo founder with tight automation, a small paid beta first, and a clear "you can always take your originals back out" promise.

## Operating model
| Area | Approach | Owner |
|---|---|---|
| Support | Email plus docs and a community forum; target first reply under 1 business day in beta; every recurring question becomes a doc or a product fix | Founder |
| Reliability | SLOs: API availability 99.9%, timeline first paint under 2 seconds, upload resume always works; status page; on-call is the founder, so alarms only for user-visible failures | Founder |
| Durability promise | Say what is true: originals stored with the provider's durability (S3 is designed for 11 nines; verify the chosen provider's statement). Offer a second copy (versioning or cross-region) as a paid add-on or Studio feature. Never promise "backup" unless it is one | Founder |
| Incidents | Runbook per failure (pipeline stuck, CDN errors, billing failure); post-mortem for any data-affecting event; tell affected users within 24 hours | Founder |
| Cost control | Budget alarms, per-tenant storage and egress metering, fair-use limits, monthly review of gross margin per plan | Founder |
| Security | Tenant-isolation tests in CI, dependency audit, least-privilege IAM, secrets in a secrets manager, external pentest before public launch | Founder + contractor |
| Data | Encryption at rest and in transit; EU region option; export and deletion self-serve | Founder |

Process maturity today: level 2 to 3 (documented and partly measured). Bottlenecks to fix first: onboarding by hand (does not scale past about 50 users), no automated processing trigger, no per-tenant cost visibility.

## Legal and compliance checklist (needs a lawyer; this is not legal advice)
- [ ] Company entity, terms of service, acceptable use policy, privacy policy, cookie policy.
- [ ] GDPR: data processing agreement, records of processing, EU data location, breach process, right to export and delete.
- [ ] EU VAT on digital services (OSS) or a Merchant of Record; VAT-inclusive consumer prices.
- [ ] Hosting user content: notice-and-takedown process, abuse reporting, and the legal duties of a hosting provider (including child-safety obligations). Get advice before launch.
- [ ] Trademark check on the name "RawVault" and domain.
- [ ] Refund and cancellation policy; what happens to files after cancellation (retention window, export).
- [ ] Sub-processors list (cloud, payment, email).

## Go-to-market
**Sequence:** waitlist and interviews, then concierge beta, then paid beta, then public launch. Do not build billing before the interview and beta signals.

| Stage | Action | Success measure |
|---|---|---|
| 1. Discovery (weeks 1 to 3) | 30 interviews with ICP A from r/fujifilm, r/ricohGR, Fuji X Weekly, DPReview; ask what they use, what broke, what they pay | 20+ describe the same pain unprompted; 30% would pay $9.99 |
| 2. Smoke test | Landing page with the positioning, three plans, waitlist and founding-member pre-order | 200 waitlist signups; 20 pre-orders |
| 3. Concierge beta (weeks 4 to 12) | Onboard 25 photographers by hand using the existing pipeline pattern; charge founding price from day one | 10+ convert to paid; fill and cost per GB measured |
| 4. Paid beta | Self-serve signup capped at a few hundred users | 100 paying users; churn under 4% monthly |
| 5. Launch | Desktop uploader GA, importers, comparison pages, camera YouTubers | 280 paying users (break-even) |

**Channels:** communities (honest participation, not spam), camera YouTubers and newsletters (offer free Pro to reviewers), SEO pages that match real searches ("Google Photos Fujifilm RAF", "Amazon Photos RAW preview", "Lightroom alternative for RAW storage"), comparison pages against each competitor, referral credit (extra storage), Fuji and Ricoh owner forums.
**Messaging by persona:** enthusiast "your whole library, by date, on your phone"; pro "send clients an expiring link in seconds"; both "originals untouched and exportable".
**Launch tier:** treat the public launch as a Tier 2 launch (small budget, founder-led); a Tier 1 press launch only makes sense after retention data.

## Metrics (weekly)
Activation (library indexed within 24 hours of signup), first share created, weekly active uploaders, storage per user versus quota, gross margin per plan, churn by plan, support tickets per 100 users, cost per GB.

## 90-day OKRs
- **O1. Prove people want it and will pay.** KR1: 30 interviews completed. KR2: 200 waitlist signups. KR3: 25 concierge beta users. KR4: 10 paying at founding prices.
- **O2. Make it safe for strangers.** KR1: tenant isolation and authorization on every route, verified by automated tests. KR2: originals on cheap storage with a measured cost per GB. KR3: private delivery in place.
- **O3. Fix the first hour.** KR1: web upload resumes after a dropped connection. KR2: desktop uploader alpha shows a library from previews in under 20 minutes for 10,000 photos.

## Risk register
| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Amazon Photos (free with Prime) is good enough | Medium | High | Target camera models it mishandles; sell the experience and export promise; do not compete on price |
| Storage costs erode margin | High if on AWS Standard | High | Move originals to cheap storage before charging; quotas, fair use, cold tier |
| Data loss event destroys trust | Low | Very high | Versioning or second copy, integrity checks, no destructive operations on originals, clear promises |
| Legal duties of hosting user content underestimated | Medium | High | Legal advice before launch; reporting and takedown workflow |
| Solo-founder overload (support, ops, building) | High | Medium | Small beta cap, automation, docs-first support, hire contractor for pentest and design |
| Google or Adobe close the RAW gap | Low to medium | Medium | Stay niche; lean on export, price and Fuji/Ricoh depth |
| Willingness to pay below $9.99 | Medium | Medium | Validate with pre-orders before building billing; fall back to Enthusiast-heavy mix |

## Decisions needed from the founder
1. **Storage backend:** AWS tiered, or Backblaze B2 / Cloudflare R2 (recommended). Affects margin most.
2. **First market and currency:** EU (VAT-inclusive euros) or US.
3. **Billing route:** Stripe with Stripe Tax, or a Merchant of Record (Paddle, Lemon Squeezy).
4. **Free tier size and founding-member offer** (10 GB free; 200 seats at $79 per year).
5. **Time commitment and runway:** about $10k cash for 12 months in the bear case, more if you pay yourself.
6. **Name and trademark:** confirm RawVault is available.
7. **Legal counsel:** who reviews terms, GDPR and hosting duties.
8. **Beta cohort:** who are the first 25 photographers you can reach personally?
