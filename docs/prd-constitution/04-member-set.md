# PRD 04 — MemberSet: the closed bitset stops being anonymous words

**Depends on:** 03 (it retypes the `Closed` variant 03 just touched —
run after to avoid conflicting edits in schema.rs).
**Modules:** `crates/bumbledb/src/schema.rs` (`Enforcement::Closed
{ members: [u64;4] }` :377, `closed_member` :386-391), its four callers
(`storage/commit/judgment.rs:224`, `schema/validate.rs:443`,
`verify_store/facts.rs:156,288`), the sealed-check compilation that
builds the bitset, closed-extension row indexing.
**Authority:** the audit's impossible-state discipline: a bare
`[u64; 4]` plus a free function encodes neither the 256-member bound,
nor the row-index domain, nor the out-of-range rule ("absent, not
error") anywhere a reader can see it.
**Representation move:** the bitset and the index each get a type; the
membership rule becomes a method contract.

## Context (decided shape)

```rust
/// A closed relation's compiled member set: one bit per sealed
/// extension row, in sealed extension order. Out-of-range indices are
/// simply absent — the query-surface rule, now a method contract.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemberSet { words: [u64; 4] }

/// Index of a row in a sealed closed extension (≤ 256 rows by the
/// existing extension bound).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct RowIndex(pub(crate) u16);

impl MemberSet {
    pub(crate) fn contains(&self, idx: RowIndex) -> bool { … }
    pub(crate) fn insert(&mut self, idx: RowIndex) { … }
}
```

- `Enforcement::Closed { members: MemberSet }`.
- `closed_member(&[u64;4], u64)` is DELETED; the four call sites move
  to `members.contains(RowIndex(..))`. Where today's callers pass a
  `u64` id, the conversion to `RowIndex` happens at the boundary that
  KNOWS the id is a row index (the closed-image lookup) — a caller
  holding an arbitrary u64 must go through a fallible narrowing that
  encodes the absent-when-out-of-range rule.
- The 256 bound: `insert` on an index ≥ 256 is unreachable by
  construction if the sealed-extension bound proves it — find the
  bound's enforcement site (extension size validation) and cite it in
  the doc comment rather than adding a runtime check; if no such bound
  exists, that is a policy-5 stop (the `[u64;4]` capacity was then
  always an unchecked assumption — record and fix).

## Technical direction

Pin first: the closed-membership judgment tests and the PRD-15-era
exhaustive `closed_member` boundary suite (schema/tests/closed_member.rs,
834 patterns × 269 ids) must pass UNCHANGED — re-anchor its calls to
the new method mechanically; assertion values identical. Then the
retype + call-site chase. The exhaustive suite is the behavior lock;
zero tolerance for delta.

## Passing criteria

- `[shape]` `grep -rn "closed_member" crates` → zero;
  `grep -rn "\[u64; 4\]\|\[u64;4\]" crates/bumbledb/src/schema.rs` →
  only inside `MemberSet`'s definition.
- `[test]` The exhaustive boundary suite green with unchanged assertion
  values (224,346 cells); engine suite green.
- `[shape]` The 256-bound citation present in `MemberSet`'s doc (or the
  policy-5 record).
- `[gate]` Fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`10-data-model.md` § closed relations: one sentence — membership is a
typed row-index query; out-of-range is absence by contract.
