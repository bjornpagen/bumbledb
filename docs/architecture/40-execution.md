# 40 — Execution

The execution engine is Free Join as specified in the paper (Wang, Willsey, Suciu,
SIGMOD 2023 — `docs/free-join-paper/`), run over snapshot-local columnar data, with
documented deviations. When this doc and the paper disagree and no `Deviation:` block
explains why, this doc is wrong.

## The staging law

**Every computation runs at the earliest stage where its inputs are fixed.** The
engine is a seven-stage evaluator, and each stage owns exactly the work whose
inputs fixed there — the ladder the comptime pass implemented end to end:

1. **Expansion** (the macros): names resolve to declaration-order ids, handles to
   row ids, the theory to descriptors — everything the source text fixes.
2. **Open** (`Db::create`/`open`): the schema validates and seals — closed
   extensions encode once into sealed ground axioms, σ-literal checks compile
   (`CompiledCheck`), statements into closed relations compile to their member
   word-sets (`Resolved::ClosedContainment` — the enforcement plan IS the answer
   set), the fingerprint pins it all.
3. **Prepare**: normalization, the grounding (elimination and evaluation — closed
   atoms fold against their sealed extensions into plan-constant handle sets),
   subsumption, the DP, and the statistics pin. A prepared plan is the theory's
   judgment of the query, not the data's.
4. **Bind**: params resolve to encoded words; `str` literals latch monotonically
   (the literal latch — resolution is a stage-4 event that never re-runs once
   fixed).
5. **Generation**: images build once per `(relation, storage_tx_id)` and are
   shared; a closed relation's image is synthesized once per process at the
   sentinel generation — the theory is its own generation.
6. **Execute**: the join, over data every earlier stage already fixed — no
   interpretation, no name resolution, no encoding work per row.
7. **Commit**: the judgments, against the final state, reading flags and sealed
   bytes computed at their own stages — never re-deriving them.

**The boundary clause, constitutional: folding produces data, never code.** Plans,
id-sets, masks, word-sets, latched words — consumed by fixed, asm-gated, measured
kernels. No JIT, ever: runtime-generated code cannot be audited by the disassembly
gates or pinned by the fact ledger (§ the staging law records the
refusal with its derivation).

**Pins acknowledge; they never re-fix inputs.** A later stage may *record* that an
earlier stage's output has drifted (a plan's pinned statistics against moved data —
`PreparedQuery::staleness`, the pull-based signal), but no stage reaches back and
re-runs an earlier one implicitly: plans are never invalidated by writes, re-prepare
is the caller's explicit act, and the fresh→FD materialization order stays where the
fingerprint fixed it. Staleness is acknowledged honestly, inputs are fixed exactly
once.

## Access paths (before any join machinery)

