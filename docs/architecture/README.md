# Bumbledb Architecture

These documents are the normative design. They are living documents, updated in place —
there are no work packets or compliance gates. Git history is the changelog; the
documents themselves describe **only the current reality**.

The formal specification lives in `lean/` — a buildable, CI-checked Lean development
that is the only normative home of bumbledb's semantics; these docs cite its theorems
by name and never restate them. The docs' surviving duties are what Lean cannot hold:
mechanism, measurement, decision records, and operations (`lean/README.md` carries the
laws).

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
6. **No history.** These documents never narrate how the design got here, cite retired
   documents, or describe previous engines. A measured number may appear as rationale
   for a current mechanism ("measured"); a story may not. Anything else lives in git
   history only.
7. **The gate law (the covenant).** A change to accepted schemas, query denotation,
   or execution semantics is not done until the Lean tree moves in the same commit;
   the CI lean lane enforces buildability (`scripts/lean.sh`) and the census enforces
   citation integrity (`scripts/spec-census.sh` — every `lean/` citation in these
   docs must resolve to a real declaration). Semantic facts are never restated in
   prose: one intuition sentence, then the theorem name.

## The documents

| Doc | Contents |
|---|---|
| `00-product.md` | Thesis, workload census, hardware, durability, deleted vocabulary, success criteria |
| `10-data-model.md` | Reading guide over `lean/Bumbledb/Values.lean`+`Schema.lean`: the six structural types, interval/ray intuition, identity, schema, modeling discipline — decisions whole, semantics by citation |
| `20-query-ir.md` | Reading guide over `lean/Bumbledb/Query/`: the pure-data IR shape and notation grammar, validation roster, the recursion cut and its fence — decisions whole, semantics by citation |
| `30-dependencies.md` | Reading guide over `lean/Bumbledb/Dependencies.lean`+`Cardinality.lean`+`Txn.lean`: the three statement forms by citation, statement grammar, the acceptance gate, enforcement mechanism, the decidability firewall |
| `40-execution.md` | Mechanism only: access paths, Free Join over COLT, anti-probes, planner, vectorization, allocation — every semantic sentence cites its `lean/Bumbledb/Exec/` theorem |
| `50-storage.md` | Mechanism only: LMDB layout, determinant namespaces as judgment accelerators, the delta write path, images — encoding laws by citation |
| `60-validation.md` | The three oracles (SQLite + naive model + the Lean denotation, `lean/conformance/`), ledger benchmark protocol, test families |
| `70-api.md` | Embedding surface: the schema! grammar, transactions (semantics by `lean/Bumbledb/Txn.lean` citation), point reads, results, ETL |
| `../cookbook.md` | The cookbook — modeling intuition as worked schemas; illustrative, never normative; `Guarantee:` labels cite `lean/` theorems, census-checked (reader: the owner and any agent writing a theory) |
| `../../lean/` | The specification itself: the value universe through the lifecycle, `Bridge.lean` (the obligation ledger), `Countermodels.lean` (the design scratchpad), the conformance corpus — `lean/README.md` carries the laws and the provenance history |

## OPEN items

- **Every measured claim is unearned**: the oracle stamp, the benchmark ALL-WIN, and
  every pinned denominator are void until derived and run on this engine. *Trigger:
  the implementation reaching the bench milestone.*
