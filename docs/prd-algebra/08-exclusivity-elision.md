# PRD 08 — Exclusivity elision: the theorem pays the union's bill

**Depends on:** 07.
**Modules:** `crates/bumbledb/src/plan/` (the disjointness analysis, beside
`provably_distinct.rs`), `exec/sink/` (the elision consumer),
`api/stats.rs` (report the proof).
**Authority:** `40-execution.md` (elision optimization precedent),
`30-dependencies.md` (the exclusivity theorem).
**Representation move:** one theorem, three consumers. The DU exclusivity
theorem ("an id cannot carry two kinds") is enforced by the checker, used by
the chase — and now spends a third time in the executor: rules that select
different values of one discriminator are **provably disjoint**, so the
cross-rule seen-set guards nothing and is deleted at plan time. The constraint
calculus pays for the query algebra. This is also the workload's own shape:
reading a discriminated union whole is a union of arm-rules, one per kind.

## Context (decided shape)

Plan-time analysis `provably_disjoint_rules(query, schema) -> bool` (a proof
flag in the validated artifact, like `distinct_bindings`):

- **Rule pair disjointness witness:** there exists a relation R and a field f
  such that both rules bind an occurrence of R whose filters pin f to
  *different* literals, **and** that occurrence's bound key columns flow to the
  same head positions in both rules (the head rows produced can only collide if
  the pinned facts coincide — which the differing literals forbid). The
  DU-arm-union query satisfies this via the parent occurrence's `kind`
  selection; the general witness is stated conservatively and misses nothing
  the workload runs.
- Pairwise over all rules; any unprovable pair ⇒ flag off, seen-set stays.
  Conservative and sound — identical to the `distinct_bindings` discipline.
- Consumers: projection sink drops the cross-rule guard (per-rule dedup
  remains as today's semantics require); aggregate sink composes this flag
  with per-rule `distinct_bindings` to elide the fold seen-set entirely for
  the DU read.
- EXPLAIN says which theorem fired (`disjoint_rules: proven (R.f)`), because a
  mechanism must name its reader and an elision must name its proof.

## Technical direction

1. The analysis beside `provably_distinct.rs`, consuming normalized filters +
   head projections; unit-level, no executor knowledge.
2. Thread the flag through `fj::validate` into the sink configuration.
3. Differential guard: a test-only override forcing the flag off must produce
   identical results on every covered query (the elision is *never* semantic).

## Passing criteria

- `[test]` The DU-arm union (two arms, `kind`-selected) proves disjoint; the
  same query with one rule's selection removed does not.
- `[test]` Differential: elided vs forced-off paths byte-identical results
  across the randomized rule-query corpus.
- `[test]` Aggregate composition: `Count` over a proven-disjoint union with
  per-rule key-covered bindings runs with zero seen-set insertions (assert via
  the counting surface), and matches the naive model.
- `[shape]` The flag appears in EXPLAIN with its witness; no runtime branch
  exists in the sink hot loop (monomorphized like the existing elision).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`40-execution.md`: the elision section gains the rule-disjointness proof and
its witness form; `30-dependencies.md` gains one line: the exclusivity
theorem's third consumer.
