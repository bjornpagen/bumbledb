/**
 * Atoms and conditions, REFERENCE-IDENTITY edition — the body vocabulary of
 * a rule, mirroring the engine IR variant for variant
 * (`bumbledb/crates/bumbledb/src/ir.rs`, the bijection target;
 * `docs/architecture/20-query-ir.md` normative). A `match` binding record
 * binds fields to VARIABLES (minted by {@link v}), params, ∈-set params, or
 * bare structural literals — a closed-reference field's literal is its
 * handle NAME, and a plain ARRAY of names there is membership
 * (unmentioned fields ARE the wildcard — no wildcard value exists);
 * `not(Rel, {...})` is negation-as-position (anti-join); `eq`/`ne` and the
 * order roster, `pointIn` (the one spelling of `ir::CmpOp::PointIn`,
 * always lowered interval-left), `allen` (the 13-bit mask pair
 * comparison), and `and`/`or` (the input condition-tree grammar) complete
 * the roster. Nothing beyond the IR exists here — and the walls the engine
 * enforces at prepare are TYPES first: a variable joins only class-equal
 * fields, judged at every binding position against the var's MINT slot
 * ({@link MintSlotOf}); because {@link JoinOk} is an equality, that ALONE
 * makes every cross-binding join transitively class-equal (the env/sibling
 * checks the name-keyed edition needed are gone). An interval-typed var
 * under a non-`pointIn` comparison is unwritable. BOUNDNESS (a negated
 * atom's variables must be positively bound) is the one check types cannot
 * carry — object identity is invisible to TS — so it is a construction-time
 * wall only.
 *
 * This module also owns the plain runtime DATA a built rule is made of
 * (`RuleData`/`RecData` and friends): frozen values the lowering walks —
 * pure data (variable references included), so lowering stays a pure,
 * stable function of the query value.
 */

import * as errors from "@superbuilders/errors"
import type { AnyField, ClosedIdField, ClosedRoster, Infer, IntervalValue } from "#fields.ts"
import type { ClassLookup, ClassRecordOf, SchemaClasses } from "#law.ts"
import type {
	AnyVar,
	ClassedField,
	Duration,
	JoinOk,
	MaskParam,
	MatchFields,
	MatchOwner,
	MintSlotOf,
	Param,
	ParamsRecord,
	ParamValueAt,
	SetParam,
	ShapeOf
} from "#query/scope.ts"
import { inferred, isTerm } from "#query/scope.ts"
import type { FieldsShape } from "#relation.ts"

/**
 * One atom-binding position as runtime data. A variable rides BY REFERENCE
 * (`ref`) — object identity is the join. `literalSet` is a membership
 * ARRAY at a closed-reference field, folded into the program.
 */
type BindingTermData =
	| { readonly kind: "var"; readonly ref: AnyVar }
	| { readonly kind: "param"; readonly name: string }
	| { readonly kind: "setParam"; readonly name: string }
	| { readonly kind: "literalSet"; readonly name: string; readonly members: readonly string[] }
	| { readonly kind: "literal"; readonly value: unknown }

/** One resolved binding: the field's name, its descriptor, its law-computed class, and the term. */
interface BindingEntry {
	readonly field: string
	readonly data: AnyField
	readonly class: string | undefined
	readonly term: BindingTermData
}

/** One EDB atom as runtime data (either polarity — polarity is the rule item's; a closed owner is a ψ atom). */
interface AtomData {
	readonly relation: MatchOwner
	readonly bindings: readonly BindingEntry[]
}

/** One comparison operator name (mirrors `ir::CmpOp`). */
type CmpKind = "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "pointIn" | "allen"

/** One comparison side as runtime data (variables and the measure ride BY REFERENCE). */
type CmpTermData =
	| { readonly kind: "var"; readonly ref: AnyVar }
	| { readonly kind: "param"; readonly name: string }
	| { readonly kind: "setParam"; readonly name: string }
	| { readonly kind: "measure"; readonly ref: AnyVar }
	| { readonly kind: "literal"; readonly value: unknown }

