# sweep-commit (T8) — baseline 2026-07-25 attribution segment

run: 2026-08-03 12:41 CDT @ fab4e28e, obs build (target/bench-obs), BUMBLEDB_BENCH_BOOST=1, scripts/measure.sh held; power: wall before AND after (Now drawing from 'AC Power';Now drawing from 'AC Power';)
protocol: defaults — sizes 4..4096, 8 samples/cell, seed 1; ephemeral windowed twins (self-gated, no stamp protocol; the lane the campaign and night never ran — first pin)

```
sweep: size 4096, sorted order
T8 commit-size sweep — judgment spans by touched-parent count (ns)
world: windowed twin, ephemeral; ambient 16384 parents x 8 children/parent; seed 1; 8 samples/cell
arms: delta = today's hash-order source probes; sorted = key-sorted probe order (hash-graded child ids); win = the already-sorted capacity walk, both arms

  size | src p50 delta src p50 sorted sorted/delta | src min delta src min sorted | win p50 delta win p50 sorted
     4 |          3916           3916       1.000x |          3125           2791 |          4166           4416
    16 |         12791          11416       0.893x |          9291           8958 |         18583          18208
    64 |         32125          33583       1.045x |         29333          27833 |         70333          72333
   256 |        119291         103666       0.869x |        114500         100333 |        277208         280208
  1024 |        349083         296000       0.848x |        325083         279250 |       1065916        1109666
  4096 |       1134333         960000       0.846x |       1127541         940791 |       3974291        3987458
```
