# IR correctness audit

## Scope (files and docs read, with line counts)

Paper (algorithmic authority):

- `docs/free-join-paper/arXiv-2301.10841v2/main.tex` (163 lines) and its inputs:
  `tex/00-abstract.tex`, `tex/01-intro.tex`, `tex/02-background.tex` (§2 basic
  concepts — the CQ assumptions normalization deviates from), `tex/03-free-join.tex`
  (GHT/plan/execution), `tex/04-optimizations.tex` (binary2fj, factor, COLT,
  vectorization, dynamic covers), `tex/05-eval.tex` (337 lines, skimmed),
  `tex/06-discussion.tex` (85 lines).

Architecture docs, in order: `README.md` (72), `00-product.md` (187),
`10-data-model.md` (228), `20-query-ir.md` (179 — the contract), `30-execution.md`
(296), `40-storage.md` (206), `50-validation.md` (180), `60-api.md` (121).

Audited files:

- `crates/bumbledb/src/ir.rs` — 292 lines (Query/Atom/Term/Value/FindTerm/CmpOp,
  `value_matches`)
- `crates/bumbledb/src/ir/validate.rs` — 986 lines (the sealed `ValidatedQuery`
  witness)
- `crates/bumbledb/src/ir/normalize.rs` — 564 lines (paper-form lowering)

Consumers read to verify the witness/normalized-form contract end-to-end (values
traced, not just skimmed): `encoding.rs` (487), `image.rs` (696), `image/view.rs`
(576), `api/prepared.rs` (~1240), `exec/sink.rs` (784, sink halves), `exec/run.rs`
(residual evaluation, D2 skip gating), `exec/colt.rs` (word extraction, `select`,
probe paths), `exec/dispatch.rs` (guard classification, `const_bytes`/`const_word`),
`plan/fj.rs` (992 — `split_filters`, `provably_distinct`, residual placement, cover
derivation), `plan/planner.rs` (caps, var densification), `storage/dict.rs`
(sentinel id), `error.rs` (ValidationError roster), `tests/edge.rs` (gate family).
`cargo test -p bumbledb --lib -- ir:: image::view`: 50 passed, 0 failed.

## Verdict

The IR layer is correct against its contract. Every one of the seventeen roster
items in `20-query-ir.md` has a code path in `validate.rs`, a distinct typed error,
and a unit test; I constructed adversarial shapes against each rule and against the
accepting side (legal-per-doc queries) and found neither a missing rejection nor a
wrong rejection. Normalization is value-faithful for every lowering: the biased
sign-flipped i64 word (`u64::from_be_bytes(encode_i64(v))`) is exactly the word form
the image builder stores, so word comparison equals value comparison for all six
operators on integers (traced across −1/0/1 and i64::MIN/MAX, and pinned by unit
tests in both `encoding.rs` and `image.rs`); Eq/Ne on bool/enum bytes and intern ids
are faithful by injectivity; order operators on non-word-faithful types are
unrepresentable past validation. Filter placement (first occurrence for
var-vs-constant, shared-atom `FieldsCompare` for same-atom var pairs, residuals for
exactly the cross-atom rest) is sound under join equality, and the witness's derived
tables (`var_types`, `param_types`, `group_key`) cannot drift from what execution
assumes — I traced each to its consumers. The findings below are notes and edge
observations, not bugs: no CRITICAL, HIGH, or MEDIUM defects were found.

## Findings

### [NOTE] Stale test comment: fixture enum has 2 variants, comment says 3

`crates/bumbledb/src/ir/validate.rs:709`. The test
`enum_ordinal_in_a_comparison_reports_the_precise_variant` says "Account.status has
3 variants; ordinal 9 is out of range", but the fixture (`validate.rs:392-397`)
declares `["Active", "Closed"]` — 2 variants. The test's assertion is unaffected
(ordinal 9 is out of range either way), so this is documentation drift inside a
test, not a behavior defect. Invariant at stake: none (comment accuracy). Fix: say
"2 variants".

### [NOTE] `resolve_filter`'s Eq-miss arms are unreachable for the Free Join path, but its doc comment implies they run

