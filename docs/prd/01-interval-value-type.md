# PRD 01 — Interval value type and encoding

**Depends on:** nothing.
**Modules:** `crates/bumbledb/src/schema.rs` (ValueType), `crates/bumbledb/src/schema/type_desc.rs`,
`crates/bumbledb/src/encoding/{encode.rs,decode.rs,layout.rs,tests.rs}`, `crates/bumbledb/src/lib.rs` (public export).
**Authority:** `docs/architecture/10-data-model.md` (type table, denotation, encoding).

## Goal

`Interval` becomes the seventh structural type: 16 bytes, `start ‖ end`, each half in
the element type's order-preserving encoding, strictly `start < end`.

## Technical direction

1. Add to the schema type layer:
   ```rust
   pub enum IntervalElement { U64, I64 }
   // ValueType gains:
   Interval { element: IntervalElement }
   ```
   Do **not** make `ValueType` recursive (no `Interval(Box<ValueType>)`) — the
   element domain is closed to the two orderable scalars and the flat enum makes
   illegal elements unrepresentable.
2. `TypeDesc` (schema/type_desc.rs) gains a 16-byte width variant. Every place that
   matches on width (encoding layout, guard slicing) now handles {1, 8, 16}.
   `FactLayout` (encoding/layout.rs) needs no algorithmic change — fields stay
   dense, declaration-ordered, unpadded; only the width table grows.
3. Encoding (encoding/encode.rs / decode.rs):
   - Encode: element-encode `start` (U64 = big-endian; I64 = sign-flipped
     big-endian — reuse the existing scalar encoders, do not duplicate them), then
     `end`, concatenated. The 16 bytes therefore sort lexicographically by
     `(start, end)` — add a doc comment stating this is load-bearing for
     `50-storage.md`'s neighbor probes.
   - Decode: decode both halves, then **validate `start < end`**; violation returns
     the existing corruption error pathway (same class as a non-0/1 Bool byte),
     never a value.
   - Encode-side validation: the public value type (below) makes `start ≥ end`
     unconstructible, so the encoder can `debug_assert!` it.
4. Public value type, exported from the crate root (`lib.rs`):
   ```rust
   pub struct Interval<T> { start: T, end: T }   // fields private
   impl Interval<i64> { pub fn new(start: i64, end: i64) -> Option<Self>; pub fn start(&self)...; pub fn end(&self)...; pub const MAX_END: i64 = i64::MAX; }
   // identical impl for u64
   ```
   `new` returns `None` on `start >= end` — parse, don't validate. Provide
   `pub fn from_start(start: T) -> Option<Self>` for the unbounded convention
   (`end = MAX_END`); `start == MAX_END` yields `None`. No other constructors, no
   `Default`, no arithmetic. Derives: `Copy, Clone, PartialEq, Eq, Hash, Debug`.
   **No `Ord`/`PartialOrd` derive** — value order is an encoding accident
   (`10-data-model.md` orderability) and must not leak into host code.
5. `ValueRef` (encoding decoded-value type) gains `IntervalU64(u64, u64)` and
   `IntervalI64(i64, i64)` variants; decode produces them, and every exhaustive
   match over `ValueRef` in the crate is extended (let the compiler find them —
   no `_ =>` arms may be added anywhere to silence this).

## Out of scope

IR `Value` variants (PRD 11), image columns (PRD 14), guard keys (PRD 06/07),
macro syntax (PRD 05).

## Passing criteria

- `[shape]` `ValueType::Interval { element }` exists with the flat `IntervalElement`
  enum; no `Box` anywhere in the type description.
- `[shape]` `Interval<T>` is exported at crate root with private fields, `new` /
  `from_start` returning `Option`, and no `Ord`/`PartialOrd` impl.
- `[test]` Round-trip property tests over both element types covering: `i64::MIN`
  start, `MAX_END` end, `start + 1 == end` minimal width, and random pairs.
- `[test]` Ordering property: for 1,000 random interval pairs, byte-wise comparison
  of encodings equals `(start, end)` tuple comparison under the element order.
- `[test]` Decode of a buffer with `start ≥ end` (both equal and inverted cases,
  both element types) returns the corruption error, never a value.
- `[gate]` No `_ =>` wildcard arm was introduced in any match over `ValueType`,
  `TypeDesc`, or `ValueRef` by this PRD.
