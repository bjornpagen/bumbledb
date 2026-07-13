# PRD 06 — PointIn: the membership predicate stops borrowing Allen's word

**Depends on:** Phase B complete (avoid double-churn; ir.rs is quiet).
**Modules:** `crates/bumbledb/src/ir.rs` (`CmpOp::Contains` :268 and
the `compare` unreachable arm :285-287), `ir/normalize/place_comparisons.rs`
(`ClassifiedComparison::ContainsVarVar` :184, `ContainsVarPoint` :209),
`ir/normalize/fold.rs:133`, `ir/validate/context.rs` (the shape arms),
`ir/render.rs`, `crates/bumbledb-query` (the macro's lowering of the
surface `in` keyword), bench querygen/translate/naive mirrors, docs.
**Authority:** the three-way `Contains` overload (audit deep issue #7),
verified current: `CmpOp::Contains` is now membership-ONLY (the
interval⊇interval form already moved to `Allen(COVERS)` — half the
audit's complaint is already fixed), but the NAME still collides with
`allen::Basic::Contains` and with dependency-containment prose. One
word, one meaning: the point predicate is `PointIn`.
**Representation move:** none — a grep-zero rename with the same
discipline as `ConditionTree`.

## Context (decided shape) — the rename ledger

- `CmpOp::Contains` → `CmpOp::PointIn` (+ the doc comment rewritten:
  "point membership: an element-typed operand against an interval
  operand; `x PointIn iv ⟺ lo ≤ x < hi`").
- `ClassifiedComparison::ContainsVarVar` → `PointInVarVar`;
  `ContainsVarPoint` → `PointInVarPoint`.
- Any `contains`-named locals/helpers on the membership path in
  normalize/validate/render/bench mirrors follow.
- NOT renamed (verified different concepts, listed in the commit
  body): `allen::Basic::Contains` (the literature term for
  interval-strictly-contains — keeps its name); dependency
  "containment"/`ContainmentStatement`/`ContainmentId` (the IND
  vocabulary — stays); `MemberSet::contains` (std-idiomatic method);
  `Iterator`/std `contains` idioms.
- The SURFACE syntax is unchanged: the query macro's `in` keyword and
  the notation renderer's output do not move — verify zero golden
  churn (the renderer emits the surface operator, not the variant
  name; if any golden contains `Contains`, that is the render of the
  IR debug form — decide: rendered IR text follows the rename, and
  those goldens update as part of this PRD, values otherwise
  identical).

## Technical direction

Compiler-driven single motion; then the string surfaces (error display
if any names the variant, IR render, docs). `20-query-ir.md`'s
comparison section states the three-concept split in one line:
"`PointIn` (point ∈ interval), `Allen(mask)` (interval × interval),
containment `<=` (views) — three predicates, three names."

## Passing criteria

- `[shape]` `grep -rn "CmpOp::Contains\|ContainsVarVar\|ContainsVarPoint" crates fuzz` → zero.
- `[shape]` `grep -rn "PointIn" crates/bumbledb/src/ir.rs` shows the
  variant; `allen.rs`'s `Contains` untouched.
- `[test]` Notation goldens: zero value changes (or the enumerated IR-
  debug renames only, listed in the commit body); full suite green;
  bounded fuzz smoke (rewrites) per policy 7.
- `[gate]` Fingerprint pin untouched (IR names are not fingerprint
  inputs — assert by the pin test); clippy; fmt.

## Doc amendments (rule 6)

`20-query-ir.md` comparison section gains the three-predicate line;
`docs/cookbook.md` membership prose says "point membership (`in`)"
wherever it says "contains" for the point predicate.