**Key-probe point lookups.** A single-atom query whose bindings cover a key of the
relation (an FD statement, including the auto-key on fresh fields —
`30-dependencies.md`) or the full fact executes as one point read: the first
fresh field's auto-key resolves as **one direct `F` get** — the fresh value IS
the `F` row id and that auto-key maintains no `U` tree, so the probe pays one
B-tree descent, not two (`50-storage.md` § key layout; ruled 2026-07-23, R16);
every other declared key (later fresh fields' auto-keys included) pays one `U`
determinant LMDB get → one `F` fetch, and the full fact one `M`-membership
get → one `F` fetch — then decode → the residual per-field filters applied to the
one hit. No images, no COLT, no plan search — and for the statement-key
(`U`-determinant) arm the one get computes exactly the rule's join denotation,
residual filters included
(`lean/Bumbledb/Exec/Rewrites.lean: keyprobe_equiv_join` — the theorem models
that arm; the full-fact `M`-membership arm is whole-fact identity below the
model, the recorded narrowing in that module's doc); an interval-final
pointwise key closes the same uniqueness premise
(`lean/Bumbledb/Exec/Rewrites.lean: keyprobe_pointwise_key_spent`). This serves the
headline "point lookup by key" workload at O(log n), including immediately after a
commit (no rebuild cost).
**Decision.** **Alternative:** COLT-only ("the join engine is the only read path") —
lost because a fully-bound lookup through images pays an O(n) scan for a one-answer result
and loses the benchmark family outright; the paper itself lists index-blindness as an
open limitation (§6). **Reverses if:** never — the determinants exist anyway (rule: every
mechanism names its reader; this is `U`/`M`'s read-side reader).

**Statically empty programs.** A rule the normalization fold refuted on
constants (`20-query-ir.md`, § normalization) is deleted at prepare — sound on
every instance, the verdict never consulted one
(`lean/Bumbledb/Exec/Rewrites.lean: statically_empty_sound`); a program whose
every rule dies prepares to the empty program. Prepared
execution has two rule kinds — key probe and Free Join — plus this
program-level empty variant. Execution binds params first — bind errors still surface, a
vacuous Allen mask param is rejected exactly as on a live plan — then
touches no images, binds no views, runs no join, and the result is the
empty buffer. Plan introspection prints `access path: statically empty` plus each dead
rule's killing condition; a dead rule inside a live program was deleted at
prepare and its record prints the same way.

**Time-range scans, point-membership scans, and interval-overlap joins are O(n)**
(image scan + filter) in v0 — decided; acceptability is policed by the latency budget
(`00-product.md`), and the range-accelerator OPEN item (which now covers interval
stabbing — "which intervals contain t" — alongside scalar ranges) triggers on
violation. One degenerate named honestly: a membership or overlap join whose
interval occurrence shares **no equality variable** with the rest of the query is a
Cartesian with a filter — O(bindings × n), like any Cartesian, and only a stabbing
structure could do better. Real interval workloads carry their group key
(per-account, per-room); the randomized generator bounds itself to that shape
(`60-validation.md`). Candidate mechanism recorded for trigger day: **determinant skip
scan** — `U` determinants are already ordered composite keys of fixed per-statement
width, so a non-prefix determinant lookup or a range scan under a low-cardinality
leading field (closed-reference discriminators) is servable with zero new structures by
cursor `set_range` prefix-hopping (O(distinct-leading-prefixes × log n)); not
applicable to interval stabbing, whose pointwise layout needs the coverage-walk
shape. Interval membership predicates lower to word comparisons over the
start/end column pair (`50-storage.md` image layout), and interval-pair
predicates classify through the configuration kernel (§ vectorized
execution — masks, not ops); both are the existing 8-byte shapes and no
new NEON widths exist. The membership kernel (`s ≤ t < e`, two unsigned
word compares) needs no ray awareness: a ray's end is just the largest end word
(`lean/Bumbledb/Values.lean: ray_is_unbounded_tail`), and validation has already
rejected `t = MAX` (the ceiling is not a point), so the kernel never sees it.

Everything else executes as Free Join.

## Inputs from normalization

Execution consumes `20-query-ir.md`'s normalized form: distinct-variable positive
atom occurrences, per-atom filter lists, a residual comparison list, and **anti-probe
filters** (lowered negated atoms).

- **Per-atom filters** evaluate at the source: a query-local **filtered view** — a
  survivor-position vector over the cached full-width image, arena-backed. On a cold
  relation, one scan produces both the cached unfiltered image and the survivor view
  (`50-storage.md`). COLT roots iterate the view; view positions index the image.
  Membership bindings against literals/params and interval predicates land here as
  two-word range filters.
- **Residual comparisons** attach to the earliest plan node at which both sides are
  bound (computed at plan time, stored in the plan). The executor's node loop gains one
  step: iterate cover → **evaluate the node's residuals** → probe siblings → enqueue
  survivors for the next node.
  In vectorized execution, residuals run as batch survivor compaction **before** the
  sibling probes — the cost-class ordering: residual operands read only cover batch
  words and already-bound outer slots, and sibling probes bind no variables, so the
  pure-ALU rejection legally precedes the memory-bound hash probes and every probe a
  residual kills is a bucket load never issued (finding 008: the ring lanes' ~0.6%-
  selective temporal filters bought nothing behind the probes; re-pin the ring A/B
  under the night-run protocol to record the recovered share). Point-membership scans
  and anti-probes stay after the probes — they are probe-class work and may read the
  pass's own sibling children.
- **Anti-probe filters** attach exactly as residuals do — to the earliest node at
  which every variable of the negated atom is bound — and evaluate as: probe the
  negated occurrence for any matching fact; a hit **rejects** the binding. The
  negated occurrence is never a cover, contributes no plan variables, and its COLT
  (or, when its bindings cover a key, its `U`/`M` determinant index — the same access-path
  hierarchy as positive lookups) is forced only to the levels the probe needs.
  In batches, anti-probe misses are survivors and hits are compacted away —
  branchless, identical machinery to residual failure. **This probe is the same
  primitive the commit-time judgment checker runs** (`50-storage.md` step 3): "no
  fact matches" implemented once, called by two owners.

**Deviation (paper §2):** the paper assumes selections pre-pushed to base tables and has
no residual concept; we own filter placement because there is no external optimizer.
**Reverses if:** never — WLOG assumption, not a design.

## The paper's core, adopted

- **GHT** (§3.1): trie; internal nodes are hash maps keyed by tuples; leaves are vectors.
- **Plan** (§3.2): a list of nodes, each a list of subatoms; a valid plan
  partitions every positive occurrence's variables and its covers bind exactly
  each node's new variables — the validity clauses are the definition
  `lean/Bumbledb/Exec/Plan.lean: PlanValid` (quantified over **occurrences**, so
  self-joins are ordinary), and the theorem validity buys: any valid plan of an
  accepted rule computes the rule's denotation, negations and residuals as
  post-filters, `lean/Bumbledb/Exec/Plan.lean: valid_plan_sound`.
  **Deviation from the paper's Definition** ("containing all new variables"): a
  subatom that also carries an already-bound variable is iterable per the paper, but
  under dynamic cover choice the executor would *rebind* the bound variable without
  re-checking the occurrence that bound it — wrong results on skewed data (found by
  audit, demonstrated with a triangle query, pinned by a regression test, and
  mechanized: `lean/Bumbledb/Countermodels.lean: loose_cover_rebinds` executes the
  paper's rule — `lean/Bumbledb/Exec/Plan.lean: PaperPlanValid`, the strictly wider
  admission per `lean/Bumbledb/Exec/Plan.lean: PlanValid.paper` — on that triangle
  instance and derives the tuple the denotation refuses). Restricting
  covers to exactly-the-new-variables loses nothing: every rule keeps at least
  one valid plan
  (`lean/Bumbledb/Exec/Plan.lean: every_rule_plannable` — the left-deep
  one-variable-per-node construction), and every `binary2fj` node's opening
  subatom qualifies (its variables are exactly the remainder). The alternative — equality-checking a mixed
  cover's old variables per iterated entry — buys generality no plan shape here needs.
- **Execution** (§3.3 vocabulary, pipelined implementation): the root or an absorb
  node supplies pending binding tuples plus carried cursor sets. Each middle node
  `pump`s those tuples, chooses a cover per parent entry, and `probe_pass` batches
  sibling probes across parents: hash, prefetch, load, then branchlessly compact
  survivors. Survivors become the next node's pending tuples; the leaf runs its batch
  paths and emits complete bindings to the sink. D2 suffix skips cancel the origin
  below the absorb node instead of unwinding a call stack.

  **The zero-arity cover collapses to one entry.** A zero-binding nonemptiness gate
  (a positive atom with no variables) reaches the executor as a zero-arity cover —
  every position yields the same empty key tuple, so under set semantics one entry
  stands for the whole suffix and `pump`/`run_node` stop after a single yield.
  Enumerating instead multiplies the join by the gate relation's fact count for zero
  distinguishable bindings; a projection's D2 first-emit skip masked that, an
  aggregate (never skips, and a gate defeats the distinct-bindings elision, so the
  seen-set runs) folded |join| × |gate| duplicate bindings — the S-scale crucible
  hang, pinned by `zero_binding_gate_yields_one_entry_not_the_relation`. The one
  consumer that can distinguish a zero-arity cover's positions is a membership probe
  reading that occurrence's cursor (each position carries its own interval columns):
  membership-probed occurrences keep enumerating.

  **Deviation (paper §3.3):** the paper presents per-tuple recursive descent with
  backtracking. BumbleDB accumulates work across node entries in the pipeline above
  so deep nodes receive full batches. Origin cancellation is sound because a late
  cancellation can only re-emit an answer already held by the spanning seen-set:
  cancellation skips work but cannot change the result under set semantics. The
  paper's cross-node-entry-accumulation caveat is therefore retired, not pending.
- **COLT** (§4.2): lazy tries — a node is offsets into the base columns or a forced
  map; roots iterate the base image (or filtered view) directly; forcing happens only
  on `get` or non-suffix `iter`. Under laziness the paper's build-phase "drop the
  trailing []" question dissolves: nothing is ever built eagerly; a last-level subatom
  that is only ever suffix-iterated is never forced by construction, and one that gets
  probed forces like any level.
- **Dynamic cover choice** (§4.4): at node entry, iterate the cover with the fewest
  keys; forced maps expose `Exact(n)`, unforced vectors `Estimate(len)` — an
  `Estimate` is duplicate-inflated by construction, but both labels are admissible
  bounds on iteration cost. v0 rule, **magnitude-first**: the smaller magnitude wins
  regardless of label; on a tie `Exact` wins (it cannot shrink); a full tie keeps the
  lowest subatom index (deterministic). A label-first rule ("an Exact
  always displaces an Estimate") iterated a 500-key forced map while a 7-fact
  param-filtered view sat unforced beside it — the measured balance wrong-cover.
- **`binary2fj` + conservative `factor()`** (§4.1): the paper's construction over the
  DP planner's left-deep output, with one correction required by its own worked
  example. Figure 8's literal pseudocode would visit the cover first, fail
  `α.vars ⊆ avs(φ)`, and abandon the node; the engine starts candidates at index 1,
  matching the prose intent that factors are drawn from the cover's siblings.

**`ValidatedPlan` contents** (the witness type execution trusts): atom occurrences with
field→column maps; the node list with subatom partitions; per-node cover sets; per-
occurrence trie schemas derived per §3.3; per-node residual **and anti-probe** lists;
per-atom filter lists; the binding-slot layout (below); and the optional
`DistinctWitness` (below). Validated once at construction; nothing
downstream re-checks — validity is what the soundness theorem takes as its premise
(`lean/Bumbledb/Exec/Plan.lean: valid_plan_sound`), so the witness carries the whole
licence.

## Set semantics in the executor

Bindings are **VarId-indexed slot arrays**. Each pipeline row carries the ancestor
slot values needed below it; leaf batches provide varying cover words beside those
outer slots, and sinks read both through one slot layout. Plan variable order is
therefore irrelevant to sinks. Slots are
words: an interval variable occupies two consecutive slots (start, end) and a
`bytes<N>` variable its ⌈N/8⌉ padded words — multi-word values enter seen-sets,
group keys, and probe keys as word tuples, the wordmap's native shape, and every
consumer walks widths rather than assuming one.

Duplicates must collapse before folding — the seen-set fold over the emitted
stream computes exactly the answer set
(`lean/Bumbledb/Exec/Dedup.lean: seenfold_is_set_semantics`):

- The **projection sink** dedups projected answers (its job anyway).
- The **aggregate sink** folds a binding only on first occurrence, using a seen-set of
  full binding tuples — the same arena-backed mechanism as projection dedup.
- **`CountDistinct`** folds through a per-group distinct-value set (one word per
  value — intern ids, encoded scalars, or interval words pairwise), arena-backed
  like the group map.
- **Arg-restriction (`ArgMax`/`ArgMin`)** is a group-state fold, not a
  post-materialization pass: per group the sink keeps the current extreme key and
  the set of surviving projected answers; a strictly-better key clears the set, an
  equal key inserts (ties are set-honest —
  `lean/Bumbledb/Query/Aggregates.lean: argmax_ties_all_kept`), a worse key is a
  no-op. Memory is O(groups × ties), and ties are structurally rare (fresh keys
  cannot tie).
- **`Pack`** is a group-state fold with a **relation-shaped finalize**
  (the coalesce spec: `lean/Bumbledb/Query/Aggregates.lean: pack_extensional`,
  `pack_canonical`): per group the sink accumulates
  the claim list — `[start, end]` encoded word pairs appended raw, pooled by
  group index (the Arg answer-set precedent, capacity retained across executions);
  finalize sorts each group's list by start word (`sort_unstable` — the in-place
  machinery, allocation-free; a pooled radix stays unearned until the bench
  shows the sort on a profile) and drives the shared segment sweep's
  (`interval/sweep.rs` — the coverage judgment's walk, `Pack`'s finalize is its
  second continuation; one fold, two consumers, proved:
  `lean/Bumbledb/Exec/Sweep.lean: pack_is_the_sweep`) maximal-run emission:
  one head answer per maximal segment.
  Identical and overlapping claims collapse in the sweep, never at fold time;
  memory is O(the group's claims) — retained high-water scratch under the
  allocation contract, gated like every sink pool. Like `CountDistinct` and
  Arg, the set-valued group state folds per binding (no gather kernel or scan
  pushdown applies).
- **Elision optimization:** the seen-set is elided when every atom occurrence's
  bound fields cover a key of its relation (typical for ledger queries that bind
  fresh ids) — the licence is
  `lean/Bumbledb/Exec/Dedup.lean: distinct_witness_licence`.
  `provably_distinct` is the only mint for
  `DistinctWitness`; `AggregateSink::without_seen_set` requires that witness by
  value, and the ordinary/union constructors cannot omit the set. Single-rule
  only: the multi-rule union keeps its spanning seen-set (keyed by
  provenance, § the rule loop; ruled 2026-07-23, R2) even when every rule
  has its own witness — deliberately distinct
  from the measured cross-rule elision refutation below.
- **Rule-disjointness knowledge:** `plan/fj/provably_disjoint.rs` recognizes a
  multi-rule program whose heads are provably pairwise disjoint — the witness
  form and its soundness are
  `lean/Bumbledb/Exec/Dedup.lean: syntactic_disjointness_sound` (conservative
  and pairwise by design: params, sets, and mixed constant forms pin nothing;
  the elision the witness could license is `disjoint_witness_licence`). The
  DU-arm union is exactly this shape. Plan introspection retains the knowledge
  as `disjoint_rules: proven (R.f)`, but execution always keeps one seen-set
  spanning a multi-rule program, keyed by provenance (§ the rule loop; ruled
  2026-07-23, R2).

  **Refutation — cross-rule dedup removal.** Three pre-isolation scale-S runs
  measured the proof-driven per-rule-drain representation 32.1%, 32.6%, and 32.4%
  slower (proof path 1396.5/1393.2/1408.3 µs p50 versus spanning
  948.0/938.8/952.1 µs). The typed isolated run at commit `39f6bee` reproduced the
  loss: 1376.9 µs versus 937.2 µs, −31.9%; per-repetition clock-normalized p50s
  were 1375.2 and 936.8 µs, both clean. Both arms emitted 82,983 bindings and
  absorbed zero, excluding D2 cancellation as the cause. The failed representation
  still built a per-rule dedup map, then copied every entry to an answer carrier and
  cleared the map at each rule boundary — extra O(n) drain/copy passes versus the
  spanning map's single final walk. It was deleted. Reconsider only for a workload
  where spanning-map probe cost measurably dominates and D2 skip provably never
  fires; any replacement must re-earn its own isolated win.

**Deviation D2 (set semantics — replaces the old D2):** the paper is bag-semantic
(leaves may carry multiplicity, output is a tuple stream). We: sets everywhere; leaves
are membership; binding dedup as above; and the executor may **skip a plan suffix after
the first witness** when (a) the active sink is the projection sink and (b) the suffix
binds only variables outside the projection set — the emitted fact cannot change, so
the pipeline cancels that binding's origin below its absorb node on the sink's first-emit
signal. The skip is **never legal under
an aggregate sink** (any new bound variable multiplies the binding set the fold is
defined over). **The skip is per-rule**: each rule of a program executes its own plan,
so a skip unwinds inside that rule only and never crosses rules — a later rule
re-deriving the same head fact is absorbed by the spanning seen-set (§ the rule loop),
which is what makes the skip's early exit harmless under union. **Reverses if:**
never — product semantics.

**Deviation D3 (sinks, not `output()`):** the executor emits complete bindings to a
private sink trait; projection-dedup and aggregate folds (semantics normative in
`20-query-ir.md`) are the two sinks. Aggregation never materializes the join. Group
maps live in sink arena state; aggregate result types: Sum(I64)→I64, Sum(U64)→U64
(i128/u128 accumulators, one final range check), Count/CountDistinct→U64,
Min/Max→input type, Arg carries→their variables' types, Pack→its input's
interval type. **Reverses if:** never structurally.

## The rule loop

A prepared query is a program — one head, a list of prepared rules, each either a
key probe or a Free Join rule carrying its own `ValidatedPlan` (the whole planning
pipeline runs for each non-key-probe rule at prepare). Execution runs the
rules **sequentially** into **one sink**: the sink resets once per execution, never
per rule, and its dedup machinery spanning rules is the *entire* implementation of set
union — one sink hearing several rules computes exactly the query union
(`lean/Bumbledb/Exec/Dedup.lean: union_regime_head_projection` for hand-written
programs, `dnf_rekey_transparent` for DNF-derived rule sets — the provenance
split below). **Union is not an
operator** — no merge node, no concat-then-dedup pass exists
anywhere in the executor; disjunction at the top is the rule list. Inter-rule parallelism
is not attempted: it is inter-query parallelism's job (the concurrency contract below)
and stays a non-goal.

- **Dedup keys split by provenance (ruled 2026-07-23, R2).** A **hand-written
  multi-rule program** keys the **head projection**, never the rule's slot
  array — its binding-slot layouts are per-rule (a `VarId` is rule-scoped
  across written rules, so a full-binding key has no cross-rule meaning) and
  the head is the only shared vocabulary; the key law is
  `lean/Bumbledb/Exec/Dedup.lean: union_regime_head_projection`. A
  **DNF-derived rule set** — the rules validation mints from one written
  rule's condition trees (`20-query-ir.md` § the input condition grammar) —
  **re-keys the union dedup on the shared slot arrays**: the disjuncts are
  clones of one rule sharing one variable scope and one binding layout, so
  the full binding set is shared vocabulary, disjunction widens membership
  without moving the fold domain (the or-transparency law,
  `lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`), and the witness's
  written-rule provenance (`20-query-ir.md` § the input condition grammar)
  is what the re-keyed dedup reads. The projection sink keys the projected
  find tuple (head-shaped already); the multi-rule aggregate sink keys per
  its provenance — for a hand-written program the **head projection**: per
  head position, the words the position reads from the rule's binding (group
  variables and fold inputs; the nullary `Count` contributes nothing).
  The single-rule aggregate keys the full slot array (its fold domain is the rule's
  distinct full bindings —
  `lean/Bumbledb/Query/Aggregates.lean: agg_over_distinct_bindings`).
  The spanning set remains under the rule-disjointness proof (§ set semantics):
  `DisjointWitness` is diagnostic knowledge, and the measured refutation above
  rejects the slower per-rule drain representation.
- **Per-rule re-aiming:** the sink's slot tables (projection slots; aggregate finds,
  group spans, head-projection spans) re-aim to each rule's binding layout at rule
  entry — head positions are fixed (arity, ops, widths, types), slots are the rule's.
  The shared maps (answers, groups, seen-sets, value sets) carry across rules untouched:
  the spanning is the point. Binding-slot scratch is shared across rules, re-sized to
  each rule's layout at rule entry; executor scratch stays per-rule (it is
  plan-shaped: slot maps, node buffers).
- **Params are query-global**: bound once, resolved into shared slots every rule
  reads; per-rule state is only what is plan-shaped (resolved filters, selections,
  the view memo). A rule whose `Eq`-anchored constant misses the dictionary
  short-circuits **that rule only** — a rule is one disjunct.
- **Key-probe rules** union through the sink like any other rule; the direct
  no-sink decode lane applies only to the single-rule key-probe program (the union must
  hear every rule).
- **The ray-probe pass (ruled 2026-07-23, R6).** The rule loop never renders
  the Ray verdict: a measure filter or residual DROPS a ray (a ray never
  *Holds*, and its Fails-vs-Ray distinction is not the mainline's to make).
  After the loop, each written rule with measure conditions runs one probe
  per measured interval variable — the rule's atoms, negations, and
  memberships with the conditions replaced by an `Allen(INTERSECTS)` filter
  against the ray probe `[MAX−1, ∞)`, which only rays intersect — through
  the ordinary Free Join machinery into an arbiter sink
  (`exec/verdict.rs`). The arbiter folds the written rule's compiled
  three-valued verdict (Or over its lowered disjuncts of And over their
  sealed comparisons — equal to the written tree's Kleene fold by
  distributivity) at every enumerated binding; the first Ray raises the
  typed `MeasureOfRay` with the offending words. Probes group on the
  witness's mint set, so a cross-written collapse still folds each rule
  over exactly its own disjuncts. Recursive programs defer the pass (no
  probe over a transient `Idb` image yet); the degenerate embedding probes
  like any query.
- **The view memo under rules:** occurrences of one relation in different rules share
  the image `Arc` by construction (one `ImageCache`, one build per
  `(relation, storage_tx_id)`), and each occurrence's filtered views memoize per
  (generation, resolved filters) exactly as within one rule — a repeat execution of
  the program rebuilds nothing in any rule.
- **Arg-restriction never crosses rules** — refused at validation
  (`20-query-ir.md` § aggregation): the restriction key is rule-scoped, outside the
  head's vocabulary.
- **`Pack` does cross rules**: its head position reads the raw claim's two
  words, so the spanning head-projection seen-set keys (group, claim) pairs —
  a claim two rules derive folds once — and the coalesce runs over the union:
  ∪ first, maximal segments at finalize
  (`lean/Bumbledb/Exec/Sweep.lean: pack_is_the_sweep` over the union regime's
  head-projection key).

## The fixpoint driver

A recursive program executes as strata of rule loops: the driver
(`api/prepared/fixpoint.rs`) runs the SCC condensation's strata in order, and
within a stratum it is semi-naive evaluation over the existing run-rule
machinery — round 0 runs the stratum's non-recursive rules through the rule
loop verbatim, round r ≥ 1 runs each recursive rule's **delta variants** with
the delta occurrence bound to round r−1's frontier, and an empty Δ ends the
stratum. The driver computes exactly the model's answers
(`lean/Bumbledb/Exec/Fixpoint.lean: evalProgram`, sound and complete against
the stratified denotation by `program_eval_sound`); termination is the
validation roster's theorem (`program_den_finite` — the fuel bound is a lemma,
`missingCount_le`), the round loop's stop-on-no-change is `fueledLoop`'s, and
strata above the output's are never evaluated (`evalProgramAt` reads the
output's table after its own stratum closes). A no-`Idb` program never reaches
the driver: it prepares as its output predicate's query, byte for byte
(`degenerate_embedding`).

- **The delta rewrite is k plans, not bookkeeping** (`DeltaVariant`,
  `api/prepared.rs`): per recursive rule, variant *i* marks recursive atom *i*
  the delta occurrence and every other same-stratum atom the accumulated
  predicate, each variant prepared once through the ordinary per-rule pipeline
  (pin-at-prepare; no round re-plans). There is **no new/old split**:
  cross-variant and cross-round re-derivation is absorbed by the predicate's
  spanning seen-set — the same argument that makes D2's late cancellation
  harmless, and the operator-level face is
  `lean/Bumbledb/Exec/Fixpoint.lean: semi_naive_agrees` (iterating on
  `T(acc) \ acc` walks the naive chain round for round). Variants are minted by
  one prepare-time parse and consumed totally by the driver —
  `ResolvableFilter`'s discipline. Delta and accumulated occurrences pin no
  statistics and cost on the selectivity ladder's floors
  (`DELTA_PLANNING_CARDINALITY` / `ACCUMULATED_PLANNING_CARDINALITY`,
  `plan/selectivity.rs` — the param-plan precedent: prepare-unknowable
  cardinalities plan on documented constants).
- **The frontier IS the sink's seen-set with a per-round watermark**: `WordMap`
  preserves insertion order with dense O(len) iteration, so round r's frontier
  is exactly the dense suffix `[watermark, len)` — one `usize` read per round
  and a cold suffix walk (`WordMap::iter_since`,
  `ProjectionSink::answers_since`); no flag, no branch, no state on the emit
  path, and a non-recursive program cannot observe the hook. Dedup keys stay
  rule-independent per the rule loop's provenance split (head-shaped across
  hand-written rules, shared-slot across a DNF-derived rule set — ruled
  2026-07-23, R2), which is precisely what
  makes the frontier readable at all. Interior predicates own
  projection-shaped seen-sets of bound variables (the validation roster
  refuses folds in interior heads — `AggregateInteriorPredicate` — and
  measures in interior heads, recursive or not — `MeasureInteriorPredicate`,
  with `MeasureInRecursiveHead` catching the recursive form first: the
  executable-class item, folds and measures legal only at the output head);
  the output predicate keeps the ordinary head-owned sink. **Union stays the sink and
  only the sink**: no merge node, no frontier queue, no worklist structure
  exists. D2's suffix skip stays per-rule and within-round.
