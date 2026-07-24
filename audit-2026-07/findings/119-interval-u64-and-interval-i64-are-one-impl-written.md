## Interval&lt;u64&gt; and Interval&lt;i64&gt; are one impl written twice; the Lean spec already has the unifying representation

category: unification | severity: low | verdict: CONFIRMED | finder: engine:interval-allen
outcome: fixed f6d1719d

### Summary

`crates/bumbledb-theory/src/interval.rs` contains two inherent impl blocks — `impl Interval<u64>` (lines 26-79) and `impl Interval<i64>` (lines 81-129) — that are verbatim-parallel across all seven members: `MAX_END`, `new`, `ray`, `fixed`, `is_ray`, `start`, `end`. The doc comment stating the point-domain law ("points are `MIN ..= MAX_END − 1`, and `end == MAX_END` denotes the unbounded ray") appears twice, character for character (lines 27-29 and 82-84). The only genuine semantic difference between the ~50-line blocks is one method call inside `fixed`: `start.checked_add(width)` (line 57) vs `start.checked_add_unsigned(width)` (line 107) — both take `width: u64` because width is a point count in both domains (the i64 doc at lines 101-103 says so itself).

The closed two-element domain is thus enforced by writing the code twice rather than by a representation that states the law once. This inverts the crate's own doctrine (representation over duplication), and — decisively — inverts the crate's own **Lean formalization**, which already models the element domain the unified way.

### Evidence

- `crates/bumbledb-theory/src/interval.rs:26-79` vs `:81-129` — byte-for-byte parallel bodies. E.g. `new` at 34-36 (`(start < end).then_some(Self { start, end })`) against 89-91 verbatim; `ray` at 42-44 vs 97-99; `is_ray` at 64-66 vs 114-116; accessors at 70-78 vs 120-128.
- `interval.rs:57` vs `:107` — the sole real delta: `checked_add` vs `checked_add_unsigned`, identical `.filter(|end| *end < Self::MAX_END)` and identical `Self::new(start, end)` tail.
- **The spec already states the law once.** `lean/Bumbledb/Values.lean:159-165` defines `class PointDomain (α : Type) ... where maxEnd : α; gap : α → α → Nat; le_refl ...` — a typeclass carrying exactly the ceiling constant and a width hook. Lines 167-175 give the two instances (`U64`, `I64`). Lines 188-227 define ONE generic `Interval α`, with `Interval.isRay` defined once against `PointDomain.maxEnd` (lines 212-213) and its decidability instance once (215-216). The Rust code that claims to mirror this spec ("mirroring `crate::Interval`", Values.lean:181) is the only side of the mirror written twice.
- Feasibility of unification, checked against all call sites: `MAX_END` is consumed downstream (`crates/bumbledb/src/api/prepared/bind.rs:805,818`, `crates/bumbledb/tests/schema_macro.rs:443`, bench crates) — a generic `pub const MAX_END: T = T::MAX_END;` on `impl<T: Element> Interval<T>` preserves the `Interval::<u64>::MAX_END` spelling exactly. No caller uses `is_ray`/`start`/`end` in a const context (grepped all crates), so losing `const fn` on `is_ray` (whose body needs generic `==`) breaks nothing; `start`/`end`/`bounds` can stay `const fn` (the file already has a generic `const fn bounds` at line 139, proving the pattern).
- The struct doc (`interval.rs:15-19`) says "the two inherent impls below are the whole surface" — the intent it documents is the **closed domain**, which a sealed trait preserves identically; duplication is the mechanism, not the requirement.

Nuances that slightly narrow the original finding: (1) the two `From<Interval<_>> for Value` impls (lines 144-158) map to *distinct* `Value` variants (`IntervalU64` vs `IntervalI64`), so they are not pure duplication — unifying them would need a variant hook on the trait and can reasonably stay as two trivial impls; (2) Lean's fixed-width carriers `FixedU64`/`FixedI64` (Values.lean:384-389) and their `not_ray` theorems are themselves duplicated, so the `fixed` member's split has a spec-side parallel — but the core surface (`maxEnd`, the parse invariant, `isRay`) does not.

### Bench impact

None — this is a code-shape finding. The failure mode is drift: any future change to the point-domain law (e.g. a tweak to the Q2 strictness in `fixed`, or a doc correction) must be applied twice, and a one-sided edit compiles silently. The duplicated doc law has already forked once in a benign way (the i64 `fixed` doc paraphrases instead of repeating the u64 one).

### Suggested fix

Mirror the Lean `PointDomain` class as a sealed Rust trait:

```rust
mod sealed { pub trait Sealed {} impl Sealed for u64 {} impl Sealed for i64 {} }
pub trait Element: sealed::Sealed + Ord + Copy {
    const MAX_END: Self;
    fn add_width(self, w: u64) -> Option<Self>; // the `gap` dual
}
```

with `impl Element for u64` (`checked_add`) and `impl Element for i64` (`checked_add_unsigned`), then one `impl<T: Element> Interval<T>` block carrying `MAX_END`, `new`, `ray`, `fixed`, `is_ray`, `start`, `end` — the law stated once, the domain still closed (the trait is sealed, so no third element type is constructible), the public call-site spelling (`Interval::<u64>::fixed(3, 5)`, `Interval::<i64>::MAX_END`) unchanged. Update the struct doc at interval.rs:15-19 to say the domain is closed by the sealed element trait rather than by the two impls. The Lean citation trail is unaffected: `PointDomain` is the spec-side name for exactly this trait.
