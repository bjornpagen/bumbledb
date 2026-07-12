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
   extensions encode once into sealed rows, σ-literal checks compile
   (`CompiledCheck`), statements into closed relations compile to their member
   word-sets (`Resolved::ClosedContainment` — the enforcement plan IS the answer
   set), the fingerprint pins it all.
3. **Prepare**: normalization, the chase (elimination and evaluation — closed
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

**Guard-probe point lookups.** A single-atom query whose bindings cover a key of the
relation (an FD statement, including the auto-key on fresh fields —
`30-dependencies.md`) or the full fact executes as: one `U`-guard (or `M`-membership)
LMDB get → one `F` fetch → decode. No images, no COLT, no plan search. This serves the
headline "point lookup by key" workload at O(log n), including immediately after a
commit (no rebuild cost).
**Decision.** **Alternative:** COLT-only ("the join engine is the only read path") —
lost because a fully-bound lookup through images pays an O(n) scan for a one-row answer
and loses the benchmark family outright; the paper itself lists index-blindness as an
open limitation (§6). **Reverses if:** never — the guards exist anyway (rule: every
mechanism names its reader; this is `U`/`M`'s read-side reader).

**Statically empty programs.** A program whose every rule the normalization
fold refuted on constants (`20-query-ir.md`, § normalization — mutually
unsatisfiable constant predicates) prepares to the `Empty` plan: the third
plan kind beside the guard probe and Free Join, classified once at prepare
like both. Execution binds params first — bind errors still surface, a
vacuous Allen mask param is rejected exactly as on a live plan — then
touches no images, binds no views, runs no join, and the result is the
empty buffer. EXPLAIN prints `access path: statically empty` plus each dead
rule's killing predicate; a dead rule inside a live program was deleted at
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
(`60-validation.md`). Candidate mechanism recorded for trigger day: **guard skip
scan** — `U` guards are already ordered composite keys of fixed per-statement
width, so a non-prefix guard lookup or a range scan under a low-cardinality
leading field (closed-reference discriminators) is servable with zero new structures by
cursor `set_range` prefix-hopping (O(distinct-leading-prefixes × log n)); not
applicable to interval stabbing, whose pointwise layout needs the coverage-walk
shape. Interval membership predicates lower to word comparisons over the
start/end column pair (`50-storage.md` image layout), and interval-pair
predicates classify through the configuration kernel (§ vectorized
execution — masks, not ops); both are the existing 8-byte shapes and no
new NEON widths exist. The membership kernel (`s ≤ t < e`, two unsigned
word compares) needs no ray awareness: a ray's end (`MAX` = ∞, the point-domain
law — `10-data-model.md`) is just the largest word, so `t = MAX−1` — the last
point — passes against `[s, MAX)` through the same two compares, and validation
has already rejected `t = MAX` (the ceiling is not a point), so the kernel never
sees it.

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
  step: iterate cover → probe siblings → **evaluate the node's residuals** → recurse.
  In vectorized execution, residuals run as batch survivor compaction after the probes.
- **Anti-probe filters** attach exactly as residuals do — to the earliest node at
  which every variable of the negated atom is bound — and evaluate as: probe the
  negated occurrence for any matching fact; a hit **rejects** the binding. The
  negated occurrence is never a cover, contributes no plan variables, and its COLT
  (or, when its bindings cover a key, its `U`/`M` guard — the same access-path
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
- **Plan** (§3.2): a list of nodes, each a list of subatoms; the plan partitions every
  positive atom occurrence's variables; per node, no two subatoms share an atom
  occurrence (validity quantifies over **occurrences** — self-joins are ordinary), and
  the cover set is every subatom whose variables are **exactly** the node's new
  variables.
  **Deviation from the paper's Definition** ("containing all new variables"): a
  subatom that also carries an already-bound variable is iterable per the paper, but
  under dynamic cover choice the executor would *rebind* the bound variable without
  re-checking the occurrence that bound it — wrong results on skewed data (found by
  audit, demonstrated with a triangle query, pinned by a regression test). Restricting
  covers to exactly-the-new-variables loses nothing: every `binary2fj` node's opening
  subatom qualifies (its variables are exactly the remainder), and GJ-style
  single-variable covers all qualify. The alternative — equality-checking a mixed
  cover's old variables per iterated entry — buys generality no plan shape here needs.
- **Execution** (§3.3): recurse node by node — choose a cover, iterate it, extend the
  binding, probe siblings, replace each occurrence's current trie, recurse; backtrack
  restores.
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
  always displaces an Estimate") iterated a 500-key forced map while a 7-row
  param-filtered view sat unforced beside it — the measured balance wrong-cover.
- **`binary2fj` + conservative `factor()`** (§4.1): exactly per paper, over the DP
  planner's left-deep output.

**`ValidatedPlan` contents** (the witness type execution trusts): atom occurrences with
field→column maps; the node list with subatom partitions; per-node cover sets; per-
occurrence trie schemas derived per §3.3; per-node residual **and anti-probe** lists;
per-atom filter lists; the binding-slot layout (below); and the
provably-distinct-bindings flag (below). Validated once at construction; nothing
downstream re-checks.

## Set semantics in the executor

Bindings are **VarId-indexed slot arrays**, written in place by the recursion and read
in place by sinks; plan variable order is therefore irrelevant to sinks. Slots are
words: an interval variable occupies two consecutive slots (start, end) and a
`bytes<N>` variable its ⌈N/8⌉ padded words — multi-word values enter seen-sets,
group keys, and probe keys as word tuples, the wordmap's native shape, and every
consumer walks widths rather than assuming one.

Two facts identical on all *bound* variables produce the same binding; the solution is a
**set** of bindings, so duplicates must collapse before folding:

- The **projection sink** dedups projected facts (its job anyway).
- The **aggregate sink** folds a binding only on first occurrence, using a seen-set of
  full binding tuples — the same arena-backed mechanism as projection dedup.
- **`CountDistinct`** folds through a per-group distinct-value set (one word per
  value — intern ids, encoded scalars, or interval words pairwise), arena-backed
  like the group map.
- **Arg-restriction (`ArgMax`/`ArgMin`)** is a group-state fold, not a
  post-materialization pass: per group the sink keeps the current extreme key and
  the set of surviving projected rows; a strictly-better key clears the set, an
  equal key inserts (ties are set-honest, `20-query-ir.md`), a worse key is a
  no-op. Memory is O(groups × ties), and ties are structurally rare (fresh keys
  cannot tie).
- **`Pack`** is a group-state fold with a **relation-shaped finalize**
  (semantics in `20-query-ir.md` § aggregation): per group the sink accumulates
  the claim list — `[start, end]` encoded word pairs appended raw, pooled by
  group index (the Arg row-set precedent, capacity retained across executions);
  finalize sorts each group's list by start word (`sort_unstable` — the in-place
  machinery, allocation-free; a pooled radix stays unearned until the bench
  shows the sort on a profile) and drives the shared segment sweep's
  (`interval/sweep.rs` — the coverage judgment's walk, `Pack`'s finalize is its
  second continuation) maximal-run emission: one head row per maximal segment.
  Identical and overlapping claims collapse in the sweep, never at fold time;
  memory is O(the group's claims) — retained high-water scratch under the
  allocation contract, gated like every sink pool. Like `CountDistinct` and
  Arg, the set-valued group state folds per row (no gather kernel or scan
  pushdown applies).
- **Elision optimization:** if every atom occurrence's bound fields cover a key of
  its relation (typical for ledger queries that bind fresh ids), distinct facts ⇒
  distinct bindings, and the plan carries a proof flag that lets the aggregate sink
  skip the seen-set entirely. Provable at plan time from the schema's FD statements —
  a representation-level fix, not a runtime branch per binding.
- **Rule-disjointness elision** (the exclusivity theorem's third consumer —
  `30-dependencies.md`): a multi-rule program's cross-rule dedup is deleted at plan
  time when the rules are **provably pairwise disjoint**
  (`plan/fj/provably_disjoint.rs`). **Witness form**: a relation R and a field f
  such that both rules bind a positive occurrence of R whose filters pin f to
  *different* concrete literals, **and** that occurrence's bound key columns flow
  to the same head positions in both rules — equal head rows would force the two
  pinned facts to agree on a key of R, one fact whose f cannot equal two different
  literals. The DU-arm union (one rule per `kind`) is exactly this shape.
  Conservative and pairwise: params, sets, and mixed constant forms pin nothing;
  any unprovable pair keeps the seen-set — correct first, elided when proven.
  Two consumers: the **projection sink** drops the cross-rule guard (its map
  drains per rule at re-aim; per-rule dedup stays, as the semantics require), and
  the **aggregate sink** composes the proof with per-rule distinct bindings and a
  head projection reading every slot (distinct bindings ⇒ distinct head tuples) to
  elide the union seen-set entirely — the same `seen: None` representation as the
  single-rule elision, no hot-loop branch. EXPLAIN names the proof:
  `disjoint_rules: proven (R.f)`. A test-only override forces the flag off and the
  differential corpus pins byte-identical results — the elision is never semantic.

**Deviation D2 (set semantics — replaces the old D2):** the paper is bag-semantic
(leaves may carry multiplicity, output is a tuple stream). We: sets everywhere; leaves
are membership; binding dedup as above; and the executor may **skip a plan suffix after
the first witness** when (a) the active sink is the projection sink and (b) the suffix
binds only variables outside the projection set — the emitted fact cannot change, so
the recursion unwinds on the sink's first-emit signal. The skip is **never legal under
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

A prepared query is a program — one head, a list of rules, each with its own
`ValidatedPlan` (the whole plan pipeline runs per rule at prepare). Execution runs the
rules **sequentially** into **one sink**: the sink resets once per execution, never
per rule, and its dedup machinery spanning rules is the *entire* implementation of set
union. **Union is not an operator** — no merge node, no concat-then-dedup pass exists
anywhere in the executor; disjunction at the top is the rule list, and what one sink
hearing several rules *means* under set semantics is exactly ∪. Inter-rule parallelism
is not attempted: it is inter-query parallelism's job (the concurrency contract below)
and stays a non-goal.

- **Dedup keys are head-shaped, never rule-slot-shaped** — the representation that
  makes cross-rule dedup work at all, since binding-slot layouts are per-rule. The
  projection sink keys the projected find tuple (head-shaped already); the multi-rule
  aggregate sink keys the **head projection** — per head position, the words the
  position reads from the rule's binding (group variables and fold inputs; the nullary
  `Count` contributes nothing), which is `20-query-ir.md`'s "aggregates read the head:
  the fold domain is the union of the rules' binding sets projected to the head".
  The single-rule aggregate keys the full slot array (its fold domain is the rule's
  distinct full bindings — the normative single-rule semantics, unchanged).
  Under the rule-disjointness proof (§ set semantics) the spanning guard is
  dropped: the projection map drains per rule and the composed aggregate
  seen-set is elided — a collision the theorem forbids needs no set to absorb it.
- **Per-rule re-aiming:** the sink's slot tables (projection slots; aggregate finds,
  group spans, head-projection spans) re-aim to each rule's binding layout at rule
  entry — head positions are fixed (arity, ops, widths, types), slots are the rule's.
  The shared maps (rows, groups, seen-sets, value sets) carry across rules untouched:
  the spanning is the point. Binding-slot scratch is shared across rules, re-sized to
  each rule's layout at rule entry; executor scratch stays per-rule (it is
  plan-shaped: slot maps, node buffers).
