# PRD 01 — One Value

**Depends on:** nothing. **First for a reason:** every later PRD that touches
judgment, render, fingerprint, validation, or the macro lands on clean ground.
**Modules:** new `crates/bumbledb/src/value.rs` (or fold into an existing
zero-dependency home), `crates/bumbledb/src/ir.rs`, `crates/bumbledb/src/schema.rs`
and everything matching on `LiteralValue`:
`schema/{validate.rs,fingerprint.rs,render.rs,runtime.rs}`,
`storage/commit/judgment.rs`, tests.
**Authority:** `30-dependencies.md` ("dependencies and queries share one
representation" — this PRD makes the sentence true in code),
`00-product.md` (representation over control flow).

## Context (decided)

The crate's most fundamental sum type exists twice, byte-identically:
`ir::Value` (ir.rs:28) and `schema::LiteralValue` (schema.rs:90) — eight variants
each, same payloads. Every match over `LiteralValue` is a duplicated branch
ladder: ~8 arms in `judgment.rs`, 8 in `render.rs`, 11 in `fingerprint.rs`, 8 in
`validate.rs`, plus runtime and tests — roughly forty match arms that exist only
because the type was written twice. The keystone chapter claims one
representation; the code keeps two. Collapse them.

## Technical direction

1. **The home:** `Value` moves to a zero-dependency module both `ir` and
   `schema` can import (`ir` already imports `schema`, so `schema` cannot import
   `ir` — a crate-root `value.rs` with no internal imports breaks the knot).
   Contents: the enum exactly as `ir::Value` is today (variant docs included),
   the `From<Interval<u64>>`/`From<Interval<i64>>` impls, and nothing else — no
   methods that belong to a consumer (encoding stays in `encoding`, rendering in
   `render`).
2. **Public surface:** `ir` re-exports it (`pub use crate::value::Value`) so the
   normative IR block in `20-query-ir.md` stays truthful; the crate root exports
   it as today. (A re-export is an API-surface choice, not a compat shim — there
   is exactly one type.)
3. **Delete `LiteralValue` entirely.** `Side.selection` becomes
   `Box<[(FieldId, Value)]>`. Every former `LiteralValue` match site now matches
   `Value` — and most should **collapse into calls to the one canonical
   implementation** rather than surviving as parallel ladders: selection-literal
   encoding goes through the canonical `encoding` path (judgment's pre-encoded
   literals and fingerprint's serialization must be the *same function*, not
   two matches that agree); rendering goes through one value formatter.
   The measure of success is deleted match arms, not renamed ones.
4. **Semantic guard, stated in `value.rs`'s doc:** `Value` is dumb data
   everywhere — `start < end` for intervals and UTF-8 for strings are boundary
   rules (IR validation; schema validation for selections), not constructor
   invariants. Nothing about either boundary's rules changes.
5. `ValueRef` (encoding) stays — the borrowed twin is a real distinction
   (owned/borrowed), documented as such where `ValueRef` is defined.
6. Rule-5 check: grep the architecture chapters for `LiteralValue`; amend any
   occurrence to `Value`.

## Passing criteria

- `[shape]` `LiteralValue` does not exist (grep whole workspace); `Value` has
  one definition in a module with zero internal imports; `ir::Value` resolves
  to it.
- `[shape]` Selection-literal canonical encoding has exactly one definition
  site, consumed by judgment and fingerprint both (grep the encode calls).
- `[shape]` Net match-arm count over value variants decreases (report the
  before/after counts in the commit body — the point is deleted ladders).
- `[test]` Existing schema/judgment/render/fingerprint tests green unchanged
  (assertions untouched — this is behavior-preserving).
- `[test]` Fingerprint stability: a schema fingerprinted before and after this
  PRD yields the identical hash (pin with one golden — the serialization must
  not have drifted during the collapse).
- `[gate]` Workspace gates green.
