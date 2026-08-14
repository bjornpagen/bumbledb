/**
 * `query()` and the IR lowering, REFERENCE-IDENTITY edition. A query is
 * built kysely-shaped — variables minted by {@link v} outside the rule and
 * reused by REFERENCE to join:
 *
 *   query(S).rule((r) => {
 *     const acct = v(Account)
 *     const h = v(Holder)
 *     return r
 *       .match(Account, { id: acct.id, holder: acct.holder })
 *       .match(Holder, { id: acct.holder })
 *       .where(r.eq(acct.holder, r.param("root")))
 *       .find({ account: acct.id, holder: acct.holder })
 *   })
 *
 * — and is an INERT value: `Query<Rels, Row, Params>` with `Row` inferred
 * from each rule's `.find` RECORD (its keys ARE the answer columns) and
 * `Params` inferred to be EXACTLY the params the rules use (params are typed
 * BY USE; a param no rule uses never registers). Variable IDENTITY is the
 * object reference: reusing one value across binding positions IS the join,
 * lowering is a pure function of the query value down to the
 * bridge's `QueryIr` (`bumbledb/crates/bumbledb/src/ir.rs`): relations by
 * declaration ordinal, variables by dense per-rule first-occurrence ids
 * (keyed on the object REFERENCE — the discipline is unchanged, only the map
 * key moved from name to reference), params by first-use order. Lowering is
 * STABLE — the same query value lowers to deeply-equal IR every time, and
 * two identically-written queries (fresh mints each) lower identically.
 * Construction validates negation safety and boundness (typed by the var's
 * label — object identity is invisible to the type tier, so these are
 * construction-time walls); everything else (types, aggregate
 * rosters, rule caps) is the ENGINE's judge, surfacing at prepare.
 */

import * as errors from "@superbuilders/errors"
import { sealedFieldsOf } from "#closed.ts"
import type { AnyField, ClosedRoster } from "#fields.ts"
import { assertDeclarationOrderKey, isIntervalValue, literalShapeError, rosterOf } from "#fields.ts"
import type { ClassRecordOf, SchemaClasses } from "#law.ts"
import type {
	AtomIr,
	ComparisonIr,
	ConditionTreeIr,
	FindTermIr,
	HeadOpIr,
	HeadTermIr,
	ParsedQuery,
	QueryParam,
	RuleIr,
	TaggedValue,
	TermIr
} from "#native.ts"
import type {
	AggData,
	AnyCond,
	AtomData,
	BindingEntry,
	BindParamsShape,
	CheckBindings,
	CheckCond,
	CmpData,
	CmpKind,
	CmpTermData,
	CondData,
	CondParamsShape,
	DerivedTable,
	FindColumn,
	FindEntryData,
	InteriorData,
	MaskData,
	MatchFields,
	MatchOwner,
	MatchShape,
	ParamUse,
	RecData,
	RecHandle,
	RecHead,
	RuleData,
	RuleItem
} from "#query/atom.ts"
import { allen, and, eq, ge, gt, le, lt, ne, not, or, pointIn } from "#query/atom.ts"
import type { CheckFind, CheckRecFind, FindShape, HeadRecordOf, RowOfFind } from "#query/find.ts"
import { count, max, min, pack, sum } from "#query/find.ts"
import { parseQueryIr } from "#query/parse-ir.ts"
import type {
	AnyVar,
	ClassedField,
	Flatten,
	InferredOf,
	ParamEntry,
	ParamsRecord,
	ShapeOf
} from "#query/scope.ts"
import {
	fieldJoins,
	inferred,
	isTerm,
	makeDuration,
	makeParam,
	makeSetParam,
	renderFieldKind,
	term
} from "#query/scope.ts"
import type { AnySchema, Schema, SchemaRelations } from "#schema.ts"

/**
 * The matchable members of a schema's record — ordinary relations AND
 * closed vocabularies (ψ query atoms; the ENGINE decides folding vs virtual
 * image, the SDK lowers pass-through).
 */
type QueryRelation<Rels extends SchemaRelations> = Extract<Rels[keyof Rels], MatchOwner>

/** Reads an inferred-params carrier off a rec reference or rule value. */
type ParamsOf<T> = InferredOf<T> extends { readonly params: infer P extends ParamsRecord } ? P : Record<never, never>

/** Reads an inferred-row carrier off a rule value or query. */
type RowOf<T> = InferredOf<T> extends { readonly row: infer R } ? R : never

/**
 * A derived table's HEAD signature as classed slots, keyed by column
 * name; `undefined` on values that carry no head.
 */
type HeadShape = Readonly<Record<string, ClassedField>> | undefined

/**
 * One finished rule as a plain value: the runtime data plus the inferred
 * row/params carrier (and, for an interior or recursive rule, the head
 * record of classed slots an `.interior(name)` join pairs against).
 * `.rule(...)` consumes it.
 */
interface RuleValue<Row, P extends ParamsRecord, Head extends HeadShape = undefined> {
	readonly rule: RuleData
	readonly [inferred]?: { readonly row: Row; readonly params: P; readonly head: Head }
}

/** Any finished rule value. */
type AnyRuleValue = RuleValue<unknown, ParamsRecord, HeadShape>

/** Reads an inferred-head carrier off a rule value or rec reference. */
type HeadOf<T> =
	InferredOf<T> extends { readonly head: infer H extends Readonly<Record<string, ClassedField>> } ? H : undefined

/** One `.interior(name)` position's judgment: a variable. */
type InteriorBindingOk<V> = V extends AnyVar ? true : false

/**
 * The validated `.interior(name)` bindings record: every entry must be a
 * variable. Arity and class against the named table's head are
 * construction-time (the name is a string, so the head is not a type-level
 * fact).
 */
type CheckInteriorBindings<B> = {
	readonly [K in keyof B]: InteriorBindingOk<B[K]> extends true ? B[K] : never
}

/** One interior-rule builder function. */
type InteriorBuild<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> = (
	r: InteriorRuleScope<Rels, Classes>
) => AnyRuleValue

/** One rec-arm builder function. */
type RecBuild<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> = (
	r: RecRuleScope<Rels, Classes>
) => AnyRuleValue

/** A build function's rule value. */
type BuiltRule<F> = F extends (r: never) => infer RV ? RV : never

/** The intersected params record of a tuple of rule builds. */
type BuildsParams<Builds extends readonly ((r: never) => AnyRuleValue)[]> = ShapeOf<
	ParamsOf<BuiltRule<Builds[number]>>
>

/**
 * The term/predicate/aggregate constructor vocabulary every rule builder
 * carries — pure value builders. Variables are minted by the free {@link v},
 * outside the rule, and reused by reference; `r` no longer mints them.
 */
interface TermOps {
	/** Names one scalar parameter: typed by its use; the key of the execute params object. */
	readonly param: typeof makeParam
	/** Names one ∈-set parameter (the IR's `ParamSet`): bound to a readonly array at execution. */
	readonly inSet: typeof makeSetParam
	/** The measure of an interval-typed variable: `|[s, e)| = e − s`, u64. */
	readonly duration: typeof makeDuration
	readonly eq: typeof eq
	readonly ne: typeof ne
	readonly lt: typeof lt
	readonly le: typeof le
	readonly gt: typeof gt
	readonly ge: typeof ge
	readonly pointIn: typeof pointIn
	readonly allen: typeof allen
	readonly and: typeof and
	readonly or: typeof or
	readonly not: typeof not
	readonly count: typeof count
	readonly sum: typeof sum
	readonly min: typeof min
	readonly max: typeof max
	readonly pack: typeof pack
}

/** The rule builder a `query(S).rule(...)` callback receives (`Classes` — the join judge's authority). */
interface QueryRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> extends TermOps {
	/** The first EDB atom of the rule: fields bind variables, params, ∈-sets, or bare literals; absence is the wildcard. */
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): QueryRuleChain<Rels, BindParamsShape<MatchFields<R>, B>, Classes>
	/**
	 * A rule may START with a finished table: an interior atom is a positive
	 * occurrence exactly as the engine represents it, so its variables ground
	 * — the identity projection `(c) | reach(c);` is spellable with no
	 * re-grounding join over a domain relation.
	 */
	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): QueryRuleChain<Rels, Record<never, never>, Classes>
}

/** The chain of a plain query rule: more atoms, residual predicates, then the head. */
interface QueryRuleChain<
	Rels extends SchemaRelations,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	/** One more positive EDB atom — variable reuse joins, class-equal by the mint-slot judgment. */
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): QueryRuleChain<Rels, Flatten<P & BindParamsShape<MatchFields<R>, B>>, Classes>
	/** One residual predicate: a comparison, an `and`/`or` tree, or a negated atom (`r.not`). */
	where<const C extends AnyCond>(
		cond: CheckCond<Classes, C> & C
	): QueryRuleChain<Rels, Flatten<P & CondParamsShape<C>>, Classes>
	/** One interior atom over a finished table (an earlier interior, or the rec). */
	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): QueryRuleChain<Rels, P, Classes>
	/** The head projection: a `find` RECORD whose keys name the answer columns. */
	find<const F extends FindShape>(entries: F & CheckFind<F>): RuleValue<RowOfFind<F>, P>
}

/** The rule builder an `interior("mid", ...)` callback receives. */
interface InteriorRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses>
	extends TermOps {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): InteriorRuleChain<Rels, BindParamsShape<MatchFields<R>, B>, Classes>
	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): InteriorRuleChain<Rels, Record<never, never>, Classes>
}

/** The chain of an interior rule: bound-variable heads only. */
interface InteriorRuleChain<
	Rels extends SchemaRelations,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): InteriorRuleChain<Rels, Flatten<P & BindParamsShape<MatchFields<R>, B>>, Classes>
	where<const C extends AnyCond>(
		cond: CheckCond<Classes, C> & C
	): InteriorRuleChain<Rels, Flatten<P & CondParamsShape<C>>, Classes>
	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): InteriorRuleChain<Rels, P, Classes>
	find<const F extends FindShape>(entries: F & CheckRecFind<F>): RuleValue<RowOfFind<F>, P, HeadRecordOf<Classes, F>>
}

