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
 * and a name-collision join is unrepresentable. Each binding position is
 * judged against the variable's MINT slot — and because {@link JoinOk} is an
 * equality, that alone makes every cross-binding join transitively
 * class-equal. Lowering is a pure function of the query value down to the
 * bridge's `ProgramIr` (`bumbledb/crates/bumbledb/src/ir.rs`): relations by
 * declaration ordinal, variables by dense per-rule first-occurrence ids
 * (keyed on the object REFERENCE — the discipline is unchanged, only the map
 * key moved from name to reference), params by first-use order. Lowering is
 * STABLE — the same query value lowers to deeply-equal IR every time, and
 * two identically-written queries (fresh mints each) lower identically.
 * Construction validates negation safety and boundness (typed by the var's
 * label — object identity is invisible to the type tier, so these are
 * construction-time walls); everything else (strata, types, aggregate
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
	PredicateDefIr,
	ProgramIr,
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
	FindColumn,
	FindEntryData,
	MaskData,
	MatchFields,
	MatchOwner,
	MatchShape,
	ParamUse,
	RecData,
	RuleData,
	RuleItem
} from "#query/atom.ts"
import { allen, and, eq, ge, gt, le, lt, ne, not, or, pointIn } from "#query/atom.ts"
import type { CheckFind, CheckRecFind, FindShape, HeadRecordOf, RowOfFind } from "#query/find.ts"
import { argMax, argMin, count, countDistinct, max, min, pack, sum } from "#query/find.ts"
import type {
	AnyVar,
	ClassedField,
	Flatten,
	InferredOf,
	JoinOk,
	MintSlotOf,
	ParamEntry,
	ParamsRecord
} from "#query/scope.ts"
import {
	fieldJoins,
	inferred,
	isTerm,
	makeDuration,
	makeMaskParam,
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
 * A recursive predicate's HEAD signature as classed slots, keyed by column
 * name — carried on the rec reference so an `idb` join can be judged against
 * it; `undefined` on values that carry no head.
 */
type HeadShape = Readonly<Record<string, ClassedField>> | undefined

/**
 * One finished rule as a plain value: the runtime data plus the inferred
 * row/params carrier (and, for a RECURSIVE rule, the head record of classed
 * slots `idb` pairs against). `.rule(...)` consumes it.
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

/**
 * A recursive predicate REFERENCE — the shape `idb()` targets carry: the
 * name (type-level identity), the runtime data (value identity), the params
 * its rules have used, and the head signature its FIRST rule sealed.
 */
interface RecRef<Name extends string, P extends ParamsRecord, Head extends HeadShape = HeadShape> {
	readonly name: Name
	readonly data: RecData
	readonly [inferred]?: { readonly params: P; readonly head: Head }
}

/** One `idb` position's judgment: a variable class-equal to the head slot when the head is carried. */
type IdbBindingOk<Classes extends SchemaClasses, HeadSlot, V> = V extends AnyVar
	? HeadSlot extends ClassedField
		? JoinOk<HeadSlot, MintSlotOf<Classes, V>> extends true
			? true
			: false
		: true
	: false

/**
 * The validated `idb` bindings record: when the target carries its head
 * (a threaded rec handle), the record's key set must EXACTLY equal the
 * head's (a missing or extra key maps every property to `never`) and each
 * variable must be class-equal to its head slot — the same wall `JoinOk`
 * holds for EDB atoms. An unthreaded handle carries no head; every entry
 * must still be a variable, arity/class judged at construction and prepare.
 */
type CheckIdbBindings<Classes extends SchemaClasses, Head, B> =
	Head extends Readonly<Record<string, ClassedField>>
		? [keyof B] extends [keyof Head]
			? [keyof Head] extends [keyof B]
				? {
						readonly [K in keyof B]: K extends keyof Head
							? IdbBindingOk<Classes, Head[K], B[K]> extends true
								? B[K]
								: never
							: never
					}
				: { readonly [K in keyof B]: never }
			: { readonly [K in keyof B]: never }
		: { readonly [K in keyof B]: B[K] extends AnyVar ? B[K] : never }

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
	/** Names one Allen-mask parameter (`MaskTerm::Param`): a bind-time 13-bit mask number. */
	readonly maskParam: typeof makeMaskParam
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
	readonly countDistinct: typeof countDistinct
	readonly sum: typeof sum
	readonly min: typeof min
	readonly max: typeof max
	readonly argMax: typeof argMax
	readonly argMin: typeof argMin
	readonly pack: typeof pack
}