- **Transient images live outside the soundness machinery**
  (`image::TransientImage`, `image/build.rs`): a round's delta and accumulated
  images are columnar transposes of the seen-set's word rows — the
  `synthesize_closed` precedent with a cheaper source, no fact-bytes decode.
  The accumulated image is **incremental**, never rebuilt per round: the
  seen-set is append-only within an execution, so each half of the ping-pong
  pair remembers its own filled floor and appends only the suffix it lags by
  (`TransientImage::append`, `image/build.rs`); the delta image stays a
  per-round transpose of the frontier suffix.
  A transient image is valid for one round of one execution, a lifetime the
  generation vocabulary cannot express, so it is **never** in the `ImageCache`
  generation map, never parked in the view memo (`Idb` occurrences bypass
  `memo.bind` and take a per-round `Colt::reset`, survivor buffers recycled
  through the existing `spare_buffers` ping-pong), and never pinned by
  `PreparedQuery::staleness` — every generation-keyed mechanism never learns
  recursion exists. The pools are prepared-query property: ping-pong slot
  pairs refilled in place through `Arc::get_mut`, sized at their high-water
  (the allocation contract below).
- **The budget is the one new trust boundary**: termination is a theorem, but
  the fixpoint's *size* is data-shaped — a foreign query may legally demand a
  quadratic closure. The driver carries a per-stratum iteration/tuple budget
  with documented defaults (`DEFAULT_FIXPOINT_ROUNDS`,
  `DEFAULT_FIXPOINT_TUPLES`) and the typed execution error
  `Error::FixpointBudgetExceeded { stratum, rounds, tuples }` — on
  `MeasureOfRay`'s model: aborts the query, the snapshot stays usable, the
  payload is ids and counts, never strings. Policy stays host-owned
  (`PreparedQuery::set_fixpoint_budget` — the staleness doctrine verbatim: the
  engine ships the typed condition, never a threshold loop); the default
  exists so the boundary is never unguarded. See the resource-limits amendment
  below.

