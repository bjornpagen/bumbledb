/**
 * Query atoms and conditions (PRD-08) — the body vocabulary of a rule,
 * mirroring the engine IR variant for variant
 * (`bumbledb/crates/bumbledb/src/ir.rs`, the bijection target;
 * `docs/architecture/20-query-ir.md` normative): `match` is the named-field
 * atom (unmentioned fields ARE the wildcard — no wildcard value exists),
 * `not` is negation-as-position (anti-join), `is`/`ne`/`lt`/`le`/`gt`/`ge`
 * are the comparison roster, `covers` is the IR's `PointIn` predicate,
 * `allen` the 13-bit-mask interval-pair comparison, `and`/`or` the input
 * condition-tree grammar (distributed to DNF by the engine's validation),
 * and `duration` the measure term. Nothing beyond the IR exists here: no
 * convenience operator fakes an unsupported comparison.
 */

import * as errors from "@superbuilders/errors"
import type { IntervalValue } from "#brand.ts"
import { phantom } from "#brand.ts"
import type { OneOf } from "#face.ts"
import type { FieldData, FieldValue } from "#fields.ts"
import type { PredicateData } from "#query/predicate.ts"
import type {
	AnyTerm,
	AnyVar,
	ItemParams,
	MaskParam,
	Param,
	ParamSet,
	ParamsRecord,
	ParamsShape,
	TermContribution,
	Var
} from "#query/scope.ts"
import { isTerm, term } from "#query/scope.ts"
import type { AnyRelation, FieldsShape, Relation } from "#relation.ts"

/**
 * One atom-binding position as runtime data: a scope term, a host literal
 * (tagged at lowering by the FIELD's structural type — a point-typed
 * literal at an interval field is the IR's membership typing rule), or an
 * `oneOf` literal set (lowered to a fresh variable plus a disjunctive
 * equality condition — the three-confinement law's rule-level OR, spelled
 * for the caller as one binding).
 */
type BindingTerm =
	| { readonly kind: "term"; readonly value: AnyTerm }
	| { readonly kind: "literal"; readonly value: unknown }
	| { readonly kind: "oneOf"; readonly values: readonly unknown[] }

/** One resolved binding: the field's name, its description, and the term. */
interface BindingEntry {
	readonly field: string
	readonly data: FieldData
	readonly term: BindingTerm
}

/** Where an atom draws its facts: a stored relation or a scope predicate. */
type AtomSourceData =
	| { readonly kind: "relation"; readonly relation: AnyRelation }
	| { readonly kind: "predicate"; readonly pred: PredicateData }

/**
 * One atom value (positive or negated — negation is a position in the
 * rule, not a kind of atom, exactly as the IR reuses `Atom` unchanged).
 * The phantom carries the params object the atom's bindings contribute.
 */
interface MatchAtom<P extends ParamsRecord> {
	readonly item: "atom"
	readonly negated: boolean
	readonly source: AtomSourceData
	readonly bindings: readonly BindingEntry[]
	readonly [phantom]?: P
}

/** One comparison side as runtime data. */
type CmpTerm =
	| { readonly kind: "term"; readonly value: AnyTerm }
	| { readonly kind: "literal"; readonly value: unknown }
	| { readonly kind: "measure"; readonly over: AnyVar }

/** The `allen` mask position: a literal 13-bit mask or a mask parameter. */
type MaskData =
	| { readonly kind: "literal"; readonly mask: number }
	| { readonly kind: "param"; readonly param: MaskParam<string> }

/** One comparison operator as runtime data (mirrors `ir::CmpOp`). */
type CmpOpData =
	| { readonly kind: "eq" }
	| { readonly kind: "ne" }
	| { readonly kind: "lt" }
	| { readonly kind: "le" }
	| { readonly kind: "gt" }
	| { readonly kind: "ge" }
	| { readonly kind: "allen"; readonly mask: MaskData }
	| { readonly kind: "pointIn" }

/** One comparison condition value. */
interface ComparisonItem<P extends ParamsRecord> {
	readonly item: "cmp"
	readonly op: CmpOpData
	readonly lhs: CmpTerm
	readonly rhs: CmpTerm
	readonly [phantom]?: P
}

