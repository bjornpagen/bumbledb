# crud — the OLTP home turf (report-class; SQLite's strong regime, benched to lose honestly)

Seed 1. One shared op stream per family, folded by both engines; the read query oracle-gated (value-identical multisets) on every lane before any timed window. ratio = ours p50 / sqlite p50 (lower is better; <1 = bumbledb faster).

## lane durable

Db::create (LMDB issues F_FULLFSYNC unconditionally on macOS) vs SQLite WAL synchronous=FULL fullfsync=ON checkpoint_fullfsync=ON, cache_size=-262144, temp_store=MEMORY, whole-file mmap (coverage asserted), wal_autocheckpoint=0 — both engines flush to media on every commit

| family | about | ours p50 (µs) | sqlite p50 (µs) | ratio | ours p99 (µs) | sqlite p99 (µs) |
|---|---|---:|---:|---:|---:|---:|
| crud_read_point | keyed point read: (id, val) by key, 3 hits + 1 miss rotation | 0.62 | 1.50 | 0.42 | 0.92 | 1.79 |
| crud_insert | one fresh Doc row per commit (fsync-bound single-writer floor) | 5041.46 | 4837.08 | 1.04 | 5358.62 | 6260.25 |
| crud_insert_10 | 10 fresh Doc rows per commit | 5267.88 | 4535.71 | 1.16 | 10411.83 | 5945.75 |
| crud_insert_100 | 100 fresh Doc rows per commit | 8419.92 | 4214.50 | 2.00 | 9633.04 | 5020.75 |
| crud_insert_1k | 1000 fresh Doc rows per commit | 20186.67 | 5158.88 | 3.91 | 21560.67 | 8595.54 |
| crud_update | one keyed Counter value replacement per commit | 4299.17 | 4243.17 | 1.01 | 5273.38 | 10651.00 |
| crud_update_hot | the same replacement pinned to one hot row (key 0 every sample) | 4250.00 | 4219.83 | 1.01 | 5261.00 | 5302.96 |
| crud_upsert | keyed upsert over twice the Counter mass (~half miss) | 5067.04 | 4325.50 | 1.17 | 6052.62 | 5346.25 |
| crud_rmw | read-modify-write round trip: point read, host +1, write back | 5120.46 | 4314.54 | 1.19 | 5666.79 | 7273.42 |
| crud_delete | one pool-row delete per commit (delete-bearing by contract) | 5064.71 | 4211.17 | 1.20 | 5638.38 | 5510.21 |
| crud_mixed_90_10 | 9 point reads + 1 single-row insert commit per sample | 5094.50 | 4202.46 | 1.21 | 5898.79 | 5475.88 |

## lane nosync

Db::ephemeral (MDB_NOSYNC: pages and meta pwritten, no sync boundary ever crossed) vs SQLite WAL synchronous=OFF fullfsync=OFF checkpoint_fullfsync=OFF, cache_size=-262144, temp_store=MEMORY, whole-file mmap (coverage asserted), wal_autocheckpoint=0 — WAL frames written, never synced (OFF, not NORMAL: NORMAL still syncs at checkpoints, which would cross-match a store kind that never syncs)

| family | about | ours p50 (µs) | sqlite p50 (µs) | ratio | ours p99 (µs) | sqlite p99 (µs) |
|---|---|---:|---:|---:|---:|---:|
| crud_read_point | keyed point read: (id, val) by key, 3 hits + 1 miss rotation | 0.50 | 1.21 | 0.41 | 0.54 | 1.25 |
| crud_insert | one fresh Doc row per commit (fsync-bound single-writer floor) | 28.38 | 15.12 | 1.88 | 41.75 | 26.67 |
| crud_insert_10 | 10 fresh Doc rows per commit | 80.75 | 20.62 | 3.92 | 115.96 | 58.75 |
| crud_insert_100 | 100 fresh Doc rows per commit | 504.83 | 104.96 | 4.81 | 523.88 | 185.58 |
| crud_insert_1k | 1000 fresh Doc rows per commit | 4410.71 | 702.71 | 6.28 | 4675.17 | 771.75 |
| crud_update | one keyed Counter value replacement per commit | 30.25 | 9.50 | 3.18 | 42.75 | 71.25 |
| crud_update_hot | the same replacement pinned to one hot row (key 0 every sample) | 27.67 | 8.88 | 3.12 | 38.46 | 29.00 |
| crud_upsert | keyed upsert over twice the Counter mass (~half miss) | 26.21 | 14.25 | 1.84 | 37.67 | 50.67 |
| crud_rmw | read-modify-write round trip: point read, host +1, write back | 27.75 | 9.88 | 2.81 | 34.92 | 13.88 |
| crud_delete | one pool-row delete per commit (delete-bearing by contract) | 25.50 | 15.04 | 1.70 | 28.88 | 34.04 |
| crud_mixed_90_10 | 9 point reads + 1 single-row insert commit per sample | 33.50 | 26.92 | 1.24 | 59.75 | 57.75 |

post-state: Doc + Counter value-identical across engines, both lanes. Every row above is report-class, never gated — no budget gate reads a crud number.