/** The `allen` mask position as runtime data. */
type MaskData = { readonly kind: "literal"; readonly mask: number } | { readonly kind: "param"; readonly name: string }

/** One comparison condition as runtime data (`mask` present exactly for `allen`). */
interface CmpData {
	readonly kind: "cmp"
	readonly op: CmpKind
	readonly mask: MaskData | undefined
	readonly lhs: CmpTermData
	readonly rhs: CmpTermData
}

/** One condition-tree node as runtime data (`ir::ConditionTree`). */
interface TreeData {
	readonly kind: "tree"
	readonly op: "and" | "or"
	readonly children: readonly CondData[]
}

/** Any condition node as runtime data. */
type CondData = CmpData | TreeData

/** One aggregate's runtime description (find vocabulary, over variable REFERENCES). */
type AggData =
	| { readonly op: "count" }
	| { readonly op: "countDistinct"; readonly over: AnyVar }
	| {
			readonly op: "fold"
			readonly fold: "sum" | "min" | "max"
			readonly over: AnyVar | { readonly duration: AnyVar }
	  }
	| { readonly op: "arg"; readonly direction: "argMax" | "argMin"; readonly over: AnyVar; readonly key: AnyVar }
	| { readonly op: "pack"; readonly over: AnyVar }

/** One classified find entry as runtime data (variables and the measure ride BY REFERENCE). */
type FindEntryData =
	| { readonly kind: "var"; readonly over: AnyVar }
	| { readonly kind: "measure"; readonly over: AnyVar }
	| { readonly kind: "aggregate"; readonly agg: AggData }

/**
 * One answer column: its name (the row object key — the find record's key,
 * so renames are real and a duplicate column is unrepresentable), its entry,
 * the classed mint SLOT its values flow from (a projected var or an
 * Arg-carried payload; `undefined` for counts/folds/measures/pack, which
 * derive numbers or intervals), and — when that slot is a closed reference —
 * the roster the decode lifts row ids back to handle NAMES through
 * (`undefined` on every bare column). The slice is SDK-side marshaling data
 * only: the wire `ProgramIr` never carries it.
 */
interface FindColumn {
	readonly name: string
	readonly entry: FindEntryData
	readonly closed: ClosedRoster | undefined
	readonly slot: ClassedField | undefined
}

/**
 * One body item of a rule, in written order. The idb join is a NAMED record
 * over the rec's head keys (`key`) bound to local variables (`ref`) — the
 * typed wall a record's unordered keys would otherwise lose.
 */
type RuleItem =
	| { readonly kind: "atom"; readonly atom: AtomData }
	| { readonly kind: "negated"; readonly atom: AtomData }
	| {
			readonly kind: "idb"
			readonly rec: RecData
			readonly bindings: ReadonlyArray<{ readonly key: string; readonly ref: AnyVar }>
			/** `true` on a negated finished-stratum atom (`r.not(rec, {...})`): probed through its anti-probe, binds nothing. */
			readonly negated: boolean
	  }
	| { readonly kind: "cond"; readonly cond: CondData }

/**
 * One use of a parameter inside a rule, in written order: the census the
 * query-level registry folds (first use mints the dense `ParamId`, first
 * FIELD-ANCHORED use types the wire). `members` is present exactly on a
 * membership-array use.
 */
interface ParamUse {
	readonly name: string
	readonly shape: "value" | "set" | "mask"
	readonly anchor: AnyField | "measure" | undefined
	readonly op: "binding" | CmpKind
	readonly members: readonly string[] | undefined
}

/** One complete rule as runtime data. */
interface RuleData {
	readonly items: readonly RuleItem[]
	readonly finds: readonly FindColumn[]
	readonly paramUses: readonly ParamUse[]
}

/**
 * One recursive predicate's runtime description — identity keys the dense
 * `PredId` at lowering. `rules` is appended by `rec.rule(...)` and sealed
 * (frozen) when the program's output is declared.
 */