## Planner

**Grounding: elimination and evaluation.** This is not dependency-theory
fresh-witness repair: it creates no values and repairs no database. The pass
GROUNDS sealed atoms by evaluating their fixed finite extensions at plan time,
the Datalog term. Placement: after normalization,
before statistics and the DP, **per rule and independently** — a union's rules
are independent conjunctive bodies, so the grounding distributes over them with no
cross-rule state and no new theory, and a rule shrinking below its cover
requirements re-validates like any rule — one fixpoint over the occurrence
table's `Role` sum (`plan/ground.rs`) running two rewrites that expose each
other: **elimination** marks provably redundant positive occurrences
`Role::Eliminated(statement)`, and **evaluation** (`plan/ground/evaluate.rs`)
marks prepare-evaluable closed-relation occurrences `Role::Folded` — marks,
never removals, so occurrence ids never move. Elimination removes atoms that
statements prove redundant; evaluation removes atoms whose extension is
stage-0-known by *running them at prepare*: `Kind(id: k, mastered == true)` is
not a join to plan — it is a three-element id-set computed before the DP ever
sees the query, residual cost zero. Both rewrites — and any chain of them with
the statically-empty kill, in any order — preserve the query's answers
(`lean/Bumbledb/Exec/Rewrites.lean: grounding_preserves_answers`,
`elimination_sound`, composed by `rewrite_composition`), and the bench
crate's dual-run differential checks the same statement empirically
(`crates/bumbledb-bench/src/differential/tests` — through the
`ground-off` switch).

*Elimination.* An accepted containment
`A(X | φ) <= B(Y | ψ)` makes the query's join of `A` to `B` on X→Y redundant
when the B occurrence contributes nothing else. The licensing conditions and
the answer-preservation proof are
`lean/Bumbledb/Exec/Rewrites.lean: ElimStep` and `elimination_sound` —
`plan/ground.rs::removable` checks them condition for condition, and every
literal carriage is (field, encoded literal) set containment, never inference —
a statement carrying a disjunctive literal-set binding answers "unknown" and
the join simply stays (no single-literal filter list can certify a set
binding; the sanctioned conservative fallback, `30-dependencies.md` § the
decidability firewall).
The one condition that is a recorded v0 refusal rather than a theorem
premise: **scalar positions only** — an interval-typed pair refuses (pointwise
coverage proves covering facts exist, not a joinable equal fact). OPEN
trigger: a census-style query that would benefit from interval-pair
elimination — until one exists the refusal stands, like the range
accelerator's trigger discipline.

