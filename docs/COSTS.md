# RawVault cost model

Measured on 2026-09-25 from the real library, plus AWS list prices. **Prices are eu-west-1 list prices as I know them; check the AWS pricing pages before quoting anyone.** Lambda compute is an estimate (CloudWatch metrics were not readable with the deploy user).

## Measured
| | |
|---|---|
| Photos | 7,826 (RAF 6,181, DNG 1,110, JPG 506, TIF 28, PNG 1) |
| Originals | 277.8 GB, average 35.5 MB per photo (RAF 38 MB, DNG 30 MB, JPG 13 MB) |
| Derived data written | 15.1 GB in 30,845 objects, **5.4 % of the originals** (about 1.9 MB and 3.9 objects per photo) |
| Timeline files | day index 9.8 KB; largest month 310 KB (compressed on the wire) |
| Run | ~22 min wall clock, 6 parallel workers, all photos, 1 failed (truncated RAF) |

## Your library today
- One-off ingestion: about **$0.50**.
- Monthly: derived storage 15.1 GB x $0.023 = **$0.35**, salt secret **$0.40**, everything else (CloudFront free tier, API Gateway, Lambda) pennies. About **$0.80 / month** on top of what you already pay to store the originals.

## Per GB of originals (what a product would pay)
| Item | Cost per GB of originals | Notes |
|---|---|---|
| Ingestion, one-off | **~$0.002** | Lambda ~2.5 GB-s per photo (~$0.001 per GB) + ~111 S3 PUTs per GB (~$0.0006) + GETs |
| Derived storage / month | **$0.00125** | 0.054 GB derived per GB x $0.023 |
| Originals storage / month | **$0.023** (S3 Standard) | Intelligent-Tiering can drop old data to ~$0.0125 then ~$0.004; Glacier Instant Retrieval ~$0.005 (90-day minimum, retrieval fee ~$0.03/GB) |
| Delivery (egress) | **$0.085 per GB actually viewed/downloaded** | CloudFront, first 1 TB/month free account-wide. Browsing is cheap (a 1,000-photo month is ~15 MB of thumbnails); downloading originals is what costs |
| **Total, storage-heavy, typical** | **~$0.025 to $0.03 / GB / month** on Standard | ~$0.01 with cold-tier originals |

Examples (Standard, light usage): 100 GB library about $2.5/month; 1 TB about $25/month.

## Fixed / per-user costs
- Cognito: free for the first 10,000 monthly active users (verify current tiers).
- Salt secret: $0.40/month (one per deployment, not per user).
- API Gateway $3.50 per million requests; one session is ~50 requests, i.e. $0.0002.
- CloudWatch Logs, alarms: pennies at this scale.

## What would have to change before this is a product (single-tenant today)
1. **Tenant isolation:** per-user prefixes, per-user ids/salts, authorization on every route by user (today one pool, one bucket, one salt).
2. **Upload path:** browser multipart upload to S3 with presigned URLs, resumable; an S3 event trigger on *your own* upload bucket to start processing (the "no bucket configuration" rule only applies to the owner's personal bucket).
3. **Privacy:** renditions are "unlisted links" today. For other people's photos use CloudFront signed cookies.
4. **Backup and durability promise:** versioning or replication (roughly doubles storage cost) or an explicit "we do not back up" stance.
5. **Billing, quotas, abuse and malware handling, GDPR (export and delete).**
6. **Pricing reality:** at ~$0.03/GB/month raw cost on S3 Standard, consumer storage prices (Google One ~$0.005/GB, Backblaze B2 ~$0.006/GB) are far below your cost. Either sell the RAW experience rather than storage, use colder tiers, or move storage to a provider without egress fees (Cloudflare R2 ~$0.015/GB and no egress charge, Backblaze B2). Processing is not the cost driver; **storage and egress are**.
