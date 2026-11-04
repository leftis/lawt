## Why
`DownloadCloudFront` and `DownloadOAI` front the photo bucket. CDK cannot update the policy of an imported bucket (synth warns about it), so the distribution cannot read anything, and the timeline uses the rendition CDN.

## What to do
- Confirm no code uses `DownloadCDNURL` or `VITE_CDN_HOST`.
- Remove the distribution, the OAI, the output and `toCloudFrontUrl`.

## Done when
- `cdk diff` shows only those removals.

## Triage
Blast radius 1 · Risk 1 · Complexity 0 · Size 0 = 2 → **Direct** track

_Source: TODO.md, 'Found during T9'_