interface RecData {
	readonly name: string
	readonly rules: RuleData[]
}

/**
 * What a binding position of field `F` accepts: a bare structural literal
 * of the field's value type, a variable/param/∈-set-param term — and, when
 * the field is interval-typed, a bare point literal. A CLOSED-reference
 * field additionally takes a plain ARRAY of handle names read as membership.
 */
type BindingInput<F extends AnyField> =
	| Infer<F>
	| (F extends ClosedIdField ? readonly Infer<F>[] : never)
	| (F extends { readonly kind: "interval" } ? bigint : never)
	| AnyVar
	| Param<string>
	| SetParam<string>

/**
 * The `match`/`not` bindings record: per field, a term or literal of that
 * field's structural type; unmentioned fields are wildcards (absence IS
 * the wildcard — the IR has no wildcard variant to spell).
 */
type MatchShape<F extends FieldsShape> = {
	readonly [K in keyof F]?: BindingInput<F[K]>
}

/**
 * One field position of a bindings record as a classed slot: the declared
 * descriptor plus the slot's law-computed class, read off the relation's
 * class record (`CR`). The one shape a binding position's join judgment
 * compares against.
 */
type SlotAt<F extends FieldsShape, CR, K> = {
	readonly field: F[K & keyof F]
	readonly class: ClassLookup<CR, K>
}

/**
 * The per-property join judgment of a bindings record: a VARIABLE binding
 * must join its own MINT slot to the position slot (a cross-class reuse maps
 * the property to `never`). Because {@link JoinOk} is an equality, judging
 * every position against the mint slot makes all cross-binding joins
 * mutually class-equal by transitivity — no env or sibling arm is needed.
 */
type CheckBindings<Classes extends SchemaClasses, F extends FieldsShape, CR, B> = {
	readonly [K in keyof B]: K extends keyof F
		? B[K] extends AnyVar
			? JoinOk<MintSlotOf<Classes, B[K]>, SlotAt<F, CR, K>> extends true
				? B[K]
				: never
			: B[K]
		: never
}

/** The params-object fragments a bindings record contributes (one union member per param use). */
type BindParams<F extends FieldsShape, B> = {
	[K in keyof B & keyof F]: B[K] extends Param<infer P extends string>
		? { readonly [Q in P]: ParamValueAt<F[K]> }
		: B[K] extends SetParam<infer P extends string>
			? { readonly [Q in P]: readonly ParamValueAt<F[K]>[] }
			: never
}[keyof B & keyof F]

/**
 * One comparison VALUE: op plus its operands, raw — the runtime
 * representation carries the operands (variable references included), so
 * `.where`'s judgment and the params inference both read the value itself.
 * `mask` is populated exactly for `allen`.
 */
interface Cmp<Op extends CmpKind, L, R, M = undefined> {
	readonly cond: "cmp"
	readonly op: Op
	readonly lhs: L
	readonly rhs: R
	readonly mask: M
}

/** One condition-tree VALUE (`and`/`or` over comparisons and nested trees). */
interface Tree<Ch extends readonly AnyTreeChild[]> {
	readonly cond: "tree"
	readonly op: "and" | "or"
	readonly children: Ch
}

/**
 * One negated-atom VALUE — negation is a position in the rule (anti-join
 * over sets, no null trick): a binding satisfies it iff NO fact matches.
 * Its variables must be positively bound in the rule — a construction-time
 * wall (BOUNDNESS is invisible to the type tier), before the engine's own
 * refusal.
 */
interface NotAtom<R extends MatchOwner, B> {
	readonly cond: "not"
	readonly relation: R
	readonly bindings: B
}

/**
 * A recursive predicate as a NEGATION target — the structural half of the
 * rec reference (`data` carries rules, never fields, so an EDB or closed
 * owner can never match it and the `not()` overloads stay disjoint).
 */
