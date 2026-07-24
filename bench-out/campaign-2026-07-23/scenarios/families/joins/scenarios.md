# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## joins (geomean ratio 0.10 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| j1_filmography | sqlite | 32 | 0.2 | 7.0 | 0.04 | 2-atom containment walk under 25%-hot fan-in skew |
| j2_costars | sqlite | 317 | 0.9 | 12.1 | 0.07 | self-join through the fact table, hot vs cold |
| j3_keyword_kind | sqlite | 53 | 3.6 | 16.5 | 0.22 | 3-way pinched by string point + year range |
| j4_five_way | sqlite | 10171 | 1481.6 | 4520.6 | 0.33 | JOB-shaped 5-way, dims filter both sides |
| j5_country_rollup | sqlite | 8 | 5074.0 | 29343.9 | 0.17 | full-join rollup: Min(year)+Count by country |
| j6_keyword_neighborhood | sqlite | 6807 | 30.5 | 1314.2 | 0.02 | fan-out explosion through shared keywords |

Overall geomean ratio across 6 queries: **0.10**.
