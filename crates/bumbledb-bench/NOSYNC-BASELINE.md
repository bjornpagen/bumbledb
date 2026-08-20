# NosyncLane commit-ladder pin

Substrate: `StoreMode::Nosync` (`Db::create_nosync` / `Db::open_nosync`)
over a durable-shaped store.

Pin: 2026-08-20 shared-machine night, Apple M2 Max, revision `4dd1ee96`,
`writes --out bench-out/night-2026-08-20/writes`, nosync lane.

| family | ours p50 ns | sqlite p50 ns | ours/sqlite |
| --- | ---: | ---: | ---: |
| `commit_b1` | 49959 | 31834 | 1.57× |
| `commit_b10` | 189291 | 246292 | 0.77× |
| `commit_b100` | 1123542 | 1820375 | 0.62× |
| `commit_b1000` | 8266042 | 9891041 | 0.84× |
| `delete_b1` | 41458 | 39166 | 1.06× |
| `delete_b10` | 173459 | 217709 | 0.80× |
| `delete_b100` | 1300250 | 2506000 | 0.52× |
| `delete_b1000` | 9500500 | 12739000 | 0.75× |
| `insert_stream` | 625392750 | 441358625 | 1.42× |