interface RecTarget {
	readonly name: string
	readonly data: RecData
	readonly [inferred]?: { readonly params: ParamsRecord; readonly head: HeadShapeOf }
}

/** The head signature a threaded rec target carries (`undefined` before its first rule seals one). */
type HeadShapeOf = Readonly<Record<string, ClassedField>> | undefined

/**
 * One negated FINISHED-STRATUM atom — negation OF a lower stratum is
 * engine-legal (the strata judge refuses only negation *through* a cycle:
 * a finished set is what keeps the operator monotone), and this value is
 * its one spelling: `r.not(reach, { c })` in an output rule rejects every
 * binding the finished stratum extends, through the engine's anti-probe.
 * Binds nothing, only rejects — every variable it names must be positively
 * bound in the rule (the same safety rule as EDB negation).
 */
interface NotIdbAtom<Target extends RecTarget, B> {
	readonly cond: "notIdb"
	readonly target: Target
	readonly bindings: B
}

/** Any comparison value. */
type AnyCmp = Cmp<CmpKind, unknown, unknown, unknown>

/** Any condition-tree child (trees hold comparisons and trees — never atoms). */
type AnyTreeChild = AnyCmp | Tree<readonly AnyTreeChild[]>

/** Any negated-atom value. */
type AnyNotAtom = NotAtom<MatchOwner, unknown>

/** Any negated finished-stratum value. */
type AnyNotIdbAtom = NotIdbAtom<RecTarget, unknown>

/** Any `.where` input: a comparison, a condition tree, or a negated atom (EDB, closed, or finished stratum). */
type AnyCond = AnyCmp | Tree<readonly AnyTreeChild[]> | AnyNotAtom | AnyNotIdbAtom

/** What `eq`'s right side accepts (`ParamSet` is `Eq`-only — the IR's rule). */
type EqRight = AnyVar | Param<string> | SetParam<string> | bigint | string | boolean | Uint8Array | IntervalValue

/** What `ne`'s right side accepts. */
type NeRight = AnyVar | Param<string> | bigint | string | boolean | Uint8Array | IntervalValue

/** One side of an order comparison: orderable terms only (the IR's comparison rules). */
type OrderSide = AnyVar | Param<string> | Duration | bigint

/** The point side of `pointIn`. */
type PointSide = AnyVar | Param<string> | bigint

/** The interval side of `pointIn`/`allen`. */
type IntervalSide = AnyVar | Param<string> | IntervalValue

/** Builds one comparison value. */
function comparison<Op extends CmpKind, L, R, M>(op: Op, lhs: L, rhs: R, mask: M): Cmp<Op, L, R, M> {
	return Object.freeze({ cond: "cmp", op, lhs, rhs, mask })
}

/**
 * Rejects a comparison with no term side: it is constant-valued, the
 * engine's own validation refuses it, and the lowering has no anchored
 * position to type the literals by — fail here with the same verdict.
 */
function assertTermSide(op: string, lhs: unknown, rhs: unknown): void {
	if (!isTerm(lhs) && !isTerm(rhs)) {
		throw errors.new(
			`${op}: a comparison without a variable or parameter side is constant-valued — write the query you mean`
		)
	}
}

/**
 * The equality comparison (`ir::CmpOp::Eq`) — a bound variable against a
 * variable (var-to-var unification, class-equal by the join judgment), a
 * param (typed by the variable), an ∈-set param (`Eq`-only), or a bare
 * literal of the variable's own value type. Prefer direct placement in
 * `match` where punning applies.
 */
function eq<L extends AnyVar, const R extends EqRight>(left: L, right: R): Cmp<"eq", L, R> {
	return comparison("eq", left, right, undefined)
}

/** Typed disequality (`ir::CmpOp::Ne`). "Not in set" has no operator — write a negated atom. */
function ne<L extends AnyVar, const R extends NeRight>(left: L, right: R): Cmp<"ne", L, R> {
	return comparison("ne", left, right, undefined)
}

