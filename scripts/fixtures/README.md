# Chart-validation fixtures

Hand-written synthetic lane reports (`host: "fixture"`, `git_rev:
"fixture"`) whose only job is to exercise every code path in
`scripts/bench_viz.py`'s lane charts — schema-exact twins of the
`to_json` outputs pinned by the shape tests in
`crates/bumbledb-bench/src/lanes/{storage,writes,curves}.rs`. They are
NEVER measurement output; no number in them was ever timed, and none may
be quoted as a claim.

- `fixture-storage-report.json` — 2 scales × 2 worlds + 2 churn rows
  (`bench-storage.svg`).
- `fixture-writes-report.json` — 2 durability lanes × the commit ladder
  + deletes + bulk (`bench-writes-rates.svg`).
- `fixture-curves-report.json` — 4 families × 3 scale points, with one
  capped point, one hand-tuned twin, and one warmth object
  (`bench-curves.svg`, `bench-warmth.svg`).

Dry-run against them into a temp dir:

```sh
python3 scripts/bench_viz.py \
    --storage-report scripts/fixtures/fixture-storage-report.json \
    --writes-report scripts/fixtures/fixture-writes-report.json \
    --curves-report scripts/fixtures/fixture-curves-report.json \
    --out-dir "$(mktemp -d)"
```
