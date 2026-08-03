# lawful — the integrity home turf (report-class)

seed 1. This world has no queries — the write families' oracle is the post-state fold over all five ordinary relations plus the naive verdict-parity test. Every row below is REPORT-class, never gated.

## the enforcement map

| law | statement notation | sqlite enforcement |
|---|---|---|
| fresh auto-key | `Task(id) -> Task` | `PRIMARY KEY ("id")` |
| fresh auto-key | `Attempt(id) -> Attempt` | `PRIMARY KEY ("id")` |
| fresh auto-key | `Steer(id) -> Steer` | `PRIMARY KEY ("id")` |
| closed auto-key | `TaskKinds(id) -> TaskKinds` | `-- unmirrored: the closed roster is static schema data; its identity lives in the referencing kind roster constraint on "Task"` |
| closed auto-key | `SteerKinds(id) -> SteerKinds` | `-- unmirrored: the closed roster is static schema data; its identity lives in the referencing kind roster constraint on "Steer"` |
| closed auto-key | `Outcome(id) -> Outcome` | `-- unmirrored: the closed roster is static schema data; its identity lives in the referencing outcome roster constraint on "Verdict"` |
| declared key | `Task(kind, subject) -> Task` | `UNIQUE ("kind", "subject")` |
| declared key | `Attempt(task, n) -> Attempt` | `UNIQUE ("task", "n")` |
| declared key | `Verdict(attempt) -> Verdict` | `UNIQUE ("attempt")` |
| declared key | `SteerScope(steer, grp) -> SteerScope` | `UNIQUE ("steer", "grp")` |
| closed-vocabulary containment | `Task(kind) <= TaskKinds(id)` | `CHECK ("kind" IN (0, 1, 2))` |
| foreign key | `Attempt(task) <= Task(id)` | `FOREIGN KEY ("task") REFERENCES "Task" ("id")` |
| foreign key | `Verdict(attempt) <= Attempt(id)` | `FOREIGN KEY ("attempt") REFERENCES "Attempt" ("id")` |
| closed-vocabulary containment | `Verdict(outcome) <= Outcome(id)` | `CHECK ("outcome" IN (0, 1, 2))` |
| closed-vocabulary containment | `Steer(kind) <= SteerKinds(id)` | `CHECK ("kind" IN (0, 1))` |
| foreign key | `Steer(task) <= Task(id)` | `FOREIGN KEY ("task") REFERENCES "Task" ("id")` |
| ψ-selected containment | `SteerScope(steer) <= Steer(id | kind == Repartition)` | `CREATE TRIGGER "lawful_steer_scope_psi" BEFORE INSERT ON "SteerScope" WHEN NOT EXISTS (SELECT 1 FROM "Steer" WHERE "id" = NEW."steer" AND "kind" = 1) BEGIN SELECT RAISE(ABORT, 'steer scope requires a Repartition steer'); END` |
| attempt-count capacity law | `Task(id) <={0..8} Attempt(task)` | `CREATE TRIGGER "lawful_attempt_window" BEFORE INSERT ON "Attempt" WHEN (SELECT COUNT(*) FROM "Attempt" WHERE "task" = NEW."task") >= 8 BEGIN SELECT RAISE(ABORT, 'attempt window exceeded'); END` |

## lane `durable`

Db::create (LMDB issues F_FULLFSYNC unconditionally on macOS) vs SQLite WAL synchronous=FULL fullfsync=ON checkpoint_fullfsync=ON, cache_size=-262144, temp_store=MEMORY, whole-file mmap (coverage asserted), wal_autocheckpoint=0 — both engines flush to media on every commit

| family | ours p50 µs | sqlite p50 µs | ratio p50 (ours/sqlite) | work | about |
|---|---:|---:|---:|---:|---|
| law_commit_attempt | 5170.875 | 4657.000 | 1.1103 | 1 | one judged Attempt insert per commit under the full law roster (key + containment + capacity) |
| law_commit_cluster | 5210.584 | 4738.000 | 1.0997 | 4 | one judged 4-row cluster per commit: attempt + verdict + steer + scope — every statement family exercised in one commit |
| law_reject_key | 5087.584 | 21.875 | 232.5753 | 1 | one REFUSED duplicate-(task, n) commit per sample (Functionality cited) |
| law_reject_containment | 58.292 | 39.500 | 1.4757 | 1 | one REFUSED absent-task commit per sample (Containment cited) |
| law_reject_window | 60.834 | 29.917 | 2.0334 | 1 | one REFUSED 9th-attempt commit on the saturated task 0 per sample (Capacity cited) |
| law_reject_scope | 32.417 | 12.291 | 2.6375 | 1 | one REFUSED Observe-steer scope commit per sample (the ψ containment cited) |