/** The shared order-comparison constructor. */
function order<Op extends "lt" | "le" | "gt" | "ge", const L extends OrderSide, const R extends OrderSide>(
	op: Op,
	left: L,
	right: R
): Cmp<Op, L, R> {
	assertTermSide(op, left, right)
	return comparison(op, left, right, undefined)
}

/** Strict less-than (`ir::CmpOp::Lt`) — orderable sides only, never intervals/bytes/strings/bools. */
function lt<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"lt", L, R> {
	return order("lt", left, right)
}

/** Less-or-equal (`ir::CmpOp::Le`). */
function le<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"le", L, R> {
	return order("le", left, right)
}

/** Strict greater-than (`ir::CmpOp::Gt`). */
function gt<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"gt", L, R> {
	return order("gt", left, right)
}

/** Greater-or-equal (`ir::CmpOp::Ge`). */
function ge<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"ge", L, R> {
	return order("ge", left, right)
}

/**
 * Point membership as a predicate (`ir::CmpOp::PointIn`) — THE one
 * spelling: `pointIn(t, w)` holds iff `w.start ≤ t < w.end`. The IR
 * orders the operands interval-left, point-right; the value stores them
 * that way whatever the surface argument order. Interval ⊇ interval is NOT
 * this operator; that predicate is `allen(a, ALLEN.covers, b)`.
 */
function pointIn<const P extends PointSide, const I extends IntervalSide>(point: P, interval: I): Cmp<"pointIn", I, P> {
	assertTermSide("pointIn", point, interval)
	return comparison("pointIn", interval, point, undefined)
}

/**
 * The 13-bit mask range: bits above the low 13 are unrepresentable in the
 * engine's `AllenMask` (`bumbledb/crates/bumbledb/src/allen.rs`:
 * `AllenMask::new` refuses them).
 */
const ALLEN_ALL_BITS = (1 << 13) - 1

/**
 * The Allen coordinate system's named constants — the 13 basics in the
 * engine's palindromic bit order plus the workload composites, values
 * identical to the engine's. Compose with `|`: `ALLEN.before | ALLEN.meets`.
 */
const ALLEN = Object.freeze({
	before: 1 << 0,
	meets: 1 << 1,
	overlaps: 1 << 2,
	starts: 1 << 3,
	during: 1 << 4,
	finishes: 1 << 5,
	equals: 1 << 6,
	finishedBy: 1 << 7,
	contains: 1 << 8,
	startedBy: 1 << 9,
	overlappedBy: 1 << 10,
	metBy: 1 << 11,
	after: 1 << 12,
	/** The point-sets share a point (9 bits; under half-open intervals *meets* shares none). */
	intersects: (1 << 2) | (1 << 3) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 9) | (1 << 10),
	/** Point-set ⊇: equals ∪ contains ∪ started-by ∪ finished-by. */
	covers: (1 << 6) | (1 << 8) | (1 << 9) | (1 << 7),
	/** Point-set ⊆ — `covers`' converse: equals ∪ during ∪ starts ∪ finishes. */
	coveredBy: (1 << 6) | (1 << 4) | (1 << 3) | (1 << 5),
	/** The point-sets share no point: before ∪ meets ∪ met-by ∪ after. */
	disjoint: (1 << 0) | (1 << 1) | (1 << 11) | (1 << 12)
})

/**
 * THE interval-pair comparison (`ir::CmpOp::Allen`): two interval terms of
 * one element type, satisfied iff the pair's classification is in the
 * 13-bit mask — a literal built from the `ALLEN` constants, or a mask
 * parameter (`r.maskParam`).
 */
function allen<const A extends IntervalSide, const M extends number | MaskParam<string>, const B extends IntervalSide>(
	left: A,
	mask: M,
	right: B
): Cmp<"allen", A, B, M> {
	assertTermSide("allen", left, right)
	if (typeof mask === "number" && (!Number.isInteger(mask) || mask < 0 || mask > ALLEN_ALL_BITS)) {
		throw errors.new(
			`allen mask ${mask} is not a 13-bit mask — build masks from the ALLEN constants (bumbledb allen.rs: bits above the low 13 are unrepresentable)`
		)
	}
	return comparison("allen", left, right, mask)
}

