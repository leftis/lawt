## Why
Nothing tells the owner when the API fails or the pipeline errors, or when costs jump.

## What to do
- Use `tracing` JSON logs in the Rust Lambdas with key, outcome and duration, and no tokens.
- CloudWatch alarms: API 5xx rate, `lawt-api` errors, `lawt-pipeline` errors.
- An AWS Budget with an email alert.

## Done when
- A forced error in each Lambda raises an alarm in a test account.

## Triage
Blast radius 1 · Risk 0 · Complexity 1 · Size 1 = 3 → **Planned** track

_Source: TODO.md, P2 foundations_