## lane `nosync`

Db::ephemeral (MDB_NOSYNC: pages and meta pwritten, no sync boundary ever crossed) vs SQLite WAL synchronous=OFF fullfsync=OFF checkpoint_fullfsync=OFF, cache_size=-262144, temp_store=MEMORY, whole-file mmap (coverage asserted), wal_autocheckpoint=0 — WAL frames written, never synced (OFF, not NORMAL: NORMAL still syncs at checkpoints, which would cross-match a store kind that never syncs)

| family | ours p50 µs | sqlite p50 µs | ratio p50 (ours/sqlite) | work | about |
|---|---:|---:|---:|---:|---|
| law_commit_attempt | 26.458 | 17.292 | 1.5301 | 1 | one judged Attempt insert per commit under the full law roster (key + containment + capacity) |
| law_commit_cluster | 49.250 | 49.917 | 0.9866 | 4 | one judged 4-row cluster per commit: attempt + verdict + steer + scope — every statement family exercised in one commit |
| law_reject_key | 14.916 | 3.250 | 4.5895 | 1 | one REFUSED duplicate-(task, n) commit per sample (Functionality cited) |
| law_reject_containment | 8.541 | 5.750 | 1.4854 | 1 | one REFUSED absent-task commit per sample (Containment cited) |
| law_reject_window | 8.750 | 3.000 | 2.9167 | 1 | one REFUSED 9th-attempt commit on the saturated task 0 per sample (Capacity cited) |
| law_reject_scope | 7.209 | 3.250 | 2.2182 | 1 | one REFUSED Observe-steer scope commit per sample (the ψ containment cited) |

### rejection latency

The `law_reject_*` rows price a REFUSED commit round-trip: on the engine, the full dependency judgment plus the abort (`Error::CommitRejected`, the complete violation set decoded); on SQLite, the constraint failure — UNIQUE, FK, or a trigger's `RAISE(ABORT)` — plus the `ROLLBACK`. No rejected sample commits anything on either engine (the post-state fold certifies it).

## Flame summaries (per family, --trace)

### durable / law_commit_attempt

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     4650.208     4650.208     4650.208     4650.208
apply_inserts                  1       20.291       20.291       20.291       20.291
write_txn                      1     4713.541       14.916     4713.541     4713.541
apply_deletes                  1       12.791       12.791       12.791       12.791
commit                         1     4698.625        7.962     4698.625     4698.625
counters_flush                 1        4.541        4.541        4.541        4.541
judgment_capacities            1        1.833        1.833        1.833        1.833
judgment_source                1        0.958        0.958        0.958        0.958
judgment_target                1        0.041        0.041        0.041        0.041
total wall 4713.541 us
```

### durable / law_commit_cluster

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1     4875.625     4875.625     4875.625     4875.625
apply_inserts                  1      134.750      134.750      134.750      134.750
write_txn                      1     5241.333      108.208     5241.333     5241.333
commit                         1     5133.125       91.627     5133.125     5133.125
counters_flush                 1       14.125       14.125       14.125       14.125
judgment_source                1        8.333        8.333        8.333        8.333
judgment_capacities            1        7.708        7.708        7.708        7.708
judgment_target                1        0.666        0.666        0.666        0.666
apply_deletes                  1        0.291        0.291        0.291        0.291
total wall 5241.333 us
```

### durable / law_reject_key

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_inserts                  1       54.958       54.958       54.958       54.958
commit                         1       97.625       42.584       97.625       97.625
write_txn                      1      127.166       29.541      127.166      127.166
apply_deletes                  1        0.083        0.083        0.083        0.083
total wall 127.166 us
```

### durable / law_reject_containment

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_inserts                  1       26.500       26.500       26.500       26.500
commit                         1       52.708       22.668       52.708       52.708
write_txn                      1       65.000       12.292       65.000       65.000
judgment_source                1        2.208        2.208        2.208        2.208
judgment_capacities            1        1.000        1.000        1.000        1.000
judgment_target                1        0.291        0.291        0.291        0.291
apply_deletes                  1        0.041        0.041        0.041        0.041
total wall 65.000 us
```