/**
 * Conjunction node of the input condition grammar (`ConditionTree::And`).
 * The rule's condition list is already a conjunction — `and` exists for
 * nesting under `or`, and the empty combination keeps the IR's algebraic
 * reading (`And([])` is true).
 */
function and<const C extends readonly AnyTreeChild[]>(...children: C): Tree<C> {
	return Object.freeze({ cond: "tree", op: "and", children: Object.freeze(children) })
}

/**
 * Disjunction node of the input condition grammar (`ConditionTree::Or`) —
 * the one place the surface admits a nested OR; validation distributes it
 * to DNF rules engine-side. `Or([])` keeps its algebraic reading (false).
 */
function or<const C extends readonly AnyTreeChild[]>(...children: C): Tree<C> {
	return Object.freeze({ cond: "tree", op: "or", children: Object.freeze(children) })
}

/**
 * Negation — anti-join over sets: `not(Rel, { field: someVar, ... })`
 * rejects every binding some matching fact extends. A negated atom binds
 * nothing, only rejects: every variable it names must be positively bound
 * in the rule, a construction-time wall (the engine's safety refusal stands
 * behind it). A CLOSED owner is legal here too — and so is a FINISHED
 * STRATUM: `not(reach, { c })` in an output rule negates the rec's
 * finished set (a named record over its head keys, variables only — the
 * same wall `idb()` holds), the one spelling of the engine-legal
 * complement query (`(n) | Node(id: n), !reach(n);` on the Rust surface).
 */
function not<R extends MatchOwner, const B extends MatchShape<MatchFields<R>>>(relation: R, bindings: B): NotAtom<R, B>
function not<Target extends RecTarget, const B extends Readonly<Record<string, AnyVar>>>(
	target: Target,
	bindings: B
): NotIdbAtom<Target, B>
function not(
	relation: MatchOwner | RecTarget,
	bindings: Readonly<Record<string, unknown>>
): NotAtom<MatchOwner, unknown> | NotIdbAtom<RecTarget, unknown> {
	if (isRecTarget(relation)) {
		return Object.freeze({ cond: "notIdb" as const, target: relation, bindings })
	}
	return Object.freeze({ cond: "not" as const, relation, bindings })
}

/**
 * THE negation-target discriminant: a rec's runtime data carries its rules,
 * a relation's its fields (and a closed relation's its handle roster) — the
 * shapes are disjoint by construction, so the dispatch is total.
 */
function isRecTarget(value: MatchOwner | RecTarget): value is RecTarget {
	return "rules" in value.data
}

/**
 * Whether a variable's OWN field is orderable (u64/i64). A CLOSED reference
 * is excluded even though its kind is `u64`: a vocabulary's declaration-id
 * order is an accident, not semantics (`docs/architecture/10-data-model.md`
 * § orderability), so every order-comparison and fold position refuses
 * closed-bound terms — this judgment is the one gate they all read, and the
 * construction-time validations in `#query/lower.ts` are its runtime twin.
 */
type OrderVarOk<V extends AnyVar> = V["field"] extends { readonly closed: ClosedRoster }
	? false
	: V["field"]["kind"] extends "u64" | "i64"
		? true
		: false

/** Whether a variable's OWN field is interval-typed. */
type IntervalVarOk<V extends AnyVar> = V["field"]["kind"] extends "interval" ? true : false

/** One order-comparison side's judgment (off the term's own field). */
type OrderSideOk<T> = T extends AnyVar
	? OrderVarOk<T>
	: T extends Duration<infer V extends AnyVar>
		? IntervalVarOk<V>
		: true

/** One point side's judgment. */
type PointSideOk<T> = T extends AnyVar ? OrderVarOk<T> : true