/** The rule builder a `query(S).rule(...)` callback receives (`Classes` — the join judge's authority). */
interface QueryRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> extends TermOps {
	/** The first EDB atom of the rule: fields bind variables, params, ∈-sets, or bare literals; absence is the wildcard. */
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): QueryRuleChain<Rels, BindParamsShape<MatchFields<R>, B>, Classes>
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
	/** The head projection: a `find` RECORD whose keys name the answer columns. */
	find<const F extends FindShape>(entries: F & CheckFind<F>): RuleValue<RowOfFind<F>, P>
}

/** The rule builder an OUTPUT rule of a `program()` receives: a query rule plus finished-stratum `idb` atoms. */
interface OutputRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> extends TermOps {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): OutputRuleChain<Rels, BindParamsShape<MatchFields<R>, B>, Classes>
}

/** The chain of an output rule: atoms, predicates, `idb` joins over the program's recs, then the head. */
interface OutputRuleChain<
	Rels extends SchemaRelations,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): OutputRuleChain<Rels, Flatten<P & BindParamsShape<MatchFields<R>, B>>, Classes>
	where<const C extends AnyCond>(
		cond: CheckCond<Classes, C> & C
	): OutputRuleChain<Rels, Flatten<P & CondParamsShape<C>>, Classes>
	/**
	 * One `idb` atom over a FINISHED stratum (any rec of this program): a
	 * NAMED join against the rec's head — the bindings record's keys are the
	 * head columns, each bound to a variable positively bound by a relation
	 * atom of the rule. Threading the rec value the last `.rule(...)` returned
	 * carries its params into `Params` AND its head signature, so the join is
	 * key-exact and class-checked against the head at compile time.
	 */
	idb<Target extends RecRef<string, ParamsRecord>, const B extends Readonly<Record<string, AnyVar>>>(
		target: Target,
		bindings: B & CheckIdbBindings<Classes, HeadOf<Target>, B>
	): OutputRuleChain<Rels, Flatten<P & ParamsOf<Target>>, Classes>
	find<const F extends FindShape>(entries: F & CheckFind<F>): RuleValue<RowOfFind<F>, P>
}

/** The rule builder a RECURSIVE rule (`rec.rule(...)`) receives. */
interface RecRuleScope<Rels extends SchemaRelations, Self extends string, Classes extends SchemaClasses = SchemaClasses>
	extends TermOps {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): RecRuleChain<Rels, Self, BindParamsShape<MatchFields<R>, B>, Classes>
}

/**
 * The chain of a recursive rule. Its `idb` accepts ONLY the rec itself (the
 * self-recursion cut) and its `find` takes bound variables only — aggregates
 * and the measure are unrepresentable in a recursive head.
 */
interface RecRuleChain<
	Rels extends SchemaRelations,
	Self extends string,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): RecRuleChain<Rels, Self, Flatten<P & BindParamsShape<MatchFields<R>, B>>, Classes>
	where<const C extends AnyCond>(
		cond: CheckCond<Classes, C> & C
	): RecRuleChain<Rels, Self, Flatten<P & CondParamsShape<C>>, Classes>
	/** The self-recursive atom: `idb(self, { headKey: boundVar })` — only this rec's own reference is accepted. */
	idb<Target extends RecRef<Self, ParamsRecord>, const B extends Readonly<Record<string, AnyVar>>>(
		target: Target,
		bindings: B & CheckIdbBindings<Classes, HeadOf<Target>, B>
	): RecRuleChain<Rels, Self, P, Classes>
	/** The recursive head: a `find` record of bound variables only; the value carries the head's classed slots for `idb` pairing. */
	find<const F extends FindShape>(entries: F & CheckRecFind<F>): RuleValue<RowOfFind<F>, P, HeadRecordOf<Classes, F>>
}

