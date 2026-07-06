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
| api-schema.md | 0 | 0 | 1 | 3 | 6 |
| oracle.md | 0 | 1 | 5 | 3 | 3 |
| concurrency-crash.md | 1 | 0 | 1 | 1 | 4 |
| **total** | **1** | **1** | **12** | **16** | **36** |

(The table originally undercounted api-schema's NOTEs by one; corrected
here when the resolution ledger enumerated every finding — 66 total.)

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


## Resolution ledger (closed 2026-07-06, hardening suite)

Every finding, resolved. `fixed (PRD NN)` = a code change in
`docs/hardening/NN` with its regression test; `documented (PRD 10)` = the
behavior is correct and is now stated where a reader will look;
`closed` = no action, with the reason. The audit is now a historical
record with zero open items.

### storage.md
| finding | resolution |
|---|---|
| [LOW] Corrupt short keys panic instead of typed Corruption | fixed (PRD 06) |
| [LOW] `create` refuses only bumbledb environments | fixed (PRD 00) |
| [NOTE] `delete_fact` does not verify outgoing R entries | documented (PRD 10 — 40-storage records the asymmetry; offline sweeper stays deferred) |
| [NOTE] Serial values from a no-op successful commit re-issued | fixed (PRD 01) |
| [NOTE] No-op delete of a never-interned string interns it | fixed (PRD 01) |
| [NOTE] Generation/row-id overflow unguarded (asymmetry) | documented (PRD 10 — 40-storage records it as a scale-axiom decision) |