- **Params are query-global**: bound once, resolved into shared slots every rule
  reads; per-rule state is only what is plan-shaped (resolved filters, selections,
  the view memo). A rule whose `Eq`-anchored constant misses the dictionary
  short-circuits **that rule only** — a rule is one disjunct.
- **Guard-probe rules** union through the sink like any other rule; the direct
  no-sink decode lane applies only to the single-rule guard program (the union must
  hear every rule).
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
  ∪ first, maximal segments at finalize (`20-query-ir.md` § aggregation,
  "across rules `Pack` folds the union").

## Planner

**The chase: elimination and evaluation.** Placement: after normalization,
before statistics and the DP, **per rule and independently** — a union's rules
are independent conjunctive bodies, so the chase distributes over them with no
cross-rule state and no new theory, and a rule shrinking below its cover
requirements re-validates like any rule — one fixpoint over the occurrence
table's `Role` sum (`plan/chase.rs`) running two rewrites that expose each
other: **elimination** marks provably redundant positive occurrences
`Role::Eliminated(statement)`, and **evaluation** (`plan/chase/evaluate.rs`)
marks prepare-evaluable closed-relation occurrences `Role::Folded` — marks,
never removals, so occurrence ids never move. Elimination removes atoms that
statements prove redundant; evaluation removes atoms whose extension is
stage-0-known by *running them at prepare*: `Kind(id: k, mastered == true)` is
not a join to plan — it is a three-element id-set computed before the DP ever
sees the query, residual cost zero.

