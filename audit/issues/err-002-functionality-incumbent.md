# err-002: `Violation::Functionality.incumbent: Option` — scalar vs pointwise as absence

- **Severity:** medium
- **Tree:** err
- **Status:** OPEN
- **Source:** audit/storage-schema.md F10
- **Depends on:** none
- **Conflicts with:** err-001, err-003 (same `Violation` / cited-facts shape)

## The bug

`error.rs:926-933` — Functionality carries `incumbent: Option<Box<[u8]>>`. Doc: "`None` for a scalar put-conflict, where the determinant bytes inside `fact` already identify the collision." Pointwise carries both parties. Two conviction shapes, one product. `cited_facts` treats incumbent as optional parallel data.

## Why it's wrong

Insight 4 — Option-as-kind. Scalar-with-incumbent and pointwise-without are representable. The probe already knew which shape it was.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree.

```rust
enum Violation {
    Functionality(FunctionalityViolation),
    Containment { .. },
    Capacity { .. },
}
enum FunctionalityViolation {
    Scalar { statement: StatementId, fact: Box<[u8]> },
    Pointwise { statement: StatementId, fact: Box<[u8]>, incumbent: Box<[u8]> },
}
```

`citation()` / `attach_cited` match. No Option.

## Acceptance criteria

- [ ] Gone: `rg -n 'incumbent: Option' crates/bumbledb/src/error.rs`.
- [ ] Scalar convictions cannot carry an incumbent; pointwise convictions must.
- [ ] Unchanged tests: functionality reject tests (scalar put-conflict vs pointwise neighbor) green; Display text identical.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Complete violation set, citation identity (statement id; no direction on keys), and decode-at-reject-time (err-003) unchanged. Bindings still see fact then incumbent on the pointwise arm.