/** The rule builder a `recursive("reach", { base, rec })` arm receives. */
interface RecRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> extends TermOps {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): RecRuleChain<Rels, BindParamsShape<MatchFields<R>, B>, Classes>
	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): RecRuleChain<Rels, Record<never, never>, Classes>
}

/**
 * The chain of a recursive arm. `.interior("reach", …)` is the self-atom
 * on rec arms (and a prior interior on either list). `find` takes bound
 * variables only — aggregates and the measure are unrepresentable in a
 * recursive head.
 */
interface RecRuleChain<
	Rels extends SchemaRelations,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): RecRuleChain<Rels, Flatten<P & BindParamsShape<MatchFields<R>, B>>, Classes>
	where<const C extends AnyCond>(
		cond: CheckCond<Classes, C> & C
	): RecRuleChain<Rels, Flatten<P & CondParamsShape<C>>, Classes>
	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): RecRuleChain<Rels, P, Classes>
	find<const F extends FindShape>(entries: F & CheckRecFind<F>): RuleValue<RowOfFind<F>, P, HeadRecordOf<Classes, F>>
}

/** A query's runtime description — everything lowering, the wire marshal, and answer decode read. */
interface QueryData {
	/** Named interiors in declaration order (DAG). */
	readonly interiors: readonly InteriorData[]
	/** The optional linear rec. */
	readonly rec: RecData | null
	/** The main rules in written order (multiple rules = set union). */
	readonly rules: readonly RuleData[]
	/** The head columns (every rule derives the same head; written order = answer column order). */
	readonly finds: readonly FindColumn[]
	/** The registered params in first-use order across the query walk (= dense `ParamId`s). */
	readonly params: readonly ParamEntry[]
}

/**
 * An inert query value. `Row` is the inferred answer-row object type;
 * `Params` the inferred execute-params object type — exactly the params the
 * rules use. Prepare with `db.prepare(q)`.
 */
interface Query<
	Rels extends SchemaRelations,
	Row,
	Params extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	readonly schema: Schema<Rels, Classes>
	readonly data: QueryData
	/** One more rule — the query's answers are the SET UNION of its rules' answers; every rule derives the same head. */
	rule<RV extends AnyRuleValue>(
		build: (r: QueryRuleScope<Rels, Classes>) => RV
	): Query<Rels, Row | RowOf<RV>, Flatten<Params & ParamsOf<RV>>, Classes>
	/** Construction error: interiors precede main rules. Uncallable after `.rule()`. */
	interior(name: string, ...builds: never[]): never
	/** Construction error: recursive precedes main rules. Uncallable after `.rule()`. */
	recursive(name: string, arms: never): never
	readonly [inferred]?: { readonly row: Row; readonly params: Params }
}

/** Any query value as lowering and the runtime consume it. */
interface AnyQuery {
	readonly schema: AnySchema
	readonly data: QueryData
}

/** Extracts a query value's inferred answer-row type. */
type QueryRow<Q extends AnyQuery> = RowOf<Q>

/** Extracts a query value's inferred execute-params type. */
type QueryParams<Q extends AnyQuery> = ParamsOf<Q>

/**
 * The entry value of `query(S)`: interiors, then optional recursive, then
 * the first `.rule` mints the query. `interior` / `recursive` exist only
 * while `Rec` is `null`; `.recursive()` moves the type parameter.
 */
type QueryStart<
	Rels extends SchemaRelations,
	Classes extends SchemaClasses = SchemaClasses,
	P extends ParamsRecord = Record<never, never>,
	Rec extends RecData | null = null
> = {
	rule<RV extends AnyRuleValue>(
		build: (r: QueryRuleScope<Rels, Classes>) => RV
	): Query<Rels, RowOf<RV>, Flatten<P & ParamsOf<RV>>, Classes>
} & (Rec extends null
	? {
			interior<const Builds extends readonly InteriorBuild<Rels, Classes>[]>(
				name: string,
				...builds: Builds
			): QueryStart<Rels, Classes, Flatten<P & BuildsParams<Builds>>, null>
			recursive<
				const Base extends readonly RecBuild<Rels, Classes>[],
				const Step extends readonly RecBuild<Rels, Classes>[]
			>(
				name: string,
				arms: { readonly base: Base; readonly rec: Step }
			): QueryStart<Rels, Classes, Flatten<P & BuildsParams<Base> & BuildsParams<Step>>, RecData>
		}
	: {
			interior: never
			recursive: never
		})

/** The frozen constructor vocabulary every rule builder spreads. */
const termOps: TermOps = Object.freeze({
	param: makeParam,
	inSet: makeSetParam,
	duration: makeDuration,
	eq,
	ne,
	lt,
	le,
	gt,
	ge,
	pointIn,
	allen,
	and,
	or,
	not,
	count,
	sum,
	min,
	max,
	pack
})

/** One rule under construction: immutable — every chain step is a fresh state. Boundness rides the `bound` set of var references. */
interface RuleBuildState {
	readonly items: readonly RuleItem[]
	readonly bound: ReadonlySet<AnyVar>
	readonly paramUses: readonly ParamUse[]
}

/** The empty rule state. */
const EMPTY_RULE: RuleBuildState = Object.freeze({
	items: Object.freeze([]),
	bound: new Set<AnyVar>(),
	paramUses: Object.freeze([])
})

/** One resolved bindings record: the atom entries, the variable references it binds, and the params it uses. */
interface ResolvedBindings {
	readonly atom: AtomData
	readonly vars: readonly AnyVar[]
	readonly uses: readonly ParamUse[]
}

/**
 * The MINT slot of a variable, the runtime twin of {@link MintSlotOf}: (i)
 * verifies the mint owner is the schema's own member value — a variable
 * minted from a foreign relation is refused, naming its label — and (ii)
 * returns the descriptor it was minted at plus the law-computed class read
 * off the schema's frozen class map. Because {@link fieldJoins} is an
 * equality, judging every binding position against this one slot makes all
 * cross-binding joins mutually class-equal by transitivity.
 */
function mintSlotOf(context: ChainContext, ref: AnyVar): ClassedField {
	if (context.theory.relations[ref.owner.name] !== ref.owner) {
		throw errors.new(
			`the variable ${ref.label} was minted from a relation schema ${context.theory.name} does not declare — mint variables with v() from the schema's own relations`
		)
	}
	return { field: ref.field, class: context.classes[ref.owner.name]?.[ref.column] }
}

/**
 * Judges one membership ARRAY at a binding position — legal exactly at a
 * CLOSED-reference field, holding ≥ 2 DISTINCT handle names. The returned
 * name is CONTENT-ADDRESSED (vocabulary + the member SET).
 */
function membershipSet(
	context: string,
	field: AnyField,
	value: readonly unknown[]
): { readonly name: string; readonly members: readonly string[] } {
	const roster = rosterOf(field)
	if (roster === undefined) {
		throw errors.new(
			`${context}: a membership array is the closed-reference spelling — ordinary field membership is a bound ∈-set param (r.inSet)`
		)
	}
	if (value.length === 0) {
		throw errors.new(`${context}: an empty membership array selects nothing — write the query you mean`)
	}
	if (value.length === 1) {
		throw errors.new(
			`${context}: a one-element membership array is the bare literal respelled — write the literal (the canonical-utterance law: one meaning, one spelling)`
		)
	}
	const seen = new Set<string>()
	const members = value.map(function memberName(member) {
		if (typeof member !== "string") {
			throw literalShapeError(context, `a ${roster.name} handle name (string)`, member)
		}
		if (seen.has(member)) {
			throw errors.new(
				`${context}: the membership array spells ${member} twice — write it once (the canonical-utterance law: one meaning, one spelling)`
			)
		}
		seen.add(member)
		return member
	})
	const key = [...members].sort()
	return { name: `∈ ${roster.name} ${JSON.stringify(key)}`, members: Object.freeze(members) }
}

/**
 * Resolves a bindings record against an atom owner's matchable fields, in
 * the record's written order: terms classify by their runtime tag,
 * everything else is a bare literal. Every VARIABLE binding judges
 * `fieldJoins(mintSlot, positionSlot)` and throws on a class-unequal reuse
 * (the runtime twin of `CheckBindings`); the bound refs are collected for
 * the rule's boundness set.
 */
function resolveBindings(
	context: ChainContext,
	label: string,
	relation: MatchOwner,
	bindings: Readonly<Record<string, unknown>>
): ResolvedBindings {
	const entries: BindingEntry[] = []
	const vars: AnyVar[] = []
	const uses: ParamUse[] = []
	const relationClasses = context.classes[relation.name]
	const ordered = sealedFieldsOf(relation)
	for (const [fieldName, value] of Object.entries(bindings)) {
		if (value === undefined) {
			continue
		}
		const declared = ordered.find(function byName(candidate) {
			return candidate.name === fieldName
		})
		if (declared === undefined) {
			throw errors.new(`${label} has no field ${fieldName}`)
		}
		const fieldClass = relationClasses?.[fieldName]
		let bound: BindingEntry["term"]
		if (isTerm(value)) {
			switch (value[term]) {
				case "var": {
					const ref = value
					const mint = mintSlotOf(context, ref)
					const positionSlot: ClassedField = { field: declared.field, class: fieldClass }
					if (!fieldJoins(mint, positionSlot)) {
						throw errors.new(
							`${label}: the variable ${ref.label} joins domain-unequal fields — minted at ${renderFieldKind(mint)}, reused at ${renderFieldKind(positionSlot)} (a var joins only class-equal slots; bare pairs only with bare)`
						)
					}
					bound = Object.freeze({ kind: "var" as const, ref })
					vars.push(ref)
					break
				}
				case "param": {
					bound = Object.freeze({ kind: "param" as const, name: value.name })
					uses.push(
						Object.freeze({
							name: value.name,
							shape: "value" as const,
							anchor: declared.field,
							op: "binding" as const,
							members: undefined
						})
					)
					break
				}
				case "setParam": {
					bound = Object.freeze({ kind: "setParam" as const, name: value.name })
					uses.push(
						Object.freeze({
							name: value.name,
							shape: "set" as const,
							anchor: declared.field,
							op: "binding" as const,
							members: undefined
						})
					)
					break
				}
				case "duration":
					throw errors.new(
						`${label}.${fieldName}: the measure is not a field-typed value — it lives in comparisons and find entries`
					)
			}
		} else if (Array.isArray(value)) {
			const set = membershipSet(`${label}.${fieldName}`, declared.field, value)
			bound = Object.freeze({ kind: "literalSet" as const, name: set.name, members: set.members })
			uses.push(
				Object.freeze({
					name: set.name,
					shape: "set" as const,
					anchor: declared.field,
					op: "binding" as const,
					members: set.members
				})
			)
		} else {
			bound = Object.freeze({ kind: "literal" as const, value })
		}
		entries.push(Object.freeze({ field: fieldName, data: declared.field, class: fieldClass, term: bound }))
	}
	return { atom: Object.freeze({ relation, bindings: Object.freeze(entries) }), vars, uses }
}

