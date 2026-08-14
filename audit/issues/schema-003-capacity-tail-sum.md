# schema-003: capacity `weight_tail` / `bound_tail` sidecar Options; judge `.expect`s the proof

- **Severity:** high
- **Tree:** schema
- **Status:** OPEN
- **Source:** audit/storage-schema.md F3
- **Depends on:** none (co-lands with schema-008)
- **Conflicts with:** schema-008 (same `CapacityStatement` constructor)

## The bug

`Weight` is already the right sum (`Unit` is a case, not an absence). Sealing then stores tails *beside* it (`schema.rs:499-507`):

```rust
pub(crate) weight_tail: Option<IntervalTail>, // Some iff DurationOf
pub(crate) bound_tail: Option<IntervalTail>,  // Some iff TargetDuration
```

`measure_weight` / `resolve_bound` (`storage/commit/judgment.rs:140-198`) take the pair and recover the proof with `.expect("validate seals a tail for every Duration weight")`. Eight combinations; two legal pairings.

## Why it's wrong

Insight 6 — the gate matched `Weight::DurationOf` and learned the `IntervalTail`, then stored an Option a Duration-less statement can still carry. The judge re-validates. Insight 4 — independent Options.

## The fix

Per proposed C9 (schema-001). Put the tail in the arm:

```rust
enum SealedWeight {
    Unit,
    Field(FieldId),
    Duration { field: FieldId, tail: IntervalTail },
}
enum SealedBound {
    Unbounded,  // schema-008
    Lit(u64),
    TargetField(FieldId),
    Duration { field: FieldId, tail: IntervalTail },
}
```

`measure_weight` matches `SealedWeight`. The expect deletes. Descriptor `Weight` / `hi: Option<Bound>` may stay hostile (like schema-010); the witness parses.

## Acceptance criteria

- [ ] Gone: `rg -n 'weight_tail: Option' crates/bumbledb/src/schema.rs`; `rg -n 'bound_tail: Option' crates/bumbledb/src/schema.rs`.
- [ ] Gone: `rg -n 'expect\("validate seals a tail' crates/bumbledb/src/storage/commit/judgment.rs`.
- [ ] Unchanged tests: capacity unit/field/Duration and bound-Duration commit tests green, assertions untouched. `CapacityRayMeasure` still the C10 refusal.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- `Weight::Unit` name and count-instance semantics locked (theory C4). Window `*` spelling at the descriptor may stay `hi: None` until schema-008 parses it on the witness. Measure arithmetic (`end − start` in encoded words) identical.