/**
 * One condition-tree node (`ir::ConditionTree`): any boolean combination
 * of comparisons — the engine's validation distributes nested OR to DNF
 * rules; the surface admits exactly what the IR admits.
 */
interface ConditionTreeItem<P extends ParamsRecord> {
	readonly item: "tree"
	readonly op: "and" | "or"
	readonly children: readonly AnyCondition[]
	readonly [phantom]?: P
}

/** Any condition value, whatever its params. */
type AnyCondition = ComparisonItem<ParamsRecord> | ConditionTreeItem<ParamsRecord>

/** Any rule body item: an atom (either polarity) or a condition. */
type AnyBodyItem = MatchAtom<ParamsRecord> | AnyCondition

/**
 * The measure of an interval-typed variable (`ir::Term::Measure`, surface
 * `Duration`): `|[s, e)| = e − s`, u64 — legal as one side of an order
 * comparison, as a projected select entry, and as the input of
 * `sum`/`min`/`max`; every other position is unwritable here exactly as
 * the IR rejects it typed. A ray has no finite measure — the engine's
 * `MeasureOfRay` execution error; exclude rays first (`allen` against a
 * bounded window).
 */
interface Duration {
	readonly measure: AnyVar
	readonly [phantom]?: bigint
}

/**
 * What a binding position of value type `V` accepts: a branded literal, a
 * disjunctive `oneOf` literal set, a `Var`/`Param`/`ParamSet` of the same
 * brand — and, when the field is interval-typed, a point-typed term (the
 * IR's membership typing rule: an element-typed term at an interval field
 * is point membership; an interval-typed term is value equality).
 */
type TermInput<V> =
	| V
	| OneOf<V>
	| Var<V>
	| Param<string, V>
	| ParamSet<string, V>
	| (V extends IntervalValue ? PointTermInput : never)

/** The point-typed terms an interval field position additionally accepts. */
type PointTermInput = bigint | Var<bigint> | Param<string, bigint> | ParamSet<string, bigint>

/**
 * The `match` bindings record: per field, a term of that field's brand;
 * unmentioned fields are wildcards (absence IS the wildcard — the IR has
 * no wildcard variant to spell).
 */
type MatchInput<Fields extends FieldsShape> = {
	readonly [K in keyof Fields]?: TermInput<FieldValue<Fields[K]>>
}

/** The params object a bindings record contributes (used by `match`). */
type BindingsParams<B> = ParamsShape<TermContribution<B[keyof B]>>

/** Narrows an `oneOf` literal set (the detection rule `where()` uses). */
function isOneOf(value: unknown): value is OneOf<unknown> {
	return typeof value === "object" && value !== null && "literals" in value && Array.isArray(value.literals)
}

/** Resolves one binding value to its runtime term. */
function bindingTermOf(context: string, value: unknown): BindingTerm {
	if (isTerm(value)) {
		if (value[term] === "maskParam") {
			throw errors.new(
				`${context}: an Allen-mask param is not a field-typed value — masks live in allen() conditions only`
			)
		}
		return Object.freeze({ kind: "term" as const, value })
	}
	if (isOneOf(value)) {
		return Object.freeze({ kind: "oneOf" as const, values: Object.freeze([...value.literals]) })
	}
	return Object.freeze({ kind: "literal" as const, value })
}

/**
 * Resolves a bindings record against an ordered field roster (shared by
 * relation atoms here and predicate atoms in `#query/predicate.ts`), in
 * the record's written order.
 */
function resolveBindings(
	context: string,
	fields: ReadonlyArray<{ readonly name: string; readonly field: FieldData }>,
	bindings: Readonly<Record<string, unknown>>
): readonly BindingEntry[] {
	const entries: BindingEntry[] = []
	for (const [fieldName, value] of Object.entries(bindings)) {
		if (value === undefined) {
			continue
		}
		const declared = fields.find(function byName(candidate) {
			return candidate.name === fieldName
		})
		if (declared === undefined) {
			throw errors.new(`${context} has no field ${fieldName}`)
		}
		entries.push(
			Object.freeze({
				field: fieldName,
				data: declared.field,
				term: bindingTermOf(`${context}.${fieldName}`, value)
			})
		)
	}
	return Object.freeze(entries)
}

