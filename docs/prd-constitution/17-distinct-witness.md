# PRD 17 — The distinct witness: an elision stops riding a bool

**Depends on:** Phase B/C complete (plan/fj quiet).
**Modules:** `crates/bumbledb/src/plan/fj/provably_distinct.rs`
(`fn provably_distinct(..) -> bool` :25), its caller in the fj
validate/build path, the aggregate-sink construction that consumes the
answer (the dedup-regime decision), `plan/fj/provably_disjoint.rs`
(the precedent — `DisjointWitness`), introspect if the elision is
surfaced.
**Authority:** the audit's evidence-object discipline + the direct
verification: the RULE-disjointness proof already earned a typed
`DisjointWitness`, but the DISTINCT-BINDINGS proof — which licenses
the aggregate sink to SKIP ITS SEEN-SET, a semantics-bearing elision —
is a bare `bool` threaded through construction. A refactor that
mis-wires one boolean silently converts set semantics into bag
semantics for one query shape. That is precisely the class of state
this codebase makes unrepresentable.
**Representation move:** the proof becomes a type; the seen-set-free
sink constructor demands it.

## Context (decided shape)

```rust
/// Proof that distinct facts imply distinct bindings for this rule:
/// every participating occurrence's bound fields cover a key of its
/// relation (the distinct-bindings elision law, 40-execution). Minted
/// ONLY by provably_distinct's proving path; carrying one is the
/// licence to build an aggregate sink without a dedup seen-set.
pub(crate) struct DistinctWitness(());
```

- `provably_distinct(..) -> Option<DistinctWitness>` (None = not
  proven; the word "false" disappears from the signature).
- The sink-construction seam splits on it structurally: the
  seen-set-free aggregate path takes `DistinctWitness` by value; the
  dedup path takes nothing. No boolean survives between the proof and
  the consumer (grep the thread).
- Mirror `DisjointWitness`'s conventions exactly (visibility,
  placement, introspect surfacing if the sibling has one — the
  stats/introspect layer reports the disjoint proof; the distinct
  proof reports the same way).
- The union-elision REFUTATION record (the reverted rsvp_union
  optimization in 40-execution) is untouched — different elision;
  this PRD's doc amendment cross-references the two so nobody
  conflates them again.
- **The bool-licensed-rewrite sweep (brief B5, approved):** after the
  witness lands, sweep the plan/prepare layers for any OTHER
  semantics-bearing decision still licensed by a bare `bool` or
  unwitnessed flag (grep `-> bool` + boolean struct fields consumed by
  plan construction; judge each: semantics-bearing → witness it or
  record a refusal with reason; heuristic-only → leave, list). The
  sweep's table lands in this file's Results — the criterion is that
  every elision/eliding rewrite names its evidence type, completing
  the proof-carrying-optimizer claim with what already exists
  (DisjointWitness, the sealed ResolvableFilter/latch specs, KeyProbe
  rules, and this PRD's DistinctWitness).

## Technical direction

Pin first: the existing distinct-elision tests (both regimes — elided
and seen-set) green before and after with unchanged values; if no test
distinguishes the two regimes observably, add the pair (same query
shape, keyed vs unkeyed occurrence, assert result equality AND — via
stats/introspect — which regime ran). Then the Option<witness>
refactor, compiler-chased.

## Passing criteria

- `[shape]` `grep -n "-> bool" crates/bumbledb/src/plan/fj/provably_distinct.rs` → zero;
  `DistinctWitness` minted at exactly one site.
- `[shape]` The seen-set-free sink constructor's signature demands the
  witness (unbuildable without it — the type IS the criterion).
- `[test]` Both-regime tests green; result equality across regimes
  pinned; full suite green; bounded fuzz smoke (rewrites + ops) per
  policy 7 — this touches the dedup semantics' licence, the exact
  thing those oracles watch.
- `[gate]` Fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`40-execution.md` § the distinct-bindings elision: the witness named;
the cross-reference to the union-elision refutation record; the
theorem↔evidence dedup row updates its cell.

## Results

`DistinctWitness` is minted only in `provably_distinct` and is stored as an
`Option` in `ValidatedPlan`. The production sink boundary has three structural
entries: ordinary single-rule dedup, mandatory union dedup, and
`without_seen_set`, whose signature consumes `DistinctWitness`. No boolean or
unwitnessed flag crosses from the proof to the elision. Plan introspection v2 and
`RuleStats::distinct_bindings` expose the proof without using that diagnostic bool
to authorize construction.

The bool-licensed-rewrite sweep covered `plan/`, `api/prepared/`, and the sink/
executor consumers reached by their decisions:

| Candidate | Judgment | Result / reason |
|---|---|---|
| Distinct-binding seen-set elision | Semantics-bearing elision | Replaced `bool` with `DistinctWitness`; only `AggregateSink::without_seen_set` consumes it. |
| Single-rule vs union aggregate dedup | Semantics-bearing representation | Replaced `(distinct, union)` booleans with `SinkProgram`, separate ordinary/union/elided constructors, and internal `DedupRegime`. |
| D2 suffix cancellation: plan node | Semantics-bearing elision | Replaced `sink_relevant: bool` with `SuffixSkip::{Forbidden, Licensed}` evidence. |
| D2 suffix cancellation: sink | Semantics-bearing elision | Replaced `may_skip() -> bool` with `SkipCapability::{Forbidden, Licensed}`; D2 requires both evidence types. |
| All-word answer finalization | Semantics-bearing representation dispatch | Replaced cached `all_words: bool` with `AnswerHeap::{Words, Bytes}`, sealed from the validated predicate. |
| Literal-resolution warm skip | Semantics-bearing lifecycle state | Replaced `resolved_complete: bool` with `ResolutionState::{Pending, Complete}`; only a completed rewrite pass arms the skip. |
| Rule-disjointness | Diagnostic theorem, deliberately unspent | Already `Option<DisjointWitness>`; public/stats booleans are read-only projections. The spanning union set remains by the measured refutation. |
| Key-probe rewrite | Semantics-bearing dispatch | Already `Option<KeyProbePlan>` / `PreparedRule::KeyProbe`; no parallel flag. |
| Grounding elimination | Semantics-bearing rewrite | Proof predicates are local booleans consumed immediately; the cross-seam evidence is typed `Role::Eliminated(statement)`. |
| Closed-relation fold | Semantics-bearing rewrite | Parser returns typed `ResolvableFilter` values and the cross-seam evidence is `Role::Folded(mark)`; local evaluator predicates do not escape. |
| Rule subsumption | Semantics-bearing rewrite | Local containment predicates mint typed `SubsumedRule`; no stored permission flag. |
| `is_point_filter`, selectivity and matching predicates | Heuristic/local propositions | Left as booleans: they either choose cost/placement or are consumed in the proving frame; they authorize no downstream semantic shortcut alone. |
| `ParamSpec::point`, `missed_params`, negation/operator booleans | Runtime/type payload | Left as data, not optimizer licenses; their enum/typed parent determines meaning and no plan rewrite consumes them. |
| View-memo hit, fused-scan engagement, counter/empty queries | Heuristic/runtime observation | Left as booleans; both paths are value-identical and differential tests pin them. |

The keyed/unkeyed `CountDistinct` pair in
`elision_skips_binding_dedup_but_count_distinct_still_collapses` returns identical
answers while asserting `proven`/`unproven` in both structured and rendered
introspection. The sink-level differential additionally forces witnessed elision
against ordinary dedup on the same proved plan.
