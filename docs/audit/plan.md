# Planner correctness audit

## Scope (files and docs read, with line counts)

Paper (algorithmic authority; `main.tex` inputs verified — `025-tale.tex`,
`07-relatedworks.tex`, `08-conclusion.tex` are commented out / empty):

- `docs/free-join-paper/arXiv-2301.10841v2/tex/02-background.tex` — 510 lines (CQ model, self-join renaming, bag-vs-set)
- `docs/free-join-paper/arXiv-2301.10841v2/tex/03-free-join.tex` — 608 lines (GHT, plan validity/covers, Fig. 5 join, §3.3 build-phase trie schemas)
- `docs/free-join-paper/arXiv-2301.10841v2/tex/04-optimizations.tex` — 478 lines (Fig. 7 `binary2fj`, Fig. 8 `factor`, COLT §4.2, vectorization §4.3, dynamic covers §4.4)

Architecture docs, in order: `README.md` (71), `00-product.md` (186), `10-data-model.md` (227),
`20-query-ir.md` (178), `30-execution.md` (295), `40-storage.md` (205), `50-validation.md` (179),
`60-api.md` (120).

Audited files:

- `crates/bumbledb/src/plan/fj.rs` — 992 lines (all)
- `crates/bumbledb/src/plan/planner.rs` — 572 lines (all)
- `crates/bumbledb/src/plan/selectivity.rs` — 378 lines (all)