`crates/bumbledb/src/api/prepared.rs:1084-1112` (consumer of normalize's output, not
an audited file — recorded because the reachability argument depends on the
IR-lowering invariant). `split_filters` (`plan/fj.rs:386-408`) routes every
Eq-against-a-constant into `selections`, and `check_selections` plus the
`debug_assert` in `run_join` (`prepared.rs:925-934`) enforce that plan-occurrence
`filters` never carry one. So `resolve_filter`'s `missed && op == Eq → Ok(None)` and
`PendingIntern` miss-under-Eq arms can never fire on the executor path — the Eq-miss
short-circuit actually lives in `resolve_selection`. The code is correct as
defense-in-depth (and `resolve_filter` would be right if the invariant ever
loosened), but its doc comment ("`Ok(None)` = a dictionary miss under `Eq`")
describes a path that today cannot execute through it. Concrete failure scenario:
none — behavior is correct either way. Fix direction: one sentence on the function
noting the Eq arms are unreachable post-split and exist as belt-and-braces, so a
future reader doesn't hunt for the caller that exercises them.

### [NOTE] Duplicate and contradictory comparisons are accepted and lower to redundant machinery

`crates/bumbledb/src/ir/validate.rs` (no dedup of `predicates`),
`normalize.rs:90-154`. Two observations, both semantically correct:

- A duplicated predicate (`x >= 100` twice) lowers to two identical filters; the
  view evaluator applies both (harmless conjunction, one wasted pass).
- Contradictory Eq constants (`x == 5` and `x == 6` with `x` bound at one field)
  lower to two selections on the same field; `Colt::select` probes both levels, the
  second probe misses inside the survivors of the first, and the query is correctly
  empty. Traced through `split_filters` (stable sort by field preserves both) and
  `Colt::select`'s level walk.

The doc's "write the query you mean" philosophy (it rejects self-comparison and
constant comparison for exactly this reason) could arguably extend to statically
false conjunctions, but the roster does not require it and the semantics are exact.
No fix needed; recorded so the acceptance is known-deliberate rather than an
oversight.

### [LOW] Var-vs-constant filters restrict only the variable's first occurrence — sound, but sibling occurrences plan and scan unfiltered

`crates/bumbledb/src/ir/normalize.rs:124-149`. For a variable bound in several atoms
(`Posting.at = t`, `Audit.at = t`, predicate `t >= ?0`), the lowered filter lands
only on the first occurrence in atom order. Correctness is airtight — join equality
propagates the restriction, and the doc comment says exactly this — but two
second-order effects exist: (a) the planner's measured-survivor statistics see the
filter on one occurrence only, so the sibling plans on its full row count even
though its effective extension is equally restricted; (b) at execution the sibling's
trie is built over the unfiltered view. Neither produces wrong results (verified by
tracing the join: any binding surviving the join satisfies the predicate via the
filtered occurrence). Concrete failure scenario: none for correctness; a skewed plan
choice is possible when the filtered variable is highly selective and the planner
puts the *unfiltered* sibling first. Fix direction (only if a benchmark ever shows
it): replicate the filter onto every occurrence of the variable, or fold the
restriction into the sibling's planning estimate.

## Checked and sound

**Validation completeness — each roster item traced to its code path and probed
with adversarial shapes:**

- Unknown relation/field ids (`check_atoms`, bounds against schema; u32/u16 index
  arithmetic safe on 64-bit).
- Duplicate `FieldId` in one atom (prefix scan, checked before typing).
- Variable type conflicts (`bind_var`, structural `ValueType` equality — enum
  identity is the ordered variant list, so `[A,B]` vs `[B,A]` correctly conflicts).
- Literal-vs-field mismatches through the shared `value_matches` (kind, enum
  ordinal range, UTF-8) — the same function validation, bind time
  (`bind_param`), and the dynamic write path call, so the rules cannot drift.
- Enum ordinal out of range: distinct diagnoses in bindings
  (`EnumOrdinalOutOfRange` with atom+field) and comparisons
  (`ComparisonEnumOrdinalOutOfRange` with predicate index).