Chains (`A<=B<=C`) close in the fixpoint; mutual `==` pairs stay acyclic by
support tracking (each elimination records its source, and a source whose chain
passes through the candidate is refused — a pair may not certify itself). The
discharged-source composition is a theorem
(`lean/Bumbledb/Exec/Rewrites.lean: chained_elimination_sound` — one chain
link, the acyclic-support premise named; deeper forests iterate the argument,
the module's recorded narrowing). Sound
here and nowhere like Postgres because no deferral modes exist: every readable
snapshot satisfies every accepted statement
(`lean/Bumbledb/Txn.lean: committed_states_model`), which is how
`elimination_sound`'s containment premise is discharged — removal is
proved result-identical under both sinks: the projection sink
(`lean/Bumbledb/Exec/Rewrites.lean: elimination_sound`) and the aggregate
sink, as two theorems — key-ness of Y keeps a dead non-key variable from
multiplying the fold domain
(`lean/Bumbledb/Exec/Dedup.lean: elimination_agg_fold_domain`, the bijective
projection, whose key premise pays off as the count transport between the
engine's full-slot fold domain and the surviving-slot domain,
`lean/Bumbledb/Exec/Dedup.lean: elimination_agg_domain_counts`), with answer
identity fiber for fiber at the surviving-slot reading
(`lean/Bumbledb/Exec/Dedup.lean: elimination_agg_sound`; the pair's recorded
scope is that module's doc). The marks' readers: plan introspection
and the structured stats (each mark rendered with its licensing statement
through `schema/render.rs`), and the DP, which sees a smaller problem.
**Alternative:** no rewrite — leave redundant existence walks to D2's
skip-suffix dynamics. **Why it loses:** the skip still pays per-binding probes
and a larger DP, and is illegal under an aggregate sink (D2's own rule), while
elimination is sink-independent and pays once at plan time. **Reverses if:**
measured plan-time cost of the fixpoint exceeds its execution savings on the
ledger suite — implausible at the 20-occurrence cap.

*Evaluation — the fold.* A positive occurrence `C` of a closed relation is
**foldable** when every one of these holds (strict; any failure leaves the atom
to join against its virtual image, which is L1-resident and always correct):

1. Every variable bound by `C` except at most one is *dead outside `C`* (no
   head use, no other occurrence, no residual/anti-probe/point-probe use). The
   at-most-one live variable must be bound at `C`'s **id position**
   `FieldId(0)` — the join variable `k` — and some other participating
   occurrence must bind `k` (the membership set needs a home).
2. `C`'s filters parse completely into the prepare-resolvable
   Eq/range/Allen/membership vocabulary over its own columns; the parser
   returns that narrowed vocabulary and the evaluator is total over it. A
   param-bearing filter REFUSES the fold
   in v0 (the bind-time fold alternative is recorded; trigger: a measured win in
   the calendar-family profile); measure filters refuse too — their ray error
   is a per-execution error, and evaluation would move it to prepare.
3. `C` is not negated (negated atoms fold to the complement — below).

The fold evaluates `C`'s filters against the sealed extension's ground axioms at prepare
(n ≤ 256, encoded-word compares and the scalar Allen classify — never a batch
kernel), producing the surviving id-set `S`. `|S| ≥ 1` with a live `k`: `C` is
marked `Role::Folded` and `S` attaches to every other occurrence binding `k`
as a **plan-constant membership** (`Eq` against `Const::WordSet`) — exactly
the param-set selection machinery, except the set is pre-resolved: a
set-bound selection level probed once per element with the survivor union,
the machinery making exactly the choices it makes for a bound param set
today. **Nothing new executes**, and a plan-constant set never counts as an
unresolved literal (the literal latch's fully-resolved fast path stays open).
`|S| == 0`: the rule is **statically empty** — the fold's rule-death channel
(`NormalizedQuery::dead`, rendered `folded to ∅: Kind{mastered == true}`),
deleted at prepare exactly like a normalize-time death, and honestly: a
refuted rule answers nothing on any agreeing instance
(`lean/Bumbledb/Exec/Rewrites.lean: ground_refuted_empty`). No live `k` (a pure
constant gate): `|S| ≥ 1` deletes the atom outright and `|S| == 0` kills the
rule — but only a **var-less** gate may delete: a dead-but-bound variable
still multiplies an aggregate's fold domain (the binding set is over all query
variables — D2), so a variable-binding gate refuses.

**The payload refusal, recorded:** a closed atom with a live non-id variable —
payload escaping to the head ("return each event's severity rank") — keeps its
join against the virtual image: the join is L1-resident, generation-immortal,
and the DP prices it honestly. Folding payload projection would require value
substitution into the head, a rewrite class with real complexity and no
measured need. Refused; trigger: the calendar family showing vocabulary-join
cost above noise.

**The complement rule (negated closed atoms).** `!Kind(id: k, mastered ==
true)` with `k` bound positively rejects a binding iff `k ∈ S` (the id is the
whole key), so it folds to membership in the **COMPLEMENT** (extension ids
minus `S`) — same machinery, complement computed at prepare. The directions,
pinned: `|S| == 0` means the anti-probe **rejects nothing** — the atom deletes
outright and the rule is NOT empty (no domain reasoning needed: `k ∉ ∅` holds
for every `k`); an **empty complement** (`S` = the whole extension) means
every binding is rejected — the rule is dead. The complement rewrite itself is
sound only under a **domain guarantee** — `k ∉ S ⟺ k ∈ complement` requires
`k` inside the extension ids, and an out-of-extension `k` would survive the
probe yet fail the membership. Two witnesses: `k` bound at the id position of
another participating occurrence of the same closed relation, or a binder
whose field carries an accepted containment into the closed relation's id
(with the statement's φ carried literally by that occurrence). No witness →
the fold refuses and the anti-probe stays. (The complement fold is deliberately
unmodeled in the spec — the recorded narrowing in
`lean/Bumbledb/Exec/Rewrites.lean`; until it is, this block is the semantic
authority and the grounding differential is its empirical check.)

Plan introspection reports folds beside eliminations, off the `Role::Folded` marks — the
surviving set as **handles**, the vocabulary's names (the handle set IS the
payload): `folded: Kind{mastered == true} → {DirectPass, JudgedPass}` (negated:
`folded: !Kind{…} → {…} rejected`); the differential off-switch
(`with_grounding_disabled`) covers the evaluator inside the same fixpoint, and the
dual-run corpus pins byte-identical results — the fold is never semantic
(`lean/Bumbledb/Exec/Rewrites.lean: grounding_preserves_answers`).
The normalization fold's narrower `with_fold_disabled` switch is compiled under
`cfg(test)` — the engine unit suites alone reach it (the `fold-off` feature that
once exposed it to the detached fuzz crate died with the fuzzing apparatus,
`60-validation.md` § the deletion record); the bench differential deliberately
uses the grounding switch
because that switch covers the evaluator in the same fixpoint.

**Rule subsumption, the restricted witness.** After elimination, if one rule's
normalized body equals a sibling's *modulo the filters elimination removed* —
identical participating atom multisets on identical head projection, with the
keeper's conditions a subset of the deleted rule's — then the keeper contains
the sibling in denotation (a body homomorphism at the identity variable
mapping) and the subsumed rule is **deleted** at prepare: classical UCQ
minimization, restricted to the cheap witness the DNF path actually produces
(a lowered `(φ ∨ true-by-elimination)` pair whose second disjunct's filter
rode the eliminated occurrence). The check is normalized-form containment
(`plan/ground.rs::subsume`), O(rules²) at prepare with rules ≤ 16, and nothing
recursive. The deletion is in the spec's rewrite chain
(`lean/Bumbledb/Exec/Rewrites.lean: subsume_containment` — the deleted rule's
answers are contained in the keeper's, and `RewriteStep.subsume` carries the
step through `rewrite_composition`). **Refused, the general form:** full
CQ-homomorphism minimization is NP-hard, so the witness never searches
variable mappings — `VarId`s must already agree, which is exactly what
DNF-cloned rules provide. Deleting a rule
never changes the head (the head-alignment invariant is re-checked after
deletion), a program shrunk to one rule sheds its union machinery like any
single-rule program, plan introspection reports deleted rules with the subsuming rule's
index (lowered-rule indices) beside the per-rule eliminated atoms, and the
differential off-switch covers both passes.

**Statistics** (all real, nothing else exists): exact per-relation fact counts
(maintained on write, stored in `S`); schema dependency knowledge (keys and
containments — `30-dependencies.md`); filter survivor counts — *measured, not
estimated*: filtered views are built before planning completes for the atoms whose
filter constants are all concrete, so the planner uses the view's actual length.
**Carve-out:** an atom whose filters involve params, param sets, or not-yet-interned
literals cannot be measured at prepare time — it plans on the selectivity ladder
(`plan/selectivity.rs`): key-exact counts, resident-image distinct counts (peeked,
never built), schema bounds (containment domains, bool), then
the documented keep-fraction floors per predicate class. A param-set position plans
as a selective equality under the documented small-set assumption
(`20-query-ir.md`). No NDV fields, no histograms; the floors are the only constants
and each is documented at its definition.

**Join cardinality estimator, written down:** for `L ⋈ R` on join variables J —
- J covers a key of R (incl. fresh auto-keys): estimate = |L| (reference walk; exact
  upper bound).
- J covers a key of L: estimate = min(est(P), |R|) — each R fact matches at most one
  prefix binding, and each prefix binding matches at most |R|; the min is the correct bound.
- Neither: estimate = |L| × |R| — **no estimate exists, so pessimism**, which pushes
  non-key joins last; that is the correct behavior, not a modeling failure.
|X| is the fact count or the filtered-view survivor count. Negated occurrences enter
no estimate — they only shrink results, and the planner treats them as free filters
(pessimistic in the right direction).

**The estimate doctrine** (ruled 2026-07-23, R19): estimates stay crude by
design — the ladder and the three arms above are the whole model, and no
histograms, NDV fields, or compound-distinct statistics exist. Precision lives
at execution time, not in the estimator: GJ-shaped plans (the GJ split, below)
plus per-entry dynamic cover choice bound skew where the data is actually in
hand — the Free Join thesis. Estimates order the DP; covers absorb what the
ordering misses. Revisit only if post-009 benches show plan-choice misses
covers can't absorb.

**Search:** exhaustive DP over positive atom occurrences, **left-deep only**,
minimizing the sum of prefix estimates *including the base relation's facts* (counting
the root iteration breaks ties toward iterating the small side). The cap is 20
occurrences (a 2²⁰-state table, ~32 MB transient plus a 16 MB per-mask
prefix-variables memo; the cap is enforced at the validation boundary as a roster
item, alongside the 128-distinct-variable bitset cap — negated occurrences count
against the roster cap but not the DP state, since they never join). Then
`binary2fj`, then `factor()`, then the **GJ split** (ruled 2026-07-23, R19): a
probe subatom carrying two or more variables first bound at different earlier
nodes splits into per-variable lookup subatoms, each placed at the node where
its variable is first bound — the lowering that produces plans at the GJ end
of the Free Join spectrum for cyclic rules, and the step that gives a
production node its second cover (under `binary2fj` + `factor()` alone every
node has exactly one, so dynamic cover choice never has a choice). The split
mints no machinery: trie schemas derive from the split subatoms per §3.3, the
partition check already admits one occurrence's variables spread across nodes,
and carried cursors route a multi-node occurrence forward. Then plan
validation into the witness.
**Decision: left-deep-only.** **Alternative:** bushy plans + materialized intermediates
(the paper decomposes bushy input into several left-deep plans and names
materialization its main bottleneck, §5/§6). **Why it lost:** materialized intermediates
have no home under the sink model and the allocation contract; left-deep + factoring
covers the design space the workload needs. **Reverses if:** a real query family shows
a bushy-only win that survives the benchmark protocol.

Plans **pin their statistics at prepare time** and are never invalidated by writes
(decision recorded in `20-query-ir.md`).

**Deviation D5 (no DuckDB):** the paper takes DuckDB's optimizer output; we grow the DP
above. **Reverses if:** never — no external SQL engine as infrastructure.

## Vectorized execution

**Deviation D4 (batching tuned to Apple Silicon):** the paper batches cover iteration
and probes siblings per batch (§4.3), hardware-generic. We: same algorithm, batch sized
to fill the M-series' ~28 MLP lanes — model: each probe is ~1–2 dependent loads, so
~28 lanes want ≥28 independent probes in flight and the batch amortizes bookkeeping
across several waves: the decided range is 64–256, the code ships 128
(`exec/run.rs` `BATCH`), and the exact number is still measurement-owned
(OPEN, README). **Probing is
two-phase**: phase one computes keys and hashes for the whole batch (pure ALU, no
memory dependence); phase two issues all bucket loads — independent chains the OoO
engine overlaps across the full MLP width. COLT's forced maps use **open addressing
with inline keys** (one probe ≈ one or two cache lines, no node chasing) and are kept
compact enough that a query's hot maps live in the 12–16 MB shared L2. **Batches are
processed branchlessly**: probe misses, residual failures, and anti-probe hits become
survivor compaction (the scalar branchless cursor-write — NEON has no compress
instruction; that is SVE, which Apple Silicon lacks), never per-tuple conditional
control flow — on a >99%-accurate TAGE predictor, the data-dependent per-tuple branch
is the only misprediction source left, so we remove it representationally. The probe
walk itself is the measured exception to naive vectorization: a
full NEON candidate sweep — all 8 bucket keys compared per probe — ran 2.7×
faster than the tag-gated scalar walk in an isolated resident-map loop and
INVERTED in situ (chain +25%, triangle +4%), because the sweep touches the key
block on every probe while the tag-gated walk's data-dependent key load never
issues on a miss; under inter-phase displacement that is an
extra line per miss, and on L2-hot always-hit paths the 2.5× instruction bill
is retire-bound loss. The bucket-of-8 SWAR group walk is
the shipped shape. **No indirect
dispatch exists in the hot path**: sinks, counters, and kernels are monomorphized
generics, never `dyn`. Explicit SIMD (128-bit = 2×u64) is confined to the
sanctioned kernel shapes:
fixed-width predicate scans (interval membership included — two-word
compares over the start/end column pair, no new width), **the
configuration kernel** (interval-pair predicates are Allen *masks*, not
ops: 8 `cmhi`/`cmeq` predicate lanes over the four endpoint words pack
into a 6-bit signature, a 64-byte nibble table held in q registers maps
signature → basic code via `tbl`, and membership is the broadcast mask's
16-byte `tbl` — one branchless, **flag-free** kernel for all 8192 masks,
dense in filter position and gathered in residual/anti-probe position,
with the flag-free law enforced structurally by `scripts/check-asm.sh`
on the release disassembly), survivor compaction,
fold/accumulate kernels (Sum/Min/Max/Count over batch columns, strided or
gathered — Sum semantics unchanged: i128 accumulation, one range check at
finalization), gather kernels (position-indexed column reads), and
software-prefetch passes (`prfm`) between probe phase 1 and phase 2.

**The portable/intrinsic split is measured, not stylistic**
(the verdict matrix is the record — crucible packet, git history at ecec1dc3):
the predicate scans, dense folds, and index gathers are `std::simd`
bodies compiled on every target — each measured at or above its retired
hand-NEON twin on the reference host (filters 1.03–1.5× faster, folds
and gathered sums at parity, gathered min/max ~1.1×), deleting the
intrinsic dual and most of the kernel layer's `unsafe`, and
Miri-interpretable. The Allen configuration kernel alone keeps hand
NEON intrinsics: its 64-byte `tbl4` signature table has no `std::simd`
primitive (the 4×`swizzle_dyn` emulation measured +8% instructions per
pair), and the flag-free asm gates forbid the bounds-check `cmp` that
safe portable code would reintroduce. The scalar SWAR group walk and
the scalar cursor-write compaction stay scalar — they are already
portable, GPR-resident, and `std::simd` offers no compress primitive.

Fold kernels
follow the **port-topology law** (measured): every flag-writing scalar op
(`adds/adcs/cmp/csel`) is confined to 3 of the reference core's 6 integer ALUs, so
exact scalar summation caps at ~2.8 flag-µops/cycle while NEON escapes the triad
and rides the 3×16 B load ports — dense exact sums measured 8.8 vs 4.0–4.6 rows/ns
at L1 (carry-counted u128 compare lanes), min/max 2.65× at every tier, with DRAM
converging all parallel kernels (~7.5 rows/ns single-core). Dense (stride-1) folds
therefore take the lane form unconditionally; strided folds stay scalar until
measured (latency×MLP-bound — a different law), and the index gathers took the
portable lane form when it measured at or above the scalar-unrolled bodies
(PRD 03's matrix). Deep-OoO scalar remains the shape
for irregular control flow —
the law is about reductions, not loops in general (`00-product.md` machine
model; unsafe policy there too). Columns are 128-byte-aligned SoA
with stride-padded bases (`50-storage.md`). Scalar fallback everywhere, equal results by
test across batch sizes. **Vectorized execution is the default and only path** — a
scalar "mode" exists solely as the degenerate batch size where useful for testing; a
"vectorized mode" that wraps scalar loops without batching is the failure shape this
sentence forbids. The former D4 caveat that only roots reliably see large batches is
retired: middle-node pumps accumulate rows across parent entries, so deep probes see
full batches even when each current subtrie has fanout 1–10. **Reverses if:**
measured equal-or-worse than scalar on the ledger suite after honest tuning.

**The scan-fold pushdown is column-hoisted.** When the last plan node is a single
subatom over an unforced suffix, positions stream to the sink as runs — no key
batch materializes. Long runs (past `SCAN_HOIST_THRESHOLD`, a measured cost
threshold and the path's only constant) run column-outer, the same shape the
gather kernels won with: each projected source column resolves its view once and
writes the run's span into the sink's row-major staging rows; each leaf residual
resolves its two operands once and compacts surviving positions in place, exactly
like the batch path's residual passes. Projection width and residual count are
therefore **unbounded by construction** — both loops iterate plan-witness lists,
so no fixed-width scratch and no eligibility branch exist to cap them. Short
(fanout-sized) runs resolve per position — both directions measured, both real.

**The leaf fast paths are measured law** (cleanup-0.5.0 ruling 6, the Measure
phase, 2026-07-19; the artifact retired with the 2026-07-20 pin swap,
`6d5560a8` — git history). The single-subatom leaf
classification (`exec/run/leaf_precompute.rs`), its dispatcher, and the
pinned-row arm (`exec/run/leaf.rs` — a batch of exactly one with every batch
scaffold skipped) were measure-or-merge twinned against the same plan with
the classification forced off: **1.69–1.71× generic/elided** end-to-end on a
mixed pinned+scan self-join (700 answers/exec, warm DRAM, interleaved
min-of-7, two process runs; pre-stated bar 1.09, the crucible ADOPT
precedent). KEEP-AS-LAW. **Reverses if:** a ledger-suite A/B on the generic
batch machinery ever lands within the house bar of this path. The same
session REFUTED the all-words finalize fast path (ruling 7): resolved/words
measured 0.996–1.005 on both sinks against the same bar, so the duplicate
`AnswerHeap::Words` route and its seal were merged into the one resolving
finalize (the gravestone lives at `api/prepared/finalize.rs`).

**COLT force is single-pass with chunked child lists, graded geometry:** forcing pushes
each offset into its key's child list, chunked over one shared position slab — a chunk
is a `(start, cap, len, next)` frame; the FIRST chunk of a chain reserves 8 positions,
later chunks 64 (chained by chunk — bounded pointer traversal, independent loads within
a chunk) — rather than the paper's growable per-key vectors or a two-pass contiguous
layout (which decodes and hashes every row twice). **Deviation:** the paper's leaves are
plain vectors; ours are chunked. **Measured (the 094 geometry pin, `exec/colt/tests/
pins.rs::chunk_geometry_force_iterate_ab`, 2026-07-24):** the graded first chunk beats
the flat 64-position geometry 0.72×/0.77×/0.86× force+iterate time at fanouts 2/4/8 and
ties (1.01×) at 64, at 0.16× chunk-pool footprint for every small fanout — the common
FK-join fanouts no longer pay a 64-position reservation per key. **Reverses if:** a
force+iterate microbenchmark shows two-pass-contiguous winning end-to-end.

## The allocation contract

**Scratch capacity is a monotone high-water: a warm prepared-query execution
performs zero heap allocations unless its intermediate sizes exceed every prior
execution's**, excluding a caller-provided result buffer. All scratch — binding
slots, probe keys, batch buffers, COLT pools, filtered views, sink state (dedup
sets, group maps, distinct sets, arg-restriction sets) — is retained-capacity
pools owned by the `PreparedQuery` (index-addressed `Vec`s that reset without
freeing; the `Arena` type proper serves only the write delta), **with the
high-water taken across all rules** — the sink and the binding-slot scratch are
shared by every rule of the program, and per-rule scratch (executor buffers,
view memos) is still the one prepared query's property — so a warm execution
allocates only when a strictly-increasing input-shape high-water
pushes a pool past every capacity it has ever held; a re-bind whose
intermediates fit anything already seen touches the allocator zero times. This
is the stronger-because-true claim, not a weakening: "zero, unconditionally"
was false at three sites whose scratch is sized by per-execution intermediates
no warmup parameter is guaranteed to dominate (origin-cancellation epochs,
absorb-node origin minting, node-to-node pending buffers —
`exec/run/cancel.rs`, `exec/run/probe_pass.rs`); monotone high-water
convergence is what the pools actually guarantee, and it is a claim the gate
can falsify. Retained scratch is O(touched data + output) per prepared query
and is documented as such (an app holding N prepared queries retains N scratch
sets); pools reach a fixpoint per **(data generation, parameter envelope,
iteration shape)** — the third axis is the fixpoint driver's (delta buffers
and per-round transient images are retained-capacity pools like every other
scratch, and the driver's execution-invariant round-to-slot assignment is what
lets one run at a new envelope grow every slot it will ever need) — once every
parameter shape the app binds has been seen at its hottest intermediates,
every subsequent execution is allocation-free until the data generation
changes.

**CI gate protocol (the definition of "steady state"):** single-threaded harness,
two measured windows. **Steady state:** the prepared query executes N warmup runs
with parameters drawn from a fixed set and no intervening writes; then M measured
runs over the same parameter set assert **zero** allocator hits, arena growth
included (growth inside a seen envelope is a failure), result buffer
caller-provided. **High-water:** after warmup on the coldest parameter, a
parameter sequence of strictly increasing selectivity — each parameter binds a
strictly hotter key — asserts that allocations occur **only** on executions
setting a new intermediate high-water: every repeat of a previously-seen
parameter, immediate or later, is allocation-silent, and the window protects its
own vacuousness — the harness must observe at least one growth event across the
escalation, or the run proves nothing. First-execution and post-commit rebuild
allocations are sanctioned and outside both windows. Param sets draw from the
fixed set like scalar params; a warm re-bind of a differently-sized set within
the documented assumption reuses pooled capacity.

**Concurrency contract:** the engine owns zero threads (`00-product.md` doctrine).
Execution is single-threaded per query; `PreparedQuery` is `!Sync` and executes from
one thread at a time; arenas imply exclusive access. **Inter-query parallelism is free
and is the intended scaling axis**: reader threads each own their prepared queries and
pools and share immutable `Arc`'d images; nothing in the executor synchronizes (the
prepared query memoizes its views per (generation, resolved filters), so a warm
execution does not even touch the shared image-cache mutex).
Intra-query parallelism is a non-goal with a recorded reversal trigger
(`00-product.md`).

**Resource limits: none in v0, stated — with one deliberate and narrow
amendment.** Dedup sets, group maps, and result buffers grow with output; a
pathological query can exceed the envelope and the OS is the backstop. The
scale axiom makes engine-imposed caps ceremony; revisit only on real pain.
**The fixpoint budget amends this stance for fixpoints only** (decided with
the fixpoint driver — § the fixpoint driver above owns the mechanism; this
section owns the stance): the OS-backstop argument
priced one join's envelope, not an unbounded round count crossing the trust
boundary — termination is a theorem of the validation roster
(`lean/Bumbledb/Exec/Fixpoint.lean: program_den_finite`), but the fixpoint's
*size* is data-shaped, and a foreign query may legally demand a quadratic
closure. The driver therefore carries an iteration/tuple budget with a
documented default and the typed execution error
`Error::FixpointBudgetExceeded` (§ the fixpoint driver). Policy stays
host-owned — the staleness doctrine verbatim: the engine ships the typed
condition, never a threshold loop; the default exists so the boundary is never
unguarded. Non-recursive execution keeps the unamended stance.

## Deviation D1 — data source

*Paper:* relations are columnar in main memory. *We:* durable data lives in LMDB;
execution reads **full-width cached columnar images** built once per
`(relation, storage_tx_id)` and shared across read transactions (`50-storage.md`).
After warmup, execution runs in exactly the paper's environment. *Why:* LMDB is the
durable truth; cold cost after a commit is an O(delta) tail decode plus one
O(relation) column memcpy for delete-free relations (copy-on-append — the
memcpy is the recorded cost the slab follow-on removes
(the copy-on-append ruling record, I1 — packet retired, history in git); at ceiling scale it
is hundreds of milliseconds, not noise) and a full O(relation) decode after a
delete. The old soundness clause — "at ≤1 GB the whole
working set caches and the write design point (≥100 reads/generation) amortizes
builds to noise" — is RETRACTED on both legs (the ≤1 GB leg fell to the 32 GiB
ceiling ruling, the ≥100-reads leg to the bursty-rare retraction,
`00-product.md`); the amortization argument is retired, replaced by
maintenance. **Reversal record, stated honestly:** the recorded trigger
("traced rebuild cost violates the latency budget despite the cache") FIRED in
substance at the new ceiling — a ceiling-scale rebuild is seconds by
arithmetic — and the remedy chosen is incremental maintenance of the cached
images (copy-on-append), NOT the recorded "persist columns instead": persisting
columns would re-open Deviation D1 wholesale, while copy-on-append keeps LMDB
the only durable truth. A documented divergence from the recorded remedy.
**Reverses if:** traced rebuild cost still violates the latency budget despite
maintenance — then persist columns.

**The closed carve-out:** a closed relation's image is not built from LMDB at
all — it is *synthesized* from the theory's sealed extension into a
per-relation `OnceLock` slot outside the generation map, once per process,
never evicted, never rebuilt (`50-storage.md` § virtual relations). Execution
consumes it exactly like any image; only its source and lifetime differ.

## Observability

**Plan introspection exists from day one** and is the debugging story. Mechanism — a
representation, not a mode: the executor is generic over a `Counters` trait;
the normal path instantiates `NoopCounters` (zero-sized, compiled to nothing — no
runtime branch, no hot-loop cost), and the plan introspection entry point instantiates the
counting implementation and **executes the query** (ANALYZE semantics), reporting **per
rule** the plan, per-node estimated vs actual cardinalities, residual and anti-probe
selectivity, cover-choice histograms (choices aggregated per node, not per entry),
and the grounding's eliminated occurrences — read straight off the plan's
`Role::Eliminated` marks, each rendered with its licensing statement through
`schema/render.rs` (e.g. `eliminated: Grading via Grading(id | kind == Det) ==
Det(grading)`) — plus the **head-level union accounting**: per rule, bindings
emitted to the shared sink vs absorbed by the spanning seen-set (absorbed =
emitted − newly-seen: two O(1) reads per rule, no per-tuple cost; an elided
seen-set absorbs nothing by proof), — multi-rule programs — the
rule-disjointness line naming its witness (`disjoint_rules: proven (R.f)`,
or `unproven`), and the subsumption record: rules deleted at prepare, each
with its subsuming rule's index (`subsumed: rule 0 by rule 1`, lowered-rule
indices — the per-rule sections are the survivors). A recursive program's
counted surface is the driver's round structure, not per-unit node stats
(one counter spans many differently shaped plan units): plan units labeled
(predicate, rule, delta variant), then per recursive stratum, per round —
round 0 the stratum's non-recursive rules — each predicate's delta rows and
the round's emitted/absorbed accounting, reported through the same
`Counters` seam's fixpoint hooks (`api/prepared/fixpoint.rs`;
`ExecutionStats::strata`). The obs registry mirrors it: one `RULE` span
per rule under the execute span (`rule_N` — the index rides in the name,
`MAX_RULES`-bounded), args (emitted, absorbed), populated on counted paths;
the driver adds one `stratum_N` span per recursive stratum
(`MAX_PREDICATES`-bounded, args (rounds, tuples)) with one `fixpoint_round`
span per round under it, args (emitted, absorbed).
The output contract is `introspection v3`: byte-identical within the version for
identical schema fingerprint, canonical query, parameter types, and features, with
the fixed ordering specified in `70-api.md`. Any content or ordering change bumps
the rendered and structured version together. Release builds contain no other instrumentation: no per-tuple labels, no
always-on counters, no diagnostics allocation anywhere in the join loops.

