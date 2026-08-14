# The feature register — whispers, verdicts, triggers

The durable record of every feature the workload has whispered for, each
investigated to a verdict (full reports in `docs/research/`). The law of this
document: a DEFERRED or REJECTED feature carries a RECORDED TRIGGER — a
concrete, observable condition under which the question reopens — so nothing
is relitigated from vibes and nothing worthy is forgotten. Investigated
2026-07-19 against primer's real workload (50 prepared queries, the store
schema, the host-gate census); the notation verdicts (5–6) recorded
2026-07-20 from the destructure-060 owner ruling; verdicts 7–14 recorded
2026-07-23 from the audit campaign (git history); verdicts 15–16 recorded 2026-07-24 from the
capacity rulings (`docs/design/capacity-laws.md` §8 ruling 4 and §8b C19);
verdict 17 recorded 2026-07-25 at the capacity campaign close; verdict 18
recorded 2026-07-25 (the temporal-capacity trigger, the family's known
next-but-one rung).

## Verdicts

### 1. Aggregate comparisons (the HAVING shape) — DEFER, weak form only
- **Strong form (interior HAVING — an aggregate feeding another rule):
  REJECTED.** Reverses the creation-quarantine ruling verbatim ("a created
  value never re-enters a derivation"); refused today by name
  (`AggregateInInterior`; measure finds likewise `MeasureInInterior`) — folds and measure finds are legal only at the main head. Not a feature — a doctrine reversal.
- **Weak form (filter an output head's fold before emit): FEASIBLE AND
  CHEAP** — head-level `having` list in the IR, one word-compare before
  `AggregateSink::finalize_into`, no change to `NegationInRec` / the one linear rec, no new Lean
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
  The real readers are serverless; `00-product.md` cuts against the shape.
  A pipeline would trade an ACID invariant for seven new failure modes.
  (Three 2026-07-19 rationale legs — darwin-only shipping, no read-only
  open, the multi-process non-goal — are dead by ruling: the lock law is a
  writer law and readers open lockless and genuinely read-only (ruled
  2026-07-23, R17), and a linux SDK CI lane is law (ruled 2026-07-23, R22).
  The verdict now rests on the serverless topology and the product shape.)
- **TRIGGERS (all must hold)**: a long-lived linux worker in primer's
  topology; a shipped, tested linux platform package (the CI half is law —
  the linux SDK lane, ruled 2026-07-23, R22); a graph workload too big
  or write-hot to materialize. The fourth 2026-07-19 leg — a genuine
  read-only/multi-process open — FIRED by ruling (R17 above); the verdict
  stands until the remaining legs hold.

### 5. Tagged-template query notation — REJECT (owner ruling 2026-07-20)
- **The tagged-template notation — a template-literal query string parsed at
  the type level — is REJECTED by direct owner ruling, 2026-07-20**, during
  the destructure-060 (0.6.0) notation decision. The owner's words, verbatim:
  "the type-level string feels like a lie". The type-level parse re-derives
  what TypeScript's own binding constructs already carry natively; a string
  that pretends to be a type is the wrong representation when real values with
  real types are available (the destructured mint, verdict 6, is what shipped
  instead).
- **The ramp stays.** The conformance corpus
  (`crates/bumbledb-query/tests/notation-corpus/`, 27 Rust⇄TS Query IR JSON
  cases) remains in-tree and available — feasibility was never the question,
  and the corpus is not deleted.
- **TRIGGER**: only a direct owner reversal reopens it. No workload
  observation, census, or whisper can — this was a taste ruling by the owner,
  and it is relitigated only by the owner. This is the register's one entry
  whose trigger is deliberately NOT an observable condition; it is stated so
  explicitly here so the asymmetry reads as intended, not as an omission.

### 6. Destructured variable mint (vars become values) — RULED AND ADOPTED, ships as 0.6.0 (owner ruling 2026-07-20)
- **The mint, adopted.** `v(relation)` mints a record of fresh query
  variables, one per column, each typed at mint by the column's law-computed
  class — a concrete mapped type over the relation's statically-known columns,
  so ES destructuring (`const { id, toGrp } = v(candidateEdge)`) preserves
  every literal and every class. Variable identity moves from name to an
  OBJECT REFERENCE: reusing the same var value across binding positions IS the
  join, and name-collision joins become unrepresentable. `select(strings)`
  dies into
  `find({ key: varOrAgg })` — the find object's keys name the result row,
  fully typed. `r.var` dies with no shim. Params stay string-named
  (`r.param`/`r.inSet`): their names are execute()'s runtime
  params-object keys — an honest load-bearing channel, not a lie.
- **The rationale (the reason, not decoration).** This is the TRUE UNION —
  TypeScript's own binding constructs carrying the calculus's classes — and
  hygienic imperative composition: each `v()` call mints a fresh batch, so
  composed rule fragments cannot capture each other's variables by accidental
  name collision.
- **The parity law (a criterion, not a hope).** Semantic parity IS LAW: the
  IR/VarId theory is UNCHANGED — lowering assigns VarIds from reference
  identity in deterministic first-use order — and the Rust macro, the wire,
  the manifest, and the fingerprints are untouched. Zero fingerprint pins
  move: `fixtures/cookbook-fingerprints.txt` is byte-identical across
  the break.
- **Status**: ships as 0.6.0, a deliberate hard break; version staged in
  lockstep, NO tag, NO publish (owner ceremony).

### 7. Measure-keyed Arg restriction — WITHDRAWN
ArgMax/ArgMin (including the measure-keyed R5 face) are killed. Remaining
folds: Count, Sum, Min, Max, Pack. Hosts that want "the row at max(key)"
compose Max then keyed get.

### 7 (historical). Measure-keyed Arg restriction — RULED IN then withdrawn (ruled 2026-07-23, R5)
- **The law.** The `ArgMax`/`ArgMin` key position admits the interval
  measure: "the longest interval per group, with its carried payload" is
  spellable on every surface — IR, validation, sink, both macro grammars,
  and TS (`argMax`/`argMin` gain the `Duration` key arm). Lands inside the
  same aggregate-law revision as R1–R3 — the aggregate spec reopens exactly
  once, and `docs/architecture/20-query-ir.md`'s "exhaustively" pinned
  position roster becomes true again.
- **Why it was the gap.** Every ingredient was already paid for — the
  measure folds under `Sum`/`Min`/`Max`, projects as a find, compares on one
  side of an order, and the aggregate sink already mints per-binding derived
  measure words and keys the Arg sweep on a scratch-row slot word. The key
  position was the one hole, and it was pinned by omission (no trigger, no
  decision) — the only Arg-family unspellable with neither a spelling nor a
  citation. Full census in finding 118.

### 8. Condition trees in the Rust text notation — RULED IN (ruled 2026-07-23, R9)
- **The law.** `or()` and `and()` condition trees enter the sacred `query!`
  grammar as an exact mirror of the TS grammar — one condition language, two
  identical surfaces, one renderer. The renderer's functional `and(..)` /
  `or(..)` spelling becomes real notation, and the render→parse round trip
  closes over the full input grammar (today it holds only because the
  notation refuses to spell what the IR accepts — finding 129).
- **Sequencing.** Lands with/after R2 (the or-transparency lowering fix) so
  the Rust surface never ships the leaky lowering.

### 9. abandon() honored in db.write — RULED IN (ruled 2026-07-23, R10)
- **The law.** Returning `abandon(payload)` from a `db.write` callback rolls
  the transaction back; `WriteResult` widens to a sum carrying
  commit-vs-abandon — the outcome is in the type, and commit is unreachable
  for a sentinel result. The silent-commit path (the callback's explicit
  decline typechecks under the void-return rule and commits anyway —
  finding 060) is unrepresentable.

### 10. Tx.insert returns the changed bit — RULED IN (ruled 2026-07-23, R11)
- **The law.** `Tx.insert` returns `{ changed, ...fresh }` — the engine's
  changed-state boolean already crosses the FFI on every call; the SDK stops
  discarding it. Restores the Rust-surface bijection that `delete` already
  honors and kills the contains-before-insert double FFI crossing in the
  idempotent-replay lane (finding 061).

### 11. Resource lifetimes are disposables — RULED IN (ruled 2026-07-23, R12)
- **The law.** The SDK assumes the latest Node 26 runtime and its explicit
  resource management. `ExhumeHandle` implements
  `Symbol.dispose`/`Symbol.asyncDispose` (whichever matches teardown
  reality); `using` / `await using` is the documented idiom; the congruence
  audit extends the protocol to every SDK object holding a native lifetime
  (exhume, snapshots/scoped reads). The zero-closables doctrine restates as:
  lifetimes are disposables, never `close()`. Kills the GC-held exclusive
  lock — same-path reuse hostage to an unforceable finalizer (finding 066).

### 12. TS explain() — WITHDRAWN as embedding API (was R13)
- Plan diagnostics (`explain` / `snap.introspect` / `prepared.staleness` /
  public `ExecutionStats`) are not host-facing SDK API. The bench harness
  may still introspect via crate-visible / `#[doc(hidden)]` paths.
  ANALYZE-grade profiling stays engine-internal.

### 13. Closed-column const accessors — RULED IN (ruled 2026-07-23, R14)
- **The law.** Closed-relation column values are expansion-time constants;
  the `schema!` macro emits `const` accessors on the host enums, rendered
  from the same parsed literals that seed the engine's extension — host and
  engine cannot drift by construction, and the emitted weld stays db-free.
  The runtime-query workaround (and its silently-drifting hand-written twin)
  dies (finding 125).

### 14. Estimator precision (histograms/statistics) — DEFER re-affirmed (ruled 2026-07-23, R19)
- **The doctrine.** Estimates stay crude; adaptivity is the doctrine. The P3
  "no histograms" ruling stands on new grounds: the Free Join thesis places
  precision at execution time — 009's GJ-shaped plans plus dynamic cover
  choice bound skew at runtime. (The overstated "WCOJ bounds the damage"
  claim in `docs/architecture/40-execution.md` is corrected in the same
  flush — true only near the GJ end, which 009 makes real.)
- **TRIGGER**: post-009 benches showing plan-choice misses the dynamic cover
  choice can't absorb — a measured mis-chosen order whose damage survives
  execution-time adaptivity. Finding 089 (the ring-closing min-fanout
  composition) is the recorded exhibit; it reopens only through this
  trigger.

### 15. Min/Max in capacity law position — REFUSED (ruled 2026-07-24, §8 ruling 4)
- The capacity statement's law-position roster is `Count` and `Sum`
  (field / Duration) — the polarity-clean folds: an upper bound is newly
  violable only by inserts into the weighed side, a floor only by deletes,
  which is what keeps the delta-scoped judgment sound. `Min`/`Max` windows
  break that polarity (a delete can move the extremum either way — the
  whole group re-derives), so the spelling is a typed refusal, not a gap.
- Path weights (`[a.b]`) carry NO register entry — that refusal is a
  boundary, not a deferral (§8 ruling 6: the composition idiom is the
  answer; recording a trigger would re-open the cached-truth drift class
  by appointment).
- **TRIGGER**: a censused workload demanding an extremal per-group law —
  and then it enters by inhabiting `AdmissibleForm` first, like every form.

### 16. Balance laws (aggregate-vs-aggregate windows) — RECORDED, deliberately unbuilt (ruled 2026-07-24, C19)
- The known next rung of the capacity generalization:
  `Sum(debits) == Sum(credits)` per transaction — an aggregate bounded by an
  aggregate rather than a window. Deliberately unbuilt: no shape ships
  before a host demands it.
- **TRIGGER**: a real host asking for a balance constraint.

### 17. Capacity laws (the weighted aggregate containment) — RULED AND ADOPTED, ships as 0.8.0 (ruled 2026-07-24, landed 2026-07-25)
- **The statement.** `Target <=[w]{lo..hi} Source` — the cardinality window
  died into the aggregate containment: per ψ-selected target fact the
  group's measure (Σ weight over deduplicated φ-selected source facts) lies
  in the window; absent bracket = unit weight, so the `<={lo..hi}` count
  utterance survived character-for-character as the unit instance. Weights
  are `[field]` / `[Duration(field)]` — calendar capacity rides the R5
  machinery free; dependent bounds read the hi slot from the target row
  (C1/C6); path weights refuse typed naming the pinned-column composition
  idiom (§8 ruling 6).   The design is `docs/design/capacity-laws.md`
  (§8 rulings 1–6, §8b C1–C19), stamped LANDED at close.
- **The hard cutover, executed:** FORMAT_VERSION 7 refuses every pre-cutover
  store on every open lane; the fingerprint mints statement-form tag 4 under
  the v5 encoding label (C5 — tag 3 never reissues) so every schema
  fingerprint moved; the conformance corpus re-baselined whole
  (`judgment-capacity-*`, C8) and the generators mint the surface day one
  (C13); the word "cardinality" survives nowhere in the mechanism (§8
  ruling 5) — the zero-trace gate ran green at campaign close.
- **The C17 measurement, DECIDED (2026-08-01):** the power-budget lane ran
  both `measure_children` arms (same protocol, each arm oracle-verified
  before timing) and the SLOT arm won every weighted row — the value slot
  carries the child's u64 weight (LE), paid once at write time
  (`commit_capacity_sum` 32.3 vs 35.2 µs, `commit_capacity_duration` 30.8
  vs 34.2 µs, min-of-3 ephemeral p50s; control 18.2 both arms). The fetch
  arm and the `CAPACITY_WEIGHT_SLOT` flag are deleted per C17's own law;
  the numbers are the CONSTRAINT comment at the walk
  (`crates/bumbledb/src/storage/commit/judgment.rs`). **OWNER RULING OWED:** the
  slot arm's recorded corner is now live — a ray Duration weight refuses at
  WRITE time, strictly stronger than C10's judge-time refusal, visible only
  for a ray child under an absent parent (documented at the constraint).
- **Reopen triggers:** none here — verdicts 15 (Min/Max in law position),
  16 (balance laws), and 18 (temporal capacity) carry the family's recorded
  triggers.

### 18. Temporal capacity laws (capacity at every instant) — RECORDED, deliberately unbuilt (recorded 2026-07-25)
- Capacity at every instant — the window bounds the stabbing-set measure
  per instant of the source's span field (k-concurrency, time-varying
  weighted capacity, coverage floors). Mechanism already understood:
  half-open discipline collapses ∀instant to a finite boundary sweep; 1-D
  Helly makes ≤k-concurrent ⟺ no (k+1)-clique under INTERSECTS; the
  order-based per-key overlap index (finding 012) is the delta-scoped
  judge's walk (an insert first violates only inside its own span);
  polarity survives (concurrency ceilings insert-violated, coverage floors
  delete/shrink-violated).
- **TRIGGER**: a real host asking for a concurrency or coverage law.
- Notation reserved: the stabbing dimension marks the source's span field,
  operator-style.

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
- An `Interior` atom is a join position (the re-grounding tax) — engine law,
  documented, ~6 recursive queries carry one extra `.match`.
- Keyed-get/typed lookup for task-by-(kind, subject) — the anyOf
  investigation's "what primer actually needs" aside; smallest of the set
  (shipped: keyed get, 70-api ledger row (b)) — the fold is now expressible
  as a point read.

## FIRED and scheduled (the owner's prioritization, 2026-07-19)

Two OPEN-ledger rows in `docs/architecture/70-api.md` whose triggers already
FIRED got their own wave (the surface-pair wave) AFTER cleanup-0.5.0 landed
and BEFORE any 1.0.0 surface freeze (they are surface additions; they belong
under the tag) — both rows shipped 2026-07-19:

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
- **Answer ordering/limit conveniences** — **WITHDRAWN as engine/SDK
  surface**: the engine never orders; hosts sort with the language's own
  comparators. Limit remains `.slice` / `truncate` / `take`. Engine-side
  top-k is refused.

## Also parked elsewhere (cross-references)

- The deletion-vector/mask fork: **REFUTED BY MEASUREMENT** (the decider
  twin, B/A 1.20–1.24 — see the incremental-images ruling record in
  `docs/architecture/`); compact-on-delete is law.
- The O(delta) slab append: scoped out with invariant language recorded
  (copy-on-append ruling record).
- The per-store map size parameter: recorded follow-up design (G1), only if
  a real ephemeral capacity need appears.
- The tagged-template query notation: REJECTED, verdict 5 above (owner ruling
  2026-07-20); the conformance corpus
  (`crates/bumbledb-query/tests/notation-corpus/`) stays as the ramp.