/**
 * The named-field atom — the semantic twin of `query!`'s `Node(id: c)`:
 * fields bind vars, params, branded literals, `oneOf` sets, or (interval
 * fields) point terms; unmentioned fields are wildcards; a zero-binding
 * atom is a nonemptiness gate on the relation (IR-legal, so writable).
 */
function match<Name extends string, Fields extends FieldsShape, const B extends MatchInput<Fields>>(
	relation: Relation<Name, Fields>,
	bindings: B
): MatchAtom<BindingsParams<B>> {
	return Object.freeze({
		item: "atom" as const,
		negated: false,
		source: Object.freeze({ kind: "relation" as const, relation }),
		bindings: resolveBindings(
			`relation ${relation.name}`,
			relation.data.fields,
			Object.fromEntries(Object.entries(bindings))
		)
	})
}

/**
 * Negation — anti-join over sets, no null trick: a binding satisfies the
 * negated atom iff NO fact matches it. Safety (every negated var bound
 * positively) is validated at `query()` construction with an error naming
 * the variable; the negated atom binds nothing, only rejects. An `oneOf`
 * binding is refused HERE, at construction: it lowers to a synthetic
 * variable bound only inside the atom plus a rule-level OR — which is
 * exactly the shape the safety rule rejects, and semantically wrong for
 * negation anyway (¬∃(f = a ∨ f = b) is a CONJUNCTION of negated atoms,
 * ¬∃(f = a) ∧ ¬∃(f = b), never one negated atom's OR).
 */
function not<P extends ParamsRecord>(atom: MatchAtom<P>): MatchAtom<P> {
	if (atom.item !== "atom" || atom.negated) {
		throw errors.new("negation is a position in the rule, not an operator — not() takes one positive atom")
	}
	for (const binding of atom.bindings) {
		if (binding.term.kind === "oneOf") {
			const source = atom.source.kind === "relation" ? atom.source.relation.name : `predicate ${atom.source.pred.name}`
			throw errors.new(
				`negated ${source} atom binds ${binding.field} with oneOf(...) — ¬∃(${binding.field} = a ∨ ${binding.field} = b) means ¬∃(${binding.field} = a) ∧ ¬∃(${binding.field} = b): write one not(match(...)) per literal, or bind a paramSet`
			)
		}
	}
	return Object.freeze({
		item: "atom" as const,
		negated: true,
		source: atom.source,
		bindings: atom.bindings
	})
}

/** Builds one comparison value. */
function comparison(op: CmpOpData, lhs: CmpTerm, rhs: CmpTerm): ComparisonItem<never> {
	return Object.freeze({ item: "cmp" as const, op: Object.freeze(op), lhs, rhs })
}

/** Narrows a `duration()` value. */
function isDuration(value: unknown): value is Duration {
	return typeof value === "object" && value !== null && "measure" in value && !isTerm(value)
}

/** Resolves one comparison side to its runtime term. */
function cmpTermOf(value: unknown): CmpTerm {
	if (isTerm(value)) {
		return Object.freeze({ kind: "term" as const, value })
	}
	if (isDuration(value)) {
		return Object.freeze({ kind: "measure" as const, over: value.measure })
	}
	return Object.freeze({ kind: "literal" as const, value })
}

/**
 * Rejects a comparison with no term side: it is constant-valued, the
 * engine's own validation refuses it, and the lowering has no field
 * position to type the literals by — fail here with the same verdict.
 */
function assertTermSide(op: string, lhs: CmpTerm, rhs: CmpTerm): void {
	if (lhs.kind === "literal" && rhs.kind === "literal") {
		throw errors.new(
			`${op}: a comparison without a variable or parameter side is constant-valued — write the query you mean`
		)
	}
}

/**
 * The equality atom (`ir::CmpOp::Eq`) — for binding a var to a param or
 * literal where punning inside `match` doesn't apply, and for var-to-var
 * unification. Prefer direct placement (`match(Account, { kind:
 * Kind.Savings })`); `is` exists for the var-to-param case. `ParamSet` is
 * legal here and under no other operator (the IR's `Eq`-only set rule).
 */