*Elimination.* An accepted containment
`A(X | φ) <= B(Y | ψ)` makes the query's join of `A` to `B` on X→Y redundant
when four conditions hold:

1. **Full-key join** — every X→Y position pair is join-covered, and every
   variable shared between the two occurrences pairs a statement position
   (uniqueness needs the whole key; the acceptance gate made Y a key of B, so a
   partial-key join refuses).
2. **B otherwise unused** — no non-Y field of B is projected, filtered, compared
   in a residual, or referenced by any other occurrence (anti-probe bindings and
   membership points included); B's own selections are a **literal subset** of ψ
   and the A occurrence's filters carry φ **literally** — (field, encoded
   literal) set containment, never inference.
3. **Variables join or die** — every variable of B is either unified with A's at
   an X→Y pair or dead in condition 2's sense.
4. **Scalar positions only** — an interval-typed pair refuses in v0: pointwise
   coverage proves covering facts exist, not a joinable equal fact. OPEN
   trigger: a census-style query that would benefit from interval-pair
   elimination — until one exists the refusal stands, like the range
   accelerator's trigger discipline.

Chains (`A<=B<=C`) close in the fixpoint; mutual `==` pairs stay acyclic by
support tracking (each elimination records its source, and a source whose chain
passes through the candidate is refused — a pair may not certify itself). Sound
here and nowhere like Postgres because no deferral modes exist: every readable
snapshot satisfies every accepted statement (`30-dependencies.md`), and Y's
key-ness maps the surviving binding set 1:1, so removal is result-identical
under both sinks — projection and aggregate alike. The marks' readers: EXPLAIN
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
2. `C` carries only Eq/range/Allen/membership filters over its own columns
   with prepare-resolvable constants. A param-bearing filter REFUSES the fold
   in v0 (the bind-time fold alternative is recorded; trigger: a measured win in
   the calendar-family profile); measure filters refuse too — their ray error
   is a per-execution error, and evaluation would move it to prepare.
