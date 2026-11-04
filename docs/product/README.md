# RawVault as a product: plan

Written 2026-09-26. Market data was researched live in September 2026 (sources in each document); financial figures are computed from **measured** RawVault costs plus explicit assumptions. Confidence tags: 🟢 sourced or measured, 🟡 estimated, 🔴 assumed, needs validation.

| Document | Covers |
|---|---|
| [01-market-and-positioning.md](01-market-and-positioning.md) | Competitors, target audience, positioning, the price anchor |
| [02-pricing-and-unit-economics.md](02-pricing-and-unit-economics.md) | Plans, margins, break-even, scenarios, VAT, guardrails (CFO view) |
| [03-access-upload-and-roadmap.md](03-access-upload-and-roadmap.md) | Web / desktop / mobile / rclone access, the upload problem, architecture, roadmap, epics |
| [04-operations-legal-and-go-to-market.md](04-operations-legal-and-go-to-market.md) | Operating model, reliability, legal, launch plan, 90-day OKRs, risks (COO + marketing view) |

## The plan in ten lines
1. **Who:** RAW-shooting enthusiasts (Fujifilm, Ricoh GR, Sony, Nikon, Canon), 50 to 500 GB, no Adobe subscription, badly served by Google Photos and Amazon Photos. Working photographers who deliver to clients are the second segment (Studio plan).
2. **Why they switch:** Google Photos supports almost no Fujifilm RAF files and shows only the embedded JPEG; Amazon Photos stores RAW but does not preview most of it. RawVault reads every RAW format, files by capture date and keeps the originals untouched and exportable.
3. **Anchor:** Adobe Lightroom 1 TB at $11.99 and SmugMug Source (RAW) at $10 per TB. Not Google's $1.99, and not Amazon's "free with Prime".
4. **Plans:** Free 10 GB, Enthusiast $4.99 (250 GB), Pro $9.99 (1 TB), Studio $24.99 (2 TB plus client galleries). Annual prices about 11 to 25 percent lower.
5. **Economics:** about 52 percent gross margin on cheap object storage; break-even around 280 paying users before founder pay. On AWS S3 Standard the plans lose money.
6. **Storage backend:** move originals to cheap S3-compatible storage (Backblaze B2 or Cloudflare R2) before charging anyone; keep small renditions on a CDN.
7. **Upload is the product's hardest part:** 350 GB over a home connection is about 39 hours. The desktop uploader extracts previews and metadata locally and uploads those first, so the library is browsable in about 15 minutes while originals follow in the background.
8. **Access:** web app (PWA) first, then a Rust desktop uploader (Tauri), rclone-compatible endpoint for power users, native mobile later.
9. **Before building billing:** interview 30 target photographers, run a paid concierge beta with about 25 of them, test the three price points.
10. **Decisions needed from the founder** are listed at the end of document 04.
