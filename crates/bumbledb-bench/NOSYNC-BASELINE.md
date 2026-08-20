# NosyncLane commit-ladder pin

Issue 33. Substrate: `StoreMode::Nosync` (`Db::create_nosync` /
`Db::open_nosync`) over a durable-shaped store — not the deleted
ephemeral kind. Campaign 1.24–1.44 is not comparable.

Pin (2026-08-20, Apple M2 Max, obs release, scale S, seed 1, 8 samples,
`writes --lanes nosync --batches 1,10,100,1000`):

| family | ours p50 ns | sqlite p50 ns | ours/sqlite |
| --- | ---: | ---: | ---: |
| `commit_b1` | 47542 | 37125 | 1.28× |
| `commit_b10` | 218542 | 317042 | 0.69× |
| `commit_b100` | 1433875 | 1940750 | 0.74× |
| `commit_b1000` | 9345041 | 8105709 | 1.15× |
| `delete_b1` | 45708 | 28583 | 1.60× |
| `delete_b10` | 175000 | 343667 | 0.51× |
| `delete_b100` | 1425958 | 2494417 | 0.57× |
| `delete_b1000` | 9338959 | 9764833 | 0.96× |
| `insert_stream` | 688915791 | 480292042 | 1.43× |
