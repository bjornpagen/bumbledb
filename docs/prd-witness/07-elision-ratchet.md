# PRD 07 — The elision ratchet: reverted

**Depends on:** 06 and its locked isolated number.
**Decision:** Branch R, selected mechanically because the isolated proof-on
path lost.
**Authority:** the refutation policy: a mechanism that measures as a loss is
reverted, while the record keeps the number and failure mechanism.

## Adjudicating measurement

The locked scale-S, seed-1, 256-sample run at commit `39f6bee` measured:

- proof path: 1,376.9 µs p50; 1,375.2 µs clock-normalized p50;
- spanning control: 937.2 µs p50; 936.8 µs clock-normalized p50;
- delta: −31.9% for the control, so the proposed optimization was a loss;
- both paths: 82,983 emitted, zero absorbed, with clean clock brackets.

The three earlier runs showed the same 32.1%, 32.6%, and 32.4% loss. The
isolated counters rule out D2 cancellation: neither path absorbed a binding.
The failed representation kept per-rule dedup, copied every map entry into a
row buffer at each rule boundary, and cleared the map. Those extra O(n)
drain/copy passes cost more than retaining one spanning map.

## Executed representation

- Every multi-rule projection and aggregate sink retains one head-projection
  seen-set spanning all rules. Single-rule distinct-bindings elision remains.
- The cross-rule executor flag, override, paired benchmark row, report delta,
  drain buffer, and mechanism-specific tests are deleted.
- `DisjointWitness` remains as EXPLAIN/statistics knowledge. Code audit found
  no chase caller of `provably_disjoint_rules`; its sole direct consumer is
  prepared-query diagnostics. The declaration-level exclusivity theorem still
  has the checker and chase consumers described in `30-dependencies.md`.
- Disjoint fixtures now prove the spanning set absorbs zero across proven
  arms. Union allocation gates continue to exercise the retained map.
- `40-execution.md` carries the permanent refutation and reversal trigger.

## Passing criteria

- `union_elided`, `force_disjoint_off`, and `rsvp_union_off` have zero hits in
  `crates`, `docs/architecture`, and `README.md`.
- Multi-rule prepared-query, aggregate, disjointness, report, and allocation
  tests pass with the always-spanning representation.
- Workspace gates are green at campaign close.
