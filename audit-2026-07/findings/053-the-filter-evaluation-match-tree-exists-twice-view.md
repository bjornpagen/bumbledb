## The FilterPredicate interpreter exists twice: view `row_matches` vs key-probe `fact_matches`

category: unification | severity: medium | verdict: CONFIRMED | finder: cross:branching

### Summary

The scalar filter evaluator over the `FilterPredicate` sum is written twice: once in `crates/bumbledb/src/image/view/apply.rs::row_matches` (operands read from warm image columns at a position) and once in `crates/bumbledb/src/exec/dispatch/key_probe_fact.rs::fact_matches` (operands sliced from encoded fact bytes). Every comparison rule — interval constants compare under `Eq` only, interval fields under `Eq`/`Ne` only, `bytes<N>` word-wise under `Eq`/`Ne` only, Allen classify-then-test, half-open point membership — is asserted in both places with byte-identical `unreachable!()` messages. Both files also carry their own copies of the `point_in` and `point_word` helpers. No test asserts parity between the two paths, so the duplication is a live drift lane, and it cuts against the project's representation-first / one-mechanism doctrine (docs/design/representation-first.md; the codebase itself cites "one mechanism, two callers" at src/exec/run/anti_probe.rs:15).

### Evidence (all verified against the working tree)

Duplicated match trees, same arms in the same order:
- `row_matches`: crates/bumbledb/src/image/view/apply.rs:117-258
- `fact_matches`: crates/bumbledb/src/exec/dispatch/key_probe_fact.rs:112-231

Identical semantics assertions in both:
- "validated: interval constants compare under Eq only" — apply.rs:136 and key_probe_fact.rs:148
- "validated: no order comparison over intervals" — apply.rs:191 and key_probe_fact.rs:169
- "validated: bytes<N> compares under Eq/Ne only" — apply.rs:142,197 and key_probe_fact.rs:155,176
- "same-fact comparison joins same-typed fields" — apply.rs:199 and key_probe_fact.rs:179
- "validated: the Allen constant side is an interval" — apply.rs:228 and key_probe_fact.rs:203
- "validated: the outer side is an interval constant" — apply.rs:239 and key_probe_fact.rs:214
- "validated: within-comparands are scalar words" — apply.rs:247 and key_probe_fact.rs:221

Duplicated helpers:
- `const fn point_in(start, end, point)` — apply.rs:86-88 == key_probe_fact.rs:105-107 (verbatim)
- `fn point_word(&ResolvedWordSource, &[Const]) -> u64` — apply.rs:63-74 == key_probe_fact.rs:91-102 (identical logic; only the `Var` arm's panic string differs)

Near-identical operand sums:
- `Operand::{Word, Byte, Pair, Block}` — apply.rs:263-268
- `FactOperand::{Word, Pair, Block}` — exec/dispatch/fact_word.rs:12-20 (bytes widen to words, absorbing the `Byte` arm)

The key-probe file acknowledges the mirroring itself: key_probe_fact.rs:109-111 — "the same word compositions the view evaluator runs over image columns (`image::view::apply`), sourced from `fact_operand` instead." Two more arms repeat the acknowledgment inline (key_probe_fact.rs:189-192 on the Allen kinds).

No parity test: crates/bumbledb/src/exec/dispatch/tests.rs (718 lines) tests classification and key-probe hit/miss/remaining-filter rejection, but never runs the same predicate through both evaluators and diffs the answers.

One nuance on the finder's doctrinal citation: anti_probe.rs:15-17 says "one mechanism, two callers" but immediately adds "The sharing is the semantic and the compaction machinery, not a common function" — the codebase does sometimes bless semantic sharing without a shared function. That precedent covered two genuinely different substrates (COLT probes vs LMDB commit probes). Here the substrate difference is exactly one seam — where a field's operand comes from — which is what a closure parameter abstracts; the case for a common function is correspondingly stronger.

Real (modest) unification costs the finding understates:
- The view side is infallible (`row_matches -> bool`); the fact side is fallible (`Result<bool>` — corruption from `fact_operand`, dict lookups from `const_operand`). A shared evaluator either returns `Result` for both or is generic over the error channel.
- Constant resolution also differs, not just field-operand sourcing: apply.rs matches `Operand` against `&Const` after `resolve()` (and handles `WordSet` arms, apply.rs:146-171, plus `AnyPointIn`, apply.rs:206-212), while key_probe_fact pre-converts constants to `FactOperand` via `const_operand` (key_probe_fact.rs:66-87) with a dictionary lookup for `PendingIntern`; the set kinds are unreachable there by classification (classify routes param-set bindings to Free Join). The shared evaluator therefore needs two parameters — field-operand source and constant-operand source — with the set arms living behind the constant source.

### Failure scenario

Not a runtime bug today: I diffed the arms and the two interpreters currently agree. The lane is drift — any semantics change (a new `FilterPredicate` kind, a changed miss semantic, a comparison-rule fix per docs/research/aggregate-comparisons.md) must land arm-for-arm in `row_matches`, `fact_matches`, and `kernel_scan`. A change landing in one interpreter but not the other makes the key-probe fast path and the Free Join / view path return different answers for the same query, with nothing in the test suite to catch it.

### Suggested fix

Hoist one evaluator parameterized over the two operand sources — `fn predicate_matches(field_op: impl Fn(FieldId) -> Result<Operand>, const_op: impl Fn(&Const) -> Result<Operand>, filter: &FilterPredicate, params: &[Const]) -> Result<bool>` — merging `Operand`/`FactOperand` into one sum (keep the `Byte` arm; the fact-bytes source simply never produces it, matching `fact_word.rs`'s widening). The view caller supplies the image-position sources (its `WordSet` handling lives in its `const_op`); the key-probe caller supplies `fact_operand` + `const_operand`. Move `point_in`/`point_word` beside the shared evaluator. `kernel_scan` stays as the view-side SIMD accelerator whose scalar reference is now the one shared function. Cheaper interim hardening if the merge is deferred: a property test that runs every filter kind through both evaluators over the same facts and asserts agreement.
