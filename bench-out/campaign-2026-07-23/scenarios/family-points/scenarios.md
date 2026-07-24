# Scenario benchmarks

Report-class measurements over non-ledger worlds; every query oracle-gated (value-identical results on both engines, every `SQLite` lane, never under a cap) before timing. Adversarial lanes run under a per-sample wall-clock cap (`SQLite`'s progress handler): a lane that trips it reports `DNF>cap` with NO percentiles — excluded from geomeans and counted. Protocol: 8 warmups, 64 samples, medians; `SQLite` file-backed WAL `synchronous=FULL`, fully indexed, prepared statements reused, ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb faster).


## points (geomean ratio 0.11 over 5 timed)

| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |
|---|---|---:|---:|---:|---:|---|
| p1_by_id | sqlite | 0 | 0.3 | 1.1 | 0.27 | fresh-id point: key probe vs B-tree descent |
| p2_by_key | sqlite | 0 | 0.9 | 1.3 | 0.69 | keyed string point: dictionary + determinant index |
| p3_bucket_fetch | sqlite | 1744 | 11.0 | 211.5 | 0.05 | small fan-out through a dimension + id ceiling |
| p4_size_band | sqlite | 0 | 0.2 | 111.9 | 0.00 | secondary range folded to Count |
| p5_keyed_get | sqlite | 0 | 1.1 | 1.4 | 0.76 | keyed get (0.5.0): the point read through Doc(key) -> Doc — determinant probe, no query machinery |

Overall geomean ratio across 5 queries: **0.11**.
