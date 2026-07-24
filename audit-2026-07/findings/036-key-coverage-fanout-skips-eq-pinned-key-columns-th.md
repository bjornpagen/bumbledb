## Key-coverage fanout skips Eq-pinned key columns that provably_distinct already counts

category: unification | severity: medium | verdict: CONFIRMED | finder: engine:plan-ir

### Summary

The predicate "do this occurrence's bound fields cover a key of its relation?" is implemented three times with two different field vocabularies. The distinctness witness (`plan/fj/provably_distinct.rs`) and the key-probe classifier (`exec/dispatch/classify.rs`) both count a field Eq-pinned to a single scalar constant as bound. The DP planner's key translation (`plan/planner/densify.rs`) counts only variable-bound fields and silently drops any key whose projection contains an Eq-pinned field — so a compound key that functionally guarantees fanout 1 under a pin-plus-join binding never enters `key_var_sets`, and `estimate.rs` falls back to the general `rows / distinct` fanout. This is the philosophy's core anti-pattern: one predicate, two encodings, and the divergence is exactly where the soundness gap lives.

### Evidence (all verified in source)

- `crates/bumbledb/src/plan/planner/densify.rs:61-68` — key translation consults only `occurrence.vars`:
  ```rust
  .filter_map(|id| {
      let mut set = 0u128;
      for field in &schema.key(*id).projection {
          let (_, var) = occurrence.vars.iter().find(|(f, _)| f == field)?;
          set |= 1 << var_index[var];
      }
      Some(set)
  })
  ```
  The `?` drops the entire key when any projection field is filter-pinned rather than var-bound. `occurrence.filters` is available at this site and never consulted.
- `crates/bumbledb/src/plan/fj/provably_distinct.rs:50-67` — the same coverage predicate, computed as vars ∪ Eq-pinned constant fields (`Const::Word | Byte | Interval | Param | PendingIntern`, sets deliberately excluded with a documented soundness argument at lines 28-31).
- `crates/bumbledb/src/exec/dispatch/classify.rs:78-88` — a third implementation (key-probe eligibility) whose coverage is entirely Eq-pin-based ("The fields bound BY VALUE: pinned to a constant by an Eq filter"), with sets excluded at lines 62-76. Two of the three sites accept pins; the planner's does not.
- `crates/bumbledb/src/plan/planner/estimate.rs:22` — `if r.key_var_sets.iter().any(|set| set & join_vars == *set) { return prefix_est; }` — the fanout-1 return is unreachable for any key with a pinned field.
- `crates/bumbledb/src/plan/selectivity.rs:27` — `DEFAULT_EQ_DISTINCT = 64`, the cold floor used in the scenario arithmetic below.
- `crates/bumbledb/src/plan/planner.rs:80-82` — the divergence is a known, documented simplification: "statements with any literal-bound or unbound field are skipped — simple and faithful to the doc's estimator."
- Tests: `plan/planner/tests.rs` covers var-bound key coverage (`key_coverage_fires_through_the_fresh_auto_key`), both pointwise-key directions, and membership-bound intervals — no test exercises an Eq-pinned key field, so nothing pins the gap either way.

### Spec check

`docs/architecture/40-execution.md` § "Join cardinality estimator, written down" defines key coverage over join variables J only, so densify is faithful to that paragraph as written. But the same doc's distinct-elision section (§ sinks, "bound fields cover a key of its relation") and its key-probe section (§ key-probe point lookups, "bindings cover a key") both use the pin-inclusive vocabulary that `provably_distinct` and `classify` implement. The estimator paragraph is the outlier, not the norm — the doc should gain the pinned-field clause along with the code.

### Semantic soundness of the missed bound

For a key on (a, b) with `b == c` Eq-pinned to one scalar constant (or param — one value at execution) and `a` bound by the join prefix: each prefix binding fixes (a, c) completely, and the Functionality statement admits at most one fact per full projection value — fanout ≤ 1, certainly. The required caveats are exactly the ones `provably_distinct` already encodes: sets (`ParamSet`/`WordSet`) pin nothing, and a pointwise key's interval field must be pinned by value (an Eq `Compare` on an interval field is value-typed by construction — membership bindings lower to `PointIn` and produce no Eq `Compare`, per classify.rs:15-19).

### Bench impact

Posting keyed (account, day), query pins `day == <param>`, joins on account, 1M rows, cold cache: occurrence rows ≈ 1M/64 ≈ 15625 (the Eq floor), account distinct floor 64, so `estimate.rs` prices the step at fanout 15625/64 ≈ 244 where the key certifies 1 — a ~244x per-step overestimate that compounds into every downstream prefix estimate, can reorder reference-walk lanes in the DP, and inflates the introspection/report honesty numbers. Correctness is unaffected (estimates only order the DP; execution bounds the damage), which is why this is medium, not high.

### Suggested fix

Extract the shared "pinned fields of an occurrence" iterator (exactly `provably_distinct`'s Const roster: Word/Byte/Interval/Param/PendingIntern under `CmpOp::Eq`; sets excluded) into one function used by both sites — three, if `classify.rs`'s by-value roster can share it. In densify's key translation, treat a pinned projection field as covered with no variable bit: build the bitset over the var-bound remainder and accept the key when pinned ∪ var-bound fields exhaust the projection. `estimate.rs:22` needs no change — a key whose only unpinned fields are join-covered yields a set satisfying `set & join_vars == *set` naturally, and an all-pinned key yields the empty set, which is correct (though note estimate.rs returns the cross-product early at `join_vars == 0` before consulting keys — an all-pinned disconnected occurrence contributes at most one row and could also be improved, but that is a separate, smaller case). Add the missing planner test: compound key, one field Eq-pinned, join on the other, assert the fanout-1 return. Update the estimator paragraph in 40-execution.md to the pin-inclusive vocabulary its other sections already use.