/** One interval side's judgment. */
type IntervalSideOk<T> = T extends AnyVar ? IntervalVarOk<T> : true

/** The `eq`/`ne` judgment: var-var joins by mint slot; var-literal is exact-typed by the var's own field. */
type EqOk<Classes extends SchemaClasses, L, R> = L extends AnyVar
	? R extends AnyVar
		? JoinOk<MintSlotOf<Classes, L>, MintSlotOf<Classes, R>> extends true
			? true
			: false
		: R extends Param<string> | SetParam<string>
			? true
			: [R] extends [Infer<L["field"]>]
				? true
				: false
	: false

/** One negated-atom binding's judgment: a variable must be class-equal (boundness is a runtime wall). */
type NotBindingOk<Classes extends SchemaClasses, S extends ClassedField, T> = T extends AnyVar
	? JoinOk<MintSlotOf<Classes, T>, S> extends true
		? true
		: false
	: true

/** The whole negated atom's judgment (`CR` — the negated relation's class record off the schema class map). */
type NotOk<Classes extends SchemaClasses, F extends FieldsShape, CR, B> = false extends {
	[K in keyof B]: NotBindingOk<Classes, SlotAt<F, CR, K>, B[K]>
}[keyof B]
	? false
	: true

/** Reads a rec target's sealed head signature off its inference slot (`undefined` on an unthreaded handle). */
type RecHeadOf<T> = T extends { readonly [inferred]?: infer S }
	? Exclude<S, undefined> extends { readonly head: infer H }
		? H
		: undefined
	: undefined

/** Reads a rec target's params record off its inference slot. */
type RecParamsOf<T> = T extends { readonly [inferred]?: infer S }
	? Exclude<S, undefined> extends { readonly params: infer P }
		? P
		: never
	: never

/** One negated finished-stratum position's judgment: a variable, class-equal to its head slot when the head is carried. */
type NotIdbBindingOk<Classes extends SchemaClasses, HeadSlot, V> = V extends AnyVar
	? HeadSlot extends ClassedField
		? JoinOk<HeadSlot, MintSlotOf<Classes, V>> extends true
			? true
			: false
		: true
	: false

/**
 * The whole negated finished-stratum atom's judgment — the negation twin of
 * the `idb()` chain's `CheckIdbBindings`: when the target carries its head
 * (a threaded rec handle), the bindings record's key set must EXACTLY equal
 * the head's and each variable must be class-equal to its head slot; an
 * unthreaded handle still takes variables only.
 */
type NotIdbOk<Classes extends SchemaClasses, Head, B> =
	Head extends Readonly<Record<string, ClassedField>>
		? [keyof B] extends [keyof Head]
			? [keyof Head] extends [keyof B]
				? false extends { [K in keyof B]: NotIdbBindingOk<Classes, Head[K & keyof Head], B[K]> }[keyof B]
					? false
					: true
				: false
			: false
		: false extends { [K in keyof B]: B[K] extends AnyVar ? true : false }[keyof B]
			? false
			: true

/**
 * One condition's judgment — the type-level twin of the engine's comparison
 * roster: class-equal joins (off the mint slots), orderable order sides (an
 * interval var under a non-`pointIn` op is exactly here refused),
 * kind-correct `pointIn`/`allen` sides, and negated-atom class safety. The
 * leading `[AnyTreeChild] extends [C]` arm is the recursion's base case.
 */
type CondOkBool<Classes extends SchemaClasses, C> = [AnyTreeChild] extends [C]
	? true
	: C extends Cmp<infer Op, infer L, infer R, unknown>
		? Op extends "eq" | "ne"
			? EqOk<Classes, L, R>
			: Op extends "lt" | "le" | "gt" | "ge"
				? [OrderSideOk<L>, OrderSideOk<R>] extends [true, true]
					? true
					: false
				: Op extends "pointIn"
					? [IntervalSideOk<L>, PointSideOk<R>] extends [true, true]
						? true
						: false
					: Op extends "allen"
						? [IntervalSideOk<L>, IntervalSideOk<R>] extends [true, true]
							? true
							: false
						: false
		: C extends Tree<infer Ch extends readonly AnyTreeChild[]>
			? false extends CondOkBool<Classes, Ch[number]>
				? false
				: true
			: C extends NotIdbAtom<infer T extends RecTarget, infer B>
				? NotIdbOk<Classes, RecHeadOf<T>, B>
				: C extends NotAtom<infer R extends MatchOwner, infer B>
					? NotOk<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
					: false

