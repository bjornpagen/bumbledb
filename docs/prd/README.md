# PRD set — the correctness and elegance pass

This directory is the complete, ordered work plan for the 2026-07-09 review
findings, the two Postgres-transfer features, and the rebuild-seam refactor. It
supersedes the retired `docs/todo/` ledger and the previous (fully executed) PRD
set — both live only in git history. When a PRD and an architecture chapter
disagree, **the chapter wins** and the PRD is amended.

## Policy (read before executing any PRD)

1. **A PRD is a work-organizational unit, not an atomic passing-code state.** The
   tree may be broken between PRDs. Never write a transitional shim, a
   compatibility alias, or a feature flag to keep old and new coexisting. Rip the
   old thing out and cut directly to the end state; downstream breakage is the
   next PRD's job. (This campaign is smaller-grained than the rebuild — most PRDs
   here *will* leave the tree green — but green-between-PRDs is never a
   requirement, only a coincidence.)
2. **Passing criteria are typed.** `[shape]` — checkable by reading or grep the
   moment the PRD lands. `[test]` — unit tests written *in this PRD* that pass
   once their dependencies exist. `[gate]` — holds when the campaign closes:
   `cargo fmt --all --check`, `clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, `scripts/check.sh`.
3. **No migrations, ever.** No PRD may write store-conversion code.
4. **No smoke-test or end-to-end-test PRDs.** Unit tests co-located with the code
   they pin are in scope and required where a PRD says so. Running the
   verify/bench harness and judging its results is orchestrator/human work — a
   PRD may *require* that a verify run happens after it (07/09 do), but running
   it is not PRD content.
5. **Vocabulary discipline:** never introduce `unique`, `fk`, `foreign`,
   `primary key`, `constraint`, `cascade`, `restrict` as identifiers or concepts.
   The vocabulary is *statement*, *functionality/key (FD)*, *containment (IND)*,
   *judgment*, *guard*, *reverse edge*.
6. **Conflict protocol:** if executing a PRD reveals the architecture docs are
   wrong or silent, stop, record the conflict in the PRD file under `## Conflict`,
   and leave the decision to the owner. Do not improvise semantics.
7. **A finished PRD is deleted from this folder in its landing commit**, and the
   README table drops its row.

## Execution order

Strict order. Two hard sequencing rules: **11 runs after 02** (its G-item test
uses 02's applied-inserts machinery), and **the full two-oracle verify runs green
immediately after 09 lands** (the chase is the only real regression risk) before
anything stacks on top. The campaign closes with re-earned benchmarks and
regenerated charts (hot paths move), which is orchestrator work, not a PRD.

| Phase | PRDs | What exists at the end |
|---|---|---|
| A — correctness | 01 02 03 | No reachable panic from valid input; oracles agree on every verdict label — each fixed by deleting the representation that made the case expressible, not by guarding it |
| B — contract & hardening | 04 05 | The allocation contract states its true invariant and the gate can see violations; reopen trust bounded; reader cap configured |
| C — the sweeper | 06 07 | `Db::verify_store`: full store coherence + global judgment re-verification, CLI-wrapped |
| D — the chase | 08 09 | Containment-implied occurrence elimination, EXPLAIN'd, differentially covered |
| E — staleness | 10 | Pull-based plan-drift signal |
| F — sweep | 11 | The minor findings, each to its pinned verdict |
| G — elegance | 12 13 14 15 16 17 | The rebuild seams removed, subsystem by subsystem, behavior-preserving |

## The PRDs

- [01 — Hoist-path scratch: delete the caps](01-hoist-eligibility.md)
- [02 — Net-disposition delta](02-applied-inserts-direction.md)
- [03 — `alloc_dyn`: parse, don't validate](03-alloc-dyn-typed-error.md)
- [04 — The high-water allocation contract](04-alloc-highwater-contract.md)
- [05 — Storage hardening](05-storage-hardening.md)
- [06 — `verify_store`: namespace coherence](06-verify-store-namespaces.md)
- [07 — `verify_store`: global judgments + CLI](07-verify-store-judgments.md)
- [08 — The chase: analysis and rewrite](08-chase-rewrite.md)
- [09 — The chase: surfaces and coverage](09-chase-surfaces.md)
- [10 — Plan staleness signal](10-staleness-signal.md)
- [11 — Minor findings sweep](11-minor-sweep.md)
- [12 — Elegance: schema, encoding, error](12-elegance-schema.md)
- [13 — Elegance: storage](13-elegance-storage.md)
- [14 — Elegance: IR and plan](14-elegance-ir-plan.md)
- [15 — Elegance: exec and image](15-elegance-exec-image.md)
- [16 — Elegance: api and macros](16-elegance-api-macros.md)
- [17 — Elegance: bench crate](17-elegance-bench.md)

## Elegance-pass constraints (bind PRDs 12–17 jointly)

Strictly behavior-preserving: no semantics change, no new features, no
error-shape changes; no test *assertion* changes (test code may restructure;
expected values may not). The unsafe-allowlisted hot modules (`exec/kernel.rs`,
`exec/colt.rs` gather/probe, `exec/wordmap.rs`, `exec/run.rs` leaf/batch,
`image.rs` decode, `obs.rs` fast clock) are touch-only-with-cause: a hot-path
refactor needs a reason stronger than taste, and any change that could plausibly
move a measured number is flagged in the commit body for the closing re-bench.
Each PRD's commit body carries a findings summary — what was deduplicated, what
moved, what died — so review is of decisions, not diffs. The known seam classes
to hunt, in priority order: near-duplicate helpers across module boundaries
(EXCEPTION: the naive model's independence from engine algorithms is a design
requirement, never a seam); idiom drift (error construction, iterator style,
`expect` messages, doc-comment voice, test naming/fixtures); altitude
misplacements (caller logic belonging in the callee's type; over-wide signatures
threading state a struct should own); dead weight (parameters no caller varies,
variants no site constructs, `pub` with one internal caller, comments narrating
the obvious); test overlap (merge and redirect, never just delete).
