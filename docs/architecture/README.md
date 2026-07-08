# Bumbledb Architecture

These documents are the normative design. They are living documents, updated in place —
there are no PRD suites, work packets, or compliance gates. Git history is the changelog.

## Rules for these docs

1. **Every decision records its strongest alternative, why it lost, and what evidence
   would reverse it** — one paragraph. If we can't articulate the alternative, the
   decision isn't made yet.
2. **Every deviation from the Free Join paper gets a `Deviation:` block**: what the paper
   says, what we do instead, why, and the reversal evidence. The paper
   (`docs/free-join-paper/`) is algorithmic authority; these docs are product authority.
   This rule covers the query model's divergences from the paper's §2 CQ assumptions,
   not just the execution doc.
3. **Every specified mechanism must name its reader** — a namespace, cache, statistic,
   or counter with no named consumer is deleted. (The anti-transcription rule; see
   post-mortem §22.)
4. **Undecided things are marked `OPEN` with a closure trigger** (the event or milestone
   that forces the decision) and listed below. An OPEN item is a real state; the failure
   mode is code deciding it silently.
5. **When implementation contradicts a doc**, the doc is amended in the same change, or
   the code change doesn't land. Docs describe the system in the present tense.
   **Standing exception, active now:** the 2026-07-08 dependency redesign rewrote this
   chapter set ahead of the code — the repo is in its documented-broken state until
   the implementation catches up, and every gap is a work item, not a doc bug.
6. History: the five discarded implementations (v1–v5) live in git before commit
   `1b65ae8`; the post-mortem that motivates many decisions is
   `docs/history/post-mortem.md`. The pre-redesign v0 engine and its measured record
   (README charts, docs/silicon2/final2.md, the 2,468-case oracle stamp) are history
   as of 2026-07-08 — cited as evidence about that engine, never as claims about this
   one.

## The documents

| Doc | Contents |
|---|---|
| `00-product.md` | Thesis, workload census, hardware, durability, deleted vocabulary, success criteria |
| `10-data-model.md` | Set semantics, the seven structural types, the interval denotation, identity, schema |
| `20-query-ir.md` | The pure-data IR: atoms, negation, membership, param sets, aggregates, validation |
| `30-dependencies.md` | The two judgments (functionality, containment), statements, pointwise lifting, the acceptance gate |
| `40-execution.md` | Access paths, Free Join over COLT, anti-probes, planner, vectorization, allocation |
| `50-storage.md` | LMDB layout, guard namespaces as judgment accelerators, the delta write path, images |
| `60-validation.md` | The two oracles (SQLite + naive model), ledger benchmark protocol, test families |
| `70-api.md` | Embedding surface: the schema! grammar, transactions, point reads, results, ETL |

## OPEN items

- **Everything measured is unearned on the new format**: the oracle stamp, the
  benchmark ALL-WIN, and every pinned denominator are void until re-derived and
  re-run post-implementation. *Trigger: the redesign implementation reaching the
  bench milestone.*
- **Recursion** (explicit semi-naive fixpoint): not designed in; no IR decision may
  assume it never comes. The modeling discipline (precomputed closures) has absorbed
  every sighting so far. *Trigger: first real query that needs one.*
- **`Pack`** (the coalescing interval aggregate — Snodgrass coalesce, `range_agg`):
  semantics sketched, result shape (a set per group) unresolved. *Trigger: a real
  need plus the shape decision (`20-query-ir.md`).*
- **Ordering/limit conveniences and top-k pushdown**: presentation-layer; results are
  sets, the host sorts. *Trigger: owner pain, or a measured materialize-then-sort
  latency-budget violation.*
- **Declared range/stabbing accelerators**: time-range, point-membership, and
  overlap scans are O(n) by decision; accelerators return only with a benchmark that
  demands them. *Trigger: latency budget violation on a range/interval family.*
- **Dictionary GC**: interned values are never reclaimed (accepted leak,
  `10-data-model.md`). *Trigger: measured dictionary growth dominating store size on
  a real churn workload.*
- **Incremental image maintenance**: images rebuild whole per state-changing commit
  by design (the write design point amortizes it). *Trigger: traced rebuild cost
  violating the latency budget despite the cache — recorded with D1's reversal.*
- **Text query language**: none; if one returns it is pure sugar lowering to
  statements and IR. *Trigger: owner want.*
- **Vectorized batch size**: 64–256 starting range decided; the number is
  measurement-owned. *Trigger: the ledger benchmark on the new format.*
- **EXPLAIN output shape**: ANALYZE semantics shipped; text stability not promised.
  *Remaining open: nothing structural.*
- **`70-api.md` open sub-items**: see that doc's own OPEN list (result ordering,
  multi-key typed `get` sugar, multi-process future).

## Closed by ruling

**2026-07-02** (recorded in the owning docs): nominal typing of any kind (rejected —
hard structural typing); Serial as a type (demoted to field generation attribute);
i128 and narrow-integer types (rejected); field-scoped images (full-width won); plan
invalidation by writes (pin-at-prepare won); multi-process access (out of envelope);
intra-query parallelism (non-goal — the engine owns zero threads); 32-bit targets
(64-bit only); constraint enforcement timing (commit-time, final state); `replace`
(unnecessary — operation order is semantically irrelevant).

**2026-07-08, the dependency redesign** (each recorded in its owning doc):
- **The dependency unification** — invariants are two judgments about queries
  (functionality, containment); *unique/foreign key/primary key/check/exclusion/
  cascade/restrict/trigger/deferrable* are deleted vocabulary (`30-dependencies.md`,
  `00-product.md`).
- **No sugar** — the schema surface is raw statements (`->`, `<=`, `==`); no
  field-level constraint modifiers, no `union` keyword (the pattern is derived, its
  theorems proved, in `30-dependencies.md`).
- **Interval as the seventh type**, with the point-set denotation; pointwise keys and
  coverage containments as theorems; order operators and Min/Max refused on it;
  uuid rejected with the serial rationale (`10-data-model.md`).
- **The IR graduates** negation (anti-join atoms with the safety rule), point
  membership (a typing rule), param sets (`IN`), `CountDistinct`, and
  Arg-restriction with set-honest ties; the outer join is a documented decomposition,
  never a node (`20-query-ir.md`).
- **WriteTx point reads** (`contains`/`get` against the delta-overlaid final-state
  view) — supersedes the 2026-07-02 "queries inside write transactions forbidden"
  ruling in its point-read half; full queries in write transactions stay forbidden
  (`70-api.md`).
- **The naive model is required infrastructure** — the second oracle, judging
  dependency semantics SQLite cannot express (`60-validation.md`).
- **Format break, no migration** — pre-redesign stores do not open; ETL is the path
  (`50-storage.md`, `70-api.md`).
- **Non-key and conditional FDs rejected**; partial keys answered by relation
  splits; general FDs answered by normalization (`30-dependencies.md`).