3. `C` is not negated (negated atoms fold to the complement — below).

The fold evaluates `C`'s filters against the sealed extension rows at prepare
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
deleted at prepare exactly like a normalize-time death. No live `k` (a pure
constant gate): `|S| ≥ 1` deletes the atom outright and `|S| == 0` kills the
rule — but only a **var-less** gate may delete: a dead-but-bound variable
still multiplies an aggregate's fold domain (the binding set is over all query
variables — D2), so a var-binding guard refuses.

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
the fold refuses and the anti-probe stays.

EXPLAIN reports folds beside eliminations, off the `Role::Folded` marks — the
surviving set as **handles**, the vocabulary's names (the handle set IS the
payload): `folded: Kind{mastered == true} → {DirectPass, JudgedPass}` (negated:
`folded: !Kind{…} → {…} rejected`); the differential off-switch
(`with_chase_disabled`) covers the evaluator inside the same fixpoint, and the
dual-run corpus pins byte-identical results — the fold is never semantic.

**Rule subsumption, the restricted witness.** After elimination, if one rule's
normalized body equals a sibling's *modulo the filters elimination removed* —
identical participating atom multisets on identical head projection, with the
keeper's conditions a subset of the deleted rule's — then the keeper contains
the sibling in denotation (a body homomorphism at the identity variable
mapping) and the subsumed rule is **deleted** at prepare: classical UCQ
minimization, restricted to the cheap witness the DNF path actually produces
(a lowered `(φ ∨ true-by-elimination)` pair whose second disjunct's filter
rode the eliminated occurrence). The check is normalized-form containment
(`plan/chase.rs::subsume`), O(rules²) at prepare with rules ≤ 16, and nothing
recursive. **Refused, the general form:** full CQ-homomorphism minimization is
NP-hard, so the witness never searches variable mappings — `VarId`s must
already agree, which is exactly what DNF-cloned rules provide. Deleting a rule
never changes the head (the head-alignment invariant is re-checked after
deletion), a program shrunk to one rule sheds its union machinery like any
single-rule program, EXPLAIN reports deleted rules with the subsuming rule's
index (lowered-rule indices) beside the per-rule eliminated atoms, and the
differential off-switch covers both passes.

