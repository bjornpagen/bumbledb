# PRD 01 — The formal anchor: the Lean model enters the repository

**Depends on:** nothing. First, because the docs PRDs cite its table.
**Modules:** new `docs/formal/` (the Lean artifact + provenance README);
`docs/architecture/30-dependencies.md` and `60-validation.md` (the
theorem↔evidence table).
**Authority:** the audit's epistemic-label discipline: every public
semantic claim is either a Lean theorem, a mathematical consequence of
Lean + validator premises, a validator acceptance rule, or a runtime
invariant — and the reader must be able to tell which.
**Representation move:** the formal model stops being a Downloads-folder
artifact and becomes a versioned repository citizen the docs can cite by
theorem name.

## Context (decided shape)

1. `docs/formal/GPT55DependencyTheory.lean` — copied verbatim from
   `/Users/bjorn/Downloads/GPT55DependencyTheory.lean`, byte-identical.
2. `docs/formal/README.md` — provenance (produced by the gpt55 audit
   2026-07-13; pinned against `98f1103`; checked with
   `leanprover/lean4:v4.32.0`, no axioms, no sorry), the honest scope
   statement (the model covers the pure dependency/query semantics — it
   does NOT model parsing, storage bytes, overflow, interning, or
   closed-extension completeness; those are Rust obligations), and the
   note that re-running the Lean checker is registered human work (no
   Lean toolchain in this repo's gates).
3. **The theorem↔evidence table**, placed in `30-dependencies.md` as a
   closing section and cross-linked from `60-validation.md`. One row
   per public semantic claim; columns: claim · Lean theorem (or
   countermodel) · Rust evidence (validator rule / representation /
   always-on check, with file anchors) · label. Required rows, minimum:
   - containment = selected projected view subset
     (`contains_iff_view_subset` · `resolve_target_key` exact-set rule).
   - bare `==` = view equality, NOT unique correspondence
     (`containsEq_iff_view_ext` + `bare_containsEq_nonunique`
     countermodel · the two-containment lowering).
   - accepted `==` = key-backed unique correspondence
     (`KeyBackedEquality.unique_target/.unique_source` · both targets
     must resolve to declared keys).
   - key ⟹ uniqueness, not existence.
   - pointwise interval key ⟹ per-group disjointness.
   - one-way coverage = source-support inclusion; overhang legal
     (`overshoot_isTiling_not_exact` countermodel · `check_coverage`
     direction).
   - exact partition = mutual coverage + pointwise keys
     (`exactTiling_iff_exactPointPartition` · the five-statement idiom,
     PRD 11).
   - empty intervals make coverage vacuous
     (`empty_nat_interval_has_no_points` · the `Interval` constructor +
     six boundary rejections + PRD 02's total encoder).
   - negation safety = positive range restriction
     (`positive_range_restriction_implies_wellscoped` ·
     `NegatedVariableUnbound`, order-independent).
   - rule union is set-idempotent (`ruleUnion_set_idempotent` · the
     sink-owned union seen-set).
   - checked bounded sums (`checkedAdd_sound` · i128/u128 accumulation
     + finalize range check).

## Technical direction

Copy the file; do not edit it (any Lean edit is a new formal-model
version and out of scope). Write the table from the reconciliation
ledger's verified anchors — every Rust evidence cell cites a mechanism
name that exists on main today; where the evidence lands in a LATER PRD
of this set (DisjointGuardProof, the partition locks), the cell says so
explicitly ("delivered by PRD 03") and PRD 18 verifies the cell was
updated when it landed.

## Passing criteria

- `[shape]` `docs/formal/GPT55DependencyTheory.lean` byte-identical to
  the source artifact (record its SHA-256 in the provenance README and
  verify).
- `[shape]` The provenance README states scope, toolchain, and the
  human-work registration.
- `[shape]` The table exists with all eleven rows minimum, every Rust
  evidence cell carrying a real mechanism name; zero TBD cells.
- `[gate]` Docs-only change; workspace gates untouched and green.

## Doc amendments (rule 6)

This PRD is its amendments; `docs/architecture/README.md` gains one
line pointing at `docs/formal/`.
