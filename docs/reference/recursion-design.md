# The recursion design — a paper proof with a seam ledger

> **Status: UNBUILT.** This document specifies a feature the engine does
> not have. It is **subordinate to the architecture chapters**: they
> describe what is; where this paper and a chapter disagree, the chapter
> wins and this paper is the thing amended. It lives in
> `docs/reference/` deliberately — it must never masquerade as the
> record of a built mechanism. Nothing here licenses code: the recorded
> refusal stands (`docs/architecture/20-query-ir.md`, the recursion
> refusal beside the rules shape), and the no-speculative-structure
> refusals in the crucible set stand with it — no `AtomSource`, no
> `PredId`, no strata, no frontier hook exists in the ENGINE tree,
> because a one-inhabitant sum is a dead arm in every consumer. The
> cuts live here instead. **The SPEC side landed 2026-07-14**: §1's IR
> cut, §2's stratification and safety theorem, §3's operator-level
> delta equivalence, and §5's round loop are now Lean theorems
> (`lean/Bumbledb/Exec/Fixpoint.lean`; the walls in
> `lean/Bumbledb/Countermodels.lean`) — there the sums land inhabited,
> because the fixpoint semantics consumes them. §4 and §5's mechanism
> rows (images, watermarks, budgets, plan variants) stay paper, per the
> mechanism fence.