Read for cross-verification of every plan consumer and producer:
`ir/normalize.rs` (564, all), `ir/validate.rs` (cap enforcement, `group_key`),
`exec/run.rs` (1451, all — executor's use of covers/residuals/slots/sink_relevant),
`exec/sink.rs` (Flow contract, elision consumption), `exec/dispatch.rs` (classify/guard path),
`exec/colt.rs` (cursor/level/suffix-iteration contract, `select`), `exec/explain.rs`
(estimates reader), `api/prepared.rs` (the prepare pipeline: stats → DP → binary2fj →
factor → validate; sink_vars source; view memo), `schema.rs` (auto-unique materialization).
Full lib test suite run: 254 passed, 0 failed.

## Verdict

The planner is in very good shape. `binary2fj` and `factor` are faithful transcriptions of
Figs. 7 and 8 (where Fig. 8's literal text is self-contradictory, the code implements the
reading the paper's own prose and clover example require), and I could not construct any
input on which `factor` breaks the partition property, changes any node's new-variable set,
or invalidates a cover — a proof sketch is recorded under "Checked and sound." The
cover-must-equal-new-vars deviation is not only sound but strictly necessary for this
executor (the paper's ⊇-cover definition is unsound under dynamic cover choice without a
per-entry equality re-check, which the code does not and need not have), and it is also
load-bearing for residual source resolution; the pipeline provably never produces a plan it
rejects. `provably_distinct` is exactly sound — I attempted counterexamples with self-joins,
constant-bound unique fields, partially-bound compound uniques, gates, and repeated in-atom
variables, and every gap resolves to the conservative side. The estimator/selectivity ladder
cannot affect results: estimates flow only into join-order choice (any permutation executes
correctly, verified by the randomized-order differential test) and EXPLAIN. The findings
below are boundary-hardening gaps against hand-built plans (the pipeline never produces
them) and documentation-level nits; none affects results for any query reaching the engine
through `prepare`.

## Findings

### [MEDIUM] validate() accepts plans that silently drop a zero-variable occurrence (nonemptiness gate)

`crates/bumbledb/src/plan/fj.rs:289-308` (partition check), with the failure landing in
`crates/bumbledb/src/exec/run.rs` (the omitted occurrence's COLT is simply never touched).

Documented invariant at stake: `PlanError`'s reason for existing — "this boundary exists
because `FjPlan` is plain data anyone can construct" (fj.rs:130-131), i.e. validate() is
supposed to reject every plan shape the executor computes wrong results on; and
20-query-ir's ruling that "an atom with zero bindings … means a nonemptiness gate on that
relation."

Failure scenario, concrete: normalized query = occ 0 `R(f0=x, f1=y)` plus occ 1 = a
zero-binding gate atom on relation `S`; hand-built plan
`[[Subatom{occ:0, vars:[x,y]}]]` — occ 1 appears in **no** node. The partition check for
occ 1 computes `seen = ∅`, `expected = ∅`, so `seen == expected` passes vacuously; no other
check quantifies over occurrences, so validate() returns `Ok`. The executor asserts
`colts.len() == plan.occurrences().len()` (both 2) and runs; the gate colt is never probed,
so with `S` **empty** the query returns all of `R` instead of the empty set — wrong results
on a validated plan. The degenerate extreme also passes: a query of only zero-var
occurrences with the empty plan `FjPlan { nodes: [] }` validates and unconditionally emits
one empty binding. Unreachable from `prepare` (the DP order contains every occurrence and
`binary2fj` gives each one an opener subatom; `factor` never removes subatoms), so this is a
latent boundary hole, not a live bug.

Fix direction: after the partition loop, require every `normalized.occurrences` entry to
appear in at least one node (for zero-var occurrences this means at least one empty-var
subatom); equivalently, track per-occurrence subatom counts during the partition pass and
add a `MissingOccurrence { occ }` error. One extra `BTreeSet`/counter, no plan-shape change.

### [LOW] Subatoms referencing an occurrence outside the normalized query are not rejected — executor panics instead

`crates/bumbledb/src/plan/fj.rs:289-308` / `466-519`. The partition and derive-node passes
only iterate `normalized.occurrences` and `plan.nodes` respectively; a hand-built subatom
with `occ: OccId(99)` (no such occurrence) is never matched by the partition loop, and
`derive_nodes` happily treats its vars as node vars (they can even become `new_vars` and
binding slots). The executor then indexes `colts[usize::from(subatom.occ.0)]` out of bounds
and panics (`run.rs:328/404`). Not a wrong-results path (it cannot get past the bounds
check), and unreachable from the pipeline (occ ids are dense indices from `normalize`), but
validate()'s contract is to be the boundary for hand-built plans, and a panic is the wrong
rejection shape. Fix direction: in `derive_nodes` (or the partition pass), reject any
subatom whose `occ` is not among `normalized.occurrences`.

### [NOTE] check_selections is unreachable inside validate() — it re-checks what split_filters just did

`crates/bumbledb/src/plan/fj.rs:357` calls `check_selections` on occurrences that
`validate` itself constructed at fj.rs:346 via `split_filters`, which unconditionally
routes every `Compare{op: Eq}` out of `filters`. Within `validate` the check is therefore
tautologically satisfied and `PlanError::SelectionOnFilteredField` cannot be returned from
this path; `check_selections` has no other production caller (only its own unit test
constructs a bad `PlanOccurrence` directly). Harmless belt-and-suspenders; worth a comment
or a debug_assert so a future reader doesn't hunt for the producer of the error. (The
executor-side twin at `prepared.rs:925` is a debug_assert, which is the honest form.)

### [NOTE] factor() diverges from Fig. 8's literal text — and is right to

`crates/bumbledb/src/plan/fj.rs:104-127`. Fig. 8 as written iterates `for α in φ` over
**all** subatoms including `φ`'s first (the opened cover) and `continue @outer`s on the
first non-hoistable one; taken literally, the cover (whose vars are the node's new vars,
never ⊆ avs) fails immediately on the paper's own clover example, and factor would be a
global no-op — contradicting the paper's stated output
`[[R(x,a),S(x),T(x)],[S(b)],[T(c)]]`. The prose ("we factor out a *lookup* only if all
previous *lookups* in the same node have also been factored out") resolves the ambiguity:
lookups start after the cover. The code's `while subatoms.len() > 1` / candidate-at-index-1
loop implements exactly that reading, verified against the paper's clover and chain
examples by fj.rs's tests. Recording it here so the divergence from the figure's literal
text is never mistaken for a transcription error.

### [NOTE] Two documentation nits: the slot-layout comment, and 30-execution's stale param carve-out sentence

(a) `crates/bumbledb/src/plan/fj.rs:359` says slots are laid out "node order, then subatom
order — dense"; within a node, `new_vars` comes from a `BTreeSet` iteration, so the true
order is node order then **VarId order**. Nothing consumes the order except `slot_of`
(self-consistent), so this is comment drift only.
(b) `docs/architecture/30-execution.md` "Statistics" still says a param-filtered atom
"plans on the base row count"; since the selection-level work landed, `occurrence_estimate`
(selectivity.rs:81-93) divides by the field's distinct-ladder count for param selections
exactly as for literals (the later perf-suite section describes this correctly). The older
sentence should be amended to "plans on the ladder estimate; only measured survivor counts
are unavailable for params." Cost-model description only; no correctness content.

### [NOTE] DP inner loop is O(2ⁿ·n²) and the table is ~32 MB at the cap, not ~24 MB

`crates/bumbledb/src/plan/planner.rs:134-164`: `prefix_vars` is refolded over all n bits
per (mask, last) pair, making the 20-occurrence worst case ~4×10⁸ operations — around a
second of prepare time, not the "instant" the module comment implies; a per-mask
`prefix_vars` memo (or using `best[prev_mask]` to carry the var set) makes it O(2ⁿ·n).
`Option<State>` is 32 bytes (no niche in State's u64s), so the full table is 32 MB
transient vs the documented ~24 MB. Both are prepare-time cost/documentation points;
plan choice and results are unaffected.

## Checked and sound

- **binary2fj fidelity (Fig. 7):** transcription verified line-by-line against the figure
  and both worked examples (clover → `[[R(x,a),S(x)],[S(b),T(x)],[T(c)]]`, chain →
  4-node plan); `available.extend` timing matches the figure's `avs(φ)` semantics (probes
  may use vars bound by the current node's opener, exactly as the paper's clover output
  requires). Each occurrence contributes exactly one opener (vars = remainder) and, except
  the first, one probe (vars = intersection with available) — so every occurrence appears
  in the plan and every node's opening subatom has exactly the node's new vars.
- **factor() preserves all plan invariants — proof sketch.** A hoist moves subatom `s`
  from node `i` to `i−1` only when `s.vars ⊆ vars(nodes[..i])` and occ(s) ∉ node i−1.
  (1) Partition: the multiset of (occ, var) pairs is unchanged. (2) `avs` of every node is
  unchanged: for k ≤ i−1 nothing below k moved; for k ≥ i, `s.vars` were already inside
  `vars(nodes[..i])`. (3) `new_vars` of every node is unchanged: node i loses only all-old
  vars; node i−1 gains only vars in `avs(i−1) ∪ vs(i−1)`. (4) Covers: index-0 openers never
  move and remain exactly-new; a hoisted subatom can only *add* a legitimate cover. (5) An
  occurrence's subatoms never reorder across nodes (a move past a node containing the same
  occurrence is blocked), so trie schemas and the executor's per-occurrence level counter
  stay aligned. (6) Repeated hoisting re-checks availability against the smaller prefix
  each round. The duplicate-occurrence-per-node property is checked explicitly per move.
- **The cover-must-equal-new-vars deviation is sound and its recorded justification holds.**
  Necessity: under dynamic cover choice the executor writes cover key words into binding
  slots with no equality re-check (`run.rs:484-486`), so a paper-legal ⊇-cover carrying a
  bound var would rebind it — the recorded triangle regression
  (`covers_never_rebind_an_already_bound_variable`) demonstrates wrong results on skew.
  Completeness: every pipeline node has an exactly-new cover (binary2fj opener; preserved
  by factor), including the ∅-new-vars edge (fully-bound probe → empty opener, which is a
  valid len-0 cover; the paper's "any subatom covers ∅" reading would itself rebind).
  Bonus: the rule is what makes `run.rs`'s residual/probe `Source` resolution total —
  every node var is either old (Slot) or a chosen-cover var (Batch), for *any* cover choice.
- **validate() vs the executor, adversarially:** probes always have fully-bound keys (any
  subatom var is old or in the cover = new set); multi-cover nodes are sound under any
  runtime choice (all covers bind the same var set); an occurrence's trie level advances
  exactly once per node containing it, matching `trie_schema` order; duplicate vars inside
  a subatom are caught by the partition pass (`seen.insert` fails), so the len+⊆ cover test
  is a true set-equality test; >256 subatoms per node cannot occur under the 20-occurrence
  cap (the `u8::try_from` expect is safe); zero-var gates, Cartesian (empty-var) probes,
  single-atom plans, cyclic plans, and all-vars-shared self-joins all validate and execute
  correctly (traced through COLT's zero-arity force/iterate and confirmed by run.rs tests).
- **Residual placement:** every residual's sides are atom-bound vars (IR roster), every
  occurrence var lands in some node's `new_vars`, and the placement scan attaches at the
  first both-bound node — which can never be a ∅-new-vars node, so evaluation (after cover
  iteration and probes, from Batch or Slot sources) never reads an unbound side. Residuals
  cannot be skipped: the D2 unwind fires only after a full witness has passed every
  residual, and it only discards witness multiplicity for the same projected fact.
- **The D2 skip machinery, planner side:** `sink_vars = witness.group_key()` = all find
  vars for projection queries (aggregates use the AggregateSink, which never returns
  `SkipSuffix`, so its group-key-based `sink_relevant` bits are inert). Propagation stops
  at the first node binding a projected var; every node the skip crosses was itself
  non-relevant, so abandoned iteration can only produce already-emitted projected facts.
- **provably_distinct is exactly sound.** The needed property is binding → witness
  injectivity: if each occurrence's determined fields (var-bound, or Eq-filtered to a
  per-execution constant — all four `Const` variants) cover a unique constraint, the
  binding determines each occurrence's fact, so distinct witnesses ⇒ distinct bindings and
  the aggregate seen-set is redundant. Counterexamples attempted and defeated: self-joins
  (each occurrence's fact determined independently, may coincide — harmless); fields whose
  unique coverage comes only from constants (≤1 matching fact, vacuously injective);
  compound uniques with a range-filtered field (not counted → flag false → seen-set kept,
  which is required: two rows differing only in the ranged field collapse to one binding);
  zero-var gates (constraints have non-empty field lists by construction → never covered →
  flag false, required since a 2-fact gate double-emits); relations with unbound columns
  (suffix iteration emits per-position duplicates exactly when no unique is covered);
  FieldsCompare-determined fields (not counted — conservative only). Consumption verified:
  the flag only elides the aggregate sink's seen-set (`sink.rs:147`); the projection sink
  dedups unconditionally; the guard path's trivial `true` is correct (≤1 emit). The flag
  is a plan-independent property of the normalized query, computed as such.
- **split_filters / Selection:** every `Eq`-against-constant (literal, param, pending
  intern — one machine) moves to `selections`; the selection/residual partition is
  exact and mutually exclusive (`unreachable!` in selectivity.rs is genuinely
  unreachable). A field both var-bound and Eq-filtered lowers to a selection trie level
  *plus* the var's join level on the same column — the narrowed subtrie makes the var bind
  the constant; correct, and provably_distinct counts the field once. Two Eq selections on
  one field: same constant → both levels hit; contradictory constants → the second probe
  misses inside the first's subtrie → correctly empty. Contradictory `x=5, x=7` via
  comparisons on a var lowers the same way. Selection + FieldsCompare compose as
  conjunction (FieldsCompare stays a view filter; selections probe the view's survivors) —
  order-independent. Eq-miss short-circuit (`resolve_predicates → Ok(false)`) is sound
  precisely because an Eq conjunct with an unmatchable constant empties the whole
  conjunctive query; `Ne` misses resolve to the sentinel and match everything, per spec.
  Sorted-by-field, stable-within-field selection order makes lowering deterministic.
- **Estimates cannot affect results.** Flow audit: `OccStats` → `plan()` → `JoinOrder`;
  `order` selects among plans that are all correct (validated by the randomized
  differential test executing random join orders at batch sizes 1/7/128 against a
  nested-loop oracle); `estimates` are carried opaquely through `validate` into
  `ValidatedPlan.estimates`, whose only reader is EXPLAIN (`explain.rs:159`, bounds-safe
  via `.get().unwrap_or(0)`). `occurrence_stats`/`distinct_of` perform read-only LMDB and
  cache peeks; their only failure mode is an `Lmdb` error aborting prepare. Zero-row and
  zero-distinct inputs are clamped (`.clamp(1, rows.max(1))`, `.max(1)`) — no division by
  zero, no zero estimates, saturating arithmetic throughout the DP.
- **densify cannot alias variables.** Distinct vars get distinct dense indices (the
  `let next = len; entry().or_insert(next)` pattern is insert-if-absent); the 128-var cap
  is enforced as a hard error at the IR boundary (`ir/validate.rs:137`, tested at 129), so
  every shift is `1u128 << k` with k ≤ 127; the planner's `debug_assert` is documentation,
  not the enforcement. Likewise `MAX_OCCURRENCES = 20` is boundary-enforced
  (`ir/validate.rs:112`), so `1u32 << n` and the u8 `last` are safe, and the DP table's
  `expect("smaller masks filled first")` holds by induction (singletons seeded; every
  candidate's `prev_mask` is nonempty and smaller). n = 1 (single-atom FJ fallback when
  the guard probe doesn't classify) reconstructs correctly from the seeded singleton.
- **Distinct ladder rungs:** single-field unique (including the serial auto-unique, which
  schema materializes as an ordinary visible constraint — verified in `schema.rs`) ⇒
  rows-exact; resident image ⇒ build-time exact counts (peek-only — prepare never builds);
  single-field FK ⇒ min(target rows, rows); enum/bool ⇒ variant-count/2; else the
  documented 64 floor. All rungs verified against the module's own ladder test; all are
  cost-only inputs.
- **Determinism:** the DP breaks ties toward the smaller trailing occurrence index (strict
  `<`), is independent of `stats` input order (looked up by occ id), and `binary2fj`/
  `factor`/`validate` are deterministic functions of (normalized, order) — asserted by the
  shuffled-stats test and the equal-plans lowering test.
- **Estimate/actual alignment in EXPLAIN:** `estimates[k]` (bindings after joining the
  k-th occurrence) is compared against `node_entries[k+1]` (`actual_after`), which is the
  matching quantity modulo the semijoin effect of node k's probes; guarded against length
  mismatch. Honesty-reporting only.