/** A query's runtime description — everything lowering, the wire marshal, and answer decode read. */
interface QueryData {
	/** The program's recursive predicates in declaration order (empty for a plain query); `PredId` = index. */
	readonly recs: readonly RecData[]
	/** The output rules in written order (multiple rules = set union). */
	readonly rules: readonly RuleData[]
	/** The head columns (every rule derives the same head; written order = answer column order). */
	readonly finds: readonly FindColumn[]
	/** The registered params in first-use order across the program walk (= dense `ParamId`s). */
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

/** The entry value of `query(S)`: the first `.rule` mints the query. */
interface QueryStart<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> {
	rule<RV extends AnyRuleValue>(
		build: (r: QueryRuleScope<Rels, Classes>) => RV
	): Query<Rels, RowOf<RV>, ParamsOf<RV>, Classes>
}

/** The frozen constructor vocabulary every rule builder spreads. */
const termOps: TermOps = Object.freeze({
	param: makeParam,
	inSet: makeSetParam,
	maskParam: makeMaskParam,
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
	countDistinct,
	sum,
	min,
	max,
	argMax,
	argMin,
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
				case "maskParam":
					throw errors.new(
						`${label}.${fieldName}: an Allen-mask param is not a field-typed value — masks live in allen() conditions only`
					)
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
function cmpTermDataOf(op: string, value: unknown): CmpTermData {
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
			case "maskParam":
				throw errors.new(`${op}: an Allen-mask param is not a comparison term — masks live in allen()'s mask position`)
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
		const lhs = cmpTermDataOf(cond.op, cond.lhs)
		const rhs = cmpTermDataOf(cond.op, cond.rhs)
		sideUses(cond.op, lhs, rhs, uses)
		sideUses(cond.op, rhs, lhs, uses)
		let mask: MaskData | undefined
		if (cond.op === "allen") {
			const maskValue = cond.mask
			if (typeof maskValue === "number") {
				mask = Object.freeze({ kind: "literal" as const, mask: maskValue })
			} else if (isTerm(maskValue) && maskValue[term] === "maskParam") {
				mask = Object.freeze({ kind: "param" as const, name: maskValue.name })
				uses.push(
					Object.freeze({
						name: maskValue.name,
						shape: "mask" as const,
						anchor: undefined,
						op: "allen" as const,
						members: undefined
					})
				)
			} else {
				throw errors.new("allen: the mask position takes a 13-bit mask number or a maskParam")
			}
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

/** Extends a rule state with one `idb` atom (a named record over head keys; vars validated at completion). */
function advanceIdb(state: RuleBuildState, rec: RecData, bindings: Readonly<Record<string, unknown>>): RuleBuildState {
	const resolved: Array<{ readonly key: string; readonly ref: AnyVar }> = []
	for (const [key, value] of Object.entries(bindings)) {
		if (value === undefined) {
			continue
		}
		if (!isTerm(value) || value[term] !== "var") {
			throw errors.new(
				`idb ${rec.name}: position ${key} takes a variable — bind literals and params through where()/match()`
			)
		}
		resolved.push(Object.freeze({ key, ref: value }))
	}
	return Object.freeze({
		items: Object.freeze([
			...state.items,
			Object.freeze({ kind: "idb" as const, rec, bindings: Object.freeze(resolved) })
		]),
		bound: state.bound,
		paramUses: state.paramUses
	})
}

/** Narrows a find entry to an aggregate value. */
function isAggregateEntry(
	value: unknown
): value is { readonly agg: string; readonly over: unknown; readonly key: unknown } {
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
function aggDataOf(
	name: string,
	entry: { readonly agg: string; readonly over: unknown; readonly key: unknown }
): AggData {
	const over = entry.over
	switch (entry.agg) {
		case "count":
			return Object.freeze({ op: "count" as const })
		case "countDistinct":
			return Object.freeze({ op: "countDistinct" as const, over: asVarTerm(`find ${name} (countDistinct)`, over) })
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
		case "argMax":
		case "argMin":
			return Object.freeze({
				op: "arg" as const,
				direction: entry.agg,
				over: asVarTerm(`find ${name} (${entry.agg})`, over),
				key: asVarTerm(`find ${name} (${entry.agg} key)`, entry.key)
			})
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
 * variable's mint slot, or an Arg-carried payload's. Counts, folds, `pack`
 * and the measure derive numbers/intervals, so they resolve no slot.
 */
function findColumnSlotOf(context: ChainContext, column: FindColumn): ClassedField | undefined {
	const entry = column.entry
	if (entry.kind === "var") {
		return mintSlotOf(context, entry.over)
	}
	if (entry.kind === "aggregate" && entry.agg.op === "arg") {
		return mintSlotOf(context, entry.agg.over)
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
		case "countDistinct":
			assertBound(where, bound, agg.over)
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
		case "arg": {
			assertBound(where, bound, agg.over)
			assertBound(where, bound, agg.key)
			assertNotClosed(where, `the ${agg.direction} key`, agg.key)
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
 * Validates one `idb` item: every head column of the rec is bound exactly
 * once (a missing or extra key is a pointed error), every bound variable is
 * positively bound by a relation atom of the rule, and each variable joins
 * its head column's classed slot. When the rec's own rule 0 is in flight
 * (`rec.rules[0]` absent), the completing rule's OWN find columns ARE the
 * head.
 */
function validateIdb(
	context: ChainContext,
	bound: ReadonlySet<AnyVar>,
	item: { readonly rec: RecData; readonly bindings: ReadonlyArray<{ readonly key: string; readonly ref: AnyVar }> },
	columns: readonly FindColumn[]
): void {
	const label = contextLabel(context)
	const head = item.rec.rules[0]
	const headColumns = head !== undefined ? head.finds : columns
	const headNames = headColumns.map(function nameOf(column) {
		return column.name
	})
	const keys = item.bindings.map(function keyOf(binding) {
		return binding.key
	})
	for (const key of keys) {
		if (!headNames.includes(key)) {
			throw errors.new(
				`${label}: idb ${item.rec.name} binds ${key}, not a head column of ${item.rec.name} (head columns: ${headNames.join(", ")})`
			)
		}
	}
	for (const name of headNames) {
		if (!keys.includes(name)) {
			throw errors.new(
				`${label}: idb ${item.rec.name} omits the head column ${name} — an idb join binds every head column of ${item.rec.name}`
			)
		}
	}
	for (const binding of item.bindings) {
		if (!bound.has(binding.ref)) {
			throw errors.new(
				`${label}: idb ${item.rec.name} names the variable ${binding.ref.label}, but no relation atom of the rule binds it — an idb atom is a join position; bind the variable through the theory's own relation first`
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
				`${label}: idb ${item.rec.name} joins the variable ${binding.ref.label} (${renderFieldKind(mint)}) at head column ${binding.key} (${renderFieldKind(headColumn.slot)}) — a var joins only class-equal slots; bare pairs only with bare`
			)
		}
	}
}

/**
 * Completes one rule: enriches the find columns (declaration-order-safe
 * keys, boundness validated, each column's classed slot and closed slice
 * resolved), then walks the body walls — negated-atom boundness safety, idb
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
		if (item.kind === "idb") {
			validateIdb(context, state.bound, item, columns)
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
	idb(target: RecRef<string, ParamsRecord>, bindings: Readonly<Record<string, unknown>>): RawChain
	find(entries: Readonly<Record<string, unknown>>): RuleValue<never, never>
}

/** The runtime rule-builder shape beneath every typed scope. */
interface RawScope extends TermOps {
	match(relation: MatchOwner, bindings: Readonly<Record<string, unknown>>): RawChain
}

/** Which rule family a chain builds — plus the schema's runtime class map and theory value (the join judge's authority). */
type ChainContext = { readonly classes: SchemaClasses; readonly theory: AnySchema } & (
	| { readonly kind: "query" }
	| { readonly kind: "rec"; readonly self: RecData }
	| { readonly kind: "output"; readonly program: ProgramState }
)

/** The diagnostic label of a chain context. */
function contextLabel(context: ChainContext): string {
	switch (context.kind) {
		case "query":
			return "query rule"
		case "rec":
			return `rec ${context.self.name} rule`
		case "output":
			return "program output rule"
	}
}

/** Validates and records one `idb` atom per the context's cut. */
function idbAdvance(
	context: ChainContext,
	state: RuleBuildState,
	target: RecRef<string, ParamsRecord>,
	bindings: Readonly<Record<string, unknown>>
): RuleBuildState {
	if (context.kind === "query") {
		throw errors.new("idb is a program construct — declare recs and outputs through program(), never a plain query()")
	}
	if (context.kind === "rec") {
		if (target.data !== context.self) {
			throw errors.new(
				`rec ${context.self.name}: a recursive rule's idb target must be the rec itself — the self-recursion-only cut (mutual recursion is unwritable; fold a finished stratum in the output rules)`
			)
		}
		return advanceIdb(state, context.self, bindings)
	}
	if (!context.program.recs.includes(target.data)) {
		throw errors.new(
			`idb ${target.name}: the rec was declared by a different program — rec identity is the membership rule`
		)
	}
	return advanceIdb(state, target.data, bindings)
}

/** Classifies one find record per the context (a recursive head projects bound variables only). */
function findColumns(context: ChainContext, entries: Readonly<Record<string, unknown>>): FindColumn[] {
	const columns: FindColumn[] = []
	for (const [name, entry] of Object.entries(entries)) {
		if (entry === undefined) {
			continue
		}
		if (context.kind === "rec" && !(isTerm(entry) && entry[term] === "var")) {
			throw errors.new(
				`rec ${context.self.name}: a recursive head projects bound variables only — aggregates and the measure read finished sets (the strata judge's quarantine, unwritable here)`
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
		idb(target, bindings) {
			return makeRawChain(context, idbAdvance(context, state, target, bindings))
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
	theory: AnySchema
): QueryRuleScope<Rels, Classes> {
	const raw = makeRawScope({ kind: "query", classes: theory.classes, theory })
	if (!isTypedScope<QueryRuleScope<Rels, Classes>>(raw)) {
		throw errors.new("query rule builder construction incomplete")
	}
	return raw
}

/** Builds one output-rule builder over a program's recs. */
function makeOutputRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	program: ProgramState
): OutputRuleScope<Rels, Classes> {
	const raw = makeRawScope({ kind: "output", program, classes: program.classes, theory: program.theory })
	if (!isTypedScope<OutputRuleScope<Rels, Classes>>(raw)) {
		throw errors.new("program output rule builder construction incomplete")
	}
	return raw
}

/** One program's build-time registry: its recs in declaration order, the theory value, and its class map. */
interface ProgramState {
	readonly recs: RecData[]
	readonly classes: SchemaClasses
	readonly theory: AnySchema
	sealed: boolean
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
	if (agg.op === "arg") {
		return `${column.name}:${agg.direction}`
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
 * Folds every rule's param uses (recs in declaration order first, output
 * rules last — exactly the lowering walk) into the query's registry: first
 * use mints the dense `ParamId`, the first FIELD-ANCHORED use types the
 * wire, and one name keeps one shape AND one closedness.
 */
function paramRegistryOf(recs: readonly RecData[], rules: readonly RuleData[]): readonly ParamEntry[] {
	const order: string[] = []
	const byName = new Map<
		string,
		{
			shape: ParamEntry["shape"]
			anchor: ParamEntry["anchor"]
			op: ParamEntry["op"]
			members: readonly string[] | undefined
			orderOp: "lt" | "le" | "gt" | "ge" | "pointIn" | undefined
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
					members: use.members,
					orderOp: isOrderOp(use.op) ? use.op : undefined
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
			if (existing.orderOp === undefined && isOrderOp(use.op)) {
				existing.orderOp = use.op
			}
		}
	}
	for (const rec of recs) {
		for (const rule of rec.rules) {
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
			const anchorRoster = anchorRosterOf(entry.anchor)
			if (entry.orderOp !== undefined && anchorRoster !== undefined) {
				throw closedOrderError(`query param ${name}`, `its ${entry.orderOp} use's anchor`, anchorRoster.name)
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
}

/**
 * Assembles the runtime query value over completed rules: every rule must
 * derive the SAME head (name and aggregate shape, position for position —
 * the decode labels and the engine's alignment rule agree), and the param
 * registry folds in program-walk order.
 */
function makeRawQuery(theory: AnySchema, recs: readonly RecData[], rules: readonly RuleData[]): RawQuery {
	const first = rules[0]
	if (first === undefined) {
		throw errors.new("a query needs at least one rule")
	}
	const signature = first.finds.map(headSignature).join(", ")
	rules.forEach(function verifyHead(rule, index) {
		const candidate = rule.finds.map(headSignature).join(", ")
		if (candidate !== signature) {
			throw errors.new(
				`every rule of a query derives the same head — rule 0 finds (${signature}), rule ${index} finds (${candidate})`
			)
		}
		// The closed slice is part of the head too: one answer column decodes
		// through one roster, so a union whose rules bind a column at different
		// vocabularies (or one closed, one bare) is refused pointed.
		rule.finds.forEach(function verifyClosedSlice(column, position) {
			const lead = first.finds[position]
			if (lead !== undefined && column.closed !== lead.closed) {
				throw errors.new(
					`every rule of a query derives the same head — the answer column ${lead.name} is ${renderClosedSlice(lead.closed)} in rule 0 but ${renderClosedSlice(column.closed)} in rule ${index} (one column decodes through one roster)`
				)
			}
			// The law-class wall on the union head: one answer column is one
			// value space, so the classed mint slot each rule binds the column
			// at must join across rules — the SAME fieldJoins judgment every
			// join/eq/negated-atom position enforces (the SDK holds it because
			// the wire IR carries no domains).
			if (lead === undefined) {
				return
			}
			if (lead.slot !== undefined && column.slot !== undefined && !fieldJoins(lead.slot, column.slot)) {
				throw errors.new(
					`every rule of a query derives the same head — the answer column ${lead.name} unions domain-unequal fields: bound at ${renderFieldKind(lead.slot)} in rule 0 but at ${renderFieldKind(column.slot)} in rule ${index} (a union column joins only class-equal slots; bare pairs only with bare)`
				)
			}
		})
	})
	const data: QueryData = Object.freeze({
		recs: Object.freeze([...recs]),
		rules: Object.freeze([...rules]),
		finds: first.finds,
		params: paramRegistryOf(recs, rules)
	})
	const value: RawQuery = {
		schema: theory,
		data,
		rule(build) {
			const built = build(makeRawScope({ kind: "query", classes: theory.classes, theory }))
			return makeRawQuery(theory, recs, [...rules, built.rule])
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
	recs: readonly RecData[],
	rules: readonly RuleData[]
): Query<Rels, Row, P, Classes> {
	const raw = makeRawQuery(theory, recs, rules)
	if (!isQueryValue<Rels, Row, P, Classes>(theory, raw)) {
		throw errors.new("query value construction incomplete")
	}
	return raw
}

/**
 * Opens a query over a schema: `query(S).rule(r => ...)`. Each `.rule` adds
 * one conjunctive rule; multiple rules are the set union. The schema's
 * law-computed class map and theory value ride into every rule builder — the
 * join walls compare against the mint slots off it.
 */
function query<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>
): QueryStart<Rels, Classes> {
	const start: QueryStart<Rels, Classes> = {
		rule<RV extends AnyRuleValue>(
			build: (r: QueryRuleScope<Rels, Classes>) => RV
		): Query<Rels, RowOf<RV>, ParamsOf<RV>, Classes> {
			const built = build(makeQueryRuleScope<Rels, Classes>(theory))
			return makeQuery<Rels, RowOf<RV>, ParamsOf<RV>, Classes>(theory, [], [built.rule])
		}
	}
	Object.freeze(start)
	return start
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
	readonly recIds: ReadonlyMap<RecData, number>
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
 * Lowers one idb atom: named bindings placed by HEAD order, `FieldId(i)` =
 * head position i. Every head column of the rec must be bound (a missing key
 * is refused pointed); the var-id assignment order is head order, so the
 * first-use numbering matches the name-keyed edition exactly.
 */
function lowerIdbAtom(
	ctx: LowerContext,
	rec: RecData,
	bindings: ReadonlyArray<{ readonly key: string; readonly ref: AnyVar }>,
	ids: VarIds
): AtomIr {
	const pred = ctx.recIds.get(rec)
	if (pred === undefined) {
		throw errors.new(`query lowering: rec ${rec.name} was declared by a different program`)
	}
	const head = rec.rules[0]
	if (head === undefined) {
		throw errors.new(`query lowering: rec ${rec.name} has no rules`)
	}
	const irBindings: Array<readonly [number, TermIr]> = head.finds.map(function lowerPosition(column, position) {
		const binding = bindings.find(function byKey(candidate) {
			return candidate.key === column.name
		})
		if (binding === undefined) {
			throw errors.new(`query lowering: idb ${rec.name} omits head column ${column.name}`)
		}
		return [position, { kind: "var", var: ids.of(binding.ref) } as const] as const
	})
	return { source: { kind: "idb", pred }, bindings: irBindings }
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
		const mask =
			maskData.kind === "literal"
				? { kind: "literal" as const, mask: maskData.mask }
				: { kind: "param" as const, param: paramIdOf(ctx, maskData.name) }
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
		case "countDistinct":
			return { kind: "aggregate", op: { kind: "countDistinct" }, over: ids.of(agg.over) }
		case "fold": {
			if ("duration" in agg.over) {
				return { kind: "aggregateMeasure", op: { kind: agg.fold }, over: ids.of(agg.over.duration) }
			}
			return { kind: "aggregate", op: { kind: agg.fold }, over: ids.of(agg.over) }
		}
		case "arg":
			return { kind: "aggregate", op: { kind: agg.direction, key: ids.of(agg.key) }, over: ids.of(agg.over) }
		case "pack":
			return { kind: "aggregate", op: { kind: "pack" }, over: ids.of(agg.over) }
	}
}

/** One aggregate's var-free head-op kind (`AggOp::head_op`). */
function headOpOf(agg: AggData): HeadOpIr {
	switch (agg.op) {
		case "count":
			return "count"
		case "countDistinct":
			return "countDistinct"
		case "fold":
			return agg.fold
		case "arg":
			return agg.direction
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
			case "idb": {
				atoms.push(lowerIdbAtom(ctx, item.rec, item.bindings, ids))
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
 * Lowers a query value to the bridge's `ProgramIr` — pure and stable: the
 * recs in declaration order (`PredId` = index), the output predicate
 * appended last. Every registered param must carry a field anchor by now.
 */
function lowerQuery(q: AnyQuery): ProgramIr {
	const theory = q.schema
	const relationIds = new Map<string, number>()
	Object.keys(theory.relations).forEach(function assignOrdinal(name, index) {
		relationIds.set(name, index)
	})
	const recIds = new Map<RecData, number>()
	q.data.recs.forEach(function assignPredId(rec, index) {
		recIds.set(rec, index)
	})
	const paramIds = new Map<string, number>()
	const params = new Map<string, ParamEntry>()
	q.data.params.forEach(function assignParamId(entry, index) {
		if (entry.anchor === undefined && entry.shape !== "mask") {
			throw errors.new(
				`query param ${entry.name} has no field-anchored use — bind it in an atom or compare it against a bound variable`
			)
		}
		paramIds.set(entry.name, index)
		params.set(entry.name, entry)
	})
	const ctx: LowerContext = { theory, relationIds, recIds, paramIds, params }
	const predicates: PredicateDefIr[] = q.data.recs.map(function lowerRec(rec) {
		const head = rec.rules[0]
		if (head === undefined) {
			throw errors.new(`query lowering: rec ${rec.name} has no rules`)
		}
		return {
			head: head.finds.map(headTermOf),
			rules: rec.rules.map(function lowerRecRule(rule) {
				return lowerRule(ctx, rule)
			})
		}
	})
	predicates.push({
		head: q.data.finds.map(headTermOf),
		rules: q.data.rules.map(function lowerOutputRule(rule) {
			return lowerRule(ctx, rule)
		})
	})
	return { predicates, output: q.data.recs.length }
}

export type {
	AnyQuery,
	AnyRuleValue,
	HeadOf,
	HeadShape,
	OutputRuleChain,
	OutputRuleScope,
	ParamsOf,
	ProgramState,
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
	RecRef,
	RecRuleChain,
	RecRuleScope,
	RowOf,
	RuleValue,
	TermOps
}
export { lowerQuery, makeOutputRuleScope, makeQuery, makeRawScope, query, taggedCmpLiteral, taggedLiteral }