function is<V, const R extends Var<V> | Param<string, V> | ParamSet<string, V> | V>(
	left: Var<V>,
	right: R
): ComparisonItem<ParamsShape<TermContribution<R>>> {
	return comparison(Object.freeze({ kind: "eq" as const }), cmpTermOf(left), cmpTermOf(right))
}

/** Typed disequality (`ir::CmpOp::Ne`). "Not in set" has no operator — write a negated atom. */
function ne<V, const R extends Var<V> | Param<string, V> | V>(
	left: Var<V>,
	right: R
): ComparisonItem<ParamsShape<TermContribution<R>>> {
	return comparison(Object.freeze({ kind: "ne" as const }), cmpTermOf(left), cmpTermOf(right))
}

/**
 * One side of an order comparison: a u64/i64-typed var or param, a bigint
 * literal, or the measure (`duration(v)`) — order operators are legal for
 * the orderable types only, never intervals/bytes/strings/bools (the IR's
 * comparison rules; each refusal is the engine's own typed diagnostic).
 */
type OrderInput = Var<bigint> | Param<string, bigint> | bigint | Duration

/** The shared order-comparison constructor. */
function order<const L extends OrderInput, const R extends OrderInput>(
	op: "lt" | "le" | "gt" | "ge",
	left: L,
	right: R
): ComparisonItem<ParamsShape<TermContribution<L> | TermContribution<R>>> {
	const lhs = cmpTermOf(left)
	const rhs = cmpTermOf(right)
	assertTermSide(op, lhs, rhs)
	return comparison(Object.freeze({ kind: op }), lhs, rhs)
}

/** Strict less-than (`ir::CmpOp::Lt`). */
function lt<const L extends OrderInput, const R extends OrderInput>(
	left: L,
	right: R
): ComparisonItem<ParamsShape<TermContribution<L> | TermContribution<R>>> {
	return order("lt", left, right)
}

/** Less-or-equal (`ir::CmpOp::Le`). */
function le<const L extends OrderInput, const R extends OrderInput>(
	left: L,
	right: R
): ComparisonItem<ParamsShape<TermContribution<L> | TermContribution<R>>> {
	return order("le", left, right)
}

/** Strict greater-than (`ir::CmpOp::Gt`). */
function gt<const L extends OrderInput, const R extends OrderInput>(
	left: L,
	right: R
): ComparisonItem<ParamsShape<TermContribution<L> | TermContribution<R>>> {
	return order("gt", left, right)
}

/** Greater-or-equal (`ir::CmpOp::Ge`). */
function ge<const L extends OrderInput, const R extends OrderInput>(
	left: L,
	right: R
): ComparisonItem<ParamsShape<TermContribution<L> | TermContribution<R>>> {
	return order("ge", left, right)
}

/** An interval-typed comparison side: a var, a param, or a `span` literal. */
type IntervalInput = Var<IntervalValue> | Param<string, IntervalValue> | IntervalValue

/** A point-typed comparison side. */
type PointInput = Var<bigint> | Param<string, bigint> | bigint

/**
 * Point membership as a predicate (`ir::CmpOp::PointIn`): `covers(iv, t)`
 * holds iff `iv.start ≤ t < iv.end` — the predicate form of the membership
 * typing rule, for terms already bound elsewhere. Interval ⊇ interval is
 * NOT this operator; that predicate is `allen(a, ALLEN.covers, b)`. The IR
 * orders the operands interval-left, point-right; so does this surface.
 */
function covers<const I extends IntervalInput, const T extends PointInput>(
	interval: I,
	point: T
): ComparisonItem<ParamsShape<TermContribution<I> | TermContribution<T>>> {
	const lhs = cmpTermOf(interval)
	const rhs = cmpTermOf(point)
	assertTermSide("covers", lhs, rhs)
	return comparison(Object.freeze({ kind: "pointIn" as const }), lhs, rhs)
}

