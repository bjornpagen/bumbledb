import * as errors from "@superbuilders/errors"
import type { AnyField, ClosedIdField, ClosedRoster, Infer, IntervalValue } from "#fields.ts"
import type { ClassLookup, ClassRecordOf, SchemaClasses } from "#law.ts"
import type {
	AnyVar,
	ClassedField,
	Duration,
	JoinOk,
	MatchFields,
	MatchOwner,
	MintSlotOf,
	Param,
	ParamValueAt,
	SetParam,
	ShapeOf
} from "#query/scope.ts"
import { isTerm, term } from "#query/scope.ts"
import type { FieldsShape } from "#relation.ts"

type BindingTermData =
	| { readonly kind: "var"; readonly ref: AnyVar }
	| { readonly kind: "param"; readonly name: string }
	| { readonly kind: "setParam"; readonly name: string }
	| { readonly kind: "literalSet"; readonly name: string; readonly members: readonly string[] }
	| { readonly kind: "literal"; readonly value: unknown }

interface BindingEntry {
	readonly field: string
	readonly data: AnyField
	readonly class: string | undefined
	readonly term: BindingTermData
}

interface AtomData {
	readonly relation: MatchOwner
	readonly bindings: readonly BindingEntry[]
}

type CmpKind = "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "pointIn" | "allen"

type CmpTermData =
	| { readonly kind: "var"; readonly ref: AnyVar }
	| { readonly kind: "param"; readonly name: string }
	| { readonly kind: "setParam"; readonly name: string }
	| { readonly kind: "measure"; readonly ref: AnyVar }
	| { readonly kind: "literal"; readonly value: unknown }

type CmpData = {
	readonly kind: "cmp"
	readonly lhs: CmpTermData
	readonly rhs: CmpTermData
} & (
	| { readonly op: { readonly kind: "allen"; readonly mask: number } }
	| { readonly op: { readonly kind: Exclude<CmpKind, "allen"> } }
)

interface TreeData {
	readonly kind: "tree"
	readonly op: "and" | "or"
	readonly children: readonly CondData[]
}

type CondData = CmpData | TreeData

type AggData =
	| { readonly op: "count" }
	| {
			readonly op: "fold"
			readonly fold: "sum" | "min" | "max"
			readonly over: AnyVar | { readonly duration: AnyVar }
	  }
	| { readonly op: "pack"; readonly over: AnyVar }

type FindEntryData =
	| { readonly kind: "var"; readonly over: AnyVar }
	| { readonly kind: "measure"; readonly over: AnyVar }
	| { readonly kind: "aggregate"; readonly agg: AggData }

interface FindColumn {
	readonly name: string
	readonly entry: FindEntryData
	readonly closed: ClosedRoster | undefined
	readonly slot: ClassedField | undefined
}

type RuleItem =
	| { readonly kind: "atom"; readonly atom: AtomData }
	| { readonly kind: "negated"; readonly atom: AtomData }
	| {
			readonly kind: "interior"
			readonly target: DerivedTable
			readonly bindings: ReadonlyArray<{ readonly key: string; readonly ref: AnyVar }>
	  }
	| {
			readonly kind: "negatedInterior"
			readonly target: DerivedTable
			readonly bindings: ReadonlyArray<{ readonly key: string; readonly ref: AnyVar }>
	  }
	| { readonly kind: "cond"; readonly cond: CondData }

interface ParamUse {
	readonly name: string
	readonly shape: "value" | "set"
	readonly anchor: AnyField | "measure" | undefined
	readonly op: "binding" | CmpKind
	readonly members: readonly string[] | undefined
}

interface RuleData {
	readonly items: readonly RuleItem[]
	readonly finds: readonly FindColumn[]
	readonly paramUses: readonly ParamUse[]
}

interface InteriorData {
	readonly name: string
	readonly finds: readonly FindColumn[]
	readonly rules: readonly RuleData[]
}

type NonEmpty<T> = readonly [T, ...T[]]

/**
 * Name-only rec identity used while base arms are in flight. Rec-base
 * arms cannot read the rec; they must not observe a head.
 */
interface RecHandle {
	readonly name: string
}

interface RecHead {
	readonly name: string
	readonly finds: NonEmpty<FindColumn>
}

interface RecData extends RecHead {
	readonly base: NonEmpty<RuleData>
	readonly rec: NonEmpty<RuleData>
}

type DerivedTable = InteriorData | RecHead

type BindingInput<F extends AnyField> =
	| Infer<F>
	| (F extends ClosedIdField ? readonly Infer<F>[] : never)
	| (F extends { readonly kind: "interval" } ? bigint : never)
	| AnyVar
	| Param<string>
	| SetParam<string>