## Measured mechanisms

Six measured decisions, enforced structurally by
`crates/bumbledb-bench/src/tripwires.rs` (never by wall clock):

- **Selection levels.** Every Eq-against-a-constant (literal or param — the
  same machine) lowers into `PlanOccurrence::selections` and becomes a
  prepended single-column COLT trie level, probed per execution with the
  resolved word (`Colt::select`). Force is O(view) once per generation, probes
  O(1) per param; views carry only residuals (ranges, Ne, `FieldsCompare`).
  **One carve-out, correctness-owned:** an occurrence carrying a measure
  predicate keeps its Eq-constants residual (`plan/fj/split_filters.rs`) —
  a selection probes only after the view (measure refinement included) is
  built, so a lifted Eq would let a row it excludes reach the subtraction,
  violating the filter-order law (`20-query-ir.md` § the measure). The
  measured atom pays with scans instead of probes.
  **Param sets ride this machinery**: a set-bound selection level probes once
  per element (k probes, k small by the documented assumption) and the
  survivor union feeds the node — never a per-element re-execution.
  **Alternative:** per-image secondary hash indexes — lost because the trie
  already *is* the index and selections compose with join levels for free.
  **Reverses if:** never; the per-param full scan it replaced was the
  measured 6.35× string-family loss.
