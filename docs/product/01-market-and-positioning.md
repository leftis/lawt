# 01. Market, audience, positioning and the price anchor

## Bottom line
Do not sell "cloud storage". Sell **"your whole RAW library, browsable by date on every screen, originals safe and exportable"** to photographers whose cameras the general-purpose services handle badly. Anchor the price to what photographers already pay for RAW workflows (Lightroom 1 TB $11.99, SmugMug Source $10/TB), not to Google's $1.99 for 100 GB.

## Market tailwind
- 9.44 million digital cameras shipped in 2025 (+11.2%, best since 2020); 6.31 million mirrorless (+12.5%). Fixed-lens compacts had a "meteoric rise". 🟢 [DPReview: CIPA 2025 data](https://www.dpreview.com/articles/2386206926/cipa-data-2025-camera-lens-shipments-fixed-lens-cameras-interchangable/)
- RawVault's own library is exactly this segment: 79% Fujifilm RAF and 14% DNG (Ricoh GR III in the samples checked). 🟢 measured
- Every RAW file is about 35 MB (measured average), so 1 TB holds about 28,000 photos. 🟢 measured

## Competitive landscape (list prices, September 2026)
| Alternative | Price | RAW reality | Weakness we exploit |
|---|---|---|---|
| **Amazon Photos** (with Prime) | "Free" unlimited photos incl. RAW | Stores RAW; no preview for many formats; photographers report RAW auto-save gaps | Experience, reliability of RAW handling. Biggest threat because it is free. 🟢 [1](https://www.digitalcameraworld.com/reviews/amazon-photos-review) [2](https://www.diyphotography.net/psa-amazon-photos-auto-save-might-not-be-auto-saving-all-of-your-raw-files/) |
| **Google One / Photos** | 100 GB $1.99, 2 TB $9.99, 5 TB $19.99 | Only embedded JPEG shown; many models unsupported (Fujifilm: only X100 Bayer listed); "failed uploads" | Fujifilm and newer/older bodies. 🟢 [pricing](https://blog.internxt.com/google-one-pricing/) [RAW](https://www.dpreview.com/forums/thread/4459339) |
| **Adobe Lightroom** | Lightroom 1 TB $11.99; Photography plan 1 TB $19.99 | Excellent RAW, but catalog-centric and a subscription lock-in | Price, lock-in, complexity for people who only want to store and browse. 🟢 [Adobe FAQ](https://helpx.adobe.com/lightroom/desktop/kb/lightroom-1tb-plan-faq.html) |
| **SmugMug Source / Flickr Pro** | RAW at $10 per TB (+$6 per extra TB); Flickr Pro $11 (annual $82) | Direct RAW competitor | Archive-only feel; no capture-date library experience. 🟢 [SmugMug](https://www.saasworthy.com/product/smugmug/pricing) [Flickr](https://www.toolsurf.com/flickr-pricing-2026/) |
| **Pixieset / Pic-Time / ShootProof** | Pixieset: 10 GB $8, 100 GB $16, 1 TB with RAW $24, 2 TB $48 (annual) | Built for client galleries | Expensive for storage alone; we only need to match share links for the Studio plan. 🟢 [Pixieset](https://picflow.com/compare/client-gallery/pixieset) |
| **Mylio Photos** | $20/month | Local-first sync | Price and complexity. 🟢 [Mylio](https://inspire.mylio.com/new-mylio-plans-a-note-from-the-mylio-ceo/) |
| **Immich / PhotoPrism** (self-hosted) | Free + your own server | RAW improving (PhotoPrism ahead) | Needs a server; our buyer will not run one. 🟢 [Contabo](https://contabo.com/blog/immich-vs-photoprism-vs-ente/) |
| **Do it yourself** (drive + rclone + B2) | B2 $6/TB, R2 $0.015/GB | Raw storage, no viewing | This is what the founder does today; the status quo for technical users. 🟢 [B2](https://www.backblaze.com/cloud-storage/pricing) [R2](https://mecanik.dev/en/posts/cloudflare-r2-pricing-explained-real-costs-vs-s3-and-backblaze/) |

## Target audience
**Beachhead (ICP A): the RAW-shooting enthusiast.**
- Shoots RAW (often RAW+JPEG) on Fujifilm X, Ricoh GR, Sony, Nikon, Canon; 50 to 500 GB, 5,000 to 15,000 photos.
- Does not want or already resents an Adobe subscription; cameras that Google Photos handles badly.
- Pain: photos scattered across cards, drives and folders named `100RICOH`; cannot browse by date; sharing a RAW is awkward; fear of losing the only copy.
- Willingness to pay: $5 to $12 per month (anchored on Lightroom, SmugMug Source). 🟡
- Where they are: r/fujifilm, r/ricohGR, Fuji X Weekly, camera YouTubers, DPReview forums.

**Second segment (ICP B): the working photographer** (weddings, events, portraits). Needs expiring share links, client galleries, download packs. Pays $25 to $50 for Pixieset-type tools; will pay for Studio. Adds delivery cost (egress), so meter it.

**Not now:** families and general consumers (Google and Amazon win on price and convenience); self-hosters (Immich is free); enterprises.

## Jobs to be done
1. "See my whole RAW library by date, fast, on my phone and laptop."
2. "Know my originals are safe and I can always take them back out."
3. "Send someone a link to a set of photos that expires."
4. "Find the shot by camera, lens, ISO or year."

## Positioning (April Dunford method)
- **Competitive alternatives:** Google Photos / Amazon Photos (status quo for casual), Lightroom cloud (pros), drive + rclone (technical), SmugMug Source (archive).
- **Unique attributes:** reads the embedded preview of every RAW format including RAF and DNG; capture-date timeline across all folders; camera/lens/ISO metadata captured at ingest; originals never modified and always exportable; RAW+JPEG kept as one photo; priced below Lightroom.
- **Category:** a *niche* category, "the RAW photo library". Avoid head-to-head with Google Photos.
- **Statement:** FOR RAW-shooting photographers WHO are tired of their photos being scattered or misread by general-purpose services, RawVault IS A cloud RAW library THAT shows every photo by capture date on any device while keeping the originals untouched and exportable, UNLIKE Google Photos and Amazon Photos which mishandle RAW, and Lightroom which locks you into a subscription and catalog.
- **Headline:** "Your RAW library, on every screen." Subhead: "Every format, by date, originals untouched."
- **Proof to collect (🔴 not yet):** 10 beta users who describe the value unprompted; before/after screenshots of RAF files in Google Photos vs RawVault.

## The price anchor
| Reference price | Role |
|---|---|
| Amazon Photos "free with Prime" | Floor we cannot beat; concede it, do not compare |
| Google 100 GB $1.99 / 2 TB $9.99 | Casual anchor; ignore for positioning |
| **SmugMug Source $10 per TB (RAW)** | **Closest direct price for RAW storage** |
| **Lightroom 1 TB $11.99** | **Photographers' mental reference price** |
| Adobe Photography plan $19.99 | Makes $9.99 feel half the price |
| Pixieset 1 TB with RAW $24, 2 TB $48 | Ceiling for Studio |

**Decision:** anchor Pro (1 TB) at **$9.99 monthly, just under SmugMug Source and Lightroom**, and show the annual price ($89, about $7.40 a month) beside "Lightroom $11.99". Use "cost per 1,000 RAW photos" framing: Pro is about $0.35 per 1,000 photos per month.

## Risks
- Amazon Photos improves RAW handling or Google adds Fujifilm support (existing weakness disappears). 🟡
- Adobe bundles cheaper storage. 🟡
- Trust: a new vendor holding irreplaceable originals. Mitigate with export at any time, durability statement, no lock-in.
- Thin margins if storage stays on AWS (see document 02).