type MatchShape<F extends FieldsShape> = {
	readonly [K in keyof F]?: BindingInput<F[K]>
}

type SlotAt<F extends FieldsShape, CR, K> = {
	readonly field: F[K & keyof F]
	readonly class: ClassLookup<CR, K>
}

type CheckBindings<Classes extends SchemaClasses, F extends FieldsShape, CR, B> = {
	readonly [K in keyof B]: K extends keyof F
		? B[K] extends AnyVar
			? JoinOk<MintSlotOf<Classes, B[K]>, SlotAt<F, CR, K>> extends true
				? B[K]
				: never
			: B[K]
		: never
}

type BindParams<F extends FieldsShape, B> = {
	[K in keyof B & keyof F]: B[K] extends Param<infer P extends string>
		? { readonly [Q in P]: ParamValueAt<F[K]> }
		: B[K] extends SetParam<infer P extends string>
			? { readonly [Q in P]: readonly ParamValueAt<F[K]>[] }
			: never
}[keyof B & keyof F]

interface Cmp<Op extends CmpKind, L, R, M = undefined> {
	readonly cond: "cmp"
	readonly op: Op
	readonly lhs: L
	readonly rhs: R
	readonly mask: M
}

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

interface NotInteriorAtom<B> {
	readonly cond: "notInterior"
	readonly name: string
	readonly bindings: B
}

type AnyCmp = Cmp<CmpKind, unknown, unknown, unknown>

type AnyTreeChild = AnyCmp | Tree<readonly AnyTreeChild[]>

type AnyNotAtom = NotAtom<MatchOwner, unknown>

type AnyNotInteriorAtom = NotInteriorAtom<unknown>

type AnyCond = AnyCmp | Tree<readonly AnyTreeChild[]> | AnyNotAtom | AnyNotInteriorAtom

type EqRight = AnyVar | Param<string> | SetParam<string> | bigint | string | boolean | Uint8Array | IntervalValue

type NeRight = AnyVar | Param<string> | bigint | string | boolean | Uint8Array | IntervalValue

type OrderSide = AnyVar | Param<string> | Duration | bigint | boolean

type PointSide = AnyVar | Param<string> | bigint

type IntervalSide = AnyVar | Param<string> | IntervalValue

function comparison<Op extends CmpKind, L, R, M>(op: Op, lhs: L, rhs: R, mask: M): Cmp<Op, L, R, M> {
	return Object.freeze({ cond: "cmp", op, lhs, rhs, mask })
}

function isVariableSide(value: unknown): boolean {
	return isTerm(value) && (value[term] === "var" || value[term] === "duration")
}

function assertTermSide(op: string, lhs: unknown, rhs: unknown): void {
	if (!isVariableSide(lhs) && !isVariableSide(rhs)) {
		throw errors.new(
			`${op}: a comparison without a variable side is constant-valued (a parameter is a constant at execution) — write the query you mean`
		)
	}
}

function eq<L extends AnyVar, const R extends EqRight>(left: L, right: R): Cmp<"eq", L, R> {
	return comparison("eq", left, right, undefined)
}

function ne<L extends AnyVar, const R extends NeRight>(left: L, right: R): Cmp<"ne", L, R> {
	return comparison("ne", left, right, undefined)
}

function order<Op extends "lt" | "le" | "gt" | "ge", const L extends OrderSide, const R extends OrderSide>(
	op: Op,
	left: L,
	right: R
): Cmp<Op, L, R> {
	assertTermSide(op, left, right)
	return comparison(op, left, right, undefined)
}

function lt<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"lt", L, R> {
	return order("lt", left, right)
}

function le<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"le", L, R> {
	return order("le", left, right)
}

function gt<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"gt", L, R> {
	return order("gt", left, right)
}

function ge<const L extends OrderSide, const R extends OrderSide>(left: L, right: R): Cmp<"ge", L, R> {
	return order("ge", left, right)
}

function pointIn<const P extends PointSide, const I extends IntervalSide>(point: P, interval: I): Cmp<"pointIn", I, P> {
	assertTermSide("pointIn", point, interval)
	return comparison("pointIn", interval, point, undefined)
}

const ALLEN_ALL_BITS = (1 << 13) - 1

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

	intersects: (1 << 2) | (1 << 3) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 9) | (1 << 10),

	covers: (1 << 6) | (1 << 8) | (1 << 9) | (1 << 7),

	coveredBy: (1 << 6) | (1 << 4) | (1 << 3) | (1 << 5),

	disjoint: (1 << 0) | (1 << 1) | (1 << 11) | (1 << 12)
})