**Statistics** (all real, nothing else exists): exact per-relation row counts
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
- J covers a key of L: estimate = min(est(P), |R|) — each R row matches at most one
  prefix row, and each prefix row matches at most |R|; the min is the correct bound.
- Neither: estimate = |L| × |R| — **no estimate exists, so pessimism**, which pushes
  non-key joins last; that is the correct behavior, not a modeling failure.
|X| is the row count or the filtered-view survivor count. Negated occurrences enter
no estimate — they only shrink results, and the planner treats them as free filters
(pessimistic in the right direction).

**Search:** exhaustive DP over positive atom occurrences, **left-deep only**,
minimizing the sum of prefix estimates *including the base relation's rows* (counting
the root iteration breaks ties toward iterating the small side). The cap is 20
occurrences (a 2²⁰-state table, ~32 MB transient plus a 16 MB per-mask
prefix-variables memo; the cap is enforced at the validation boundary as a roster
item, alongside the 128-distinct-variable bitset cap — negated occurrences count
against the roster cap but not the DP state, since they never join). Then
`binary2fj`, then `factor()`, then plan validation into the witness.
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
across several waves: starting range 64–256, measured (OPEN, README). **Probing is
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
generics, never `dyn`. NEON (`cfg(aarch64)`, 128-bit = 2×u64) is confined to the
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
software-prefetch passes (`prfm`) between probe phase 1 and phase 2. Fold kernels
follow the **port-topology law** (measured): every flag-writing scalar op
(`adds/adcs/cmp/csel`) is confined to 3 of the reference core's 6 integer ALUs, so
exact scalar summation caps at ~2.8 flag-µops/cycle while NEON escapes the triad
and rides the 3×16 B load ports — dense exact sums measured 8.8 vs 4.0–4.6 rows/ns
at L1 (carry-counted u128 via `vcgtq_u64`), min/max 2.65× at every tier, with DRAM
converging all parallel kernels (~7.5 rows/ns single-core). Dense (stride-1) folds
therefore take NEON unconditionally; strided and gathered folds stay scalar until
measured (latency×MLP-bound — a different law). Deep-OoO scalar remains the shape
for irregular control flow —
the law is about reductions, not loops in general (`00-product.md` machine
model; unsafe policy there too). Columns are 128-byte-aligned SoA
with pitch-padded bases (`50-storage.md`). Scalar fallback everywhere, equal results by
test across batch sizes. **Vectorized execution is the default and only path** — a
scalar "mode" exists solely as the degenerate batch size where useful for testing; a
"vectorized mode" that wraps scalar loops without batching is the failure shape this
sentence forbids. Honest caveat, stated:
deep in the plan the batch source is the current subtrie, whose fanout on reference
walks is often 1–10 — large batches are reliably available only at the root;
cross-node-entry batch accumulation is future work, not assumed. **Reverses if:**
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

**COLT force is single-pass with chunked child lists:** forcing pushes each offset into
its key's child list, chunked (64 offsets per arena chunk, chained by chunk — bounded
pointer-chase, independent loads within a chunk), rather than the paper's growable
per-key vectors or a two-pass contiguous layout (which decodes and hashes every row
twice). **Deviation:** the paper's leaves are plain vectors; ours are
chunked. **Reverses if:** a force+iterate microbenchmark shows two-pass-contiguous
winning end-to-end.

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
sets); pools reach a fixpoint per **(data generation, parameter envelope)** —
once every parameter shape the app binds has been seen at its hottest
intermediates, every subsequent execution is allocation-free until the data
generation changes.

**CI gate protocol (the definition of "steady state"):** single-threaded harness,
two measured windows. **Steady state:** the prepared query executes N warmup runs
with parameters drawn from a fixed set and no intervening writes; then M measured
runs over the same parameter set assert **zero** allocator hits, arena growth
included (growth inside a seen envelope is a failure), result buffer
caller-provided. **High-water:** after warmup on the coldest parameter, a
parameter sequence of strictly increasing selectivity — each parameter binds a
strictly hotter key — asserts that allocations occur **only** on executions
setting a new intermediate high-water: every repeat of a previously-seen
parameter, immediate or later, is allocation-silent, and the window guards its
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

