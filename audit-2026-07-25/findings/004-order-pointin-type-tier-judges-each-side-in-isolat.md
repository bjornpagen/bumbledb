## Order/pointIn type tier judges each side in isolation — every cross-domain pairing compiles; R3's bool admission widened the hole

bug | high | CONFIRMED | ts-surface-fresh
outcome: fixed 9241feb3

### Summary

The TS type tier's condition judgment (`CondOkBool`, `ts/src/query/atom.ts:653-670`) checks order-comparison sides **independently** — the order arm is `[OrderSideOk<L>, OrderSideOk<R>] extends [true, true]` with no cross-side domain-agreement judgment — and the `pointIn` arm's `PointSideOk` (atom.ts:571) never compares the point's kind to the interval's element. The engine enforces exactly those pairwise walls at prepare (`crates/bumbledb/src/ir/validate/context.rs:1148-1150` for order var-var, `:1178` for order literals, `:1306` for pointIn), refusing with the index-only, field-unnamed `IllegalComparison { index }`. Since `boolean` joined `OrderSide` (atom.ts:339, the R3 bool-orders ruling), the compile-clean mismatch surface covers bool-vs-numeric as well as u64-vs-i64. The construction tier doesn't catch it either: `validateCond` (`ts/src/query/lower.ts:968-1000`) holds the class wall for `eq`/`ne` var-var only (the `cond.op === "eq" || cond.op === "ne"` guard at :984); order ops get boundness plus the closed-reference screen, never kind agreement.

This directly breaks the module's own advertised contract: atom.ts:14-17 — "the walls the engine enforces at prepare are TYPES first" — and the `CondOkBool` docstring (atom.ts:646-651) calling itself "the type-level twin of the engine's comparison roster." `EqOk` (atom.ts:577-587) IS pairwise via `JoinOk`; the order and pointIn arms are the asymmetry.

### Evidence (all verified against the working tree)

- `ts/src/query/atom.ts:564-568` — `OrderSideOk<T>` judges a var by `OrderVarOk` (own field kind only), a `Duration` by its own var's intervalness, and everything else `true`. No sibling in scope.
- `ts/src/query/atom.ts:571` — `PointSideOk<T> = T extends AnyVar ? NumericVarOk<T> : true` — accepts any numeric var; the interval side's element never enters the judgment.
- `ts/src/query/atom.ts:658-665` — the order and pointIn arms of `CondOkBool` are per-side conjunctions, contrast `EqOk`'s `JoinOk<MintSlotOf<L>, MintSlotOf<R>>`.
- `ts/src/query/atom.ts:339` — `type OrderSide = AnyVar | Param<string> | Duration | bigint | boolean` (R3).
- `crates/bumbledb/src/ir/validate/context.rs:1139-1156` — `OrdVarVar`: after the per-operand screen, `if *self.resolved_var_type(*rhs) != lhs_type { return Err(ValidationError::IllegalComparison { index }); }`. `:1178` — `OrdVarConst` types the literal via `check_const(index, constant, &var_type)`. `:1301-1308` — `PointInVarVar`: `if *self.resolved_var_type(*rhs) != element_type(element) { return Err(IllegalComparison { index }) }`.
- `ts/src/query/lower.ts:984-994` — the construction-tier class wall fires only for `eq`/`ne` var-var.
- **Compile probe** (temporary `ts/test/__verify_probe.test.ts`, since deleted): schema with `flag: bool, count: u64, score: i64, window: interval(u64)`; all four of `lt(flag, count)`, `lt(count, score)`, `pointIn(score, window)`, `lt(count, true)` pass `pnpm exec tsc --noEmit` with **exit 0, zero diagnostics**.
- **Runtime probe** against the built addon (`Db.create` + `db.prepare`):
  - `lt(flag, count)` → `bumbledb irError (prepare): comparison 0: type rules violated`
  - `lt(count, score)` → same
  - `pointIn(score, window)` → same
  - `lt(count, true)` → `comparison literal: expected bigint, got boolean` (the sibling-typed literal path: `taggedCmpLiteral`, lower.ts:1647-1671, delegating to `taggedLiteral` lower.ts:1606-1610 — a lowering-time shape throw, not a compile refusal).

One line-number correction to the original finding: the sibling-typed literal logic at lower.ts:1594-1636 is `taggedLiteral`; the comparison-position wrapper `taggedCmpLiteral` is :1647-1671. The substance is unchanged.

### Failure scenario / impact

`r.where(lt(flag, count))` (bool vs u64) or `r.where(lt(score, true))` against a real schema typechecks, constructs, and freezes; the failure surfaces only at `db.prepare()` as an index-only engine error naming neither field nor type — and the boolean-literal spelling dies with a shape message ("expected bigint") that describes the literal's encoding, not the domain disagreement. No wrong results are possible (the engine wall holds), but the advertised compile-time wall — both sides' domains being statically known via var fields and `const`-inferred literal types — silently isn't there, and the diagnostic a user eventually gets is the worst one in the estate.

### Suggested fix

Add the pairwise judgment `EqOk` already models: in `CondOkBool`'s order arm, when both sides carry a known domain (var field kind, literal type — `bigint` splitting on nothing so staying wild against either numeric is fine, `boolean` exact, `Duration` = u64-measure), require agreement; in the pointIn arm, judge the point's kind against the interval side's element. Params and lone-literal-vs-param pairings stay wild, exactly as `EqOk` treats them. Mirror the same check into `validateCond`'s order/pointIn path for untyped callers, with a pointed message naming both fields and kinds (the `eq`/`ne` wall at lower.ts:990-992 is the template), matching the engine's context.rs:1148 verdict but named.