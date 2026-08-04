# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 1 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## joins (geomean ratio 0.15 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| j1_filmography | sqlite | 128 | 39.8 | 81.5 | 0.49 | 2-atom containment walk under 25%-hot fan-in skew |
| j2_costars | sqlite | 1207 | 21.5 | 316.2 | 0.07 | self-join through the fact table, hot vs cold |
| j3_keyword_kind | sqlite | 197 | 65.0 | 183.7 | 0.35 | 3-way pinched by string point + year range |
| j4_five_way | sqlite | 2244 | 1114.0 | 3826.7 | 0.29 | JOB-shaped 5-way, dims filter both sides |
| j5_country_rollup | sqlite | 8 | 4862.9 | 27868.5 | 0.17 | full-join rollup: Min(year)+Count by country |
| j6_keyword_neighborhood | sqlite | 21089 | 658.2 | 32826.8 | 0.02 | fan-out explosion through shared keywords |

Overall geomean ratio across 6 queries: **0.15**.

## Allocations (per query, --alloc)

| query | allocs | alloc bytes | deallocs | dealloc bytes |
|---|---:|---:|---:|---:|
| j1_filmography | 2 | 32 | 1 | 24 |
| j2_costars | 2 | 32 | 1 | 24 |
| j3_keyword_kind | 2 | 56 | 1 | 48 |
| j4_five_way | 2 | 104 | 1 | 96 |
| j5_country_rollup | 2 | 16 | 1 | 8 |
| j6_keyword_neighborhood | 2 | 32 | 1 | 24 |
