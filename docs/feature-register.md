# The feature register — whispers, verdicts, triggers

The durable record of every feature the workload has whispered for, each
investigated to a verdict (full reports in `docs/research/`). The law of this
document: a DEFERRED or REJECTED feature carries a RECORDED TRIGGER — a
concrete, observable condition under which the question reopens — so nothing
is relitigated from vibes and nothing worthy is forgotten. Investigated
2026-07-19 against primer's real workload (50 prepared queries, the store
schema, the host-gate census).

## Verdicts

### 1. Aggregate comparisons (the HAVING shape) — DEFER, weak form only
- **Strong form (interior HAVING — an aggregate feeding another rule):
  REJECTED.** Reverses the creation-quarantine ruling verbatim ("a created
  value never re-enters a derivation"); refused today by name
  (`AggregateInteriorPredicate`). Not a feature — a doctrine reversal.
- **Weak form (filter an output head's fold before emit): FEASIBLE AND
  CHEAP** — head-level `having` list in the IR, one word-compare before
  `AggregateSink::finalize_into`, zero stratification impact, no new Lean
  axioms, SQLite oracles it natively. PRD-ready design parked in
  `docs/research/aggregate-comparisons.md` (IR slot, validation roster,
  notation shapes, corpus cases).
- **Why deferred**: the complete evidence is four one-liner host folds in one
  primer module (`positionExtent`'s max/count compare, `riCoverageCount`'s
  `< 2n` floor, siblings) — doctrinal citations, not pain.
- **TRIGGERS**: (a) a measured materialize-then-filter budget violation;
  (b) the host-fold register outgrowing one module; (c) any agg-vs-PARAM
  sighting (a configurable threshold gate — the shape that would pay).

### 2. Disjunctive containment (sum-domain references) — REJECT
- The motivating column (`task.subject`) disproved the motivation twice: it
  is a TAGGED sum (`task.kind` selects the target — expressible TODAY as
  source-selected conditional containments, stronger and cheaper), and its
  subjects dangle BY DESIGN (the task ledger is append-only over deletable
  operands — no containment of any shape may hold it). Exists-in-any over
  per-(relation,field) fresh mints mostly certifies numeric coincidence; the
  TS class-wall carve (union-find → DAG) would unravel the one-generator
  teaching. Cookbook law 3 was the honest answer all along.
- **Engineering pre-survey preserved** (`docs/research/disjunctive-containment.md`
  + the engine sub-report): descriptor/fingerprint/codec extend mechanically;
  the delete side needs cross-target re-probes; the Lean path is moderate.
- **TRIGGER**: a censused workload with an UNTAGGED sum pointer over ONE
  shared id space (supplied, not fresh) whose references must hold at every
  commit. None sighted.
- **Free action, still open**: primer's `Supervise` arm (subject → `task.id`,
  never deleted) is declarable today as a source-selected containment.

### 3. Mintable pins (the Lane-1 ψ reshape) — RECOMMEND, sequenced
- Reshape primer's `Pin`/`Outcome`/`SteerKind` to payload-tier
  (`mintable`/`writable: bool`) and replace three bare vocab containments
  with ψ-selected ones — the frozen-dead-handle prose becomes law. Zero
  engine work (verified: the ψ fold + member-set machinery covers it), zero
  call-site churn on the 0.4.0 string surface, one <100-line primer commit +
  a disposable-store rebuild.
- **SEQUENCED BEHIND**: primer's lattice-cutover packet (rewrites the same
  schema file; explicitly deferred this reshape).
- **OWNER RULING PENDING**: flag vs funeral — mark dead handles
  `mintable: false` (era history as data; recommended) or delete them under
  the funeral precedent.