/** Extends a rule state with one positive atom; the bound variable references accumulate into the boundness set. */
function advanceMatch(
	context: ChainContext,
	state: RuleBuildState,
	relation: MatchOwner,
	bindings: Readonly<Record<string, unknown>>
): RuleBuildState {
	const resolved = resolveBindings(context, `relation ${relation.name}`, relation, bindings)
	const bound = new Set(state.bound)
	for (const ref of resolved.vars) {
		bound.add(ref)
	}
	return Object.freeze({
		items: Object.freeze([...state.items, Object.freeze({ kind: "atom" as const, atom: resolved.atom })]),
		bound,
		paramUses: Object.freeze([...state.paramUses, ...resolved.uses])
	})
}

/** Resolves one comparison side to its runtime term (variables and the measure ride by reference). */
function cmpTermDataOf(value: unknown): CmpTermData {
	if (isTerm(value)) {
		switch (value[term]) {
			case "var":
				return Object.freeze({ kind: "var" as const, ref: value })
			case "param":
				return Object.freeze({ kind: "param" as const, name: value.name })
			case "setParam":
				return Object.freeze({ kind: "setParam" as const, name: value.name })
			case "duration":
				return Object.freeze({ kind: "measure" as const, ref: value.over })
		}
	}
	return Object.freeze({ kind: "literal" as const, value })
}

/**
 * One comparison side's contribution to the param census: a param/set side
 * anchors to its SIBLING — a variable's field descriptor or the measure; an
 * unanchorable use records with no anchor.
 */
function sideUses(op: CmpKind, side: CmpTermData, sibling: CmpTermData, uses: ParamUse[]): void {
	if (side.kind !== "param" && side.kind !== "setParam") {
		return
	}
	let anchor: AnyField | "measure" | undefined
	if (sibling.kind === "var") {
		anchor = sibling.ref.field
	} else if (sibling.kind === "measure") {
		anchor = "measure"
	} else {
		anchor = undefined
	}
	uses.push(
		Object.freeze({
			name: side.name,
			shape: side.kind === "param" ? ("value" as const) : ("set" as const),
			anchor,
			op,
			members: undefined
		})
	)
}

/** Lowers one condition VALUE to its runtime data, recording param uses. */
function condDataOf(cond: AnyCond, uses: ParamUse[]): CondData {
	if (cond.cond === "cmp") {
		const lhs = cmpTermDataOf(cond.lhs)
		const rhs = cmpTermDataOf(cond.rhs)
		sideUses(cond.op, lhs, rhs, uses)
		sideUses(cond.op, rhs, lhs, uses)
		let mask: MaskData | undefined
		if (cond.op === "allen") {
			const maskValue = cond.mask
			if (typeof maskValue !== "number") {
				throw errors.new("allen: the mask position takes a 13-bit mask number built from the ALLEN constants")
			}
			mask = Object.freeze({ kind: "literal" as const, mask: maskValue })
		}
		return Object.freeze({ kind: "cmp" as const, op: cond.op, mask, lhs, rhs })
	}
	if (cond.cond === "tree") {
		const children = cond.children.map(function lowerChild(child) {
			return condDataOf(child, uses)
		})
		return Object.freeze({ kind: "tree" as const, op: cond.op, children: Object.freeze(children) })
	}
	throw errors.new(
		"a negated atom is not a condition-tree node — pass not(...) to where() directly, never inside and()/or()"
	)
}

/** Extends a rule state with one `.where` item (a condition or a negated atom). */
function advanceWhere(context: ChainContext, state: RuleBuildState, cond: AnyCond): RuleBuildState {
	if (typeof cond !== "object" || cond === null || !("cond" in cond)) {
		throw errors.new("where() takes a comparison, an and()/or() tree, or a negated atom")
	}
	if (cond.cond === "notInterior") {
		const bindings: Readonly<Record<string, unknown>> = Object.fromEntries(
			Object.entries(cond.bindings ?? {}).filter(function defined([, value]) {
				return value !== undefined
			})
		)
		return notInteriorAdvance(context, state, cond.name, bindings)
	}
	if (cond.cond === "not") {
		const relation: MatchOwner = cond.relation
		const bindings: Readonly<Record<string, unknown>> = Object.fromEntries(
			Object.entries(cond.bindings ?? {}).filter(function defined([, value]) {
				return value !== undefined
			})
		)
		const resolved = resolveBindings(context, `negated relation ${relation.name}`, relation, bindings)
		return Object.freeze({
			items: Object.freeze([...state.items, Object.freeze({ kind: "negated" as const, atom: resolved.atom })]),
			bound: state.bound,
			paramUses: Object.freeze([...state.paramUses, ...resolved.uses])
		})
	}
	const uses: ParamUse[] = []
	const data = condDataOf(cond, uses)
	return Object.freeze({
		items: Object.freeze([...state.items, Object.freeze({ kind: "cond" as const, cond: data })]),
		bound: state.bound,
		paramUses: Object.freeze([...state.paramUses, ...uses])
	})
}

/**
 * Extends a rule state with one interior atom (a named record over head keys;
 * vars validated at completion). A POSITIVE interior atom is a positive
 * occurrence exactly as the engine represents it (`check_atoms` walks Interior
 * and Edb in one loop), so its variables GROUND: they enter the rule's
 * boundness set, may ride the head, and satisfy negation safety — the
 * interior-only identity projection of a finished table is spellable with no
 * re-grounding join. A NEGATED one binds nothing, only rejects.
 */
function advanceInterior(
	state: RuleBuildState,
	target: DerivedTable,
	bindings: Readonly<Record<string, unknown>>,
	negated: boolean
): RuleBuildState {
	const resolved: Array<{ readonly key: string; readonly ref: AnyVar }> = []
	for (const [key, value] of Object.entries(bindings)) {
		if (value === undefined) {
			continue
		}
		if (!isTerm(value) || value[term] !== "var") {
			throw errors.new(
				`interior ${target.name}: position ${key} takes a variable — bind literals and params through where()/match()`
			)
		}
		resolved.push(Object.freeze({ key, ref: value }))
	}
	const bound = new Set(state.bound)
	if (!negated) {
		for (const binding of resolved) {
			bound.add(binding.ref)
		}
	}
	return Object.freeze({
		items: Object.freeze([
			...state.items,
			Object.freeze({ kind: "interior" as const, target, bindings: Object.freeze(resolved), negated })
		]),
		bound,
		paramUses: state.paramUses
	})
}

/** Narrows a find entry to an aggregate value. */
function isAggregateEntry(value: unknown): value is { readonly agg: string; readonly over: unknown } {
	return typeof value === "object" && value !== null && "agg" in value
}

/** Narrows a value to a variable term, else a pointed refusal. */
function asVarTerm(context: string, value: unknown): AnyVar {
	if (isTerm(value) && value[term] === "var") {
		return value
	}
	throw errors.new(`${context}: expected a variable`)
}

/** Classifies one aggregate find entry into its runtime data (variables ride by reference). */
function aggDataOf(name: string, entry: { readonly agg: string; readonly over: unknown }): AggData {
	const over = entry.over
	switch (entry.agg) {
		case "count":
			return Object.freeze({ op: "count" as const })
		case "sum":
		case "min":
		case "max": {
			if (isTerm(over) && over[term] === "var") {
				return Object.freeze({ op: "fold" as const, fold: entry.agg, over })
			}
			if (isTerm(over) && over[term] === "duration") {
				return Object.freeze({ op: "fold" as const, fold: entry.agg, over: Object.freeze({ duration: over.over }) })
			}
			throw errors.new(`find ${name} (${entry.agg}): takes a variable or r.duration(v)`)
		}
		case "pack":
			return Object.freeze({ op: "pack" as const, over: asVarTerm(`find ${name} (pack)`, over) })
		default:
			throw errors.new(`find ${name}: unknown aggregate ${entry.agg}`)
	}
}

/**
 * Classifies one find entry into its named answer column (the KEY names the
 * column, `count` included). The `slot`/`closed` slices are resolved LATER,
 * at rule completion, where boundness and the mint slots are in hand.
 */
function findColumnOf(name: string, entry: unknown): FindColumn {
	if (isTerm(entry)) {
		if (entry[term] === "var") {
			return Object.freeze({
				name,
				entry: Object.freeze({ kind: "var" as const, over: entry }),
				closed: undefined,
				slot: undefined
			})
		}
		if (entry[term] === "duration") {
			return Object.freeze({
				name,
				entry: Object.freeze({ kind: "measure" as const, over: entry.over }),
				closed: undefined,
				slot: undefined
			})
		}
		throw errors.new(
			`find ${name}: a ${entry[term]} is not projectable — find takes variables, r.duration(v), or aggregates`
		)
	}
	if (isAggregateEntry(entry)) {
		return Object.freeze({
			name,
			entry: Object.freeze({ kind: "aggregate" as const, agg: aggDataOf(name, entry) }),
			closed: undefined,
			slot: undefined
		})
	}
	throw errors.new(`find ${name}: not a find entry — find takes variables, r.duration(v), or aggregates`)
}