### image.md
| finding | resolution |
|---|---|
| [LOW] Image build trusts the stored row count for slab sizing | fixed (PRD 06) |
| [NOTE] `get_or_build` doc fused onto `peek` | fixed (PRD 10) |
| [NOTE] `Const::PendingIntern` doc overstates miss semantics | fixed (PRD 10) |
| [NOTE] Cold dual-output builder is `#[cfg(test)]`-only | documented (PRD 10 — the builder's doc records the production two-pass path) |

### ir.md
| finding | resolution |
|---|---|
| [NOTE] Stale "3 variants" test comment | fixed (PRD 10) |
| [NOTE] `resolve_filter` Eq-miss arms unreachable but doc implies live | fixed (PRD 10) |
| [NOTE] Duplicate/contradictory comparisons accepted | documented (PRD 10 — `validate`'s doc states the deliberate acceptance) |
| [LOW] Var-vs-constant filters restrict only the first occurrence | closed — sound (a plan-quality skew, not a correctness hole); revisit with estimator work |

### plan.md
| finding | resolution |
|---|---|
| [MEDIUM] validate() accepts plans dropping a zero-var occurrence | fixed (PRD 03) |
| [LOW] Unknown-occurrence subatoms reach the executor as panics | fixed (PRD 03) |
| [NOTE] check_selections unreachable inside validate() | fixed (PRD 03 — demoted to debug_assert with the producer comment) |
| [NOTE] factor() diverges from Fig. 8's text — and is right to | closed by audit (the existing comment is the record) |
| [NOTE] Slot-layout comment + stale carve-out sentence | fixed (PRD 03 slot comment; PRD 10 carve-out sentence) |
| [NOTE] DP inner loop O(2ⁿ·n²); table ~32 MB not ~24 MB | fixed (PRD 10 — per-mask prefix-vars memo; comment states the true sizes) |

### colt.md
| finding | resolution |
|---|---|
| [MEDIUM] Forcing under an outstanding token reinterprets it | fixed (PRD 04) |
| [LOW] `Cursor::Row` iteration ignores `max` | fixed (PRD 04) |
| [LOW] `start()` without `select()` debug-guarded only | fixed (PRD 04) |
| [NOTE] Chunk token packing wraps at 2³²-scale chunk counts | closed by audit — beyond the u32 position space itself (closure comment at the mint site, PRD 04) |
| [NOTE] `position_matches` truncates via `zip` | fixed (PRD 04) |
| [NOTE] Pre-probe growth counts appends | closed by audit — at most one doubling of over-size (closure comment at the site, PRD 04) |
| [NOTE] `WordMap::grow` re-allocates the dense list | fixed (PRD 04) |

### executor.md
| finding | resolution |
|---|---|
| [MEDIUM] 30-execution states the superseded cover rule | fixed (PRD 10) |
| [MEDIUM] NEON survivor-compaction claim contradicts the code | fixed (PRD 10) |
| [NOTE] Aggregate skip-legality on a single point of enforcement | fixed (PRD 05) |
| [NOTE] Phase 1 hashes probes that never use the hash | fixed (PRD 05) |
| [LOW] u32 conversion expects beyond the stated envelope | documented (PRD 06 non-goal — envelope panics stay documented `# Panics` contracts) |

### sink-pipeline.md
| finding | resolution |
|---|---|
| [MEDIUM] Parked placeholder COLTs pin prepare-generation images | fixed (PRD 02) |
| [LOW] Result byte-heap offsets panic past 4 GiB | fixed (PRD 06) |
| [NOTE] Failed execution leaves partial rows in the buffer | documented (PRD 10 — 60-api's ignore-`out`-on-`Err` contract) |
| [NOTE] ProjectionSink's SkipSuffix safe only jointly with run.rs | fixed (PRD 05 — the bits encode the rule; both comments cross-reference) |
| [NOTE] `resolve_filter` Eq-miss short-circuit dead on this path | fixed (PRD 10 — the doc names the belt-and-braces role) |

### api-schema.md
| finding | resolution |
|---|---|
| [MEDIUM] Serial ids minted in a net-no-op commit escape | fixed (PRD 01) |
| [LOW] Deleting a never-interned string interns it | fixed (PRD 01) |
| [LOW] Out-of-range `RelationId` panics on the ETL surface | fixed (PRD 06) |
| [LOW] Nothing binds a `PreparedQuery` to its `Db` | fixed (PRD 00) |
| [NOTE] `bulk_load` counts changed, not consumed | documented (PRD 10 — 60-api states changed-not-consumed) |
| [NOTE] `compact()` concurrency safe but undocumented | documented (PRD 10) |
| [NOTE] Constraint ids are materialized order, not declaration | documented (PRD 10 — 10-data-model states the materialized rule) |
| [NOTE] Nested `Db::write` self-deadlocks | fixed (PRD 00) |
| [NOTE] `Fact::encode_read` has no engine caller | documented (PRD 10 — 60-api states it as host surface) |
| [NOTE] FK duplicate-field misuses `UniqueDuplicateField` | fixed (PRD 06 — renamed `ConstraintDuplicateField`) |

### oracle.md
| finding | resolution |
|---|---|
| [HIGH] The stamp does not invalidate on code changes | fixed (PRD 07 — binary fingerprint) |
| [MEDIUM] No cross-atom residual anywhere in the oracle | fixed (PRD 08 spread family; PRD 09 generator) |
| [MEDIUM] Gates never false; no relation ever empty | fixed (PRD 09 — the empty-store pass) |
| [MEDIUM] No cyclic join despite 50-validation's promise | fixed (PRD 08 — triangle family) |
| [MEDIUM] U64 aggregates never generated; sum-bound hole | fixed (PRD 09) |
| [MEDIUM] Asserted coverage contract weaker than documented | fixed (PRD 09 — the per-(op, type) matrix) |
| [LOW] NUL in a String literal truncates the SQL | fixed (PRD 07) |
| [LOW] Divergence-by-error is a panic, not a bundle | fixed (PRD 07) |
| [LOW] Boundary param sets probe only minima | fixed (PRD 09) |
| [NOTE] balance's Sum is not a balance | fixed (PRD 08) |
| [NOTE] A stamp can be earned with zero randomized cases | fixed (PRD 07 — the report provenance shows the count; zero stays legal, visibly) |
| [NOTE] Family param functions outside the family digest | fixed (PRD 07 — subsumed: params are code, code is the binary fingerprint) |

### concurrency-crash.md
| finding | resolution |
|---|---|
| [CRITICAL] Cross-environment execution aliases the generation clock | fixed (PRD 00 — the audit's repro is the regression test) |
| [MEDIUM] Parked COLTs pin prepare-generation images | fixed (PRD 02) |
| [LOW] Nested `db.write` self-deadlocks | fixed (PRD 00) |
| [NOTE] Panic between LMDB commit and eviction leaks the cache | closed — leaks, never corrupts; the next commit's eviction reclaims, and the unwind killed the writer anyway |
| [NOTE] Multi-process access unguarded | fixed (PRD 00 — the advisory lock) |
| [NOTE] alloc_gate depends on the one-test invariant | fixed (PRD 10 — `--test-threads=1` in check.sh + the named invariant header) |
| [NOTE] No prepared-execution-vs-writer concurrency test | fixed (PRD 00 — the generation-atomicity family) |