function allen<const A extends IntervalSide, const B extends IntervalSide>(
	left: A,
	mask: number,
	right: B
): Cmp<"allen", A, B, number> {
	assertTermSide("allen", left, right)
	if (!Number.isInteger(mask) || mask < 0 || mask > ALLEN_ALL_BITS) {
		throw errors.new(
			`allen mask ${mask} is not a 13-bit mask — build masks from the ALLEN constants (bumbledb allen.rs: bits above the low 13 are unrepresentable)`
		)
	}
	return comparison("allen", left, right, mask)
}

function and<const C extends readonly AnyTreeChild[]>(...children: C): Tree<C> {
	return Object.freeze({ cond: "tree", op: "and", children: Object.freeze(children) })
}

function or<const C extends readonly AnyTreeChild[]>(...children: C): Tree<C> {
	return Object.freeze({ cond: "tree", op: "or", children: Object.freeze(children) })
}

/**
 * Negation — anti-join over sets: `not(Rel, { field: someVar,... })`
 * rejects every binding some matching fact extends. A negated atom binds
 * nothing, only rejects: every variable it names must be positively bound
 * in the rule, a construction-time wall (the engine's safety refusal stands
 * behind it). A CLOSED owner is legal here too — and so is a FINISHED
 * TABLE: `not("reach", { c })` in a main rule negates the rec's finished
 * set (a named record over its head keys, variables only), the one
 * spelling of the engine-legal complement query.
 */
function not<R extends MatchOwner, const B extends MatchShape<MatchFields<R>>>(relation: R, bindings: B): NotAtom<R, B>
function not<const Name extends string, const B extends Readonly<Record<string, AnyVar>>>(
	name: Name,
	bindings: B
): NotInteriorAtom<B>
function not(
	relation: MatchOwner | string,
	bindings: Readonly<Record<string, unknown>>
): NotAtom<MatchOwner, unknown> | NotInteriorAtom<unknown> {
	if (typeof relation === "string") {
		return Object.freeze({ cond: "notInterior" as const, name: relation, bindings })
	}
	return Object.freeze({ cond: "not" as const, relation, bindings })
}

/**
 * Whether a variable's OWN field is NUMERIC (u64/i64) — the judgment the
 * point side of `pointIn` and the `sum` input read: a point lives in the
 * interval's element domain, and a quantifier is not an addition, so bool
 * (orderable, never numeric) is exactly here refused. A CLOSED reference
 * is excluded even though its kind is `u64`: a vocabulary's declaration-id
 * order is an accident, not semantics 
 * § orderability), so every order-comparison and fold position refuses
 * closed-bound terms — the construction-time validations in
 * `#query/lower.ts` are that ban's runtime twin.
 */
type NumericVarOk<V extends AnyVar> = V["field"] extends { readonly closed: ClosedRoster }
	? false
	: V["field"]["kind"] extends "u64" | "i64"
		? true
		: false

type OrderVarOk<V extends AnyVar> = V["field"]["kind"] extends "bool" ? true : NumericVarOk<V>

type IntervalVarOk<V extends AnyVar> = V["field"]["kind"] extends "interval" ? true : false

type CmpVarSideOk<L, R> = L extends AnyVar | Duration ? true : R extends AnyVar | Duration ? true : false

type OrderDomain<T> = T extends AnyVar
	? OrderVarOk<T> extends true
		? T["field"]["kind"]
		: never
	: T extends Duration<infer V extends AnyVar>
		? IntervalVarOk<V> extends true
			? "duration"
			: never
		: T extends Param<string>
			? "open"
			: T extends bigint
				? "integer"
				: T extends boolean
					? "bool"
					: never

type OrderDomainsOk<A, B> = [A] extends [never]
	? false
	: [B] extends [never]
		? false
		: A extends "open"
			? true
			: B extends "open"
				? true
				: A extends "duration"
					? B extends "u64" | "integer"
						? true
						: false
					: B extends "duration"
						? A extends "u64" | "integer"
							? true
							: false
						: A extends "bool"
							? B extends "bool"
								? true
								: false
							: B extends "bool"
								? false
								: A extends "integer"
									? true
									: B extends "integer"
										? true
										: A extends B
											? true
											: false

type OrderPairOk<L, R> = CmpVarSideOk<L, R> extends true ? OrderDomainsOk<OrderDomain<L>, OrderDomain<R>> : false

type IntervalElementDomain<T> = T extends AnyVar
	? T["field"] extends { readonly kind: "interval"; readonly element: infer E extends "u64" | "i64" }
		? E
		: never
	: "open"

type PointDomain<T> = T extends AnyVar ? (NumericVarOk<T> extends true ? T["field"]["kind"] : never) : "open"

type ElementMeets<A, B> = [A] extends [never]
	? false
	: [B] extends [never]
		? false
		: A extends "open"
			? true
			: B extends "open"
				? true
				: A extends B
					? true
					: false

