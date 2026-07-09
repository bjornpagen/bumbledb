# PRD set — the representation, correctness, and elegance pass

This directory is the complete, ordered work plan for the 2026-07-09 review
findings, the representation-collapse audit that followed it, the two
Postgres-transfer features, and the rebuild-seam refactor. It supersedes the
retired `docs/todo/` ledger and the previous (fully executed) PRD set — both
live only in git history. When a PRD and an architecture chapter disagree,
**the chapter wins** and the PRD is amended.

The set's organizing principle is the house axiom taken literally: when a case
shows up, change the representation until the case stops being expressible;
add a branch only at a trust boundary or where the machine model demands the
split. Phase R exists because the audit found the codebase's own foundations
violating it (the same sum type written four times; a derived pairing
reconstructed by search; bookkeeping accumulated where it is a pure function).
Those collapse **first**, so every later PRD lands on clean ground.

## Policy (read before executing any PRD)

1. **A PRD is a work-organizational unit, not an atomic passing-code state.**
   Never write a transitional shim, a compatibility alias, or a feature flag to
   keep old and new coexisting. Rip the old thing out and cut directly to the
   end state; downstream breakage is the next PRD's job. (Most PRDs here will
   leave the tree green — that is a coincidence, never a requirement.)
2. **Passing criteria are typed.** `[shape]` — checkable by reading or grep the
   moment the PRD lands. `[test]` — unit tests written *in this PRD* that pass
   once their dependencies exist. `[gate]` — holds when the campaign closes:
   `cargo fmt --all --check`, `clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, `scripts/check.sh`.
3. **No migrations, ever.** No PRD may write store-conversion code.
4. **No smoke-test or end-to-end-test PRDs.** Unit tests co-located with the
   code they pin are in scope and required where a PRD says so. Running the
   verify/bench harness is orchestrator/human work — a PRD may *require* that a
   verify run happens after it (12 does), but running it is not PRD content.
5. **Vocabulary discipline:** never introduce `unique`, `fk`, `foreign`,
   `primary key`, `constraint`, `cascade`, `restrict` as identifiers or
   concepts. The vocabulary is *statement*, *functionality/key (FD)*,
   *containment (IND)*, *judgment*, *guard*, *reverse edge*.
6. **Conflict protocol:** if executing a PRD reveals the architecture docs are
   wrong or silent, stop, record the conflict in the PRD file under
   `## Conflict`, and leave the decision to the owner.
7. **A finished PRD is deleted from this folder in its landing commit**, and
   the README table drops its row.

## Execution order

Strict order. Hard sequencing rules: **15 requires 10**, and **the full
two-oracle verify runs green immediately after 12 lands** (the chase is the
only real regression risk) before anything stacks on top. The campaign closes with re-earned benchmarks
and regenerated charts (hot paths move in 04, 15, and possibly 19) — 
orchestrator work, not a PRD.

| Phase | PRDs | What exists at the end |
|---|---|---|
| A — correctness | 04 05 06 | No reachable panic from valid input; oracles agree on every verdict label — each fixed by deleting the representation that made the case expressible, not by guarding it |
| B — contract & hardening | 07 08 | The allocation contract states its true invariant and the gate can see violations; reopen trust bounded; reader cap configured |
| C — the sweeper | 09 10 | `Db::verify_store`: full store coherence + global judgment re-verification, CLI-wrapped |
| D — the chase | 11 12 | Containment-implied occurrence elimination via the `Role` sum, EXPLAIN'd, differentially covered |
| E — staleness | 13 | Pull-based plan-drift signal |
| F — sweep | 14 | The minor findings, each to its pinned verdict |
| G — commit as data | 15 | `CommitPlan`: the commit's bookkeeping computed as a pure function, the applier a dumb executor |
| H — elegance | 16 17 18 19 20 21 | The rebuild seams removed, subsystem by subsystem, behavior-preserving |

## The PRDs

- [08 — Storage hardening](08-storage-hardening.md)
- [09 — `verify_store`: namespace coherence](09-verify-store-namespaces.md)
- [10 — `verify_store`: global judgments + CLI](10-verify-store-judgments.md)
- [11 — The chase: analysis and rewrite](11-chase-rewrite.md)
- [12 — The chase: surfaces and coverage](12-chase-surfaces.md)
- [13 — Plan staleness signal](13-staleness-signal.md)
- [14 — Minor findings sweep](14-minor-sweep.md)
- [15 — CommitPlan: compute, don't accumulate](15-commit-plan.md)
- [16 — Elegance: schema, encoding, error](16-elegance-schema.md)
- [17 — Elegance: storage](17-elegance-storage.md)
- [18 — Elegance: IR and plan](18-elegance-ir-plan.md)
- [19 — Elegance: exec and image](19-elegance-exec-image.md)
- [20 — Elegance: api and macros](20-elegance-api-macros.md)
- [21 — Elegance: bench crate](21-elegance-bench.md)

## Refusals (recorded, with reasons — do not re-litigate)

The representation audit pushed on these and they pushed back; each survives
as a deliberate decision:

- **`Side` does not become `ir::Atom`.** A statement side has no variables —
  its positions are ground identity, and forcing `Term`/`VarId` machinery into
  schema would merge two *essentially* different things. Share the `Value`
  (PRD 01); keep the shapes distinct. Representation collapses accidental
  cases; this difference is essential.
- **The three per-node rejection lists stay split by kind.** Grouped-by-kind
  is the representation of the batching law (pure-ALU residuals vs two-phase
  batched probes); one interleaved predicate list would force per-item
  dispatch. PRD 19 mandates the comment at the definition site.
- **The guard-probe fast path stays separate from Free Join.** It exists to
  answer point lookups without an image build — a measured property, not a
  control-flow patch.
- **`next_origin` stays u32 with a checked increment** (PRD 14-C). The
  representation fix — u64 origins — doubles a hot per-row scratch array's
  width against a beyond-axiom case; the boundary check at mint granularity is
  the cheaper honest shape.
- **Boundary guards are not branches to eliminate.** The reopen-trust ceiling
  (PRD 08) and the dynamic-surface witnesses (PRD 06) check once at a trust
  boundary — that is parse-don't-validate working, not a violation of it.

## Elegance-pass constraints (bind PRDs 16–21 jointly)

Strictly behavior-preserving: no semantics change, no new features, no
error-shape changes; no test *assertion* changes (test code may restructure;
expected values may not). The unsafe-allowlisted hot modules (`exec/kernel.rs`,
`exec/colt.rs` gather/probe, `exec/wordmap.rs`, `exec/run.rs` leaf/batch,
`image.rs` decode, `obs.rs` fast clock) are touch-only-with-cause: a hot-path
refactor needs a reason stronger than taste, and any change that could
plausibly move a measured number is flagged in the commit body for the closing
re-bench. Each PRD's commit body carries a findings summary — what was
deduplicated, what moved, what died — so review is of decisions, not diffs.
The known seam classes to hunt, in priority order: near-duplicate helpers
across module boundaries (EXCEPTION: the naive model's independence from
engine algorithms is a design requirement, never a seam); idiom drift (error
construction, iterator style, `expect` messages, doc-comment voice, test
naming/fixtures); altitude misplacements (caller logic belonging in the
callee's type; over-wide signatures threading state a struct should own); dead
weight (parameters no caller varies, variants no site constructs, `pub` with
one internal caller, comments narrating the obvious); test overlap (merge and
redirect, never just delete).
