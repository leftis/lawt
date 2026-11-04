# 02. Pricing plans and unit economics (CFO view)

## Bottom line
Price the plans against the photographer anchors (Lightroom $11.99, SmugMug Source $10), **but only launch after moving originals off S3 Standard.** On cheap object storage the blend earns about **52% gross margin** and breaks even at about **280 paying users** (before founder pay). On AWS S3 Standard every 1 TB plan loses money.

## Inputs
| Input | Value | Confidence |
|---|---|---|
| Average RAW file | 35.5 MB (7,826 real files) | 🟢 measured |
| Derived data (thumbnail, medium, full JPEG, records) | 5.4% of originals | 🟢 measured |
| Processing cost | about $0.002 per GB of originals, one-off | 🟡 estimated (Lambda estimated, S3 measured) |
| All-in cost per GB per month: cheap object storage (B2/R2 class) | $0.0075 | 🟡 [B2 $6/TB, R2 $0.015/GB](https://www.backblaze.com/cloud-storage/pricing) |
| All-in cost per GB per month: AWS tiered | $0.012 | 🟡 |
| All-in cost per GB per month: AWS S3 Standard | $0.027 | 🟢 list price + measured ratios |
| Per-user fixed (auth, API, logs) | $0.10 per month | 🔴 |
| Card fees | 2.9% + $0.30 per charge | 🟡 |
| Quota fill | 60% average | 🔴 validate in beta |
| Billing mix | 50% annual | 🔴 |

## Plans
| Plan | Monthly | Annual | Storage | About RAW photos | For |
|---|---|---|---|---|---|
| **Free** | $0 | | 10 GB | about 280 | Try it with a real shoot; timeline, viewer, 1 share link |
| **Enthusiast** | $4.99 | $44 | 250 GB | about 7,000 | The Fuji or Ricoh shooter with a few years of photos |
| **Pro** (recommended) | $9.99 | $89 | 1 TB | about 28,000 | Serious enthusiasts, semi-pros; sharing, search, bin |
| **Studio** | $24.99 | $249 | 2 TB | about 56,000 | Client galleries, expiring links, download packs, team seat later |
| Extra storage | $7 per TB per month | | | | Deliberately thin margin: discourages hoarding |

Guardrails: hard storage caps (never "unlimited"), fair-use download limit of 2 times stored data per month, cold tier for originals older than 90 days, one account per person.
Founding-member offer: Pro at $79 per year, locked for life, capped at 200 seats (cash and validation; 🟡 risk of low future ARPU).

## Margins by plan (cheap object storage, per user per month)
| Plan | Billing | Fill 60% | Fill 100% |
|---|---|---|---|
| Enthusiast | monthly / annual | 67% / 63% | 52% / 43% |
| Pro | monthly / annual | 48% / 35% | 18% / **-6%** |
| Studio | monthly / annual | 59% / 53% | 35% / 24% |
Pro on annual billing at a full terabyte is loss-making. That is why quotas are capped and fill is monitored.

## Blended economics (mix 55% Enthusiast, 40% Pro, 5% Studio)
| Storage backend | ARPU | Cost of service | Gross profit per user | Gross margin |
|---|---|---|---|---|
| **Cheap object storage** | $7.01 | $3.33 | $3.67 | **52%** |
| AWS tiered | $7.01 | $5.06 | $1.95 | 28% |
| AWS S3 Standard | $7.01 | $10.79 | -$3.79 | **-54%** |
Average stored per paying user: about 380 GB. Storage businesses run structurally lower margins than SaaS (target above 65%); improve with pricing, tiering, dedupe of RAW+JPEG, and cheaper backends.

## LTV, CAC, payback (cheap storage; gross profit $3.67 per user per month)
| Monthly churn | LTV | CAC $25 (community, organic) | CAC $60 (paid) |
|---|---|---|---|
| 2% | $184 | LTV:CAC 7.3, payback 6.8 months | 3.1, 16.3 months |
| 3% | $122 | 4.9, 6.8 months | 2.0, 16.3 months |
| 5% | $73 | 2.9, 6.8 months | 1.2, 16.3 months |
🔴 Churn and CAC are assumptions. Photo archives are sticky (moving hundreds of GB is painful), which favours low churn, but that also raises the bar of trust at signup.

## Break-even and cash
- Fixed costs (assumed): infra baseline $50, tools $100, legal and accounting $300, support tools $50, domain and misc $20, marketing $500 = **$1,020 per month**. 🔴
- **Break-even: 278 paying users** (no founder pay); **1,095 users** with $3,000 per month founder pay.
- Scenarios (paying users):

| Scenario | Month 12 | Month 24 | ARR at M24 | Gross profit after fixed at M24 | Stored at M24 |
|---|---|---|---|---|---|
| Bear | 150 | 400 | $34k | $0.4k per month | 153 TB |
| Base | 500 | 1,800 | $151k | $5.6k per month | 688 TB |
| Bull | 1,500 | 6,000 | $504k | $21k per month | 2.3 PB |
- **Cash need, first 12 months, no founder pay:** bear about **$8.7k**, base about break-even (-$0.3k). Plan for about $10k of runway before paying anyone, including yourself. 🟡

## VAT and store fees (often forgotten)
- EU consumer prices must include VAT. At 24% (Greece), Pro at $9.99 nets $8.06 and margin drops from 48% to 36%. Price EU customers in euros VAT-inclusive, or accept about 12 points less margin. 🟡
- A Merchant of Record (Paddle, Lemon Squeezy) handles VAT worldwide but costs about 5% plus $0.50 per sale, which hurts the $4.99 plan most. Compare with Stripe Tax before choosing. 🟡
- Native app stores take 15 to 30% of in-app subscriptions. Keep billing on the web and check current store rules on external payment links. 🟡

## Levers to raise margin (in order)
1. Move originals to cheap storage (28% to 52%).
2. Keep only thumbnails and medium images hot; full-size JPEGs on demand or cold.
3. Annual billing (removes the $0.30 fee per charge).
4. Dedupe RAW+JPEG pairs and identical files by hash.
5. Raise Pro to $11.99 monthly if beta willingness to pay allows (adds about 15 points).
6. Sell Studio and add-ons (delivery, extra seats) which carry the margin.

## How to validate prices before building billing
1. Van Westendorp survey (four price questions) with 30+ target photographers; run on the beta cohort.
2. Landing page with the three plans and a real "reserve your place" deposit or founding-member pre-order; measure clicks and pre-orders per plan.
3. Concierge beta: 25 photographers, hand-onboarded, charged from day one at Pro $9.99. Measure fill, downloads and retention.
4. Decision rule: proceed if at least 30% of interviewed ICP A say they would pay $9.99 and at least 10 of 25 beta users convert to paid. 🔴 thresholds are proposals.

## Pricing page checklist
Three plans plus Free, "Most popular" on Pro, annual toggle on by default with "$89 per year, less than Lightroom's $11.99 per month", RAW-photo counts next to GB, FAQ (cancel any time, export all originals, what happens at the limit, refunds, security, VAT).