/**
 * The orderable ban's pointed refusal (`docs/architecture/10-data-model.md`
 * § orderability): a closed reference is equality-and-membership only.
 */
function closedOrderError(context: string, position: string, vocabulary: string): Error {
	return errors.new(
		`${context}: ${position} is a ${vocabulary} reference — declaration order is an accident, not semantics: vocabularies do not order (docs/architecture/10-data-model.md; equality, membership, and counting remain)`
	)
}

/** The comparison ops under the orderable ban (order roster + point membership). */
function isOrderOp(op: CmpKind | "binding"): op is "lt" | "le" | "gt" | "ge" | "pointIn" {
	return op === "lt" || op === "le" || op === "gt" || op === "ge" || op === "pointIn"
}

/** Requires a variable to be bound by a relation atom of the rule (the boundness wall — invisible to the type tier). */
function assertBound(where: string, bound: ReadonlySet<AnyVar>, ref: AnyVar): void {
	if (!bound.has(ref)) {
		throw errors.new(`${where}: the variable ${ref.label} is not bound by a relation atom of the rule`)
	}
}

/** Requires a variable to be interval-typed (the measure's and pack's domain), off its own descriptor. */
function assertInterval(where: string, ref: AnyVar): void {
	if (ref.field.kind !== "interval") {
		throw errors.new(
			`${where}: ${ref.label} is not interval-typed — the measure is defined over interval-typed variables only`
		)
	}
}

/** Requires a variable's own field to be non-closed (the orderable ban's runtime twin). */
function assertNotClosed(where: string, position: string, ref: AnyVar): void {
	const roster = rosterOf(ref.field)
	if (roster !== undefined) {
		throw closedOrderError(where, `${position} ${ref.label}`, roster.name)
	}
}

/**
 * The classed mint slot one answer column's VALUES flow from: a projected
 * variable's mint slot. Counts, folds, `pack` and the measure derive
 * numbers/intervals, so they resolve no slot.
 */
function findColumnSlotOf(context: ChainContext, column: FindColumn): ClassedField | undefined {
	const entry = column.entry
	if (entry.kind === "var") {
		return mintSlotOf(context, entry.over)
	}
	return undefined
}

/** Validates one find column's variable references (boundness + the orderable/interval walls, off the var's own field). */
function validateColumn(context: ChainContext, bound: ReadonlySet<AnyVar>, column: FindColumn): void {
	const where = `${contextLabel(context)} find ${column.name}`
	const entry = column.entry
	if (entry.kind === "var") {
		assertBound(where, bound, entry.over)
		return
	}
	if (entry.kind === "measure") {
		assertBound(where, bound, entry.over)
		assertInterval(where, entry.over)
		return
	}
	const agg = entry.agg
	switch (agg.op) {
		case "count":
			return
		case "fold": {
			if ("duration" in agg.over) {
				assertBound(where, bound, agg.over.duration)
				assertInterval(where, agg.over.duration)
				return
			}
			assertBound(where, bound, agg.over)
			assertNotClosed(where, `the ${agg.fold} input`, agg.over)
			return
		}
		case "pack":
			assertBound(where, bound, agg.over)
			assertInterval(where, agg.over)
			return
	}
}

/**
 * Validates one condition's variable references against the rule's bound
 * set — and, for `eq`/`ne` over two variables, holds the class wall through
 * the mint slots (the unification IS a join; bare pairs only with bare).
 */
function validateCond(context: ChainContext, bound: ReadonlySet<AnyVar>, cond: CondData): void {
	const label = contextLabel(context)
	if (cond.kind === "cmp") {
		for (const side of [cond.lhs, cond.rhs]) {
			if (side.kind === "var") {
				assertBound(label, bound, side.ref)
				const roster = rosterOf(side.ref.field)
				if (isOrderOp(cond.op) && roster !== undefined) {
					throw closedOrderError(label, `the ${cond.op} side ${side.ref.label}`, roster.name)
				}
			}
			if (side.kind === "measure") {
				assertBound(label, bound, side.ref)
				assertInterval(label, side.ref)
			}
		}
		if ((cond.op === "eq" || cond.op === "ne") && cond.lhs.kind === "var" && cond.rhs.kind === "var") {
			assertBound(label, bound, cond.lhs.ref)
			assertBound(label, bound, cond.rhs.ref)
			const lhs = mintSlotOf(context, cond.lhs.ref)
			const rhs = mintSlotOf(context, cond.rhs.ref)
			if (!fieldJoins(lhs, rhs)) {
				throw errors.new(
					`${label}: ${cond.op}(${cond.lhs.ref.label}, ${cond.rhs.ref.label}) unifies domain-unequal fields — ${cond.lhs.ref.label} bound at ${renderFieldKind(lhs)}, ${cond.rhs.ref.label} at ${renderFieldKind(rhs)} (a var joins only class-equal slots; bare pairs only with bare)`
				)
			}
		}
		return
	}
	for (const child of cond.children) {
		validateCond(context, bound, child)
	}
}

/**
 * Validates one interior item: every head column of the table is bound exactly
 * once (a missing or extra key is a pointed error) and each variable joins
 * its head column's classed slot. A POSITIVE interior atom GROUNDS its
 * variables (a positive occurrence, exactly the engine's representation),
 * so no boundness precondition exists; a NEGATED one binds nothing — its
 * variables must be positively bound elsewhere in the rule, the same
 * safety rule as EDB negation. When the table's own first rule is in flight
 * (`finds` empty), the completing rule's OWN find columns ARE the head.
 */
function validateInterior(
	context: ChainContext,
	bound: ReadonlySet<AnyVar>,
	item: {
		readonly target: DerivedTable
		readonly bindings: ReadonlyArray<{ readonly key: string; readonly ref: AnyVar }>
		readonly negated: boolean
	},
	columns: readonly FindColumn[]
): void {
	const label = contextLabel(context)
	const headColumns = item.target.finds.length > 0 ? item.target.finds : columns
	const headNames = headColumns.map(function nameOf(column) {
		return column.name
	})
	const keys = item.bindings.map(function keyOf(binding) {
		return binding.key
	})
	for (const key of keys) {
		if (!headNames.includes(key)) {
			throw errors.new(
				`${label}: interior ${item.target.name} binds ${key}, not a head column of ${item.target.name} (head columns: ${headNames.join(", ")})`
			)
		}
	}
	for (const name of headNames) {
		if (!keys.includes(name)) {
			throw errors.new(
				`${label}: interior ${item.target.name} omits the head column ${name} — an interior join binds every head column of ${item.target.name}`
			)
		}
	}
	for (const binding of item.bindings) {
		if (item.negated && !bound.has(binding.ref)) {
			throw errors.new(
				`${label}: negated interior ${item.target.name} names the variable ${binding.ref.label}, but no positive atom of the rule binds it — a negated atom binds nothing, only rejects (the safety rule)`
			)
		}
		const headColumn = headColumns.find(function byName(column) {
			return column.name === binding.key
		})
		if (headColumn === undefined || headColumn.slot === undefined) {
			continue
		}
		const mint = mintSlotOf(context, binding.ref)
		if (!fieldJoins(headColumn.slot, mint)) {
			throw errors.new(
				`${label}: interior ${item.target.name} joins the variable ${binding.ref.label} (${renderFieldKind(mint)}) at head column ${binding.key} (${renderFieldKind(headColumn.slot)}) — a var joins only class-equal slots; bare pairs only with bare`
			)
		}
	}
}

/**
 * Completes one rule: enriches the find columns (declaration-order-safe
 * keys, boundness validated, each column's classed slot and closed slice
 * resolved), then walks the body walls — negated-atom boundness safety, interior
 * head pairing, and condition validation.
 */
function completeRule(context: ChainContext, state: RuleBuildState, rawColumns: readonly FindColumn[]): RuleData {
	const label = contextLabel(context)
	if (rawColumns.length === 0) {
		throw errors.new(`${label}: a find needs at least one entry`)
	}
	const columns = rawColumns.map(function enrichColumn(column): FindColumn {
		assertDeclarationOrderKey(`${label} find column`, column.name)
		validateColumn(context, state.bound, column)
		const slot = findColumnSlotOf(context, column)
		return Object.freeze({ name: column.name, entry: column.entry, slot, closed: rosterOf(slot?.field) })
	})
	for (const item of state.items) {
		if (item.kind === "negated") {
			for (const binding of item.atom.bindings) {
				if (binding.term.kind === "var" && !state.bound.has(binding.term.ref)) {
					throw errors.new(
						`${label}: negated ${item.atom.relation.name} atom binds the variable ${binding.term.ref.label} at position ${binding.field}, but no positive atom of the rule binds it — a negated atom binds nothing, only rejects (the safety rule)`
					)
				}
			}
		}
		if (item.kind === "interior") {
			validateInterior(context, state.bound, item, columns)
		}
		if (item.kind === "cond") {
			validateCond(context, state.bound, item.cond)
		}
	}
	return Object.freeze({ items: state.items, finds: Object.freeze(columns), paramUses: state.paramUses })
}

/** Builds one typed rule value over completed rule data. */
function makeRuleValue<Row, P extends ParamsRecord>(rule: RuleData): RuleValue<Row, P> {
	return Object.freeze({ rule })
}

/**
 * The one runtime chain every context shares — non-generic on purpose. The
 * typed chain interfaces apply at the scope factories' boundaries.
 */
