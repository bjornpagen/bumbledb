# crud — the OLTP home turf (report-class; SQLite's strong regime, benched to lose honestly)

Seed 1. One shared op stream per family, folded by both engines; the read query oracle-gated (value-identical multisets) on every lane before any timed window. ratio = ours p50 / sqlite p50 (lower is better; <1 = bumbledb faster).

## lane durable

Db::create (LMDB issues F_FULLFSYNC unconditionally on macOS) vs SQLite WAL synchronous=FULL fullfsync=ON checkpoint_fullfsync=ON, cache_size=-262144, temp_store=MEMORY, mmap_size=1GiB, wal_autocheckpoint=0 — both engines flush to media on every commit

| family | about | ours p50 (µs) | sqlite p50 (µs) | ratio | ours p99 (µs) | sqlite p99 (µs) |
|---|---|---:|---:|---:|---:|---:|
| crud_read_point | keyed point read: (id, val) by key, 3 hits + 1 miss rotation | 0.67 | 1.54 | 0.43 | 0.75 | 2.00 |
| crud_insert | one fresh Doc row per commit (fsync-bound single-writer floor) | 5145.33 | 4632.75 | 1.11 | 8634.17 | 6624.62 |
| crud_insert_10 | 10 fresh Doc rows per commit | 5177.88 | 4687.92 | 1.10 | 6638.54 | 6294.00 |
| crud_insert_100 | 100 fresh Doc rows per commit | 7845.08 | 4623.92 | 1.70 | 10248.38 | 5268.25 |
| crud_insert_1k | 1000 fresh Doc rows per commit | 23404.83 | 5164.33 | 4.53 | 24728.38 | 6306.79 |
| crud_update | one keyed Counter value replacement per commit | 5057.54 | 4593.17 | 1.10 | 5365.88 | 7196.50 |
| crud_update_hot | the same replacement pinned to one hot row (key 0 every sample) | 5124.46 | 4147.17 | 1.24 | 7870.79 | 5178.62 |
| crud_upsert | keyed upsert over twice the Counter mass (~half miss) | 4665.25 | 4250.50 | 1.10 | 5588.04 | 5325.62 |
| crud_rmw | read-modify-write round trip: point read, host +1, write back | 5578.58 | 4588.25 | 1.22 | 6507.42 | 5567.33 |
| crud_delete | one pool-row delete per commit (delete-bearing by contract) | 5063.42 | 4547.25 | 1.11 | 8083.54 | 7125.08 |
| crud_mixed_90_10 | 9 point reads + 1 single-row insert commit per sample | 5242.62 | 4592.33 | 1.14 | 7177.12 | 5648.29 |

## lane nosync

Db::ephemeral (MDB_NOSYNC: pages and meta pwritten, no sync boundary ever crossed) vs SQLite WAL synchronous=OFF fullfsync=OFF checkpoint_fullfsync=OFF, cache_size=-262144, temp_store=MEMORY, mmap_size=1GiB, wal_autocheckpoint=0 — WAL frames written, never synced (OFF, not NORMAL: NORMAL still syncs at checkpoints, which would cross-match a store kind that never syncs)

| family | about | ours p50 (µs) | sqlite p50 (µs) | ratio | ours p99 (µs) | sqlite p99 (µs) |
|---|---|---:|---:|---:|---:|---:|
| crud_read_point | keyed point read: (id, val) by key, 3 hits + 1 miss rotation | 0.50 | 1.21 | 0.41 | 0.83 | 1.75 |
| crud_insert | one fresh Doc row per commit (fsync-bound single-writer floor) | 37.46 | 15.75 | 2.38 | 53.71 | 28.88 |
| crud_insert_10 | 10 fresh Doc rows per commit | 101.67 | 21.33 | 4.77 | 218.21 | 66.46 |
| crud_insert_100 | 100 fresh Doc rows per commit | 703.33 | 115.62 | 6.08 | 1036.88 | 262.12 |
| crud_insert_1k | 1000 fresh Doc rows per commit | 6529.33 | 760.46 | 8.59 | 7031.42 | 858.21 |
| crud_update | one keyed Counter value replacement per commit | 47.29 | 10.00 | 4.73 | 104.75 | 73.38 |
| crud_update_hot | the same replacement pinned to one hot row (key 0 every sample) | 35.75 | 9.21 | 3.88 | 50.96 | 18.62 |
| crud_upsert | keyed upsert over twice the Counter mass (~half miss) | 31.79 | 13.88 | 2.29 | 52.38 | 108.00 |
| crud_rmw | read-modify-write round trip: point read, host +1, write back | 44.08 | 10.08 | 4.37 | 62.08 | 14.83 |
| crud_delete | one pool-row delete per commit (delete-bearing by contract) | 46.96 | 15.79 | 2.97 | 74.71 | 45.54 |
| crud_mixed_90_10 | 9 point reads + 1 single-row insert commit per sample | 48.83 | 27.62 | 1.77 | 66.79 | 70.67 |

post-state: Doc + Counter value-identical across engines, both lanes. Every row above is report-class, never gated — no budget gate reads a crud number.