### durable / law_reject_window

```text
span                       calls     total_us      self_us       p50_us       max_us
commit                         1       49.958       27.501       49.958       49.958
apply_inserts                  1       17.833       17.833       17.833       17.833
write_txn                      1       57.375        7.417       57.375       57.375
judgment_capacities            1        3.000        3.000        3.000        3.000
judgment_source                1        1.500        1.500        1.500        1.500
judgment_target                1        0.083        0.083        0.083        0.083
apply_deletes                  1        0.041        0.041        0.041        0.041
total wall 57.375 us
```

### durable / law_reject_scope

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_inserts                  1       18.541       18.541       18.541       18.541
commit                         1       33.625       13.419       33.625       33.625
write_txn                      1       40.250        6.625       40.250       40.250
judgment_source                1        1.333        1.333        1.333        1.333
judgment_target                1        0.208        0.208        0.208        0.208
judgment_capacities            1        0.083        0.083        0.083        0.083
apply_deletes                  1        0.041        0.041        0.041        0.041
total wall 40.250 us
```

### nosync / law_commit_attempt

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1       11.708       11.708       11.708       11.708
apply_inserts                  1        4.333        4.333        4.333        4.333
commit                         1       20.541        2.543       20.541       20.541
write_txn                      1       22.416        1.875       22.416       22.416
counters_flush                 1        0.916        0.916        0.916        0.916
judgment_capacities            1        0.666        0.666        0.666        0.666
judgment_source                1        0.375        0.375        0.375        0.375
judgment_target                1        0.000        0.000        0.000        0.000
apply_deletes                  1        0.000        0.000        0.000        0.000
total wall 22.416 us
```

### nosync / law_commit_cluster

```text
span                       calls     total_us      self_us       p50_us       max_us
lmdb_commit                    1       23.625       23.625       23.625       23.625
apply_inserts                  1       13.541       13.541       13.541       13.541
write_txn                      1       49.750        3.917       49.750       49.750
counters_flush                 1        3.291        3.291        3.291        3.291
commit                         1       45.833        3.252       45.833       45.833
judgment_source                1        1.333        1.333        1.333        1.333
judgment_capacities            1        0.750        0.750        0.750        0.750
judgment_target                1        0.041        0.041        0.041        0.041
apply_deletes                  1        0.000        0.000        0.000        0.000
total wall 49.750 us
```

### nosync / law_reject_key

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_inserts                  1        7.416        7.416        7.416        7.416
commit                         1       10.750        3.334       10.750       10.750
write_txn                      1       12.333        1.583       12.333       12.333
apply_deletes                  1        0.000        0.000        0.000        0.000
total wall 12.333 us
```

### nosync / law_reject_containment

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_inserts                  1        3.750        3.750        3.750        3.750
commit                         1        7.625        3.294        7.625        7.625
write_txn                      1        9.250        1.625        9.250        9.250
judgment_source                1        0.333        0.333        0.333        0.333
judgment_capacities            1        0.166        0.166        0.166        0.166
judgment_target                1        0.041        0.041        0.041        0.041
apply_deletes                  1        0.041        0.041        0.041        0.041
total wall 9.250 us
```

### nosync / law_reject_window

```text
span                       calls     total_us      self_us       p50_us       max_us
apply_inserts                  1        3.625        3.625        3.625        3.625
commit                         1        7.666        3.084        7.666        7.666
write_txn                      1        9.083        1.417        9.083        9.083
judgment_capacities            1        0.666        0.666        0.666        0.666
judgment_source                1        0.250        0.250        0.250        0.250
judgment_target                1        0.041        0.041        0.041        0.041
apply_deletes                  1        0.000        0.000        0.000        0.000
total wall 9.083 us
```

### nosync / law_reject_scope

```text
span                       calls     total_us      self_us       p50_us       max_us
commit                         1        6.666        3.376        6.666        6.666
apply_inserts                  1        2.916        2.916        2.916        2.916
write_txn                      1        7.708        1.042        7.708        7.708
judgment_source                1        0.333        0.333        0.333        0.333
judgment_target                1        0.041        0.041        0.041        0.041
judgment_capacities            1        0.000        0.000        0.000        0.000
apply_deletes                  1        0.000        0.000        0.000        0.000
total wall 7.708 us
```

