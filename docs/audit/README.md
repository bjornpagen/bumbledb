# The deep correctness audit (2026-07-06)

Ten parallel auditors, one per subsystem plus two cross-cutting (the oracle
itself, and concurrency/crash seams). Every auditor read the Free Join paper
and all eight architecture docs **before** reading a line of code; every
finding carries a concrete failure scenario (no pattern-matched speculation),
and every report ends with an explicit "checked and sound" coverage record.

## Totals

| report | CRITICAL | HIGH | MEDIUM | LOW | NOTE |
|---|---|---|---|---|---|
| storage.md | 0 | 0 | 0 | 2 | 4 |
| image.md | 0 | 0 | 0 | 1 | 3 |
| ir.md | 0 | 0 | 0 | 1 | 3 |
| plan.md | 0 | 0 | 1 | 1 | 4 |
| colt.md | 0 | 0 | 1 | 2 | 4 |
| executor.md | 0 | 0 | 2 | 1 | 2 |
| sink-pipeline.md | 0 | 0 | 1 | 1 | 3 |
| api-schema.md | 0 | 0 | 1 | 3 | 5 |
| oracle.md | 0 | 1 | 5 | 3 | 3 |
| concurrency-crash.md | 1 | 0 | 1 | 1 | 4 |
| **total** | **1** | **1** | **12** | **16** | **35** |

## The headline

The engine's core algorithm is clean: no wrong-results, panic, or UB path was
found in the executor, COLT, sinks, IR, or planner **through the production
pipeline** — the set-semantics machinery, the D2 skip, cover choice, the
elision proof, the memo swap atomicity, and the value encodings all verified
by concrete trace. The two worst findings live at the *edges*: an API shape
that lets derived state cross database boundaries, and a verification stamp
that does not know what code it vouches for.

## The fix queue, ranked

1. **[CRITICAL] Cross-database memo poisoning** (concurrency-crash.md) —
   `PreparedQuery`'s view memo keys on the bare u64 generation with no
   environment identity; executing a prepared query against a second `Db`
   (or a wiped-and-recreated store) at a coinciding generation silently
   returns the *other* database's data. Verified by repro through the public
   API. Fix: brand `PreparedQuery` to its `Db` (make cross-db execution
   unrepresentable) or fold an environment epoch into every memo key.
2. **[HIGH] The verify stamp tracks no code identity** (oracle.md) — the
   stamp hashes `CARGO_PKG_VERSION` (pinned 0.1.0 forever), so after any
   engine/translator change `bench` accepts the stale stamp and brands a
   never-verified engine VERIFIED. Convention (`scripts/bench.sh` re-runs
   verify) is the only current protection. Fix: fold a code-identity
   ingredient (runtime git rev + dirty flag, or a build fingerprint) into
   `stamp_value`.
3. **[MEDIUM] Serial ids re-issued after a net-no-op commit**
   (api-schema.md) — a committed write that nets to nothing skips the
   counter flush, but ids minted inside it already escaped via the closure's
   return value; a later transaction re-issues them, contradicting the
   never-reissue guarantee.
4. **[MEDIUM] Oracle coverage holes** (oracle.md) — querygen never produces
   cross-atom residual comparisons, cyclic joins, false gates / empty
   relations, or U64 aggregates: four live engine subsystems where a bug is
   currently invisible to SQLite comparison. Plus the remaining oracle
   MEDIUMs recorded in its report.
5. **[MEDIUM] Doc contradictions that would re-plant fixed bugs**
   (executor.md) — `30-execution.md:74` still states the superseded
   label-first cover rule (the "measured wrong-cover"); the NEON
   survivor-compaction claim contradicts the scalar implementation.
6. **[MEDIUM] Parked COLTs pin prepare-generation images**
   (sink-pipeline.md) — placeholder views hold `Arc<RelationImage>` from
   prepare for the query's lifetime; memory retention only, but contradicts
   40-storage's steady-state-heap claim.
7. **[MEDIUM] `validate()` accepts plans that drop a zero-var occurrence**
   (plan.md) — the partition check is vacuous for empty var sets; a
   hand-built plan omitting a gate returns all of R instead of the empty
   set. Unreachable from `prepare`; the boundary exists to reject exactly
   this.
8. **[MEDIUM] COLT `BatchToken` reinterpretation is undefended** (colt.md) —
   forcing a node mid-drain silently reinterprets a chunk token as a dense
   index; structurally unreachable today, zero-cost to assert.
9. **[LOW] Corruption paths that panic instead of returning typed errors**
   (storage.md, image.md) — short R/F keys and corrupted row counters fail
   as slice/overflow panics rather than the documented `Corruption` errors.
10. **[LOW/NOTE] The rest** — per-report: doc-comment fusion on
    `ImageCache::peek`, plan-skew on sibling-occurrence filters (ir.md), and
    the remaining notes.

## Reading order for the fix loop

Start with `concurrency-crash.md` and `oracle.md` (the two top findings and
their exact repros), then `api-schema.md`; the remaining reports are
independent and can be fixed in any order. Every report's "checked and
sound" section is part of the record — what was verified matters as much as
what was found.
