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
| cardinality window | `Task(id) <={0..8} Attempt(task)` | `CREATE TRIGGER "lawful_attempt_window" BEFORE INSERT ON "Attempt" WHEN (SELECT COUNT(*) FROM "Attempt" WHERE "task" = NEW."task") >= 8 BEGIN SELECT RAISE(ABORT, 'attempt window exceeded'); END` |

## lane `durable`

Db::create (LMDB issues F_FULLFSYNC unconditionally on macOS) vs SQLite WAL synchronous=FULL fullfsync=ON checkpoint_fullfsync=ON, cache_size=-262144, temp_store=MEMORY, mmap_size=1GiB, wal_autocheckpoint=0 — both engines flush to media on every commit

| family | ours p50 µs | sqlite p50 µs | ratio p50 (ours/sqlite) | work | about |
|---|---:|---:|---:|---:|---|
| law_commit_attempt | 4237.458 | 4958.125 | 0.8546 | 64 | one judged Attempt insert per commit under the full law roster (key + containment + window) |
| law_commit_cluster | 5182.958 | 4511.792 | 1.1488 | 256 | one judged 4-row cluster per commit: attempt + verdict + steer + scope — every statement family exercised in one commit |
| law_reject_key | 4395.875 | 7.666 | 573.4249 | 64 | one REFUSED duplicate-(task, n) commit per sample (Functionality cited) |
| law_reject_containment | 26.750 | 13.958 | 1.9165 | 64 | one REFUSED absent-task commit per sample (Containment cited) |
| law_reject_window | 28.458 | 7.708 | 3.6920 | 64 | one REFUSED 9th-attempt commit on the saturated task 0 per sample (Cardinality cited) |
| law_reject_scope | 18.291 | 6.417 | 2.8504 | 64 | one REFUSED Observe-steer scope commit per sample (the ψ containment cited) |

## lane `nosync`

Db::ephemeral (MDB_NOSYNC: pages and meta pwritten, no sync boundary ever crossed) vs SQLite WAL synchronous=OFF fullfsync=OFF checkpoint_fullfsync=OFF, cache_size=-262144, temp_store=MEMORY, mmap_size=1GiB, wal_autocheckpoint=0 — WAL frames written, never synced (OFF, not NORMAL: NORMAL still syncs at checkpoints, which would cross-match a store kind that never syncs)

| family | ours p50 µs | sqlite p50 µs | ratio p50 (ours/sqlite) | work | about |
|---|---:|---:|---:|---:|---|
| law_commit_attempt | 23.667 | 17.625 | 1.3428 | 64 | one judged Attempt insert per commit under the full law roster (key + containment + window) |
| law_commit_cluster | 47.459 | 53.000 | 0.8955 | 256 | one judged 4-row cluster per commit: attempt + verdict + steer + scope — every statement family exercised in one commit |
| law_reject_key | 15.125 | 3.250 | 4.6538 | 64 | one REFUSED duplicate-(task, n) commit per sample (Functionality cited) |
| law_reject_containment | 9.500 | 5.500 | 1.7273 | 64 | one REFUSED absent-task commit per sample (Containment cited) |
| law_reject_window | 9.709 | 3.042 | 3.1917 | 64 | one REFUSED 9th-attempt commit on the saturated task 0 per sample (Cardinality cited) |
| law_reject_scope | 7.250 | 2.750 | 2.6364 | 64 | one REFUSED Observe-steer scope commit per sample (the ψ containment cited) |

### rejection latency

The `law_reject_*` rows price a REFUSED commit round-trip: on the engine, the full dependency judgment plus the abort (`Error::CommitRejected`, the complete violation set decoded); on SQLite, the constraint failure — UNIQUE, FK, or a trigger's `RAISE(ABORT)` — plus the `ROLLBACK`. No rejected sample commits anything on either engine (the post-state fold certifies it).