interface RawChain {
	match(relation: MatchOwner, bindings: Readonly<Record<string, unknown>>): RawChain
	where(cond: AnyCond): RawChain
	interior(name: string, bindings: Readonly<Record<string, unknown>>): RawChain
	find(entries: Readonly<Record<string, unknown>>): RuleValue<never, never>
}

/** The runtime rule-builder shape beneath every typed scope. */
interface RawScope extends TermOps {
	match(relation: MatchOwner, bindings: Readonly<Record<string, unknown>>): RawChain
	interior(name: string, bindings: Readonly<Record<string, unknown>>): RawChain
}

/** The declared derived tables a chain may name. */
interface DerivedEnv {
	readonly interiors: readonly InteriorData[]
	readonly rec: RecHandle | RecHead | RecData | null
}

/** Which rule family a chain builds — plus the schema's runtime class map and theory value (the join judge's authority). */
type ChainContext = { readonly classes: SchemaClasses; readonly theory: AnySchema } & DerivedEnv &
	(
		| { readonly kind: "query" }
		| { readonly kind: "interior"; readonly self: string }
		| { readonly kind: "rec-base"; readonly self: RecHandle }
		| { readonly kind: "rec-arm"; readonly self: RecHead }
	)

/** The diagnostic label of a chain context. */
function contextLabel(context: ChainContext): string {
	switch (context.kind) {
		case "query":
			return "query rule"
		case "interior":
			return `interior ${context.self} rule`
		case "rec-base":
			return `recursive ${context.self.name} base`
		case "rec-arm":
			return `recursive ${context.self.name} rec`
	}
}

/** Resolves a derived-table name against the context's visible tables. */
function lookupDerived(context: ChainContext, name: string): DerivedTable {
	const interior = context.interiors.find(function byName(candidate) {
		return candidate.name === name
	})
	if (interior !== undefined) {
		if (context.kind === "interior" && name === context.self) {
			throw errors.new(
				`interior ${name}: an interior does not read itself — declaration order is topological (a self-read is InteriorNotPrior)`
			)
		}
		return interior
	}
	if (context.rec !== null && context.rec.name === name) {
		if (context.kind === "interior") {
			throw errors.new(
				`interior ${context.self}: interiors cannot read the rec — this cut's interiors are a prefix`
			)
		}
		if (context.kind === "rec-base") {
			throw errors.new(
				`recursive ${context.rec.name}: a base arm does not read the rec — self-atoms belong on rec arms`
			)
		}
		if (!("finds" in context.rec)) {
			throw errors.new(`recursive ${context.rec.name}: rec arms resolve the rec head after base arms seal it`)
		}
		return context.rec
	}
	throw errors.new(`${contextLabel(context)}: no derived table named ${name} is in scope`)
}

/** Validates and records one interior atom per the context's cut. */
function interiorAdvance(
	context: ChainContext,
	state: RuleBuildState,
	name: string,
	bindings: Readonly<Record<string, unknown>>
): RuleBuildState {
	return advanceInterior(state, lookupDerived(context, name), bindings, false)
}

/**
 * Validates and records one NEGATED finished-table atom — main and interior
 * rules: a finished set is a set. Rec bodies refuse every negation
 * (`NegationInRec` — self is the wall; EDB / earlier-interior is this-cut).
 */
function notInteriorAdvance(
	context: ChainContext,
	state: RuleBuildState,
	name: string,
	bindings: Readonly<Record<string, unknown>>
): RuleBuildState {
	if (context.kind === "rec-base" || context.kind === "rec-arm") {
		throw errors.new(
			`recursive ${context.self.name}: a recursive rule negates no table — self-negation is negation through the cycle (a finished set is what keeps the operator monotone), and a finished table's fold belongs in the main rules`
		)
	}
	return advanceInterior(state, lookupDerived(context, name), bindings, true)
}

/** Classifies one find record per the context (interior and rec heads project bound variables only). */
function findColumns(context: ChainContext, entries: Readonly<Record<string, unknown>>): FindColumn[] {
	const columns: FindColumn[] = []
	const derivedHead = context.kind !== "query"
	for (const [name, entry] of Object.entries(entries)) {
		if (entry === undefined) {
			continue
		}
		if (derivedHead && !(isTerm(entry) && entry[term] === "var")) {
			const who =
				context.kind === "interior"
					? `interior ${context.self}`
					: `recursive ${context.self.name}`
			throw errors.new(
				`${who}: a recursive head projects bound variables only — aggregates and the measure read finished sets (unwritable here)`
			)
		}
		columns.push(findColumnOf(name, entry))
	}
	return columns
}

/** Builds one runtime chain (immutably — every step is a fresh chain over fresh state). */
function makeRawChain(context: ChainContext, state: RuleBuildState): RawChain {
	const chain: RawChain = {
		match(relation, bindings) {
			return makeRawChain(context, advanceMatch(context, state, relation, bindings))
		},
		where(cond) {
			return makeRawChain(context, advanceWhere(context, state, cond))
		},
		interior(name, bindings) {
			return makeRawChain(context, interiorAdvance(context, state, name, bindings))
		},
		find(entries) {
			return makeRuleValue<never, never>(completeRule(context, state, findColumns(context, entries)))
		}
	}
	Object.freeze(chain)
	return chain
}

/** Builds one runtime rule-builder over a context. */
function makeRawScope(context: ChainContext): RawScope {
	const scope: RawScope = {
		...termOps,
		match(relation, bindings) {
			return makeRawChain(context, advanceMatch(context, EMPTY_RULE, relation, bindings))
		},
		interior(name, bindings) {
			return makeRawChain(context, interiorAdvance(context, EMPTY_RULE, name, bindings))
		}
	}
	Object.freeze(scope)
	return scope
}

/**
 * The rule builders' trusted admission seam — THE home of the
 * trusted-admission-seam pattern the other mint guards cite: the raw builder
 * is one runtime shape for every context, and this guard verifies the
 * checkable fact — the builder verbs exist — before the value is admitted at
 * its TYPED face. The type-level judgments (class-equal joins, the recursion
 * cut) live in the interfaces themselves; boundness is a construction-time
 * validation in this module (object identity is invisible to the type tier).
 */
function isTypedScope<S>(scope: RawScope): scope is RawScope & S {
	return typeof scope.match === "function"
}

/** Builds one query-rule builder (the typed face of the raw builder). */
function makeQueryRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: AnySchema,
	env: DerivedEnv
): QueryRuleScope<Rels, Classes> {
	const raw = makeRawScope({ kind: "query", classes: theory.classes, theory, ...env })
	if (!isTypedScope<QueryRuleScope<Rels, Classes>>(raw)) {
		throw errors.new("query rule builder construction incomplete")
	}
	return raw
}

/** Builds one interior-rule builder. */
function makeInteriorRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: AnySchema,
	env: DerivedEnv,
	self: string
): InteriorRuleScope<Rels, Classes> {
	const raw = makeRawScope({ kind: "interior", self, classes: theory.classes, theory, ...env })
	if (!isTypedScope<InteriorRuleScope<Rels, Classes>>(raw)) {
		throw errors.new("interior rule builder construction incomplete")
	}
	return raw
}

/** Builds one rec-arm builder. */
function makeRecRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: AnySchema,
	env: DerivedEnv,
	self: RecHandle,
	kind: "rec-base"
): RecRuleScope<Rels, Classes>
function makeRecRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: AnySchema,
	env: DerivedEnv,
	self: RecHead,
	kind: "rec-arm"
): RecRuleScope<Rels, Classes>
function makeRecRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: AnySchema,
	env: DerivedEnv,
	self: RecHandle | RecHead,
	kind: "rec-base" | "rec-arm"
): RecRuleScope<Rels, Classes> {
	const raw =
		kind === "rec-base"
			? makeRawScope({
					kind: "rec-base",
					self: self as RecHandle,
					classes: theory.classes,
					theory,
					...env
				})
			: makeRawScope({
					kind: "rec-arm",
					self: self as RecHead,
					classes: theory.classes,
					theory,
					...env
				})
	if (!isTypedScope<RecRuleScope<Rels, Classes>>(raw)) {
		throw errors.new("recursive rule builder construction incomplete")
	}
	return raw
}

/** Renders one head column's closed slice for the rule-alignment check's diagnostics. */
function renderClosedSlice(closed: ClosedRoster | undefined): string {
	return closed === undefined ? "a bare value" : `a ${closed.name} reference`
}

/** Renders one head column's signature for the rule-alignment check. */
function headSignature(column: FindColumn): string {
	const entry = column.entry
	if (entry.kind === "var" || entry.kind === "measure") {
		return `${column.name}:var`
	}
	const agg = entry.agg
	if (agg.op === "fold") {
		return `${column.name}:${agg.fold}`
	}
	return `${column.name}:${agg.op}`
}

/** The roster a param anchor carries: present exactly on a closed-reference field anchor. */
function anchorRosterOf(anchor: AnyField | "measure" | undefined): ClosedRoster | undefined {
	return anchor === "measure" ? undefined : rosterOf(anchor)
}

/** Renders one param anchor's closedness for the registry's coherence diagnostics. */
function renderParamAnchor(roster: ClosedRoster | undefined): string {
	return roster === undefined ? "a non-closed position" : `a ${roster.name} reference`
}

/**
 * Folds every rule's param uses (interiors in declaration order, then rec
 * base, then rec arms, then main — exactly the lowering walk) into the
 * query's registry: first use mints the dense `ParamId`, the first
 * FIELD-ANCHORED use types the wire, and one name keeps one shape AND one
 * closedness. The orderable ban needs no registry arm: an order use always
 * anchors its SIBLING's domain (the no-variable-side spelling is refused
 * at the comparison constructor), so a closed-anchored param under an
 * order op dies at the one-domain wall here — and an order use whose
 * sibling is itself closed-bound dies at the comparison's own var-side
 * wall first.
 */
