# schema-005: `IntervalTail.width: Option<u64>` — general vs fixed as absence

- **Severity:** medium
- **Tree:** schema
- **Status:** OPEN
- **Source:** audit/storage-schema.md F6
- **Depends on:** none
- **Conflicts with:** schema-002, schema-003 (tail type; land the enum, they store it)

## The bug

`crates/bumbledb/src/schema.rs:195-198`:

```rust
pub(crate) struct IntervalTail {
    /// `Some(w)` = the fixed width; `None` = general (`start ‖ end`).
    pub(crate) width: Option<u64>,
}
```

`ValueType::Interval { width: Option<u64> }` (`bumbledb-theory/src/schema.rs:114-117`) is the same Option-as-kind. Every `match self.width { None => 16, Some(_) => 8 }` reconstitutes the encoding. `Weight` already refused this: unit is a case, not an absence.

## Why it's wrong

Insight 8 — absence is a representational choice. Two encodings are two cases, not a missing width. Insight 3 — the special case (16-byte vs 8-byte tail) belongs to the representation.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree. Sealed witness (required):

```rust
enum IntervalTail { General, Fixed { width: u64 } }
```

`bytes()` / `words()` match. Cheap extra: `ValueType::Interval` as `General { element } | Fixed { element, width }` — this is Rust, not a C ABI. Spec/fingerprint encodings keep their current tag; the Rust type parses.

## Acceptance criteria

- [ ] Gone: `rg -n 'width: Option<u64>' crates/bumbledb/src/schema.rs` on `IntervalTail`.
- [ ] `IntervalTail::bytes` is a match on the enum, not `None => 16`.
- [ ] Unchanged tests: interval key/containment/capacity and fixed-width interval tests green, assertions untouched.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Encoding bytes identical (16 general, 8 fixed; `start_word + w` is the encoded end). Q2 bound unchanged. Descriptor JSON/spec spellings need not change in the same commit if `ValueType` stays Option at the theory boundary — but the *sealed* tail must be the sum.
