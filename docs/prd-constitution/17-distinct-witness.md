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
