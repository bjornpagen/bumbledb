# schema-002: `KeyStatement` flattens `FunctionalityEvidence` into `tail: Option` + `fresh_row: bool`

- **Severity:** high
- **Tree:** schema
- **Status:** OPEN
- **Source:** audit/storage-schema.md F2
- **Depends on:** none (co-lands with schema-006)
- **Conflicts with:** schema-006, store-002 (same flags; land the sum first)

## The bug

`crates/bumbledb/src/schema/validate.rs:405-411` parses a functionality into a sum:

```rust
enum FunctionalityEvidence {
    Scalar,
    Pointwise(DisjointDeterminantProof, IntervalTail),
}
```

Sealing (`validate.rs:148-158`) throws the proof away:

```rust
tail: match evidence {
    FunctionalityEvidence::Pointwise(_, tail) => Some(tail),
    FunctionalityEvidence::Scalar => None,
},
fresh_row: projection.len() == 1
    && relations[relation.0 as usize].fresh_row_field() == Some(projection[0]),
```

`KeyStatement` (`schema.rs:397-428`) is then two independent flags — four states, three valid (fresh-row is U64, never an interval). `pointwise()` is `tail.is_some()`. Plan copies `pointwise: statement.tail` onto `DeterminantOp` (`storage/commit/plan.rs:378-383`). Judgment and point reads re-test `fresh_row`. `DisjointDeterminantProof` survives only on containment `Enforcement::IntervalCoverage`.

The one-word expect lives at the probe sites, not in schema: `api/db/get.rs:109` and `exec/dispatch/key_probe_fact.rs:275` — `expect("a fresh-row determinant is one u64 word")`.

## Why it's wrong

Insight 6 — parse, don't validate: the gate learned Scalar vs Pointwise-with-proof and returned `Option` + `bool`. Insight 4 — two flags, a nonsense combination, guards everywhere the key travels.

## The fix

Implementable under C1–C8. Proposed C9 would pin this shape; this issue is not blocked on C9.

```rust
enum KeyForm {
    FreshRow  { id: StatementId, relation: RelationId, field: FieldId },
    Scalar    { id, relation, projection: Box<[FieldId]> },
    Pointwise { id, relation, projection, tail: IntervalTail,
                disjoint: DisjointDeterminantProof },
}
```

- `pointwise()` dies; consumers match `KeyForm`.
- Plan/judgment/point-read match the form. FreshRow is one word by type — the `try_into().expect("a fresh-row determinant is one u64 word")` sites delete.
- `DisjointDeterminantProof` stays on the Pointwise arm (containment `IntervalCoverage` already does this).
- `DeterminantOp.pointwise: Option<IntervalTail>` dies with the copy; the insert neighbor-probe is a Pointwise-only field (or a Pointwise arm of the op).

## Acceptance criteria

- [ ] Gone: `rg -n 'fresh_row: bool' crates/bumbledb/src/schema.rs`; `rg -n 'fn pointwise' crates/bumbledb/src/schema.rs`; `rg -n 'FunctionalityEvidence' crates/bumbledb/src/schema/validate.rs` → the evidence enum either *is* `KeyForm` or is inlined into it (no flatten).
- [ ] Gone: `rg -n 'pointwise: statement.tail' crates/bumbledb/src/storage/commit/plan.rs`.
- [ ] Gone: `rg -n 'a fresh-row determinant is one u64 word' crates/bumbledb/src`.
- [ ] Unchanged tests: key/pointwise/fresh-row commit and point-read tests green, assertions untouched.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- R16 one-id-allocator semantics identical (fresh field IS the `F` row id; no `U` tree). Pointwise neighbor probe unchanged. `KeyId` arena stays.
- Do not make FreshRow pointwise-capable. Do not drop `IntervalTail` off the Pointwise arm and re-walk the projection.