- **The chain-window class** (interval intersection along paths — "the
  window over which an entire path holds"): outside the landed recursion
  surface by the safety theorem's premise, because the intersected window
  is a *created* head value (`20-query-ir.md` § engine recursion, the
  chain-window fence — the lattice-closure termination sketch is recorded
  there, beside what keeps it open). The closure idiom carries the window
  in the host's frontier meanwhile (`../cookbook.md` recipe 24). *Trigger:
  a real workload dominated by interval-intersection-along-paths — it
  re-opens theory before engineering.*
- **Ordering/limit conveniences and top-k pushdown**: presentation-layer; results are
  sets, the host sorts. *Trigger: owner pain, or a measured materialize-then-sort
  latency-budget violation.*
- **Declared range/stabbing accelerators**: time-range, point-membership, and
  overlap scans are O(n) by decision; accelerators return only with a benchmark that
  demands them. Candidate mechanism on trigger: determinant skip scan (cursor `set_range`
  prefix-hopping over existing `U` namespaces, O(distinct-prefix × log n)) — for
  non-prefix determinant lookups and low-cardinality-leading range scans; interval
  stabbing needs the coverage-walk shape instead (`40-execution.md`). *Trigger:
  latency budget violation on a range/interval family.*
- **Grounding interval-pair elimination**: pointwise coverage proves that covering facts
  exist, not that an interval pair is equal and joinable, so interval-typed statement
  positions refuse grounding elimination. *Trigger: a census-style query that would
  benefit from interval-pair elimination (`40-execution.md` § the grounding).*
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
- **Vectorized batch size**: 64–256 starting range decided; the number is
  measurement-owned. *Trigger: the ledger benchmark.*
- **Unit-slot determinant halving**: a fixed-width interval position stores one
  word, so `interval<E, 1>` sidecar facts and their pointwise-key determinants
  halve against the general spelling — a candidate store-size/write-path win,
  measurement-owned. *Trigger: the ledger benchmark.*
- **`70-api.md` open sub-items**: see that doc's own OPEN list (result ordering,
  multi-key typed `get` sugar, multi-process future).

## Closed by ruling

Each recorded with its rationale in the owning doc; listed here so nothing is
re-litigated by accident:

- **Invariants are statements about queries** (functionality, containment, the
  cardinality window);
  *unique / referential / primary key / check / exclusion / cascade / restrict /
  trigger / deferrable* are deleted vocabulary (`30-dependencies.md`, `00-product.md`).
- **No sugar** — the schema surface is raw statements (`->`, `<=`, `==`); no
  field-level constraint modifiers, no `union` keyword (the pattern is derived, its
  theorems proved, in `30-dependencies.md`).
- **Interval is the last type**, with the point-set denotation; pointwise keys and
  coverage containments as theorems; order operators and Min/Max refused on it; uuid
  rejected with the fresh rationale (`10-data-model.md`).
- **`bytes<N>` replaced variable `bytes`** — and the enum died into the closed
  relation, leaving six pure value types: intern what
  repeats (`str`), inline what identifies (`bytes<N>`); order operators and Min/Max
  refused on it (a digest's lexicographic order is an encoding artifact); the
  dictionary is str-only and its key hash carries no tag (`10-data-model.md`,
  `50-storage.md`).
- **The IR carries** negation (anti-join atoms with the safety rule), point membership
  (a typing rule), param sets (`IN`), `CountDistinct`, Arg-restriction with
  set-honest ties, and the relation-shaped `Pack` (one answer per (group, maximal
  segment) — the coalescing fold); the outer join is a documented decomposition,
  never a node (`20-query-ir.md`).
- **The query surface is the IR, permanently — pure data** (the text-language OPEN
  item, closed by the sharper ruling): no builder, macro, or text syntax in the
  engine, ever; sugar is downstream territory in any language, lowering to IR; the
  `schema!` grammar is open-ended with one categorical boundary — the macro speaks
  the theory language, never the query language; the IR-validation path is a trust
  boundary (no panic reachable from IR data, adversarially swept); `ir::render` is
  the read-side syntax (`20-query-ir.md`, `70-api.md`).
- **WriteTx point reads** (`contains`/`get` against the delta-overlaid final-state
  view); full queries in write transactions are forbidden (`70-api.md`).
- **Plan introspection output** is the versioned `introspection v3` contract:
  deterministic content and ordering within a version, with rendered and structured
  surfaces incremented together (`40-execution.md`, `70-api.md`).
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
- **The engine judges satisfaction, never implication** — consequence among
  statements is a compiled witness, a conservative optimization, or diagnostics;
  never a required procedure (`30-dependencies.md`, the decidability firewall).
- **Statements quantify over stored relations, permanently** — no predicate
  vocabulary in the statement language, before or after recursion
  (`30-dependencies.md`).
- **A created value never re-enters a derivation** — heads bind, filters compare,
  folds create at the answer boundary only; future interval operators must be
  lattice-closed (`20-query-ir.md` § the creation quarantine).
- **Queries stay query-shaped** — the caps are product decisions; no rule-program
  runtime, no stored rules, no magic sets; a deductive database is a named
  non-goal (`20-query-ir.md` § engine recursion, `00-product.md`).
