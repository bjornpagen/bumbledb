# crud — the OLTP home turf (report-class; SQLite's strong regime, benched to lose honestly)

Seed 1. One shared op stream per family, folded by both engines; the read query oracle-gated (value-identical multisets) on every lane before any timed window. ratio = ours p50 / sqlite p50 (lower is better; <1 = bumbledb faster).

## lane durable

Db::create (LMDB issues F_FULLFSYNC unconditionally on macOS) vs SQLite WAL synchronous=FULL fullfsync=ON checkpoint_fullfsync=ON, cache_size=-262144, temp_store=MEMORY, whole-file mmap (coverage asserted), wal_autocheckpoint=0 — both engines flush to media on every commit

| family | about | ours p50 (µs) | sqlite p50 (µs) | ratio | ours p99 (µs) | sqlite p99 (µs) |
|---|---|---:|---:|---:|---:|---:|
| crud_read_point | keyed point read: (id, val) by key, 3 hits + 1 miss rotation | 0.58 | 1.29 | 0.45 | 0.75 | 1.71 |
| crud_insert | one fresh Doc row per commit (fsync-bound single-writer floor) | 4648.46 | 4619.67 | 1.01 | 5363.71 | 5915.62 |
| crud_insert_10 | 10 fresh Doc rows per commit | 4301.62 | 4222.92 | 1.02 | 5567.21 | 5473.46 |
| crud_insert_100 | 100 fresh Doc rows per commit | 7609.50 | 4471.04 | 1.70 | 10466.25 | 8367.25 |
| crud_insert_1k | 1000 fresh Doc rows per commit | 24214.92 | 5150.88 | 4.70 | 27547.58 | 5944.38 |
| crud_update | one keyed Counter value replacement per commit | 4735.38 | 4393.88 | 1.08 | 6047.21 | 7103.46 |
| crud_update_hot | the same replacement pinned to one hot row (key 0 every sample) | 4614.75 | 4155.62 | 1.11 | 6334.92 | 4725.00 |
| crud_upsert | keyed upsert over twice the Counter mass (~half miss) | 4239.08 | 4364.83 | 0.97 | 5664.96 | 5340.71 |
| crud_rmw | read-modify-write round trip: point read, host +1, write back | 4678.96 | 4650.42 | 1.01 | 5925.08 | 5740.33 |
| crud_delete | one pool-row delete per commit (delete-bearing by contract) | 5030.17 | 4261.46 | 1.18 | 6344.62 | 5712.88 |
| crud_mixed_90_10 | 9 point reads + 1 single-row insert commit per sample | 4549.92 | 4426.25 | 1.03 | 5980.75 | 26245.71 |

## lane nosync

Db::ephemeral (MDB_NOSYNC: pages and meta pwritten, no sync boundary ever crossed) vs SQLite WAL synchronous=OFF fullfsync=OFF checkpoint_fullfsync=OFF, cache_size=-262144, temp_store=MEMORY, whole-file mmap (coverage asserted), wal_autocheckpoint=0 — WAL frames written, never synced (OFF, not NORMAL: NORMAL still syncs at checkpoints, which would cross-match a store kind that never syncs)

| family | about | ours p50 (µs) | sqlite p50 (µs) | ratio | ours p99 (µs) | sqlite p99 (µs) |
|---|---|---:|---:|---:|---:|---:|
| crud_read_point | keyed point read: (id, val) by key, 3 hits + 1 miss rotation | 0.50 | 1.21 | 0.41 | 0.67 | 1.42 |
| crud_insert | one fresh Doc row per commit (fsync-bound single-writer floor) | 29.62 | 16.29 | 1.82 | 67.21 | 29.29 |
| crud_insert_10 | 10 fresh Doc rows per commit | 93.29 | 28.71 | 3.25 | 207.50 | 101.25 |
| crud_insert_100 | 100 fresh Doc rows per commit | 625.17 | 119.17 | 5.25 | 850.00 | 251.71 |
| crud_insert_1k | 1000 fresh Doc rows per commit | 5845.17 | 797.00 | 7.33 | 6446.21 | 882.38 |
| crud_update | one keyed Counter value replacement per commit | 36.38 | 11.12 | 3.27 | 57.92 | 117.04 |
| crud_update_hot | the same replacement pinned to one hot row (key 0 every sample) | 36.12 | 10.33 | 3.50 | 82.25 | 44.79 |
| crud_upsert | keyed upsert over twice the Counter mass (~half miss) | 37.75 | 16.75 | 2.25 | 63.33 | 61.33 |
| crud_rmw | read-modify-write round trip: point read, host +1, write back | 40.17 | 12.38 | 3.25 | 86.46 | 26.75 |
| crud_delete | one pool-row delete per commit (delete-bearing by contract) | 32.29 | 18.00 | 1.79 | 40.92 | 41.38 |
| crud_mixed_90_10 | 9 point reads + 1 single-row insert commit per sample | 43.42 | 34.12 | 1.27 | 116.46 | 93.71 |

post-state: Doc + Counter value-identical across engines, both lanes. Every row above is report-class, never gated — no budget gate reads a crud number.
