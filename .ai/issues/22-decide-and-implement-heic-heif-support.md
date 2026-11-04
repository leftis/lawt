## Why
HEIC files are listed with a placeholder tile. Decoding needs libheif or a pure-Rust option, and cross-compiling native code is the risk. The owner has no HEIC files today.

## What to do
- Evaluate `libheif-rs` with static linking and a pure-Rust decoder.
- Build for arm64 Lambda with cargo-lambda and measure the binary size.
- If neither works cleanly, document HEIC as unsupported in the README.

## Done when
- Decision recorded in `.ai/DECISIONS.md`, and either HEIC renders or the limit is documented.

## Triage
Blast radius 0 · Risk 0 · Complexity 2 · Size 1 = 3 → **Spec** track

_Source: ADR 0001, format policy_