function paramRegistryOf(
	interiors: readonly InteriorData[],
	rec: RecData | null,
	rules: readonly RuleData[]
): readonly ParamEntry[] {
	const order: string[] = []
	const byName = new Map<
		string,
		{
			shape: ParamEntry["shape"]
			anchor: ParamEntry["anchor"]
			op: ParamEntry["op"]
			members: readonly string[] | undefined
		}
	>()
	function fold(uses: readonly ParamUse[]): void {
		for (const use of uses) {
			const existing = byName.get(use.name)
			if (existing === undefined) {
				order.push(use.name)
				byName.set(use.name, {
					shape: use.shape,
					anchor: use.anchor,
					op: use.op,
					members: use.members
				})
				continue
			}
			if ((existing.members === undefined) !== (use.members === undefined)) {
				throw errors.new(
					`query param ${use.name} collides with a membership array's registry entry — name the param differently`
				)
			}
			if (existing.shape !== use.shape) {
				throw errors.new(
					`query param ${use.name} is used both as a ${existing.shape} param and a ${use.shape} param — one name, one shape`
				)
			}
			if (existing.anchor !== undefined && use.anchor !== undefined) {
				const registered = anchorRosterOf(existing.anchor)
				const anchored = anchorRosterOf(use.anchor)
				if (registered !== anchored) {
					throw errors.new(
						`query param ${use.name} is anchored at ${renderParamAnchor(registered)} and at ${renderParamAnchor(anchored)} — a closed-anchored param translates handle names through ONE roster (one name, one domain); name the params differently`
					)
				}
			}
			if (existing.anchor === undefined && use.anchor !== undefined) {
				existing.anchor = use.anchor
				existing.op = use.op
			}
		}
	}
	for (const interior of interiors) {
		for (const rule of interior.rules) {
			fold(rule.paramUses)
		}
	}
	if (rec !== null) {
		for (const rule of rec.base) {
			fold(rule.paramUses)
		}
		for (const rule of rec.rec) {
			fold(rule.paramUses)
		}
	}
	for (const rule of rules) {
		fold(rule.paramUses)
	}
	return Object.freeze(
		order.map(function entryOf(name): ParamEntry {
			const entry = byName.get(name)
			if (entry === undefined) {
				throw errors.new(`query param ${name} lost its registry entry`)
			}
			/**
			 * A membership array's handle names are program constants, so the
			 * entry stores the resolved IMAGE: each name rides the one
			 * roster-verification point (`taggedHandleId`, through
			 * `taggedCmpLiteral`) exactly once, HERE — an out-of-roster name
			 * fails at build, and every execute returns this frozen value by
			 * reference.
			 */
			let membership: QueryParam | undefined
			if (entry.members !== undefined) {
				const anchor = entry.anchor
				if (anchor === undefined) {
					throw errors.new(`query param ${name} lost its membership anchor`)
				}
				membership = Object.freeze({
					kind: "set" as const,
					values: Object.freeze(
						entry.members.map(function tagMember(member, index) {
							return Object.freeze(taggedCmpLiteral(`membership array ${name}[${index}]`, anchor, member, entry.op))
						})
					)
				})
			}
			return Object.freeze({ name, shape: entry.shape, anchor: entry.anchor, op: entry.op, membership })
		})
	)
}

/** The runtime query shape beneath the typed `Query` face. */
interface RawQuery {
	readonly schema: AnySchema
	readonly data: QueryData
	rule(build: (r: RawScope) => RuleValue<never, never>): RawQuery
	interior(name: string, ...builds: never[]): never
	recursive(name: string, arms: never): never
}

/** Asserts every rule in a list derives the same head (name, aggregate shape, closed slice, class). */
function assertAlignedHeads(label: string, rules: readonly RuleData[]): void {
	const first = rules[0]
	if (first === undefined) {
		throw errors.new(`${label} needs at least one rule`)
	}
	const signature = first.finds.map(headSignature).join(", ")
	rules.forEach(function verifyHead(rule, index) {
		const candidate = rule.finds.map(headSignature).join(", ")
		if (candidate !== signature) {
			throw errors.new(
				`every rule of ${label} derives the same head — rule 0 finds (${signature}), rule ${index} finds (${candidate})`
			)
		}
		rule.finds.forEach(function verifyClosedSlice(column, position) {
			const lead = first.finds[position]
			if (lead !== undefined && column.closed !== lead.closed) {
				throw errors.new(
					`every rule of ${label} derives the same head — the head column ${lead.name} is ${renderClosedSlice(lead.closed)} in rule 0 but ${renderClosedSlice(column.closed)} in rule ${index} (one column decodes through one roster)`
				)
			}
			if (lead === undefined) {
				return
			}
			if (lead.slot !== undefined && column.slot !== undefined && !fieldJoins(lead.slot, column.slot)) {
				throw errors.new(
					`every rule of ${label} derives the same head — the head column ${lead.name} is bound at ${renderFieldKind(lead.slot)} in rule 0 but at ${renderFieldKind(column.slot)} in rule ${index} (a head column joins only class-equal slots; bare pairs only with bare)`
				)
			}
		})
	})
}

function afterMainError(what: string): Error {
	return errors.new(
		`query: ${what} after a main rule is unwritable — declaration order is interiors, then rec, then main`
	)
}

/**
 * Assembles the runtime query value over completed rules: every rule must
 * derive the SAME head (name and aggregate shape, position for position —
 * the decode labels and the engine's alignment rule agree), and the param
 * registry folds in query-walk order.
 */
function makeRawQuery(
	theory: AnySchema,
	interiors: readonly InteriorData[],
	rec: RecData | null,
	rules: readonly RuleData[]
): RawQuery {
	assertAlignedHeads("a query", rules)
	const first = rules[0]
	if (first === undefined) {
		throw errors.new("a query needs at least one rule")
	}
	const env: DerivedEnv = { interiors, rec }
	const data: QueryData = Object.freeze({
		interiors: Object.freeze([...interiors]),
		rec,
		rules: Object.freeze([...rules]),
		finds: first.finds,
		params: paramRegistryOf(interiors, rec, rules)
	})
	const value: RawQuery = {
		schema: theory,
		data,
		rule(build) {
			const built = build(makeRawScope({ kind: "query", classes: theory.classes, theory, ...env }))
			return makeRawQuery(theory, interiors, rec, [...rules, built.rule])
		},
		interior() {
			throw afterMainError("interior")
		},
		recursive() {
			throw afterMainError("recursive")
		}
	}
	Object.freeze(value)
	return value
}

/**
 * The query values' trusted admission seam (the {@link isTypedScope} pattern):
 * the checkable fact — the value was assembled over the identical theory —
 * is verified before the raw value is admitted at its typed face.
 */
function isQueryValue<Rels extends SchemaRelations, Row, P extends ParamsRecord, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>,
	value: RawQuery
): value is RawQuery & Query<Rels, Row, P, Classes> {
	return value.schema === theory
}

/** Assembles one typed query value (rules already completed). */
function makeQuery<Rels extends SchemaRelations, Row, P extends ParamsRecord, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>,
	interiors: readonly InteriorData[],
	rec: RecData | null,
	rules: readonly RuleData[]
): Query<Rels, Row, P, Classes> {
	const raw = makeRawQuery(theory, interiors, rec, rules)
	if (!isQueryValue<Rels, Row, P, Classes>(theory, raw)) {
		throw errors.new("query value construction incomplete")
	}
	return raw
}

/** Collects one Interior from its builders. */
function collectInterior<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>,
	env: DerivedEnv,
	name: string,
	builds: readonly InteriorBuild<Rels, Classes>[]
): InteriorData {
	if (builds.length === 0) {
		throw errors.new(`query: interior ${name} needs at least one rule`)
	}
	const rules = builds.map(function buildRule(buildOne) {
		return buildOne(makeInteriorRuleScope<Rels, Classes>(theory, env, name)).rule
	})
	assertAlignedHeads(`interior ${name}`, rules)
	const first = rules[0]
	if (first === undefined) {
		throw errors.new(`query: interior ${name} needs at least one rule`)
	}
	return Object.freeze({ name, finds: first.finds, rules: Object.freeze(rules) })
}

/** Collects the Rec from tagged base/rec builder arrays. */
function collectRec<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>,
	interiors: readonly InteriorData[],
	name: string,
	baseBuilds: readonly RecBuild<Rels, Classes>[],
	recBuilds: readonly RecBuild<Rels, Classes>[]
): RecData {
	if (baseBuilds.length === 0) {
		throw errors.new(`query: recursive ${name} has no base arms`)
	}
	if (recBuilds.length === 0) {
		throw errors.new(`query: recursive ${name} has no rec arms`)
	}
	const handle: RecHandle = Object.freeze({ name })
	const baseEnv: DerivedEnv = { interiors, rec: handle }
	const base = baseBuilds.map(function buildBase(buildOne) {
		return buildOne(makeRecRuleScope<Rels, Classes>(theory, baseEnv, handle, "rec-base")).rule
	})
	assertAlignedHeads(`recursive ${name}`, base)
	const first = base[0]
	if (first === undefined) {
		throw errors.new(`query: recursive ${name} has no base arms`)
	}
	const firstFind = first.finds[0]
	if (firstFind === undefined) {
		throw errors.new(`query: recursive ${name} has no head`)
	}
	const finds: RecHead["finds"] = [firstFind, ...first.finds.slice(1)]
	const head: RecHead = Object.freeze({ name, finds })
	const recEnv: DerivedEnv = { interiors, rec: head }
	const rec = recBuilds.map(function buildRec(buildOne) {
		return buildOne(makeRecRuleScope<Rels, Classes>(theory, recEnv, head, "rec-arm")).rule
	})
	assertAlignedHeads(`recursive ${name}`, [...base, ...rec])
	const firstRec = rec[0]
	if (firstRec === undefined) {
		throw errors.new(`query: recursive ${name} has no rec arms`)
	}
	const sealedBase: RecData["base"] = [first, ...base.slice(1)]
	const sealedRec: RecData["rec"] = [firstRec, ...rec.slice(1)]
	const recData: RecData = Object.freeze({
		name,
		finds,
		base: sealedBase,
		rec: sealedRec
	})
	return recData
}