- Param anchoring is total by representation (every param position is a field
  binding or sits opposite a typed variable; param-only comparisons are already
  `ConstantComparison`) — verified no accepted shape leaves a param untyped.
  Conflicting anchors rejected; density enforced over `params_seen` (the gap error
  names the missing id; the u16 conversion in the error path cannot overflow since
  position < 65536 by construction).
- Comparison typing: order ops only on U64/U64 and I64/I64 (`cmp_legal` against the
  var side, cross-checked against the other var's type); Eq/Ne all six types,
  same-type only; no coercion anywhere.
- Constant comparisons (lit-lit, param-lit, param-param) and self-comparison
  (`x op x`) rejected; match-arm ordering in both `check_comparisons` and
  `place_comparisons` verified against the Var-Var case shadowing the or-pattern.
- Unbound find variables, including aggregate inputs; comparison-only variables
  (either side); empty finds; duplicate find terms (structural `FindTerm` equality
  — `[Min(x), Max(x)]` correctly legal, `[Count, Count]` correctly duplicate); no
  atoms.
- Aggregate rules: Count strictly nullary (`CountWithVariable`), Sum/Min/Max
  require a variable (`AggregateWithoutVariable`), inputs U64/I64 only
  (`AggregateInputType`), aggregate-over-group-key rejected, all-aggregate finds
  (empty group key) accepted.
- Planner caps at the boundary: >20 atom occurrences (`MAX_OCCURRENCES`, the DP's
  2²⁰ table) and >128 distinct variables (the u128 bitsets) — so `OccId`'s u16, the
  planner's dense `var_index`, and the bitset widths are true invariants. Sparse
  `VarId` values are safe: every downstream consumer densifies
  (`plan/fj.rs` slots, `planner.rs` var_index) rather than indexing by `VarId.0`.

**No wrong rejections** — accepted and traced: zero-binding atoms (nonemptiness
gates), nullary relations, repeated in-atom variables of equal type, self-joins,
literal-on-the-left order comparisons, params anchored only by comparisons,
duplicate identical predicates, 20-atom and 128-var queries at the caps,
all-aggregate finds, `Min`/`Max` pairs over one variable.

**Witness derived tables cannot disagree with execution:**

- `var_types`: first-binding-wins with conflicts rejected, so every binding site of
  a var agrees; consumed by `find_specs` for result types and the `signed` flag the
  aggregate sink dispatches on — the flag matches the column word form by
  construction.
- `param_types`: dense, id-ordered iteration is positional; `bind_param` checks
  count and structural type per execution through the same `value_matches`.
- `group_key`: a `BTreeSet` used only for membership (plan `sink_relevant` bits);
  output column order comes from `finds` order in both `find_specs` and
  `AggregateSink::group_slots` — the set's ordering never leaks into results.

**Normalization value-faithfulness, per lowering:**

- `lower_literal`: Bool → strict 0/1 byte; Enum → range-checked ordinal byte
  (witness-sealed, so unvalidated ordinals cannot reach it); U64 → identity word;
  I64 → `u64::from_be_bytes(encode_i64(v))`, the sign-flipped biased word.
  `image.rs` stores exactly `u64::from_be_bytes(canonical bytes)` per 8-byte column
  and the validated raw byte per 1-byte column, so filter words and column words are
  the same encoding — verified equal by construction and by
  `columns_equal_per_field_decode_of_the_scan`.
- Biased i64 order traced at the boundaries: −1 → `0x7FFF…FF`, 0 → `0x8000…00`,
  1 → `0x8000…01`, i64::MIN → `0x0000…00`, i64::MAX → `0xFFFF…FF`; u64 order equals
  i64 order (also pinned by `i64_order_preservation_across_sign_boundary` and
  `i64_word_order_matches_logical_order`). The kernel's range rewriting
  (`Lt → [0, c−1]`, `Gt → [c+1, MAX]` with `checked_sub`/`checked_add` guarding the
  x < MIN / x > MAX empty cases) operates in biased space, where it is exact
  because the bias is an order isomorphism onto the full contiguous u64 range.
- String/Bytes → `PendingIntern{tag, bytes}` resolved per execution: Eq miss
  short-circuits the whole conjunctive query to empty (sound: an unmatchable
  Eq on any occurrence empties the conjunction — and only Eq short-circuits);
  Ne miss resolves to the sentinel id `u64::MAX`, which `dict.rs:84` asserts is
  never minted, so `Ne sentinel` matches every stored value — the doc's
  per-operator miss semantics, exactly. Interning is injective (collision axiom
  recorded in `10-data-model.md`), so id equality is value equality.
- Repeated in-atom variables: first field stays the variable position, later
  positions lower to `FieldsCompare{first, later, Eq}` — executed against a real
  image in the normalize test suite; same-type guarantee makes the evaluator's
  same-width column pairing (`Words/Words`, `Bytes/Bytes`) total.
- Same-atom var-vs-var comparisons under every operator lower to `FieldsCompare`
  with operand order preserved (lhs field left, rhs field right — no flip needed),
  faithful for all six ops on integers (both columns biased identically) and for
  Eq/Ne on byte and intern columns; order ops on non-integers are rejected
  upstream. When the pair also spans other atoms, one shared-atom filter is
  sufficient by join equality.
- Cross-atom pairs — exactly those — become residuals; `plan/fj.rs` places each at
  the earliest node where both sides are bound (an unplaceable residual is a plan
  error, unreachable for planner-built plans since every var is bound by some
  node), and `run.rs` evaluates them over binding-slot words, where u64 compare is
  value compare for every legal (type, op) pair.
- `flip` is the correct mirror (`c ≤ x ⇔ x ≥ c`; Eq/Ne fixed points), applied only
  when the constant is on the left; `var_on_left` is recomputed from the original
  comparison, so `(Param, Var)` and `(Var, Param)` orientations both land right.
- Guard-probe path consistency: `const_bytes` emits `Word::to_be_bytes` /
  raw byte / `encode_u64(intern id)` — byte-identical to the canonical fact
  encoding the `U`/`M` keys were built from, for all six types.

**Set/aggregation semantics preserved end to end:**

- Normalize drops no variables and no constraints; the satisfying-assignment set
  over all query variables is invariant under the lowering.
- The projection sink dedups projected tuples; the aggregate sink dedups full
  bindings (all slots = all query variables) unless the plan's
  `provably_distinct` flag holds. That proof was audited: bound fields
  (vars + Eq-constant fields) covering a unique constraint per occurrence implies
  binding→fact injectivity per occurrence and single-entry trie leaves, so each
  distinct binding is emitted exactly once; gate-style occurrences (no bound
  fields) fail the proof closed, keeping the dedup that collapses their
  row-multiplicity (the `zero_binding_gate_with_global_count` edge test pins the
  gate family; Count over a 2-row gate would be caught by the retained seen-set).
- The D2 subtree skip is requested only by the projection sink
  (`AggregateSink::emit` always returns `Continue`), and skipped suffix entries can
  only re-derive an already-emitted projected tuple — residuals evaluated in the
  suffix cannot be bypassed for any *new* tuple because the skip propagates only
  through nodes binding no sink-relevant variable.
- Sum accumulates i128/u128 with one finalize-time range check (`Overflow`);
  signed sums unbias words before accumulating; Min/Max fold raw words, correct
  because the word form is order-preserving for both integer types; Count is
  `|binding set|` as U64. Empty input yields zero groups — the empty set, not a
  0/NULL row.
- Per-execution intern resolution can never serve stale under the view memo: a
  dictionary miss can only flip to a hit via a commit; if the storage tx id
  advanced, the memo's generation key misses; if the resolved word changed at the
  same generation (impossible for facts, but harmless regardless), the memo's
  resolved-filter key misses. Selections are re-probed every execution by design.

**Infallibility of `normalize`:** both `expect`s (`atom count fits u16`,
`comparison variables are atom-bound`) and both `unreachable!`s are discharged by
witness invariants (caps and the constant-comparison/comparison-only-variable
rejections); the sealed `ValidatedQuery` (private fields, crate-internal
constructor) makes unvalidated input to `normalize` unrepresentable.