- **The view-memo LRU.** Each occurrence memoizes `MEMO_SLOTS = 4`
  (generation, resolved residual filters) bindings — one active whose COLT
  the executor consumes, three parked slots (empty at prepare) and swapped
  in on hit. Prepare pins nothing: every COLT starts over `View::Unbound`
  (no image Arc — a prepared-but-never-executed query holds zero image
  memory), and the first execution binds via the ordinary miss path. Each
  bind first reaps parked entries below the requested generation (provably
  unhittable — their pools and image Arcs die at the first post-commit
  execution); parking prefers an empty slot, then evicts by LRU, and a
  stale or unbound active rebuilds in place so selection-only occurrences
  never park. Sound because generational immutability makes a view valid
  for its whole generation. Memory bound: four COLT high-waters per
  occurrence per prepared query, current-generation images only.
  **The sentinel generation:** a closed relation's occurrence binds at
  `GENERATION_CLOSED = u64::MAX` — the theory is its image's generation, so
  the binding can never go stale. The sentinel is maximal and storage
  generations only advance, so the reaping pass (which drops parked bindings
  *strictly below* the reader's generation) never touches it: warm forever,
  zero rebuilds across commits. A sentinel constant, deliberately not an
  `Option<Generation>` threaded through the memo — every existing comparison
  keeps compiling and meaning the right thing.
- **Magnitude-first cover choice.** `KeyCount` labels mean keys-exact vs
  positions-upper-bound; both are admissible iteration-cost bounds, so
  `better_cover` compares magnitudes and uses the label only on ties. A
  label-first "Exact displaces Estimate" rule iterates a 500-key forced map over a
  7-fact view — the measured wrong-cover this rule exists to prevent.
- **Dense map iteration and occupancy sizing.** Forced maps carry a dense
  occupied-slot list (iteration is O(keys), never O(capacity); the map
  `BatchToken` is a dense index) and size from
  `next_pow2(clamp(count/8, 16, 2·count))` with rehash-doubling at the 0.4
  max load (`(len+1)·5 > nbuckets·16`, 5/16 = 1/(8·0.4) at 8 slots per bucket)
  (fresh slab ranges at the tail; old ranges reclaimed at reset — a ≤2×
  transient).
- **The finalize intern memo.** `ResolveMemo` maps `(intern word, tag)` to a
  byte range per finalize: each distinct string resolves through LMDB once
  and lands in the result buffer once (`dict_resolve` fires per miss, so the
  trace count is the distinct count). Cross-execution caching stays out — an
  unbounded-memory policy the measured problem never needed. (Literal
  resolution on the *input* side is the latch below — the two memos face
  opposite directions and share nothing.)
- **The literal latch.** The dictionary is append-only, so `str`-literal
  resolution is **monotone**: a hit is a hit forever, a miss may become a
  hit but never the reverse — and ids outlive the environment (never
  reused, never reclaimed; the accepted-leak axiom's second dividend). A
  resolved `PendingIntern` therefore rewrites its plan-template slot to its
  `Const::Word` once, permanently (the latch IS the rewrite — no parallel
  resolution state), decrementing the prepared query's pending-literal
  count; `literal_latch` fires once per distinct literal, ever. A miss
  stays live: the template keeps its bytes and re-checks each execution,
  with the miss semantics (`Eq` short-circuit, `Ne` sentinel) verbatim — a
  per-execution empty verdict, never a plan verdict: the miss and the fold's
  instance-independent refutation are two distinct constructors
  (`lean/Bumbledb/Exec/Rewrites.lean: EmptyAt`).
  When the count is zero and the query has no params of any shape,
  `resolve_filters` is **skipped entirely** — the resolved tables were
  written once and are final (one cold branch at rule entry). Sound
  because the prepared query owns its plan (`!Sync`, environment-instance-pinned)
  and generational immutability never invalidates a word. The latch writes
  fixed-size words into existing slots — the alloc gate's `literal-latch`
  scenario pins zero allocation across the crossing.

Prepare-time statistics live in `plan/selectivity.rs` (the distinct ladder:
key-exact, resident-image exact via `ImageCache::peek` — prepare never
builds — schema bounds, documented floors) and the DP's join steps multiply
per-binding fanout `rows / distinct(join field)` with key coverage pinning
fanout to 1.

**Estimator record (2026-07-12, scale-S read-family reports):** the observed
Plan introspection estimate/executed-actual factor is classed by query hypergraph, not
presented as one estimator-accuracy bound. Among profiled acyclic ledger and
calendar families the worst was 691.2× (`conflict_free`); the cyclic class was
4761.9× (`triangle`, its only member). The derivation is the three regenerated
scale-S reports at the repository's 2026-07-12 family roster. These numbers are
execution-work ratios: a node's `actual` is the next executed-node entry count,
or final sink emissions, after legal D2 cancellation. They are therefore not
pure denotation-cardinality error. The fixture
`cyclic_estimate_diagnosis_is_p3_not_a_domain_or_range_defect` separates the
premises: with exact resident distincts and a three-axiom closed domain, a toy
cycle's full-head estimates/actuals are `24/24, 192/192, 576/192` (P3's closing
two-variable independence error); its narrow projected head executes
`24/24, 192/24, 576/24`, with 21 emissions absorbed, because D2 stops existential
work. The closed-domain rung is applied correctly (P1), and no range exists in
the fixture (P2). Cyclic estimates are not governed by a fixed factor: they order
the exhaustive DP, and neither estimates nor their error affect correctness.
The damage bound is a plan-shape property, stated exactly (ruled 2026-07-23,
R19, correcting this record's earlier claim): a binary-shaped FJ plan carries
no AGM bound — its closing probe is a binary hash join and bounds nothing —
and the worst-case-optimal guarantee holds at and near the GJ end of the plan
spectrum, which the GJ split (§ planner) produces for cyclic rules. No
histograms or new tuning rung are earned by this diagnosis — the P3 ruling
re-affirmed on the estimate doctrine's grounds (§ planner): precision lives at
execution time, in GJ-shaped plans and dynamic cover choice.