This paper is written against the **post-crucible shapes**: PRD 04's
`Predicate`/`PredicateColumn` and `ValidatedQuery { predicate, rules }`
(the one signature derivation), PRD 05's `ConditionTree`/
`Rule.conditions` vocabulary, and PRD 08's `ClassifiedComparison`
witness discipline. File paths cite the crucible baseline; where a path
has moved, the mechanism name is authoritative and the executor
re-locates it (the crucible README's own rule).

**What the paper buys.** "Prepared for recursion" becomes a checkable
claim instead of a mood: every future diff is mapped onto a named,
existing seam with an estimated blast radius (§9, the seam ledger), and
the claim the ledger proves is that the post-trigger campaign is a
**6–8 PRD set**, not a comptime-sized one. The trigger is PRD 06's,
recorded in `20-query-ir.md` with the closure-idiom recipes as its
working alternative; its three clauses map onto this paper as:

- **Unbounded or large depth** (the host loop's per-round query cost
  stops being noise): activates the full ledger, rows R1–R16.
- **Closure composed into a larger plan** (the reachable set must join
  further inside one plan for performance): activates the full ledger;
  rows 1–2 are where composition becomes *representable* at all.
- **The chain-window class** (interval intersection along paths): NOT
  covered by this design — §8 fences it as the open theory item. A
  trigger firing on this clause alone re-opens theory before it
  re-opens engineering.

## 1. The IR cut

A query is already a non-recursive Datalog program — one head, a rule
list, one step short of the fixpoint because a rule's head is never a
body atom (`20-query-ir.md` § the query shape; the recursion refusal
names this exact landing pad). The cut takes that step and nothing
else:

```rust
Program {
    predicates: Vec<PredicateDef>,   // ≥1, ≤ MAX_PREDICATES (16); PredId = index
    output:     PredId,              // the program's answer predicate
}
PredicateDef {
    head:  Vec<HeadTerm>,            // today's Query.head, verbatim
    rules: Vec<Rule>,                // today's Query.rules, verbatim
}
```

The **degenerate form is today's `Query`**: a one-`PredicateDef`
program with no `Idb` atom is field-for-field the current shape, and
the embedding precedent is `Query::single` (the single-rule query
embeds the conjunctive query unchanged — `20-query-ir.md`). The pure-
data surface ruling holds unamended: plain owned data, no behavior, no
builder; the trust-boundary law (`20-query-ir.md` § validation
boundary) extends its roster rather than growing an exception.

**PRD 04's type IS the IDB typing rule, verbatim.** Validation derives
one sealed `Predicate` per `PredicateDef` — the same one derivation, in
the same `ir/validate/finds.rs` territory, quantified over predicates
instead of run once — and the per-rule alignment rule restates
unchanged: *every rule derives its predicate*. The validated witness
becomes `ValidatedProgram { predicates: Box<[(Predicate,
Box<[RuleWitness]>)]>, output: PredId }`. The public-IR fence survives:
`Predicate` still never appears in `ir.rs` (PRD 04's criterion); the
IR-side reference is a bare `PredId`. The fence sentence on
`Predicate`'s doc — *referenced by NOTHING* — is the one sentence this
design rewrites, to *referenced only by `PredId`, from inside the same
`Program`*. That rewrite is what "the trigger firing" means; it goes
through this ledger, not around it.

**Atom sources — the deferred one-line sum:**

```rust
AtomSource = Edb(RelationId) | Idb(PredId)
```

`Atom.relation: RelationId` becomes `Atom.source: AtomSource`. An
`Idb` atom's bindings address **head positions**: `FieldId(i)` is the
target predicate's column `i`, typed by its sealed
`Predicate.columns[i].ty`. This is not a pun — `FieldId` is already
positional, never nominal (hard structural typing; the closed
relation's synthetic id field at `FieldId(0)` is the standing
precedent), and the membership typing rule (`20-query-ir.md`) reads
through it unchanged: an interval-typed predicate column participates
in point membership exactly as an interval field does.

**The consumer list, with the estimated diff each pays** (why the sum
stays deferred — five consumers, none free):

| consumer | mechanism | change | est. |
|---|---|---|---|
| validation typing | the per-rule bivalent anchor fixpoint (`ir/validate.rs`, `validate/context.rs`) | `Idb` anchors resolve against `Predicate.columns` instead of the schema relation; roster items for unknown `PredId` and arity mismatch | ~40 lines |
| normalize | the occurrence table (`ir/normalize/normalize.rs`) | `Occurrence.relation` → `source`; filters and residuals are slot/word-shaped and indifferent | ~30 lines |
| grounding | elimination + evaluation (`plan/ground.rs`, `plan/ground/evaluate.rs`) | one guard each: both rewrites refuse `Idb` occurrences — statements quantify over stored relations *permanently* (`30-dependencies.md`, the stored-relations decision record — undecidable predicate containment is the rationale, so this guard discharges a law, not a convenience) and sealed extensions exist only for closed relations, so elimination has no licensing statement and evaluation no stage-0 rows | ~10 lines |
| view binding | the per-occurrence bind loop (`api/prepared/run_join.rs`, `view_memo.rs`) | `Idb` occurrences take the transient-image bind (§4) instead of `ImageCache::get_or_build` | ~40 lines |
| statistics | the selectivity ladder (`plan/selectivity.rs`) | `Idb` occurrences pin no row counts and cost on the ladder's floors (§3); the staleness surface already knows the shape — negated and grounding-discharged occurrences carry no pin today (`70-api.md` § transactions) | ~30 lines |

## 2. Stratification

**The predicate dependency graph.** One node per `PredId`; an edge
Q → P for every body atom of a P-rule over Q, labeled positive,
negated, or fold-input (the atom's variables feed an aggregate in P's
head). The graph is bounded by `MAX_PREDICATES` × per-predicate
`MAX_RULES` — boundary guards in the roster, documented at their
definitions like every cap (`20-query-ir.md` § validation boundary).

**SCC condensation.** Strata are the condensation's topological order,
computed at validation with an explicit work-list (iterative Tarjan) —
the trust-boundary law's convention: the nesting-depth judge is already
iterative *because* the walks it guards recurse, and the strata judge
follows it. Mutual recursion within one SCC is ordinary: the stratum's
predicates iterate jointly under one round loop (§5).

**The refusals, typed.** Through a cycle, refused with named errors:

- `ValidationError::NegationThroughCycle { pred, via }` — a negated
  atom whose target shares the atom's own SCC. Negation *of* lower
  strata is legal: a lower stratum is a finished set before this
  stratum's operator runs, so the operator stays monotone.
- `ValidationError::AggregationThroughCycle { pred, via }` — a fold in
  a head whose rule body reads the head's own SCC. Aggregation *of*
  lower strata is legal for the same reason: an `Idb` atom under a
  fold reads a finished set.
- `ValidationError::MeasureInRecursiveHead { pred }` — a `Measure` find in a
  recursive predicate's head.
  Two derivations. First, the safety theorem below requires heads to
  project **bound** variables, and the measure is a computation, not a
  binding — value creation in a head exits the theorem (it is §8's
  whole subject). Second, the **error-timing ruling**, inherited from
  the grounding-evaluator's measure refusal (`plan/ground/evaluate.rs`:
  a fold must not move the ray error across stages): `MeasureOfRay` is
  the engine's one runtime type error, and inside a fixpoint the round
  at which a ray reaches a recursive head depends on iteration order —
  the same store would error after differing partial work. The typed
  rejection keeps the error surface deterministic. The measure over a
  *lower* stratum from a non-recursive head stays legal: it evaluates
  after the fixpoint is a set.

**The safety theorem** (why the budget in §5 bounds cost, never
termination): under set semantics (no multiplicities to diverge),
with every head position projecting a bound variable (no value
creation — the measure refusal above; recursive heads carry no folds by
the cycle refusal), every bound variable draws its words from stored
columns or lower-strata predicates — finite sets. Each predicate is
therefore a subset of a finite product of active-domain words, the
per-stratum immediate-consequence operator is monotone on a finite
powerset lattice, and the least fixpoint is reached in finitely many
rounds, each round finitely many Free Join executions. Termination is
a theorem of the roster, not a runtime hope. The roster is not a new
law: it is the creation quarantine (`20-query-ir.md` § the creation
quarantine — a created value never re-enters a derivation) restated
for fixpoint topology — one law, two enforcement sites.

## 3. The delta rewrite

**Per recursive rule, k plan variants.** A rule of a stratum-σ
predicate with k ≥ 1 body atoms over stratum-σ predicates prepares k
variants: variant *i* marks recursive atom *i* the **delta occurrence**
(bound per round to Δᵣ₋₁'s image) and every other same-stratum atom the
**accumulated** predicate. Cross-variant and cross-round re-derivation
is absorbed by the predicate's spanning seen-set — the same argument
that makes D2's late cancellation harmless (`40-execution.md` § the
paper's core: a re-emitted row is a row the seen-set already holds;
under set semantics over-derivation skews cost, never results).
**Decision:** no new/old split. **Alternative:** classical semi-naive
bookkeeping (P_old/P_new per atom). **Why it loses:** the split is a
second dedup structure standing beside the sink's, and the sink already
IS the set (§ the rule loop's union law). **Reverses if:** a measured
closure workload shows re-derivation cost above the split's
bookkeeping — the same seam either way; the ledger row does not move.

**Prepared once, on the ladder's floors.** All k variants run the
ordinary per-rule pipeline at prepare (`api/prepared/build.rs`: the
whole planning pipeline already runs per rule). Δ and accumulated
cardinalities are prepare-unknowable, exactly like param-bound filter
survivorship, so those occurrences cost on the selectivity ladder's
documented floors — the **param-plan precedent verbatim**
(`40-execution.md` § statistics, the carve-out). No round ever
re-plans: pin-at-prepare, and pins acknowledge, never re-fix (the
staging law's clause).

**Typed variants, on the parse-don't-classify shape.** One prepare-time
parser mints a closed sum per recursive rule —

```rust
RecursiveRule { variants: Box<[DeltaVariant]>, /* rule scratch */ }
DeltaVariant  { delta: OccId, plan: ValidatedPlan }
```

— minted exclusively by that parse and consumed totally by the driver,
which is `ResolvableFilter`'s discipline exactly
(`plan/ground/evaluate.rs`: minted by `parse_resolvable`, consumed
totally by `surviving_ids`) and PRD 08's `ClassifiedComparison` law
(classification carries its proof; no consumer re-derives).
`PreparedRule` grows one arm — `Recursive(RecursiveRule)` beside
`FreeJoin`/`KeyProbe` (`api/prepared.rs`) — and the arm lands **inhabited
on the day it lands**, never before: the one-inhabitant refusal is why
this paper exists.

## 4. Transient images

**Per-iteration delta images are built from tuple buffers on the
`synthesize_closed` precedent.** The image machinery is already
source-agnostic after decode: allocate → decode plan → fill → seal,
and `synthesize_closed` (`image/build.rs`) proves a non-LMDB source
rides it whole — the sealed extension synthesizes through exactly the
plan a stored fact would, column layout, stride padding, and cardinality
counters untouched. The delta builder is the same shape with a cheaper
source: the round's frontier rows are already encoded column words in
the seen-set (§5), so the build is a columnar transpose of a dense
suffix, no fact-bytes decode at all. Accumulated-predicate images are
the same builder over the whole seen-set, rebuilt per round or grown
in place — an implementation choice priced inside one ledger row.

**NEVER memoized — the design's load-bearing invariant-preservation
argument.** The view memo's soundness axiom is *"generational
immutability makes a view valid for its whole generation"*
(`40-execution.md` § the view-memo LRU). A delta image is valid for
**one round of one execution** — a lifetime the generation vocabulary
cannot express — so delta images live entirely outside the axiom's
machinery:

- never in the `ImageCache` generation map (`image/cache.rs` diff:
  **zero lines** — and the closed carve-out's `OnceLock` slots already
  prove images can live outside the map);
- never parked in the view memo: `Idb` occurrences bypass `memo.bind`
  and take a per-round `Colt::reset(view)` — the ordinary miss path in
  `run_join.rs`, survivor buffers recycled through the existing
  `spare_buffers` ping-pong;
- never pinned by `PreparedQuery::staleness`: the accumulated set is
  execution-local truth; there is nothing to drift against.

Every generation-keyed mechanism — cache eviction, memo reaping, the
sentinel generation, the staleness signal — **never learns recursion
exists**. That is the argument that this campaign cannot corrupt the
engine's warm-path soundness story, and it is the invariant column's
most repeated entry in §9.

Delta buffers and per-round images join the allocation contract as
retained-capacity pools on the prepared query: the monotone high-water
claim is unchanged in kind, its fixpoint now per (data generation,
parameter envelope, **iteration-shape high-water**).

## 5. The driver

**Per stratum, in condensation order; semi-naive over the existing
run-rule machinery.** Round 0 runs the stratum's non-recursive rules
(base cases) through the rule loop verbatim — rules sequentially into
one sink, the sink's dedup spanning them (`40-execution.md` § the rule
loop). Round r ≥ 1 runs each recursive rule's k variants through
`run_join` with the delta occurrence bound to Δᵣ₋₁ and same-stratum
occurrences bound to the accumulated images (§4). An empty Δ ends the
stratum. Interior (non-output) predicates own a projection-shaped
seen-set each — they are projection-shaped by construction (§2 keeps
folds out of recursive heads) — while the output predicate keeps the
ordinary head-owned sink; **union stays the sink and only the sink**:
no merge node, no frontier queue, no worklist structure exists,
because the disjunction-is-data law already paid for this.

**The frontier IS the sink's seen-set with a per-round watermark — the
one future hook, one method.** `WordMap` preserves insertion order
with dense O(len) iteration (the dense rule, `exec/wordmap.rs`), so
round r's frontier is exactly the entries in `[watermark_r, len)`: one
`usize` read per round and a dense-suffix walk into the next delta
image. The hook is a cold `drain_since(watermark)`-shaped reader on
the seen-set owners (`ProjectionSink`/`AggregateSink`, `exec/sink.rs`)
— no flag, no branch, no state on the emit path; a non-recursive
program cannot observe it. Dedup keys stay head-shaped (the rule
loop's representation law), which is precisely what makes the frontier
readable at all. D2's suffix skip stays per-rule and within-round —
its legality condition (the emitted fact cannot change) is
round-local and unchanged.

**The budget — the one new trust boundary.** Termination is §2's
theorem, but the fixpoint's *size* is data-shaped: a foreign query may
legally demand a quadratic closure. The driver carries an
iteration/tuple budget with a documented default and the typed
execution error

```rust
Error::FixpointBudgetExceeded { stratum, rounds, tuples }
```

— an execution error on `MeasureOfRay`'s model (aborts the query, the
snapshot stays usable, payload is ids and counts, never strings). This
deliberately and narrowly amends the recorded v0 stance "resource
limits: none" (`40-execution.md`): the OS-backstop argument priced one
join's envelope, not an unbounded round count crossing the trust
boundary. Policy stays host-owned — the staleness doctrine verbatim:
the engine ships the typed condition, never a threshold loop; the
default exists so the boundary is never unguarded.

**Observability** rides the existing `Counters` seam and the `rule_N`
span precedent (`40-execution.md` § observability): per-stratum,
per-round delta sizes and absorbed counts on counted paths only;
`NoopCounters` keeps the release path at zero.

## 6. The oracles

**The naive model: the ten-line naive fixpoint.** Per stratum, loop —
evaluate every rule against the current predicate sets by the existing
nested-loop evaluator (`bumbledb-bench/src/naive/query.rs`), union,
stop on no change. Naive, not semi-naive, deliberately: the model's
correctness is definitional and its independence is the recorded trust
root (the independence law — the model imports the engine's types
only; the crucible README's no-fuzzing-the-judge refusal leans on
exactly this simplicity).

**SQLite: linear recursion only.** A self-recursive, single-recursive-
atom, projection-shaped predicate translates to `WITH RECURSIVE` under
`UNION` (SQLite's `UNION` is ∪ under the `DISTINCT` discipline the
translator already enforces — `60-validation.md` § the value mapping).
Non-linear rules, mutual recursion, and aggregation over recursive
strata join the enumerated `Inexpressible` set
(`translate::sqlite_expressible`), counted and reported, never
silently skipped — the **ψ-subset division-of-labor precedent
verbatim**: `Pack` and the write judgments are already naive-only, and
the harness's expressibility gate is the mechanism that keeps
naive-only honest.

**The generator: the recursive-shape arm.** Beside
`querygen/shapes_rules.rs`: bounded predicate counts (2–3), bounded
recursive atoms per rule (1–2), graph-shaped corpus data with
closure sizes bounded by construction — the cost-bound rule's sibling
(`60-validation.md`: bounding reachable closure size is the same duty
as bounding reachable sums). Coverage-contract rows asserted per run:
linear self-recursion; a mutual pair; a non-linear rule; negation of a
lower stratum; a fold over a recursive predicate from a higher
stratum; the budget trip (constructed, not hoped for); and the
empty-Δ-at-round-1 boundary. Draws ride `corpus_gen::rng`'s entropy
seam unchanged.

**The shipping law, restated: the oracle lands before the evaluator.**
The post-trigger campaign's first engine-adjacent PRD is bench-side —
the naive fixpoint, the translator rows, the generator arm, and
hand-verified closure goldens over a fixed tree — so the differential
exists on day one of evaluator work. The engine was built
measurement-first and is being made adversary-first; recursion arrives
under both disciplines or not at all.

## 7. The notation

**Named heads; bare rules remain the output predicate.**

```text
path(x, z) | edge(x, y), path(y, z);
path(x, z) | edge(x, z);
(x, z)     | path(x, z), Root(id: x);
```

Grammar delta over `20-query-ir.md` § the query notation: `rule :=
[name] '(' head ')' '|' body ';'`, and a body atom may name a
predicate where it names a relation. **Text-level backward
compatible:** every existing query parses unchanged and denotes what
it denoted — a program whose every rule is bare is today's one-
predicate program.

Predicate names are a text-layer sidecar, exactly as variable names
are (`20-query-ir.md`: ids are all the IR carries): the engine sees
dense `PredId`s; names never enter the IR, the fingerprint, or any
engine surface. Resolution is macro-local — predicate names cannot
ride the theory's id constants because they are query-local, not
schema emissions — so a typo is a macro error at the literal, and a
predicate spelled like a relation is a macro error too (unwritable
ambiguity, the punning law's discipline). The renderer prints interior
predicates with synthesized names (`p0`, `p1` — the `v{id}`/`?{id}`
convention extended) and output rules bare; the round-trip goldens
(`bumbledb-query/tests/notation.rs`, `render(lower(text))` byte-equal)
extend with named-head cases, and the anti-drift discipline — one
grammar, three consumers, renderer as spec — inherits unamended.

## 8. The research item, honestly fenced: chain windows

The chain-window class — PRD 06's third trigger clause — wants *the
interval over which an entire path holds*: mandate chains, hierarchy
validity, `path(x, z, w) | edge(x, y, w₁), path(y, z, w₂), w = w₁ ∩ w₂`.
The head position `w` is **created** (`[max(s₁, s₂), min(e₁, e₂))` —
max/min over the endpoint lattices), not bound — which exits §2's
safety theorem at its premise and therefore exits this design.
**The ledger's 6–8 PRDs do not buy this class.**

Recorded for the day it is sighted — the termination argument sketch:
the intersection's endpoints are *selected* from stored endpoints,
never invented, so the representable windows over a finite store are a
finite set (≤ n² endpoint pairs) and the immediate-consequence
operator stays monotone on a finite lattice — termination survives
value creation of this lattice-closed shape. That observation is now
the architecture's standing fence for every future interval operator
(`20-query-ir.md` § the creation quarantine): lattice-closed,
endpoint-selecting operations are the only candidates ever;
endpoint-inventing ones (shift, widen, arithmetic on bounds) are
refused categorically. What keeps it open
anyway: (a) **emptiness** — `max(starts) ≥ min(ends)` must kill the
tuple by a typed rule rather than construct the unrepresentable
`start ≥ end` interval (the constructor invariant is a boundary law,
`10-data-model.md`); (b) **the result shape** — per (x, z) the honest
answer is a *set* of maximal windows, which is `Pack`-shaped: a
relation-shaped fold inside a fixpoint, composing exactly the two
things §2 keeps apart; (c) **the frontier key** must include the
created column, growing the fixpoint from |reachable pairs| to
|reachable pairs × windows|. Each is answerable; none is answered
here. Until then the closure idiom computes chain windows the honest
way — the window carried in the host's frontier set, one intersection
per hop (the cookbook's closure recipe is the pattern's home).

**A standing ruling, re-examined as instructed:** the
`NormalizedQuery.dead` rendered-string channel (`ir/normalize.rs`)
gains **no programmatic consumer** in this design — a dead rule inside
a fixpoint deletes at prepare exactly as today, and the driver never
reads the string. The rendered-string ruling stands.

## 9. The seam ledger

Every cut above, on the seam it lands on. Diff sizes are estimates
against the real files at the crucible baseline (`git` verified), not
invented; "invariant preserved" names the law the diff must leave
untouched — the review criterion for the future PRD, written today.

| # | cut | seam (post-crucible shape) | files | est. diff | invariant preserved |
|---|---|---|---|---|---|
| 1 | `Program` wrapper + `PredId` | the pure-data IR; the degenerate-embedding precedent (`Query::single`) | `crates/bumbledb/src/ir.rs` | +90/−10 | the surface ruling (plain owned data, no behavior); degenerate form ≡ today's `Query` |
| 2 | `AtomSource` one-line sum | `Atom.relation`'s five consumers (§1's table) | `ir.rs`, `ir/validate/context.rs`, `ir/normalize/normalize.rs`, `plan/ground.rs`, `plan/ground/evaluate.rs`, `api/prepared/run_join.rs`, `plan/selectivity.rs` | +150 | occurrence ids never move (`Role`'s law); statements speak only about stored relations, so both grounding rewrites refuse `Idb` in one guard |
| 3 | per-predicate signature | PRD 04's one derivation (`ir/validate/finds.rs` territory), quantified over predicates | `ir/validate.rs`, `ir/validate/finds.rs` | +60 | one signature, one derivation; the head-alignment rule per predicate; `Predicate` still absent from `ir.rs` |
| 4 | stratification + safety roster | the validation boundary's iterative-judge convention (the nesting cap's precedent) | new `ir/validate/strata.rs`, `ir/validate.rs`, `error.rs` | +280 | roster exhaustive, no panic from IR data; the four named errors (§2); caps documented at definition |
| 5 | delta-variant plans | the per-rule prepare pipeline + the selectivity ladder's floors (the param-plan precedent) | `api/prepared/build.rs`, `plan/selectivity.rs`, `plan/planner.rs` | +220 | pin-at-prepare — no round re-plans; the staging law |
| 6 | typed variant sum | the parse-don't-classify discipline (`ResolvableFilter`, `ClassifiedComparison`) | `api/prepared.rs` (`PreparedRule`), `api/prepared/build.rs` | +120 | witnesses minted once, consumed totally; the arm lands inhabited (the one-inhabitant refusal) |
| 7 | transient delta images | `synthesize_closed`'s pure synthesis path | `image/build.rs`, `image/view.rs` | +100 | the generation axiom — a view is valid for its whole generation — untouched because delta images never enter cache or memo; `image/cache.rs` diff = 0 lines |
| 8 | `Idb` view binding | `run_join`'s per-occurrence bind loop + `spare_buffers` ping-pong | `api/prepared/run_join.rs`, `api/prepared/view_memo.rs` | +80 | memo soundness untouched (`Idb` bypasses `memo.bind`); warm re-binds allocation-silent via recycled buffers |
| 9 | the per-stratum driver | the rule loop (one sink hears every rule; sink resets once per execution) | new `api/prepared/fixpoint.rs`, `api/prepared/execute.rs` | +260 | union is the sink — no merge node, no worklist; D2 skip per-rule, within-round |
| 10 | frontier watermark | `WordMap`'s dense insertion-order rule | `exec/wordmap.rs`, `exec/sink.rs`, `exec/sink/projection/sink.rs` | +60 | zero hot-path cost when unused (a cold method, no emit-path branch); dedup keys stay head-shaped |
| 11 | the budget | the typed-execution-error convention (`MeasureOfRay`'s model) | `error.rs`, `api/prepared/execute.rs` | +50 | error payloads are ids and counts; policy host-owned (the staleness doctrine); the v0 no-limits stance amended for fixpoints only, recorded |
| 12 | plan introspection + stats | the `Counters` seam; the `rule_N` span precedent | `exec/introspection.rs`, `api/stats.rs`, `obs.rs` names | +120 | `NoopCounters` compiles to nothing; no always-on instrumentation |
| 13 | naive fixpoint oracle | the naive model's definitional evaluator | `crates/bumbledb-bench/src/naive/query.rs` | +80 | the independence law (types only); the trust root stays definitional — naive, never semi-naive |
| 14 | SQLite lane + gate | the IR→SQL translator + the enumerated `Inexpressible` set | `bumbledb-bench/src/translate/query.rs`, `translate/types.rs`, `translate/goldens.rs` | +150 | nothing silently skipped; typed-identity error parity; linear-only, division of labor recorded |
| 15 | generator arm | the coverage-contract convention + the entropy seam | new `bumbledb-bench/src/querygen/shapes_recursive.rs`, `querygen/coverage.rs`, `querygen/shapes.rs` | +260 | coverage asserted per run; the cost-bound rule (closure size bounded by construction); `corpus_gen::rng` untouched |
| 16 | notation + renderer | the render grammar; round-trip goldens; the macro's local names | `ir/render.rs`, `crates/bumbledb-query/src/lib.rs`, `bumbledb-query/tests/notation.rs`, `docs/cookbook.md` | +200 | one grammar, three consumers, byte-exact goldens; bare rules = the output predicate (text-level compatibility); names never fingerprinted |

Total ≈ +2,300 lines across engine, bench, and macro crates; the
largest single-file diff ≈ 300 lines; **zero rows invent a mechanism
without a named precedent**, and two rows are proofs of absence
(`image/cache.rs` at zero lines; the fingerprint pin untouched —
queries are outside the declaration surface the fingerprint hashes,
policy 7's criterion discharged by representation).

**The PRD grouping the ledger proves** (rows → PRDs, honoring the
shipping law's order):

- **R1 — the cut**: rows 1–3 (IR wrapper, `AtomSource`, per-predicate
  signature).
- **R2 — the fence**: row 4 (stratification, the safety roster).
- **R3 — the oracles**: rows 13–15 (model fixpoint, translator,
  generator — lands before any evaluator PRD).
- **R4 — the rewrite**: rows 5–6 (variant plans, the typed sum).
- **R5 — the images**: rows 7–8 (transient synthesis, `Idb` binding).
- **R6 — the driver**: rows 9–11 (the loop, the watermark, the
  budget).
- **R7 — the surfaces**: rows 12 + 16 (plan introspection/stats, notation,
  cookbook and chapter amendments).

Seven PRDs; six if R4 and R5 merge (they share `build.rs`), eight if
R7 splits its observability and notation halves. **The claim: the
post-trigger campaign is a 6–8 PRD set** — every diff on a named seam,
every seam with its invariant stated in advance, and the chain-window
class honestly outside it.