**Resource limits: none in v0, stated.** Dedup sets, group maps, and result buffers
grow with output; a pathological query can exceed the envelope and the OS is the
backstop. The scale axiom makes engine-imposed caps ceremony; revisit only on real pain.

## Deviation D1 — data source

*Paper:* relations are columnar in main memory. *We:* durable data lives in LMDB;
execution reads **full-width cached columnar images** built once per
`(relation, storage_tx_id)` and shared across read transactions (`50-storage.md`).
After warmup, execution runs in exactly the paper's environment. *Why:* LMDB is the
durable truth; at ≤1 GB the whole working set caches and the write design point
(≥100 reads/generation) amortizes builds to noise. **Reverses if:** traced rebuild cost
violates the latency budget despite the cache — then persist columns instead.

**The closed carve-out:** a closed relation's image is not built from LMDB at
all — it is *synthesized* from the theory's sealed extension into a
per-relation `OnceLock` slot outside the generation map, once per process,
never evicted, never rebuilt (`50-storage.md` § virtual relations). Execution
consumes it exactly like any image; only its source and lifetime differ.

## Observability

**EXPLAIN exists from day one** and is the debugging story. Mechanism — a
representation, not a mode: the executor is generic over a `Counters` trait;
the normal path instantiates `NoopCounters` (zero-sized, compiled to nothing — no
runtime branch, no hot-loop cost), and the EXPLAIN entry point instantiates the
counting implementation and **executes the query** (ANALYZE semantics), reporting **per
rule** the plan, per-node estimated vs actual cardinalities, residual and anti-probe
selectivity, cover-choice histograms (choices aggregated per node, not per entry),
and the chase's eliminated occurrences — read straight off the plan's
`Role::Eliminated` marks, each rendered with its licensing statement through
`schema/render.rs` (e.g. `eliminated: Grading via Grading(id | kind == Det) ==
Det(grading)`) — plus the **head-level union accounting**: per rule, bindings
emitted to the shared sink vs absorbed by the spanning seen-set (absorbed =
emitted − newly-seen: two O(1) reads per rule, no per-tuple cost; an elided
seen-set absorbs nothing by proof), — multi-rule programs — the
rule-disjointness line naming its witness (`disjoint_rules: proven (R.f)`,
or `unproven`), and the subsumption record: rules deleted at prepare, each
with its subsuming rule's index (`subsumed: rule 0 by rule 1`, lowered-rule
indices — the per-rule sections are the survivors). The obs registry mirrors it: one `RULE` span
per rule under the execute span (`rule_N` — the index rides in the name,
`MAX_RULES`-bounded), args (emitted, absorbed), populated on counted paths.
Output shape: OPEN. Release builds contain no other instrumentation: no per-tuple labels, no
always-on counters, no diagnostics allocation anywhere in the join loops.

## Measured mechanisms

Six measured decisions, enforced structurally by
`crates/bumbledb-bench/src/tripwires.rs` (never by wall clock):

- **Selection levels.** Every Eq-against-a-constant (literal or param — the
  same machine) lowers into `PlanOccurrence::selections` and becomes a
  prepended single-column COLT trie level, probed per execution with the
  resolved word (`Colt::select`). Force is O(view) once per generation, probes
  O(1) per param; views carry only residuals (ranges, Ne, `FieldsCompare`).
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
  7-row view — the measured wrong-cover this rule exists to prevent.
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
  with the miss semantics (`Eq` short-circuit, `Ne` sentinel) verbatim.
  When the count is zero and the query has no params of any shape,
  `resolve_predicates` is **skipped entirely** — the resolved tables were
  written once and are final (one cold branch at rule entry). Sound
  because the prepared query owns its plan (`!Sync`, env-instance-guarded)
  and generational immutability never invalidates a word. The latch writes
  fixed-size words into existing slots — the alloc gate's `literal-latch`
  scenario pins zero allocation across the crossing.

Prepare-time statistics live in `plan/selectivity.rs` (the distinct ladder:
key-exact, resident-image exact via `ImageCache::peek` — prepare never
builds — schema bounds, documented floors) and the DP's join steps multiply
per-binding fanout `rows / distinct(join field)` with key coverage pinning
fanout to 1 (measured worst est/actual across the ledger families: ≤ 3.3×,
against five orders of magnitude for naive row-product estimation).