### 4. Graph read-models (bumbledb serving primer's postgres) — REJECT
- The exhibit (`course-closure.ts`) is the SOLVED state: closure recomputes
  in the same postgres transaction as edge writes — staleness is
  unrepresentable; it is primer's only recursion-shaped postgres workload.
  The real readers are serverless; the SDK ships darwin-only; no read-only
  open exists; multi-process is a recorded v0 non-goal; `00-product.md` cuts
  against the shape. A pipeline would trade an ACID invariant for seven new
  failure modes.
- **TRIGGERS (all must hold)**: a long-lived linux worker in primer's
  topology; a shipped, tested linux platform package; a genuine
  read-only/multi-process open (owner decisions); a graph workload too big
  or write-hot to materialize.

## The host-fold register (the census's residual unspellables)

Query shapes primer legitimately folds in the HOST, each a recorded citation
(the system working as designed — the engine judges, the host folds), watched
by trigger 1(b) above:

- `positionExtent` — aggregate-vs-aggregate compare (max vs count).
- `riCoverageCount`/`riUnderCovered` — aggregate floor (`count < 2n`).
- `dissolveUnjudged` / `preCourseUnjudged` — latest-attempt argmax with
  verdict-absence over the kind-scoped `task.subject` join (also gated by
  the sum-domain rejection above).
- `emptyContractField` — string whitespace predicate (host-residence class
  by doctrine).
- `confusablePairKey` — intra-row `a < b` normalization.
- The serialization census folds (`macroOrder`, string-keyed counting) —
  sequence folds with no query spelling, host-residence by citation.
- The idb re-grounding tax (an idb atom is a join position) — engine law,
  documented, ~6 recursive queries carry one extra `.match`.
- Keyed-get/typed lookup for task-by-(kind, subject) — the anyOf
  investigation's "what primer actually needs" aside; smallest of the set
  (shipped: keyed get, 70-api ledger row (b)) — the fold is now expressible
  as a point read.

## FIRED and scheduled (the owner's prioritization, 2026-07-19)

Two OPEN-ledger rows in `docs/architecture/70-api.md` whose triggers already
FIRED get their own planning wave AFTER cleanup-0.5.0 lands and BEFORE any
1.0.0 surface freeze (they are surface additions; they belong under the tag):

- **Keyed get** — **shipped (this wave, 2026-07-19)**: reading through the
  declared key FDs IS the obvious spelling on both the read scope and the
  write transaction — Rust `snap.get(key)` / `tx.get(key)` over the generated
  `Key` values, TS `get(relation, keyStatement, key)` on
  `Db`/`ReadScope`/`Tx`; the terminal record is
  `docs/architecture/70-api.md` ledger row (b), the at-most-one answer
  derived (`lean/Bumbledb/Dependencies.lean: keyed_get_at_most_one`), pinned
  by `crates/bumbledb/tests/keyed_get.rs`, `ts/test/keyed-get.test.ts`, and
  cookbook recipe 30. Evidence (the record of why): primer
  re-implements keyed lookup host-side five ways, the ETL shadows its own key
  laws with five host maps, and the existing primary-key get goes unused
  (~15 workaround sites total). The keys are laws; the surface exposes them.
- **Answer ordering/limit conveniences** — the census-split sorting half
  (four hand-rolled bigint comparators, every rank/pos consumer sorting
  host-side). Host-side, on the `query!` quarantine — the engine never
  orders; that ruling stands.

## Also parked elsewhere (cross-references)

- The deletion-vector/mask fork: **REFUTED BY MEASUREMENT** (the decider
  twin, B/A 1.20–1.24 — see the incremental-images ruling record in
  `docs/architecture/`); compact-on-delete is law.
- The O(delta) slab append: scoped out with invariant language recorded
  (copy-on-append ruling record).
- The per-store map size parameter: recorded follow-up design (G1), only if
  a real ephemeral capacity need appears.
- The tagged-template query notation: the conformance corpus
  (`crates/bumbledb-query/tests/notation-corpus/`) is the pre-built ramp;
  a taste decision, not a feasibility one.
