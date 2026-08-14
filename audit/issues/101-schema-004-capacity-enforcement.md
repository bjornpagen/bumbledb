# schema-004: capacity reuses containment `Enforcement`, so `IntervalCoverage` is representable then `unreachable!`

- **Severity:** medium
- **Tree:** schema
- **Status:** OPEN
- **Source:** audit/storage-schema.md F5
- **Depends on:** none
- **Conflicts with:** none (CapacityStatement.enforcement field only)

## The bug

`CapacityStatement.enforcement` (`schema.rs:489-493`) is the containment enum `Enforcement`, whose third arm is `IntervalCoverage`. The comment admits the lie: "capacity projections refuse interval positions, so `IntervalCoverage` is unreachable." `check_capacity` (`storage/commit/judgment.rs:1395-1398`) matches it and panics:

```rust
Enforcement::IntervalCoverage { .. } => {
    unreachable!(
        "capacity statements refuse interval positions in projections at the gate"
    )
}
```

The roster parsed the refusal (`CapacityIntervalPosition`); the witness kept the wider type.

## Why it's wrong

Insight 7 — a tag legal in every statement form, forbidden in one, is a polymorphism not yet named. The containment sum is the *right* coordinate for containments; sharing it with capacity is Fowler's type-code.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree.

```rust
enum CapacityEnforcement {
    ScalarProbe { target_key: KeyId, key_permutation: Box<[u16]> },
    Closed { members: MemberSet },
}
```

Containments keep three-arm `Enforcement`. The capacity `unreachable!` deletes.

## Acceptance criteria

- [ ] Gone: `rg -n 'IntervalCoverage' crates/bumbledb/src/storage/commit/judgment.rs` matches containment paths only, not `check_capacity`.
- [ ] `CapacityStatement.enforcement` is `CapacityEnforcement` (or equivalent two-arm sum), not `Enforcement`.
- [ ] Unchanged tests: capacity commit/sweep tests green, assertions untouched.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Probe-ability rule reused verbatim (same `resolve_target_key` for ScalarProbe/Closed). Interval positions on capacity *projections* stay the typed roster refusal `CapacityIntervalPosition`.
- Do **not** replace the `unreachable!` with a runtime skip / `continue`. The arm must become unrepresentable. Assertions never weakened.
