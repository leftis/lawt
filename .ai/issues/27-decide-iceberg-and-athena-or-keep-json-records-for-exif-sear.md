## Why
`docs/adr/0001-iceberg-catalog.md` proposed Iceberg. The first full run showed that per-photo JSON records and a rebuilt timeline give the same experience at 8,000 photos without Athena. Iceberg would add open-format analytics and ad-hoc SQL.

## What to do
- Compare: a Parquet export of the records queried by Athena or DuckDB, against an in-app index.
- Write the result in the ADR with cost and latency numbers.

## Done when
- ADR updated with a decision and the issue closed.

_Source: ADR 0001_
