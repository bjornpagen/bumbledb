# schema-008: sealed `hi: Option<Bound>` — `*` as absence, contradicting `Weight::Unit`

- **Severity:** medium
- **Tree:** schema
- **Status:** OPEN
- **Source:** audit/storage-schema.md F16
- **Depends on:** schema-003
- **Conflicts with:** schema-003 (same constructor)

## The bug

Theory `Weight` is explicit: "`Unit` is a case, not an absence." Sealed `CapacityStatement.hi: Option<Bound>` (`schema.rs:484-487`) is the `*` spelling as `None`. `resolve_bound` (`judgment.rs:172-180`) returns `Option<u64>`; the judge does `hi.is_some_and(|hi| measure > u128::from(hi))`. Unbounded is a real window, not a missing bound.

## Why it's wrong

Insight 8 — the same module already made unit a case. `*` is the ceiling sibling of unit: a spelling, not a hole. Insight 4 — `hi: None` vs `Some(Bound)` vs a Duration bound that also needs `bound_tail` (schema-003).

## The fix

Implementable under C1–C8 with schema-003 (not blocked on proposed CONTRACT C9). `SealedBound::Unbounded` is the witness spelling. Descriptor `hi: Option<Bound>` may stay the hostile `*` spelling (schema-010 analog). Witness parses. Judge matches Unbounded vs a resolved ceiling — no `is_some_and` on the window.

## Acceptance criteria

- [ ] Gone: `rg -n 'hi: Option<Bound>' crates/bumbledb/src/schema.rs` on `CapacityStatement` (descriptor may still have it).
- [ ] Judge window compare is a match on unbounded vs ceiling, not `hi.is_some_and`.
- [ ] Unchanged tests: `0..n`, `{n}`, `lo..*`, and dependent-bound capacity tests green.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Lands with schema-003. `*` meaning identical (no upper bound). C6 dependent floors stay unrepresentable. Vacuous `0..*` stays a roster refusal at the descriptor.
