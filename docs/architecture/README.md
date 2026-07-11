# Bumbledb Architecture

These documents are the normative design. They are living documents, updated in place —
there are no work packets or compliance gates. Git history is the changelog; the
documents themselves describe **only the current reality**.

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
   or counter with no named consumer is deleted. (The anti-transcription rule.)
4. **Undecided things are marked `OPEN` with a closure trigger** (the event or milestone
   that forces the decision) and listed below. An OPEN item is a real state; the failure
   mode is code deciding it silently.
5. **When implementation contradicts a doc**, the doc is amended in the same change, or
   the code change doesn't land. Docs describe the system in the present tense.
   **Standing exception, active now:** the docs lead the code — the repo is in its
   documented-broken state until the implementation catches up (the work plan is
   `docs/prd/`), and every gap is a work item, not a doc bug.
6. **No history.** These documents never narrate how the design got here, cite retired
   documents, or describe previous engines. A measured number may appear as rationale
   for a current mechanism ("measured"); a story may not. Anything else lives in git
   history only.

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

- **Every measured claim is unearned**: the oracle stamp, the benchmark ALL-WIN, and
  every pinned denominator are void until derived and run on this engine. *Trigger:
  the implementation reaching the bench milestone.*
- **Recursion** (explicit semi-naive fixpoint): not designed in; no IR decision may
  assume it never comes. The modeling discipline (precomputed closures) has absorbed
  every sighting so far. *Trigger: first real query that needs one.*
- **Ordering/limit conveniences and top-k pushdown**: presentation-layer; results are
  sets, the host sorts. *Trigger: owner pain, or a measured materialize-then-sort
  latency-budget violation.*
- **Declared range/stabbing accelerators**: time-range, point-membership, and
  overlap scans are O(n) by decision; accelerators return only with a benchmark that
  demands them. Candidate mechanism on trigger: guard skip scan (cursor `set_range`
  prefix-hopping over existing `U` namespaces, O(distinct-prefix × log n)) — for
  non-prefix guard lookups and low-cardinality-leading range scans; interval
  stabbing needs the coverage-walk shape instead (`40-execution.md`). *Trigger:
  latency budget violation on a range/interval family.*
- **Dictionary GC**: interned values are never reclaimed — including ids no
  committed fact ever referenced (a no-op insert's interns flush with any
  state-changing commit; accepted leak, `10-data-model.md`). The trigger profile
  is **repeated-text churn only**: the content-churn (digest) population left the
  dictionary entirely with variable `bytes` — `bytes<N>` values are inline — so
  the accepted leak is back to its original compression scope. *Trigger: measured
  dictionary growth — both classes counted — dominating store size on a real
  text-churn workload.*
- **The dictionary contraction**: with the dictionary str-only, the forward key
  hashes raw bytes with no type tag and the reverse entry is the raw bytes — a
  storage-format simplification confirming the deleted type was accidental. A
  *re*-expansion (any second interned type) would resurrect the tag byte and is a
  format change. *Trigger: a real schema surfacing variable-width binary with
  genuine reuse — the recorded reversal condition of the `bytes<N>` cut
  (`10-data-model.md`).*
- **`M`-key width**: membership keys carry the full 32-byte blake3; truncating to
  16 bytes shrinks every `M` key ~40% (B-tree fanout and node count — a real,
  benchmarkable write-path and store-size effect) at the cost of dropping the
  adversarial collision margin from 2¹²⁸ to 2⁶⁴ (accidental stays negligible,
  ~2⁻¹⁰⁵ at the scale axiom). *Trigger: a measured write-path or store-size
  violation attributable to `M`-key width; the decision then weighs the 2⁶⁴
  margin against the number (`10-data-model.md` identity-hash decision).*
- **Incremental image maintenance**: images rebuild whole per state-changing commit
  by design (the write design point amortizes it). *Trigger: traced rebuild cost
  violating the latency budget despite the cache — recorded with D1's reversal.*
- **Text query language**: none; if one returns it is pure sugar lowering to
  statements and IR. *Trigger: owner want.*
- **Vectorized batch size**: 64–256 starting range decided; the number is
  measurement-owned. *Trigger: the ledger benchmark.*
- **EXPLAIN output shape**: ANALYZE semantics shipped; text stability not promised.
  *Remaining open: nothing structural.*
- **`70-api.md` open sub-items**: see that doc's own OPEN list (result ordering,
  multi-key typed `get` sugar, multi-process future).

## Closed by ruling

Each recorded with its rationale in the owning doc; listed here so nothing is
re-litigated by accident:

- **Invariants are two judgments about queries** (functionality, containment);
  *unique / foreign key / primary key / check / exclusion / cascade / restrict /
  trigger / deferrable* are deleted vocabulary (`30-dependencies.md`, `00-product.md`).
- **No sugar** — the schema surface is raw statements (`->`, `<=`, `==`); no
  field-level constraint modifiers, no `union` keyword (the pattern is derived, its
  theorems proved, in `30-dependencies.md`).
- **Interval is the seventh type**, with the point-set denotation; pointwise keys and
  coverage containments as theorems; order operators and Min/Max refused on it; uuid
  rejected with the fresh rationale (`10-data-model.md`).
- **`bytes<N>` replaced variable `bytes`** — the roster stays at seven: intern what
  repeats (`str`), inline what identifies (`bytes<N>`); order operators and Min/Max
  refused on it (a digest's lexicographic order is an encoding artifact); the
  dictionary is str-only and its key hash carries no tag (`10-data-model.md`,
  `50-storage.md`).
- **The IR carries** negation (anti-join atoms with the safety rule), point membership
  (a typing rule), param sets (`IN`), `CountDistinct`, Arg-restriction with
  set-honest ties, and the relation-shaped `Pack` (one row per (group, maximal
  segment) — the coalescing fold); the outer join is a documented decomposition,
  never a node (`20-query-ir.md`).
- **WriteTx point reads** (`contains`/`get` against the delta-overlaid final-state
  view); full queries in write transactions are forbidden (`70-api.md`).
- **The naive model is required infrastructure** — the second oracle, judging
  dependency semantics SQLite cannot express (`60-validation.md`).
- **No prior on-disk format opens; no migration path exists** — ETL is the story
  (`50-storage.md`, `70-api.md`).
- **Non-key and conditional FDs rejected**; partial keys answered by relation splits;
  general FDs answered by normalization (`30-dependencies.md`).
- **Nominal typing rejected everywhere** — hard structural typing; names live in host
  newtypes (`10-data-model.md`).
- **Fresh is a generation attribute**, not a type; it auto-materializes a key FD.
- **Dependency enforcement is commit-time, final-state, only** — no per-operation
  checking, no deferral modes.
- **No `replace` operation** — operation order is semantically irrelevant;
  delete+insert is the idiom.
- **Full-width images, pin-at-prepare plans, one process, zero engine threads,
  intra-query parallelism a non-goal, 64-bit only** (`00-product.md`,
  `40-execution.md`, `50-storage.md`).
