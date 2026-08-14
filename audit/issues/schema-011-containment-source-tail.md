# schema-011: containment `source_tail: Option` is IntervalCoverage's sidecar

- **Severity:** medium
- **Tree:** schema
- **Status:** OPEN
- **Source:** adversarial validation of storage-schema F2/F3 leftovers (not in the dump as its own F)
- **Depends on:** none (IntervalTail enum is schema-005; can land the Option-into-arm first)
- **Conflicts with:** none (ContainmentStatement / `Enforcement::IntervalCoverage` only; schema-004 is capacity)

## The bug

Containments already have a three-arm `Enforcement` (`schema.rs:277-297`): `ScalarProbe` | `IntervalCoverage` | `Closed`. Sealing then stores the SOURCE interval encoding beside it (`schema.rs:447`):

```rust
pub(crate) source_tail: Option<IntervalTail>,
```

Validate copies `relations[source].interval_tail(&source.projection)` (`validate.rs:189-190`). Coverage judgment recovers the proof with expect:

```rust
// judgment.rs:1192-1194
let source_tail = probe.source_tail.expect("coverage probes carry their source tail");
// judgment.rs:639-641
let source_tail = schema.source_tail(statement)
    .expect("an interval containment has an interval source position");
```

`Schema::source_tail` (`schema.rs:253-255`) is a pass-through of the Option. ScalarProbe/Closed-with-a-tail and IntervalCoverage-without are representable. The positional-type gate already made "coverage ⇒ source has an interval" a fact; the witness stored a hole.

This is not schema-002 (`KeyStatement.tail` / `KeyForm`) and not schema-003 (capacity weight/bound tails). Target-side pointwise lives on the key; this Option is the *source* encoding of the same seam (Q1: the two tails may differ in width).

## Why it's wrong

Insight 6 — validate learned the source tail and returned `Option`. Insight 4 — `Enforcement` already names whether coverage runs; the sidecar restates that with a hole. The expects are shotgun parsing.

## The fix

Implementable under C1–C8. Put the source tail in the arm that needs it:

```rust
enum Enforcement {
    ScalarProbe { target_key: KeyId, key_permutation: Box<[u16]> },
    IntervalCoverage {
        target_key: KeyId,
        key_permutation: Box<[u16]>,
        disjoint: DisjointDeterminantProof,
        source_tail: IntervalTail, // SOURCE encoding; target tail stays on KeyForm::Pointwise
    },
    Closed { members: MemberSet },
}
```

`ContainmentStatement.source_tail` and `Schema::source_tail` die. `check_coverage` matches `IntervalCoverage { source_tail, .. }`. The expects delete.

Do **not** fold this into schema-004: capacity must *lose* `IntervalCoverage`; containments must *keep* it and give it the tail.

## Acceptance criteria

- [ ] Gone: `rg -n 'source_tail: Option' crates/bumbledb/src/schema.rs`.
- [ ] Gone: `rg -n 'expect\("coverage probes carry their source tail' crates/bumbledb/src`; `rg -n 'an interval containment has an interval source position' crates/bumbledb/src`.
- [ ] `Enforcement::IntervalCoverage` carries `source_tail: IntervalTail`; ScalarProbe/Closed cannot.
- [ ] Unchanged tests: pointwise containment commit/sweep/Q1 mixed-width tests green, assertions untouched. Coverage walk still width-blind across the seam.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Q1 unchanged: source and target tails may differ; both stay; do not merge them; do not re-walk either projection.
- Do not replace the expects with a runtime skip of coverage. Do not drop reverse-edge tail bytes.
- Containment `==` / `mirror` pairing unchanged (schema-007).
