# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## olap (geomean ratio 0.01 over 6 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| o1_revenue_by_region | sqlite | 6 | 458.2 | 229139.1 | 0.00 | full-fact Sum through one dimension, 6 groups |
| o2_category_window | sqlite | 12 | 457.8 | 22007.8 | 0.02 | Sum+Count by category inside day windows |
| o3_promo_split | sqlite | 2 | 336.7 | 95263.1 | 0.00 | bool group key, full-scan fold |
| o4_segment_category | sqlite | 64 | 27849.0 | 371225.1 | 0.08 | two-dimension rollup, 64 groups, 3-way join |
| o5_store_extremes | sqlite | 200 | 704.8 | 149807.1 | 0.00 | Min+Max per store, 200 groups |
| o6_brand_drill | sqlite | 0 | 1.8 | 562.9 | 0.00 | selective brand point + day range, one Sum |

Overall geomean ratio across 6 queries: **0.01**.