/** The validated `.where` argument (intersect with the inferred condition type). */
type CheckCond<Classes extends SchemaClasses, C> = CondOkBool<Classes, C> extends true ? C : never

/** The `eq`/`ne` params contribution: the param typed by the left variable's own field. */
type EqParams<L, R> = L extends AnyVar
	? R extends Param<infer P extends string>
		? { readonly [Q in P]: Infer<L["field"]> }
		: R extends SetParam<infer P extends string>
			? { readonly [Q in P]: readonly Infer<L["field"]>[] }
			: never
	: never

/** An order side's params contribution (order params are always `bigint`). */
type OrderSideParams<T> = T extends Param<infer P extends string> ? { readonly [Q in P]: bigint } : never

/** An interval side's params contribution. */
type IntervalSideParams<T> = T extends Param<infer P extends string> ? { readonly [Q in P]: IntervalValue } : never

/** The mask position's params contribution. */
type MaskParams<M> = M extends MaskParam<infer P extends string> ? { readonly [Q in P]: number } : never

/**
 * One condition's params-object fragments (a union; the rule builder folds
 * them into the inferred `Params` record) — every param typed by its use.
 * The leading arm is the same base case as {@link CondOkBool}'s.
 */
type CondParams<C> = [AnyTreeChild] extends [C]
	? never
	: C extends Cmp<infer Op, infer L, infer R, infer M>
		? Op extends "eq" | "ne"
			? EqParams<L, R>
			: Op extends "lt" | "le" | "gt" | "ge"
				? OrderSideParams<L> | OrderSideParams<R>
				: Op extends "pointIn"
					? IntervalSideParams<L> | OrderSideParams<R>
					: Op extends "allen"
						? IntervalSideParams<L> | IntervalSideParams<R> | MaskParams<M>
						: never
		: C extends Tree<infer Ch extends readonly AnyTreeChild[]>
			? CondParams<Ch[number]>
			: C extends NotIdbAtom<infer T extends RecTarget, unknown>
				? RecParamsOf<T>
				: C extends NotAtom<infer R extends MatchOwner, infer B>
					? BindParams<MatchFields<R>, B>
					: never

/** The flattened params record one bindings record contributes. */
type BindParamsShape<F extends FieldsShape, B> = ShapeOf<BindParams<F, B>>

/** The flattened params record one condition contributes. */
type CondParamsShape<C> = ShapeOf<CondParams<C>>

export type {
	AggData,
	AnyCmp,
	AnyCond,
	AnyNotAtom,
	AnyTreeChild,
	AtomData,
	BindingEntry,
	BindingInput,
	BindingTermData,
	BindParams,
	BindParamsShape,
	CheckBindings,
	CheckCond,
	Cmp,
	CmpData,
	CmpKind,
	CmpTermData,
	CondData,
	CondOkBool,
	CondParams,
	CondParamsShape,
	FindColumn,
	FindEntryData,
	IntervalSide,
	IntervalVarOk,
	MaskData,
	MatchFields,
	MatchOwner,
	MatchShape,
	NotAtom,
	NotIdbAtom,
	OrderSide,
	OrderVarOk,
	ParamUse,
	PointSide,
	RecData,
	RecTarget,
	RuleData,
	RuleItem,
	SlotAt,
	Tree,
	TreeData
}
export { ALLEN, ALLEN_ALL_BITS, allen, and, comparison, eq, ge, gt, le, lt, ne, not, or, pointIn }