type PointInPairOk<L, R> =
	CmpVarSideOk<L, R> extends true ? ElementMeets<IntervalElementDomain<L>, PointDomain<R>> : false

type AllenPairOk<L, R> =
	CmpVarSideOk<L, R> extends true ? ElementMeets<IntervalElementDomain<L>, IntervalElementDomain<R>> : false

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

type NotBindingOk<Classes extends SchemaClasses, S extends ClassedField, T> = T extends AnyVar
	? JoinOk<MintSlotOf<Classes, T>, S> extends true
		? true
		: false
	: true

type NotOk<Classes extends SchemaClasses, F extends FieldsShape, CR, B> = false extends {
	[K in keyof B]: NotBindingOk<Classes, SlotAt<F, CR, K>, B[K]>
}[keyof B]
	? false
	: true

type NotInteriorBindingOk<V> = V extends AnyVar ? true : false

type NotInteriorOk<B> = false extends { [K in keyof B]: NotInteriorBindingOk<B[K]> }[keyof B] ? false : true

/**
 * One condition's judgment — the type-level twin of the engine's comparison
 * roster: class-equal joins (off the mint slots), PAIRWISE-judged order
 * domains (an interval var under a non-`pointIn` op is refused per side;
 * bool-vs-numeric, u64-vs-i64, and measure-vs-non-u64 are refused as
 * pairs — the engine's same-type rule), element-matched `pointIn`/`allen`
 * pairs, the no-variable-side (constant comparison) rule, and
 * negated-atom class safety. The leading `[AnyTreeChild] extends [C]` arm
 * is the recursion's base case.
 */
type CondOkBool<Classes extends SchemaClasses, C> = [AnyTreeChild] extends [C]
	? true
	: C extends Cmp<infer Op, infer L, infer R, unknown>
		? Op extends "eq" | "ne"
			? EqOk<Classes, L, R>
			: Op extends "lt" | "le" | "gt" | "ge"
				? OrderPairOk<L, R>
				: Op extends "pointIn"
					? PointInPairOk<L, R>
					: Op extends "allen"
						? AllenPairOk<L, R>
						: false
		: C extends Tree<infer Ch extends readonly AnyTreeChild[]>
			? false extends CondOkBool<Classes, Ch[number]>
				? false
				: true
			: C extends NotInteriorAtom<infer B>
				? NotInteriorOk<B>
				: C extends NotAtom<infer R extends MatchOwner, infer B>
					? NotOk<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
					: false

type CheckCond<Classes extends SchemaClasses, C> = CondOkBool<Classes, C> extends true ? C : never

type EqParams<L, R> = L extends AnyVar
	? R extends Param<infer P extends string>
		? { readonly [Q in P]: Infer<L["field"]> }
		: R extends SetParam<infer P extends string>
			? { readonly [Q in P]: readonly Infer<L["field"]>[] }
			: never
	: never

type OrderSideParams<T, Sib> =
	T extends Param<infer P extends string>
		? { readonly [Q in P]: Sib extends AnyVar ? Infer<Sib["field"]> : bigint }
		: never

type PointParams<T> = T extends Param<infer P extends string> ? { readonly [Q in P]: bigint } : never

type IntervalSideParams<T> = T extends Param<infer P extends string> ? { readonly [Q in P]: IntervalValue } : never

type CondParams<C> = [AnyTreeChild] extends [C]
	? never
	: C extends Cmp<infer Op, infer L, infer R, infer _M>
		? Op extends "eq" | "ne"
			? EqParams<L, R>
			: Op extends "lt" | "le" | "gt" | "ge"
				? OrderSideParams<L, R> | OrderSideParams<R, L>
				: Op extends "pointIn"
					? IntervalSideParams<L> | PointParams<R>
					: Op extends "allen"
						? IntervalSideParams<L> | IntervalSideParams<R>
						: never
		: C extends Tree<infer Ch extends readonly AnyTreeChild[]>
			? CondParams<Ch[number]>
			: C extends NotInteriorAtom<unknown>
				? never
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
	DerivedTable,
	FindColumn,
	FindEntryData,
	InteriorData,
	IntervalSide,
	IntervalVarOk,
	MatchFields,
	MatchOwner,
	MatchShape,
	NotAtom,
	NotInteriorAtom,
	NumericVarOk,
	OrderSide,
	OrderVarOk,
	ParamUse,
	PointSide,
	RecData,
	RecHandle,
	RecHead,
	RuleData,
	RuleItem,
	SlotAt,
	Tree,
	TreeData
}
export { ALLEN, ALLEN_ALL_BITS, allen, and, comparison, eq, ge, gt, le, lt, ne, not, or, pointIn }