/** Builds the query start (interiors, then optional rec, then the first main rule). */
function makeQueryStart<
	Rels extends SchemaRelations,
	Classes extends SchemaClasses,
	P extends ParamsRecord,
	Rec extends RecData | null
>(
	theory: Schema<Rels, Classes>,
	interiors: readonly InteriorData[],
	rec: Rec
): QueryStart<Rels, Classes, P, Rec> {
	const env: DerivedEnv = { interiors, rec }
	const start = {
		interior<const Builds extends readonly InteriorBuild<Rels, Classes>[]>(
			name: string,
			...builds: Builds
		): QueryStart<Rels, Classes, Flatten<P & BuildsParams<Builds>>, null> {
			if (rec !== null) {
				throw errors.new(
					"query: interior after recursive is unwritable — declaration order is interiors, then rec, then main"
				)
			}
			if (interiors.some(function sameName(interior) { return interior.name === name })) {
				throw errors.new(`query: interior ${name} is already declared — names are unique`)
			}
			if (name.length === 0) {
				throw errors.new("query: an interior needs a name")
			}
			const data = collectInterior(theory, env, name, builds)
			return makeQueryStart<Rels, Classes, Flatten<P & BuildsParams<Builds>>, null>(theory, [...interiors, data], null)
		},
		recursive<
			const Base extends readonly RecBuild<Rels, Classes>[],
			const Step extends readonly RecBuild<Rels, Classes>[]
		>(
			name: string,
			arms: { readonly base: Base; readonly rec: Step }
		): QueryStart<Rels, Classes, Flatten<P & BuildsParams<Base> & BuildsParams<Step>>, RecData> {
			if (rec !== null) {
				throw errors.new("query: a second recursive is unwritable — this cut admits one rec SCC")
			}
			if (interiors.some(function sameName(interior) { return interior.name === name })) {
				throw errors.new(`query: interior and recursive share the name ${name}`)
			}
			if (name.length === 0) {
				throw errors.new("query: recursive needs a name")
			}
			const data = collectRec(theory, interiors, name, arms.base, arms.rec)
			return makeQueryStart<Rels, Classes, Flatten<P & BuildsParams<Base> & BuildsParams<Step>>, RecData>(
				theory,
				interiors,
				data
			)
		},
		rule<RV extends AnyRuleValue>(
			build: (r: QueryRuleScope<Rels, Classes>) => RV
		): Query<Rels, RowOf<RV>, Flatten<P & ParamsOf<RV>>, Classes> {
			const built = build(makeQueryRuleScope<Rels, Classes>(theory, env))
			return makeQuery<Rels, RowOf<RV>, Flatten<P & ParamsOf<RV>>, Classes>(
				theory,
				interiors,
				rec,
				[built.rule]
			)
		}
	}
	Object.freeze(start)
	return start as unknown as QueryStart<Rels, Classes, P, Rec>
}

/**
 * Opens a query over a schema: `query(S).rule(r => ...)`, optionally with
 * `interior` / `recursive` first. Each `.rule` adds one conjunctive rule;
 * multiple rules are the set union. The schema's law-computed class map and
 * theory value ride into every rule builder — the join walls compare
 * against the mint slots off it.
 */
function query<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>
): QueryStart<Rels, Classes> {
	return makeQueryStart<Rels, Classes, Record<never, never>, null>(theory, [], null)
}

/**
 * Tags one closed-reference literal: the handle NAME, verified against the
 * roster and translated to its declaration-order row id, tagged u64. THE
 * single roster-verification point of the query surface.
 */
function taggedHandleId(
	context: string,
	closed: { readonly name: string; readonly handles: readonly string[] },
	value: unknown
): TaggedValue {
	if (typeof value !== "string") {
		throw literalShapeError(context, `a ${closed.name} handle name (string)`, value)
	}
	const id = closed.handles.indexOf(value)
	if (id < 0) {
		throw errors.new(
			`${context}: "${value}" is not a handle of ${closed.name} — the roster is ${closed.handles.join(", ")}`
		)
	}
	return { kind: "u64", value: BigInt(id) }
}

/**
 * Tags one literal in an interval element domain: a bigint tags as the
 * element (the membership typing rule's point side), an interval-shaped
 * value as the interval (value equality).
 */
function taggedAtElementDomain(context: string, element: "u64" | "i64", value: unknown): TaggedValue {
	if (typeof value === "bigint") {
		if (element === "u64") {
			return { kind: "u64", value }
		}
		return { kind: "i64", value }
	}
	if (isIntervalValue(value)) {
		if (element === "u64") {
			return { kind: "intervalU64", start: value.start, end: value.end }
		}
		return { kind: "intervalI64", start: value.start, end: value.end }
	}
	throw literalShapeError(context, "bigint (point) or { start, end } (interval)", value)
}

/**
 * Tags one host literal at a FIELD position (atom bindings): the field's
 * structural kind directs the tag, never a guess.
 */
function taggedLiteral(context: string, field: AnyField, value: unknown): TaggedValue {
	const roster = rosterOf(field)
	if (roster !== undefined) {
		return taggedHandleId(context, roster, value)
	}
	switch (field.kind) {
		case "bool": {
			if (typeof value !== "boolean") {
				throw literalShapeError(context, "boolean", value)
			}
			return { kind: "bool", value }
		}
		case "u64": {
			if (typeof value !== "bigint") {
				throw literalShapeError(context, "bigint", value)
			}
			return { kind: "u64", value }
		}
		case "i64": {
			if (typeof value !== "bigint") {
				throw literalShapeError(context, "bigint", value)
			}
			return { kind: "i64", value }
		}
		case "str": {
			if (typeof value !== "string") {
				throw literalShapeError(context, "string", value)
			}
			if (!value.isWellFormed()) {
				throw literalShapeError(context, "well-formed string", value)
			}
			return { kind: "string", value }
		}
		case "bytes": {
			if (!(value instanceof Uint8Array)) {
				throw literalShapeError(context, "Uint8Array", value)
			}
			return { kind: "fixedBytes", value }
		}
		case "interval":
			return taggedAtElementDomain(context, field.element, value)
	}
}

/**
 * Tags one host literal at a COMPARISON or PARAM position, where the SIBLING
 * anchors the type: a measure sibling is u64, an interval-field sibling
 * contributes its element domain, a scalar sibling its own type. At
 * `pointIn` the operand order is interval-left, point-right, so an
 * interval-shaped literal beside a scalar element-typed sibling is the LEGAL
 * interval operand of `pointIn(t, span(...))`; under every other operator an
 * interval shape against a scalar sibling stays refused.
 */
function taggedCmpLiteral(
	context: string,
	sibling: AnyField | "measure",
	value: unknown,
	op: CmpKind | "binding"
): TaggedValue {
	if (sibling === "measure") {
		if (typeof value !== "bigint") {
			throw literalShapeError(context, "bigint (the measure is u64)", value)
		}
		return { kind: "u64", value }
	}
	if (rosterOf(sibling) === undefined && sibling.kind === "interval") {
		return taggedAtElementDomain(context, sibling.element, value)
	}
	if (
		op === "pointIn" &&
		rosterOf(sibling) === undefined &&
		(sibling.kind === "u64" || sibling.kind === "i64") &&
		isIntervalValue(value)
	) {
		return taggedAtElementDomain(context, sibling.kind, value)
	}
	return taggedLiteral(context, sibling, value)
}

/** The shared lowering context of one `lowerQuery` run. */
interface LowerContext {
	readonly theory: AnySchema
	readonly relationIds: ReadonlyMap<string, number>
	readonly interiorIds: ReadonlyMap<string, number>
	readonly paramIds: ReadonlyMap<string, number>
	readonly params: ReadonlyMap<string, ParamEntry>
}

/** One rule's dense variable numbering: first occurrence in written order, keyed on the object REFERENCE. */
interface VarIds {
	of(ref: AnyVar): number
}

/** Creates one rule-scoped variable numberer. */
function freshVarIds(): VarIds {
	const assigned = new Map<AnyVar, number>()
	return {
		of(ref) {
			const existing = assigned.get(ref)
			if (existing !== undefined) {
				return existing
			}
			const id = assigned.size
			assigned.set(ref, id)
			return id
		}
	}
}

/** Resolves a param name to its dense positional id. */
function paramIdOf(ctx: LowerContext, name: string): number {
	const id = ctx.paramIds.get(name)
	if (id === undefined) {
		throw errors.new(`query lowering: param ${name} is not in the query's registry`)
	}
	return id
}

/**
 * Lowers one EDB atom (either polarity). A CLOSED owner lowers through the
 * same edb source, with field ordinals over the SEALED shape.
 */
function lowerAtom(ctx: LowerContext, atom: AtomData, ids: VarIds): AtomIr {
	const member = ctx.theory.relations[atom.relation.name]
	if (member !== atom.relation) {
		throw errors.new(
			`query lowering: relation ${atom.relation.name} is not the relation value schema ${ctx.theory.name} declares`
		)
	}
	const relationId = ctx.relationIds.get(atom.relation.name)
	if (relationId === undefined) {
		throw errors.new(`query lowering: relation ${atom.relation.name} has no ordinal`)
	}
	const ordered = sealedFieldsOf(atom.relation)
	const bindings: Array<readonly [number, TermIr]> = atom.bindings.map(function lowerBinding(binding) {
		const ordinal = ordered.findIndex(function byName(candidate) {
			return candidate.name === binding.field
		})
		if (ordinal < 0) {
			throw errors.new(`query lowering: relation ${atom.relation.name} has no field ${binding.field}`)
		}
		return [ordinal, lowerBindingTerm(ctx, `${atom.relation.name}.${binding.field}`, binding, ids)] as const
	})
	return { source: { kind: "edb", relation: relationId }, bindings }
}