/**
 * The 13-bit mask range: bits above the low 13 are unrepresentable in the
 * engine's `AllenMask` (`bumbledb/crates/bumbledb/src/allen.rs`:
 * `AllenMask::new` refuses them) — the check here is the same boundary,
 * moved to construction where the message can name the constants.
 */
const ALLEN_ALL_BITS = (1 << 13) - 1

/**
 * The Allen coordinate system's named constants — the 13 basics in the
 * engine's palindromic bit order (bit i = basic i:
 * `bumbledb/crates/bumbledb/src/allen.rs`, a specified representation)
 * plus the workload composites, values identical to the engine's. Compose
 * with `|`: `ALLEN.before | ALLEN.meets`.
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
 * parameter (`$.allenParam`). Vacuous masks (empty/full) are the engine's
 * two distinct typed rejections; nothing is pre-judged here beyond the
 * representable bit range.
 */
function allen<
	const A extends IntervalInput,
	const M extends number | MaskParam<string>,
	const B extends IntervalInput
>(
	left: A,
	mask: M,
	right: B
): ComparisonItem<ParamsShape<TermContribution<A> | TermContribution<M> | TermContribution<B>>> {
	const lhs = cmpTermOf(left)
	const rhs = cmpTermOf(right)
	assertTermSide("allen", lhs, rhs)
	const maskValue: number | MaskParam<string> = mask
	if (typeof maskValue === "number") {
		if (!Number.isInteger(maskValue) || maskValue < 0 || maskValue > ALLEN_ALL_BITS) {
			throw errors.new(
				`allen mask ${maskValue} is not a 13-bit mask — build masks from the ALLEN constants (bumbledb allen.rs: bits above the low 13 are unrepresentable)`
			)
		}
		return comparison(
			Object.freeze({
				kind: "allen" as const,
				mask: Object.freeze({ kind: "literal" as const, mask: maskValue })
			}),
			lhs,
			rhs
		)
	}
	return comparison(
		Object.freeze({
			kind: "allen" as const,
			mask: Object.freeze({ kind: "param" as const, param: maskValue })
		}),
		lhs,
		rhs
	)
}

/**
 * Conjunction node of the input condition grammar (`ConditionTree::And`).
 * The rule's condition list is already a conjunction — `and` exists for
 * nesting under `or`, and the empty combination keeps the IR's algebraic
 * reading (`And([])` is true).
 */
function and<const C extends readonly AnyCondition[]>(
	...children: C
): ConditionTreeItem<ParamsShape<ItemParams<C[number]>>> {
	return Object.freeze({
		item: "tree" as const,
		op: "and" as const,
		children: Object.freeze([...children])
	})
}

/**
 * Disjunction node of the input condition grammar (`ConditionTree::Or`) —
 * the one place the surface admits a nested OR; validation distributes it
 * to DNF rules engine-side (OR is data or it is nothing). `Or([])` keeps
 * its algebraic reading (false: the rule denotes nothing).
 */
function or<const C extends readonly AnyCondition[]>(
	...children: C
): ConditionTreeItem<ParamsShape<ItemParams<C[number]>>> {
	return Object.freeze({
		item: "tree" as const,
		op: "or" as const,
		children: Object.freeze([...children])
	})
}

/**
 * The measure term — surface `Duration`, IR `Measure`: the point-set
 * cardinality `end − start` of an interval-typed variable, u64. See
 * {@link Duration} for the legal positions.
 */
function duration<IV extends IntervalValue>(over: Var<IV>): Duration {
	if (over.data.type.kind !== "interval") {
		throw errors.new(
			`duration(${over.relation}.${over.field}): the measure is defined over interval-typed variables only`
		)
	}
	return Object.freeze({ measure: over })
}

export type {
	AnyBodyItem,
	AnyCondition,
	AtomSourceData,
	BindingEntry,
	BindingsParams,
	BindingTerm,
	CmpOpData,
	CmpTerm,
	ComparisonItem,
	ConditionTreeItem,
	Duration,
	IntervalInput,
	MaskData,
	MatchAtom,
	MatchInput,
	OrderInput,
	PointInput,
	PointTermInput,
	TermInput
}
export { ALLEN, allen, and, covers, duration, ge, gt, is, le, lt, match, ne, not, or, resolveBindings }
