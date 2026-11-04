## Why
The scrubber has slider semantics but no keyboard control. The viewer does not trap focus. Layout was checked at 375 px and desktop only.

## What to do
- Arrow, Page and Home/End keys on the scrubber.
- Focus trap and focus return in the viewer.
- Respect `prefers-reduced-motion`.
- Check 375, 768 and 1024 px widths and contrast of all text.

## Done when
- An automated axe check passes and the three widths are checked by hand.

## Triage
Blast radius 0 · Risk 0 · Complexity 1 · Size 1 = 2 → **Direct** track

_Source: TODO.md_