/** Lowers one binding term. A membership ARRAY lowers to the existing param-set term over its content-addressed entry. */
function lowerBindingTerm(ctx: LowerContext, context: string, binding: BindingEntry, ids: VarIds): TermIr {
	const bound = binding.term
	switch (bound.kind) {
		case "var":
			return { kind: "var", var: ids.of(bound.ref) }
		case "param":
			return { kind: "param", param: paramIdOf(ctx, bound.name) }
		case "setParam":
			return { kind: "paramSet", param: paramIdOf(ctx, bound.name) }
		case "literalSet":
			return { kind: "paramSet", param: paramIdOf(ctx, bound.name) }
		case "literal":
			return { kind: "literal", value: taggedLiteral(context, binding.data, bound.value) }
	}
}

/**
 * Lowers one interior atom: named bindings placed by HEAD order, `FieldId(i)` =
 * head position i. Every head column of the table must be bound (a missing key
 * is refused pointed); the var-id assignment order is head order, so the
 * first-use numbering matches the name-keyed edition exactly.
 */
function lowerInteriorAtom(
	ctx: LowerContext,
	target: DerivedTable,
	bindings: ReadonlyArray<{ readonly key: string; readonly ref: AnyVar }>,
	ids: VarIds
): AtomIr {
	const interior = ctx.interiorIds.get(target.name)
	if (interior === undefined) {
		throw errors.new(`query lowering: derived table ${target.name} was not declared on this query`)
	}
	if (target.finds.length === 0) {
		throw errors.new(`query lowering: derived table ${target.name} has no head`)
	}
	const irBindings: Array<readonly [number, TermIr]> = target.finds.map(function lowerPosition(column, position) {
		const binding = bindings.find(function byKey(candidate) {
			return candidate.key === column.name
		})
		if (binding === undefined) {
			throw errors.new(`query lowering: interior ${target.name} omits head column ${column.name}`)
		}
		return [position, { kind: "var", var: ids.of(binding.ref) } as const] as const
	})
	return { source: { kind: "interior", interior }, bindings: irBindings }
}

/** Lowers one comparison side; literals tag by the sibling's anchor (op-aware at `pointIn`). */
function lowerCmpTerm(ctx: LowerContext, side: CmpTermData, sibling: CmpTermData, ids: VarIds, op: CmpKind): TermIr {
	switch (side.kind) {
		case "var":
			return { kind: "var", var: ids.of(side.ref) }
		case "param":
			return { kind: "param", param: paramIdOf(ctx, side.name) }
		case "setParam":
			return { kind: "paramSet", param: paramIdOf(ctx, side.name) }
		case "measure":
			return { kind: "measure", var: ids.of(side.ref) }
		case "literal": {
			const anchor = cmpAnchorOf(ctx, sibling)
			if (anchor === undefined) {
				throw errors.new(
					"query lowering: a comparison literal needs a bound-variable, measure, or anchored-param sibling to type it"
				)
			}
			return { kind: "literal", value: taggedCmpLiteral("comparison literal", anchor, side.value, op) }
		}
	}
}

/** Resolves the anchor a comparison literal tags by: the sibling variable's field, the measure, or an anchored param. */
function cmpAnchorOf(ctx: LowerContext, sibling: CmpTermData): AnyField | "measure" | undefined {
	if (sibling.kind === "var") {
		return sibling.ref.field
	}
	if (sibling.kind === "measure") {
		return "measure"
	}
	if (sibling.kind === "param" || sibling.kind === "setParam") {
		return ctx.params.get(sibling.name)?.anchor
	}
	return undefined
}

/** Lowers one comparison. */
function lowerComparison(ctx: LowerContext, cmp: CmpData, ids: VarIds): ComparisonIr {
	if (cmp.op === "allen") {
		const maskData = cmp.mask
		if (maskData === undefined) {
			throw errors.new("query lowering: an allen comparison lost its mask")
		}
		const mask = maskData.mask
		return {
			op: { kind: "allen", mask },
			lhs: lowerCmpTerm(ctx, cmp.lhs, cmp.rhs, ids, "allen"),
			rhs: lowerCmpTerm(ctx, cmp.rhs, cmp.lhs, ids, "allen")
		}
	}
	return {
		op: { kind: cmp.op },
		lhs: lowerCmpTerm(ctx, cmp.lhs, cmp.rhs, ids, cmp.op),
		rhs: lowerCmpTerm(ctx, cmp.rhs, cmp.lhs, ids, cmp.op)
	}
}

/** Lowers one condition node (comparison leaf or and/or tree). */
function lowerCondition(ctx: LowerContext, cond: CondData, ids: VarIds): ConditionTreeIr {
	if (cond.kind === "cmp") {
		return { kind: "leaf", cmp: lowerComparison(ctx, cond, ids) }
	}
	return {
		kind: cond.op,
		children: cond.children.map(function lowerChild(child) {
			return lowerCondition(ctx, child, ids)
		})
	}
}

/** Lowers one find entry to its per-rule find term. */
function lowerFind(entry: FindEntryData, ids: VarIds): FindTermIr {
	if (entry.kind === "var") {
		return { kind: "var", var: ids.of(entry.over) }
	}
	if (entry.kind === "measure") {
		return { kind: "measure", var: ids.of(entry.over) }
	}
	const agg = entry.agg
	switch (agg.op) {
		case "count":
			return { kind: "aggregate", op: { kind: "count" } }
		case "fold": {
			if ("duration" in agg.over) {
				return { kind: "aggregateMeasure", op: { kind: agg.fold }, over: ids.of(agg.over.duration) }
			}
			return { kind: "aggregate", op: { kind: agg.fold }, over: ids.of(agg.over) }
		}
		case "pack":
			return { kind: "aggregate", op: { kind: "pack" }, over: ids.of(agg.over) }
	}
}

/** One aggregate's var-free head-op kind (`AggOp::head_op`). */
function headOpOf(agg: AggData): HeadOpIr {
	switch (agg.op) {
		case "count":
			return "count"
		case "fold":
			return agg.fold
		case "pack":
			return "pack"
	}
}

/** One find entry's var-free head shape. */
function headTermOf(column: FindColumn): HeadTermIr {
	const entry = column.entry
	if (entry.kind === "var" || entry.kind === "measure") {
		return { kind: "var" }
	}
	return { kind: "aggregate", op: headOpOf(entry.agg) }
}

/** Lowers one rule: body walked in written order (var ids by first occurrence), finds last. */
function lowerRule(ctx: LowerContext, rule: RuleData): RuleIr {
	const ids = freshVarIds()
	const atoms: AtomIr[] = []
	const negated: AtomIr[] = []
	const conditions: ConditionTreeIr[] = []
	for (const item of rule.items) {
		switch (item.kind) {
			case "atom": {
				atoms.push(lowerAtom(ctx, item.atom, ids))
				break
			}
			case "negated": {
				negated.push(lowerAtom(ctx, item.atom, ids))
				break
			}
			case "interior": {
				const bucket = item.negated ? negated : atoms
				bucket.push(lowerInteriorAtom(ctx, item.target, item.bindings, ids))
				break
			}
			case "cond": {
				conditions.push(lowerCondition(ctx, item.cond, ids))
				break
			}
		}
	}
	return {
		finds: rule.finds.map(function findOf(column) {
			return lowerFind(column.entry, ids)
		}),
		atoms,
		negated,
		conditions
	}
}

/**
 * Lowers a query value to the bridge's `QueryIr` — pure and stable: interiors
 * in declaration order, optional rec, then main. Every registered param must
 * carry a field anchor by now.
 */
function lowerQuery(q: AnyQuery): ParsedQuery {
	const theory = q.schema
	const relationIds = new Map<string, number>()
	Object.keys(theory.relations).forEach(function assignOrdinal(name, index) {
		relationIds.set(name, index)
	})
	const interiorIds = new Map<string, number>()
	q.data.interiors.forEach(function assignInteriorId(interior, index) {
		interiorIds.set(interior.name, index)
	})
	if (q.data.rec !== null) {
		interiorIds.set(q.data.rec.name, q.data.interiors.length)
	}
	const paramIds = new Map<string, number>()
	const params = new Map<string, ParamEntry>()
	q.data.params.forEach(function assignParamId(entry, index) {
		if (entry.anchor === undefined) {
			throw errors.new(
				`query param ${entry.name} has no field-anchored use — bind it in an atom or compare it against a bound variable`
			)
		}
		paramIds.set(entry.name, index)
		params.set(entry.name, entry)
	})
	const ctx: LowerContext = { theory, relationIds, interiorIds, paramIds, params }
	return parseQueryIr({
		interiors: q.data.interiors.map(function lowerInterior(interior) {
			return {
				head: interior.finds.map(headTermOf),
				rules: interior.rules.map(function lowerInteriorRule(rule) {
					return lowerRule(ctx, rule)
				})
			}
		}),
		rec:
			q.data.rec === null
				? null
				: {
						head: q.data.rec.finds.map(headTermOf),
						base: q.data.rec.base.map(function lowerBase(rule) {
							return lowerRule(ctx, rule)
						}),
						rec: q.data.rec.rec.map(function lowerRecArm(rule) {
							return lowerRule(ctx, rule)
						})
					},
		head: q.data.finds.map(headTermOf),
		rules: q.data.rules.map(function lowerMainRule(rule) {
			return lowerRule(ctx, rule)
		})
	})
}

export type {
	AnyQuery,
	AnyRuleValue,
	HeadOf,
	HeadShape,
	InteriorRuleChain,
	InteriorRuleScope,
	ParamsOf,
	Query,
	QueryData,
	QueryParams,
	QueryRelation,
	QueryRow,
	QueryRuleChain,
	QueryRuleScope,
	QueryStart,
	RawChain,
	RawScope,
	RecRuleChain,
	RecRuleScope,
	RowOf,
	RuleValue,
	TermOps
}
export { lowerQuery, makeRawScope, query, taggedCmpLiteral, taggedLiteral }
