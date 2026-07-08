# PRD 03 — Schema validation: the roster and the acceptance gate

**Depends on:** 02.
**Modules:** `crates/bumbledb/src/schema/validate.rs`, `crates/bumbledb/src/error.rs` (`SchemaError`).
**Authority:** `docs/architecture/30-dependencies.md` (§ the acceptance gate, § validation roster).

## Goal

`SchemaDescriptor::validate()` rewritten: field checks stay, constraint checks are
replaced by the statement roster, and every accepted statement gets its `Resolved`
data computed. The roster in `30-dependencies.md` is exhaustive — implement all of
it, nothing else.

## Technical direction

1. Keep from the current `validate.rs`: relation/field name duplication checks,
   enum shape checks (nonempty, ≤256, no duplicate variants), serial-must-be-U64.
   Delete: everything mentioning constraints (auto-unique materialization moves to
   the statement materializer from PRD 02; FK resolution dies).
2. Per-statement checks, in order, each with a distinct `SchemaError` variant
   (extend the enum; carry `StatementId` + the offending `FieldId`/position):
   - unknown relation / field ids on either side;
   - empty projection; duplicate field within a projection or selection;
   - **Functionality:** selection must be empty (reject "conditional keys");
     projection over the whole relation is legal only implicitly (the judgment is
     `R(X) -> R`; there is no Y side to check); at most one `Interval` field in the
     projection and it must be the **last** position; duplicate-Functionality check
     (two FDs with identical ordered projections on one relation — pure write
     amplification, reject, same rationale as the old duplicate-unique rule);
     guard width: Σ field widths ≤ `MAX_GUARD_WIDTH` (import from storage keys;
     16-byte intervals count as 16).
   - **Containment:** |source.projection| == |target.projection|; positional
     structural type equality (`ValueType` derive-eq, exactly like the old FK rule);
     a selected field must not appear in the same side's projection; selection
     literal type equality against the field's `ValueType` (enum ordinal in range;
     UTF-8 for strings; `start < end` for interval literals);
     **target-key resolution:** the target projection, as a *set*, must equal the
     field set of some `Functionality` statement on the target relation — resolve
     it, record `target_key`, and compute `key_permutation` (mapping statement
     projection order → the key's guard order). Ambiguity is impossible (duplicate
     FDs are rejected above).
     **Pointwise gate:** if any projection position is `Interval` (both sides —
     type equality already forces both-or-neither), require exactly one such
     position, require the resolved target key to carry its interval field (i.e.
     the key is pointwise), and record `interval_position`.
   - **Duplicate statements:** two statements with identical descriptors after
     normalization (selections sorted by FieldId) are rejected.
3. Output: sealed `Schema` with the `Statement`/`Resolved` list and the
   per-relation `keys`/`outgoing`/`incoming` indices (PRD 02 shapes).
4. Every rejection message's doc comment cites the roster line in
   `30-dependencies.md` it implements. The roster is a checklist: implement it
   top-to-bottom and tick each item off in the PRD file when done.

## Out of scope

Fingerprint (PRD 04). IR validation (PRD 12). Enforcement (PRDs 07–09).

## Passing criteria

- `[shape]` One `SchemaError` variant per roster line; no catch-all
  `InvalidStatement` variant exists.
- `[test]` A reject-corpus unit test module (mirroring the existing
  `schema/tests/reject.rs` style) with **at least one test per roster line**,
  each asserting the exact error variant: FD-with-selection, non-final interval,
  two intervals, guard overflow (construct with enough 16-byte fields), containment
  arity mismatch, positional type mismatch, selected-and-projected field, no
  matching key, interval containment against a non-pointwise key, out-of-range enum
  selection literal, `start ≥ end` interval literal, duplicate statement.
- `[test]` An accept test: the `30-dependencies.md` example schema (Holder /
  Account / SavingsTerms with its three statements plus serials) validates, and the
  test asserts each statement's `Resolved` contents (target keys, permutation,
  interval positions) exactly.
- `[test]` A `==`-shaped pair (two mirrored Containments) validates, with each
  direction independently resolved.
