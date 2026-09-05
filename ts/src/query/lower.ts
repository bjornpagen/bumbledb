import { sealedFieldsOf } from "#closed.ts"
import { AuthoringError, SdkInvariantError } from "#errors.ts"
import type { AnyClosedRoster, AnyField } from "#fields.ts"
import {
	assertDeclarationOrderKey,
	f64 as f64Field,
	isFloatIntervalValue,
	isIntervalValue,
	literalShapeError,
	rosterOf,
	u64 as u64Field
} from "#fields.ts"
import { Id128 } from "#id128.ts"
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
	ScalarExprIr,
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
import type { QueryNode } from "#query/compute.ts"
import { computeFieldOf, computeVarsOf, isComputeExpr, MAX_COMPUTE_DEPTH } from "#query/compute.ts"
import { literalWireOf } from "#scalar.ts"
import type { CheckFind, CheckRecFind, FindShape, HeadRecordOf, RowOfFind } from "#query/find.ts"
import { count, max, mean, min, pack, sum } from "#query/find.ts"
import { parseQueryIr } from "#query/parse-ir.ts"
import type {
	AnyVar,
	ClassedField,
	ExactVars,
	Flatten,
	InferredOf,
	ParamEntry,
	ParamsRecord,
	ShapeOf,
	VarsOf
} from "#query/scope.ts"
import {
	fieldAntiJoins,
	fieldJoins,
	headFieldJoins,
	importFacadeOf,
	importFacadeOfOwner,
	inferred,
	isImportedSource,
	isTerm,
	labelImport,
	makeParam,
	makeSetParam,
	renderFieldKind,
	term
} from "#query/scope.ts"
import type { AnySchema, Schema, SchemaRelations } from "#schema.ts"

type QueryRelation<Rels extends SchemaRelations> = Extract<Rels[keyof Rels], MatchOwner>

type ParamsOf<T> = InferredOf<T> extends { readonly params: infer P extends ParamsRecord } ? P : Record<never, never>

type RowOf<T> = InferredOf<T> extends { readonly row: infer R } ? R : never

type HeadShape = Readonly<Record<string, ClassedField>> | undefined

interface RuleValue<Row, P extends ParamsRecord, Head extends HeadShape = undefined> {
	readonly rule: RuleData
	readonly [inferred]?: { readonly row: Row; readonly params: P; readonly head: Head }
}

type AnyRuleValue = RuleValue<unknown, ParamsRecord, HeadShape>

type InteriorBindingOk<V> = V extends AnyVar ? true : false

type CheckInteriorBindings<B> = {
	readonly [K in keyof B]: InteriorBindingOk<B[K]> extends true ? B[K] : never
}

type InteriorBuild<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> = (
	r: InteriorRuleScope<Rels, Classes>
) => AnyRuleValue

type RecBuild<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> = (
	r: RecRuleScope<Rels, Classes>
) => AnyRuleValue

type BuiltRule<F> = F extends (r: never) => infer RV ? RV : never

type BuildsParams<Builds extends readonly ((r: never) => AnyRuleValue)[]> = ShapeOf<ParamsOf<BuiltRule<Builds[number]>>>

interface TermOps {
	readonly param: typeof makeParam

	readonly inSet: typeof makeSetParam
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
	readonly mean: typeof mean
	readonly min: typeof min
	readonly max: typeof max
	readonly pack: typeof pack
}

interface QueryRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> extends TermOps {
	/**
	 * The FULL binding: every column of R bound to its own v(R) mint — the
	 * identity atom, stated as a signature so it holds for GENERIC R too
	 * (VarsOf unifies with itself by identity; the general form's deferred
	 * conditionals cannot). The mint invariant — a variable's mint slot IS its
	 * position slot, same owner, same column — discharges the join judgment by
	 * construction; an all-var record contributes no params, so the chain starts
	 * paramless. {@link ExactVars} maps a foreign key to `never`, so an
	 * aliased extra-key record falls to the general form's judgment.
	 */
	match<R extends QueryRelation<Rels>, B extends VarsOf<R>>(
		relation: R,
		bindings: B & ExactVars<R, B>
	): QueryRuleChain<Rels, Record<never, never>, Classes>

	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): QueryRuleChain<Rels, BindParamsShape<MatchFields<R>, B>, Classes>

	/**
	 * Nonrecursive composition (chapter 34): a typed query template of the
	 * SAME schema is a relation expression — its whole body splices as a
	 * derived stage; every head column must be bound to a variable minted
	 * by `v(imported)`. Naming materializes nothing.
	 */
	match<Q extends AnyQuery, const B extends ImportMatchShape<Q>>(
		imported: Q,
		bindings: B
	): QueryRuleChain<Rels, Record<never, never>, Classes>

	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): QueryRuleChain<Rels, Record<never, never>, Classes>
}

type ImportMatchShape<Q extends AnyQuery> = Readonly<Record<keyof RowOf<Q> & string, AnyVar>>

interface QueryRuleChain<
	Rels extends SchemaRelations,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	/**
	 * The FULL binding: every column of R bound to its own v(R) mint — the
	 * identity atom, generic R included. The mint invariant (a variable's mint
	 * slot IS its position slot) discharges the join judgment by construction;
	 * an all-var record contributes no params — P rides through unchanged.
	 * {@link ExactVars} maps a foreign key to `never`, so an aliased
	 * extra-key record falls to the general form's judgment.
	 */
	match<R extends QueryRelation<Rels>, B extends VarsOf<R>>(
		relation: R,
		bindings: B & ExactVars<R, B>
	): QueryRuleChain<Rels, P, Classes>

	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): QueryRuleChain<Rels, Flatten<P & BindParamsShape<MatchFields<R>, B>>, Classes>

	match<Q extends AnyQuery, const B extends ImportMatchShape<Q>>(imported: Q, bindings: B): QueryRuleChain<Rels, P, Classes>

	where<const C extends AnyCond>(
		cond: CheckCond<Classes, C> & C
	): QueryRuleChain<Rels, Flatten<P & CondParamsShape<C>>, Classes>

	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): QueryRuleChain<Rels, P, Classes>

	find<const F extends FindShape>(entries: F & CheckFind<F>): RuleValue<RowOfFind<F>, P>
}

interface InteriorRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses>
	extends TermOps {
	/**
	 * The FULL binding: every column of R bound to its own v(R) mint — the
	 * identity atom, generic R included. The mint invariant (a variable's mint
	 * slot IS its position slot) discharges the join judgment by construction;
	 * an all-var record contributes no params, so the chain starts paramless.
	 * {@link ExactVars} maps a foreign key to `never`, so an aliased
	 * extra-key record falls to the general form's judgment.
	 */
	match<R extends QueryRelation<Rels>, B extends VarsOf<R>>(
		relation: R,
		bindings: B & ExactVars<R, B>
	): InteriorRuleChain<Rels, Record<never, never>, Classes>
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): InteriorRuleChain<Rels, BindParamsShape<MatchFields<R>, B>, Classes>
	match<Q extends AnyQuery, const B extends ImportMatchShape<Q>>(
		imported: Q,
		bindings: B
	): InteriorRuleChain<Rels, Record<never, never>, Classes>
	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): InteriorRuleChain<Rels, Record<never, never>, Classes>
}

interface InteriorRuleChain<
	Rels extends SchemaRelations,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	/**
	 * The FULL binding: every column of R bound to its own v(R) mint — the
	 * identity atom, generic R included. The mint invariant (a variable's mint
	 * slot IS its position slot) discharges the join judgment by construction;
	 * an all-var record contributes no params — P rides through unchanged.
	 * {@link ExactVars} maps a foreign key to `never`, so an aliased
	 * extra-key record falls to the general form's judgment.
	 */
	match<R extends QueryRelation<Rels>, B extends VarsOf<R>>(
		relation: R,
		bindings: B & ExactVars<R, B>
	): InteriorRuleChain<Rels, P, Classes>
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): InteriorRuleChain<Rels, Flatten<P & BindParamsShape<MatchFields<R>, B>>, Classes>
	match<Q extends AnyQuery, const B extends ImportMatchShape<Q>>(
		imported: Q,
		bindings: B
	): InteriorRuleChain<Rels, P, Classes>
	where<const C extends AnyCond>(
		cond: CheckCond<Classes, C> & C
	): InteriorRuleChain<Rels, Flatten<P & CondParamsShape<C>>, Classes>
	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): InteriorRuleChain<Rels, P, Classes>
	/**
	 * Nonrecursive derived stages emit aggregate/computed outputs too
	 * (C05): only the RECURSIVE head stays projection-only.
	 */
	find<const F extends FindShape>(entries: F & CheckFind<F>): RuleValue<RowOfFind<F>, P, HeadRecordOf<Classes, F>>
}

interface RecRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> extends TermOps {
	/**
	 * The FULL binding: every column of R bound to its own v(R) mint — the
	 * identity atom, generic R included. The mint invariant (a variable's mint
	 * slot IS its position slot) discharges the join judgment by construction;
	 * an all-var record contributes no params, so the chain starts paramless.
	 * {@link ExactVars} maps a foreign key to `never`, so an aliased
	 * extra-key record falls to the general form's judgment.
	 */
	match<R extends QueryRelation<Rels>, B extends VarsOf<R>>(
		relation: R,
		bindings: B & ExactVars<R, B>
	): RecRuleChain<Rels, Record<never, never>, Classes>
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): RecRuleChain<Rels, BindParamsShape<MatchFields<R>, B>, Classes>
	/** Frozen finite nonrecursive imports may feed base/step (chapter 34). */
	match<Q extends AnyQuery, const B extends ImportMatchShape<Q>>(
		imported: Q,
		bindings: B
	): RecRuleChain<Rels, Record<never, never>, Classes>
	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): RecRuleChain<Rels, Record<never, never>, Classes>
}

interface RecRuleChain<
	Rels extends SchemaRelations,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	/**
	 * The FULL binding: every column of R bound to its own v(R) mint — the
	 * identity atom, generic R included. The mint invariant (a variable's mint
	 * slot IS its position slot) discharges the join judgment by construction;
	 * an all-var record contributes no params — P rides through unchanged.
	 * {@link ExactVars} maps a foreign key to `never`, so an aliased
	 * extra-key record falls to the general form's judgment.
	 */
	match<R extends QueryRelation<Rels>, B extends VarsOf<R>>(
		relation: R,
		bindings: B & ExactVars<R, B>
	): RecRuleChain<Rels, P, Classes>
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Classes, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): RecRuleChain<Rels, Flatten<P & BindParamsShape<MatchFields<R>, B>>, Classes>
	match<Q extends AnyQuery, const B extends ImportMatchShape<Q>>(imported: Q, bindings: B): RecRuleChain<Rels, P, Classes>
	where<const C extends AnyCond>(
		cond: CheckCond<Classes, C> & C
	): RecRuleChain<Rels, Flatten<P & CondParamsShape<C>>, Classes>
	interior<const B extends Readonly<Record<string, AnyVar>>>(
		name: string,
		bindings: B & CheckInteriorBindings<B>
	): RecRuleChain<Rels, P, Classes>
	find<const F extends FindShape>(entries: F & CheckRecFind<F>): RuleValue<RowOfFind<F>, P, HeadRecordOf<Classes, F>>
}

type QueryData =
	| {
			readonly kind: "cq"

			readonly interiors: readonly InteriorData[]

			readonly rules: readonly RuleData[]

			readonly finds: readonly FindColumn[]

			readonly params: readonly ParamEntry[]
	  }
	| {
			readonly kind: "reach"

			readonly interiors: readonly InteriorData[]

			readonly rec: RecData

			readonly rules: readonly RuleData[]

			readonly finds: readonly FindColumn[]

			readonly params: readonly ParamEntry[]
	  }

interface Query<
	Rels extends SchemaRelations,
	Row,
	Params extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	readonly schema: Schema<Rels, Classes>
	readonly data: QueryData

	rule<RV extends AnyRuleValue>(
		build: (r: QueryRuleScope<Rels, Classes>) => RV
	): Query<Rels, Row | RowOf<RV>, Flatten<Params & ParamsOf<RV>>, Classes>

	/** A diagnostic name for composition/tracing — never a schema relation. */
	named(label: string): Query<Rels, Row, Params, Classes>

	interior(name: string, ...builds: never[]): never

	reach(name: string, arms: never): never
	readonly [inferred]?: { readonly row: Row; readonly params: Params }
}

interface AnyQuery {
	readonly schema: AnySchema
	readonly data: QueryData
}

type QueryRow<Q extends AnyQuery> = RowOf<Q>

type QueryParams<Q extends AnyQuery> = ParamsOf<Q>

type QueryStart<
	Rels extends SchemaRelations,
	Classes extends SchemaClasses = SchemaClasses,
	P extends ParamsRecord = Record<never, never>
> = {
	rule<RV extends AnyRuleValue>(
		build: (r: QueryRuleScope<Rels, Classes>) => RV
	): Query<Rels, RowOf<RV>, Flatten<P & ParamsOf<RV>>, Classes>
	interior<const Builds extends readonly InteriorBuild<Rels, Classes>[]>(
		name: string,
		...builds: Builds
	): QueryStart<Rels, Classes, Flatten<P & BuildsParams<Builds>>>
	reach<const Base extends readonly RecBuild<Rels, Classes>[], const Step extends readonly RecBuild<Rels, Classes>[]>(
		name: string,
		arms: { readonly base: Base; readonly rec: Step }
	): QueryReachStart<Rels, Classes, Flatten<P & BuildsParams<Base> & BuildsParams<Step>>>
}

type QueryReachStart<
	Rels extends SchemaRelations,
	Classes extends SchemaClasses = SchemaClasses,
	P extends ParamsRecord = Record<never, never>
> = {
	rule<RV extends AnyRuleValue>(
		build: (r: QueryRuleScope<Rels, Classes>) => RV
	): Query<Rels, RowOf<RV>, Flatten<P & ParamsOf<RV>>, Classes>
}

const termOps: TermOps = Object.freeze({
	param: makeParam,
	inSet: makeSetParam,
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
	mean,
	min,
	max,
	pack
})

interface RuleBuildState {
	readonly items: readonly RuleItem[]
	readonly bound: ReadonlySet<AnyVar>
	readonly paramUses: readonly ParamUse[]
}

const EMPTY_RULE: RuleBuildState = Object.freeze({
	items: Object.freeze([]),
	bound: new Set<AnyVar>(),
	paramUses: Object.freeze([])
})

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
	const facade = importFacadeOfOwner(ref.owner)
	if (facade !== undefined) {
		// An import var's mint slot is the imported head column's slot:
		// the descriptor it projected and the carrier class that survived
		// the projection (aggregate outputs are bare derived scalars).
		const column = facade.columns.get(ref.column)
		if (column === undefined || (facade.source.schema as AnySchema) !== context.theory) {
			throw new AuthoringError({
				message: `the variable ${ref.label} was minted from a query template schema ${context.theory.name} does not own`
			})
		}
		return column.slot ?? { field: ref.field, class: undefined }
	}
	if (context.theory.relations[ref.owner.name] !== ref.owner) {
		throw new AuthoringError({
			message: `the variable ${ref.label} was minted from a relation schema ${context.theory.name} does not declare — mint variables with v() from the schema's own relations`
		})
	}
	return { field: ref.field, class: context.classes[ref.owner.name]?.[ref.column] }
}

function membershipSet(
	context: string,
	field: AnyField,
	value: readonly unknown[]
): { readonly name: string; readonly members: readonly string[] } {
	const roster = rosterOf(field)
	if (roster === undefined) {
		throw new AuthoringError({
			message: `${context}: a membership array is the closed-reference spelling — ordinary field membership is a bound ∈-set param (r.inSet)`
		})
	}
	if (value.length === 0) {
		throw new AuthoringError({
			message: `${context}: an empty membership array selects nothing — write the query you mean`
		})
	}
	if (value.length === 1) {
		throw new AuthoringError({
			message: `${context}: a one-element membership array is the bare literal respelled — write the literal (the canonical-utterance law: one meaning, one spelling)`
		})
	}
	const seen = new Set<string>()
	const members = value.map(function memberName(member) {
		if (typeof member !== "string") {
			throw literalShapeError(context, `a ${roster.name} handle name (string)`, member)
		}
		if (seen.has(member)) {
			throw new AuthoringError({
				message: `${context}: the membership array spells ${member} twice — write it once (the canonical-utterance law: one meaning, one spelling)`
			})
		}
		seen.add(member)
		return member
	})
	const key = [...members].sort()
	return { name: `∈ ${roster.name} ${JSON.stringify(key)}`, members: Object.freeze(members) }
}

function resolveBindings(
	context: ChainContext,
	label: string,
	relation: MatchOwner,
	bindings: Readonly<Record<string, unknown>>,
	joins: (a: ClassedField, b: ClassedField) => boolean = fieldJoins
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
			throw new AuthoringError({ message: `${label} has no field ${fieldName}` })
		}
		const fieldClass = relationClasses?.[fieldName]
		let bound: BindingEntry["term"]
		if (isTerm(value)) {
			switch (value[term]) {
				case "var": {
					const ref = value
					const mint = mintSlotOf(context, ref)
					const positionSlot: ClassedField = { field: declared.field, class: fieldClass }
					if (!joins(mint, positionSlot)) {
						throw new AuthoringError({
							message: `${label}: the variable ${ref.label} joins domain-unequal fields — minted at ${renderFieldKind(mint)}, reused at ${renderFieldKind(positionSlot)} (a var joins only class-equal slots; bare pairs only with bare)`
						})
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

function cmpTermDataOf(value: unknown): CmpTermData {
	if (isTerm(value)) {
		switch (value[term]) {
			case "var":
				return Object.freeze({ kind: "var" as const, ref: value })
			case "param":
				return Object.freeze({ kind: "param" as const, name: value.name })
			case "setParam":
				return Object.freeze({ kind: "setParam" as const, name: value.name })
		}
	}
	return Object.freeze({ kind: "literal" as const, value })
}

function sideUses(op: CmpKind, side: CmpTermData, sibling: CmpTermData, uses: ParamUse[]): void {
	if (side.kind !== "param" && side.kind !== "setParam") {
		return
	}
	const anchor = sibling.kind === "var" ? sibling.ref.field : undefined
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

function condDataOf(cond: AnyCond, uses: ParamUse[]): CondData {
	if (cond.cond === "cmp") {
		const lhs = cmpTermDataOf(cond.lhs)
		const rhs = cmpTermDataOf(cond.rhs)
		sideUses(cond.op, lhs, rhs, uses)
		sideUses(cond.op, rhs, lhs, uses)
		if (cond.op === "allen") {
			const maskValue = cond.mask
			if (typeof maskValue !== "number") {
				throw new AuthoringError({
					message: "allen: the mask position takes a 13-bit mask number built from the ALLEN constants"
				})
			}
			return Object.freeze({
				kind: "cmp" as const,
				op: { kind: "allen" as const, mask: maskValue },
				lhs,
				rhs
			})
		}
		return Object.freeze({ kind: "cmp" as const, op: { kind: cond.op }, lhs, rhs })
	}
	if (cond.cond === "tree") {
		const children = cond.children.map(function lowerChild(child) {
			return condDataOf(child, uses)
		})
		return Object.freeze({ kind: "tree" as const, op: cond.op, children: Object.freeze(children) })
	}
	throw new AuthoringError({
		message: "a negated atom is not a condition-tree node — pass not(...) to where() directly, never inside and()/or()"
	})
}

function advanceWhere(context: ChainContext, state: RuleBuildState, cond: AnyCond): RuleBuildState {
	if (typeof cond !== "object" || cond === null || !("cond" in cond)) {
		throw new AuthoringError({ message: "where() takes a comparison, an and()/or() tree, or a negated atom" })
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
		const resolved = resolveBindings(context, `negated relation ${relation.name}`, relation, bindings, fieldAntiJoins)
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

function advanceInterior(
	state: RuleBuildState,
	target: DerivedTable,
	bindings: Readonly<Record<string, unknown>>,
	kind: "interior" | "negatedInterior"
): RuleBuildState {
	const resolved: Array<{ readonly key: string; readonly ref: AnyVar }> = []
	for (const [key, value] of Object.entries(bindings)) {
		if (value === undefined) {
			continue
		}
		if (!isTerm(value) || value[term] !== "var") {
			throw new AuthoringError({
				message: `interior ${target.name}: position ${key} takes a variable — bind literals and params through where()/match()`
			})
		}
		resolved.push(Object.freeze({ key, ref: value }))
	}
	const bound = new Set(state.bound)
	if (kind === "interior") {
		for (const binding of resolved) {
			bound.add(binding.ref)
		}
	}
	return Object.freeze({
		items: Object.freeze([...state.items, Object.freeze({ kind, target, bindings: Object.freeze(resolved) })]),
		bound,
		paramUses: state.paramUses
	})
}

function isAggregateEntry(value: unknown): value is { readonly agg: string; readonly over?: unknown } {
	return typeof value === "object" && value !== null && "agg" in value
}

/** Narrows a value to a variable term, else a pointed refusal. */
function asVarTerm(context: string, value: unknown): AnyVar {
	if (isTerm(value) && value[term] === "var") {
		return value
	}
	throw new AuthoringError({ message: `${context}: expected a variable` })
}

function aggDataOf(name: string, entry: { readonly agg: string; readonly over?: unknown }): AggData {
	if (entry.agg === "count") {
		return Object.freeze({ op: "count" as const })
	}
	const over = entry.over
	switch (entry.agg) {
		case "sum":
		case "mean":
		case "min":
		case "max": {
			if (isTerm(over) && over[term] === "var") {
				return Object.freeze({ op: "fold" as const, fold: entry.agg, over })
			}
			throw new AuthoringError({ message: `find ${name} (${entry.agg}): takes a variable` })
		}
		case "pack":
			return Object.freeze({ op: "pack" as const, over: asVarTerm(`find ${name} (pack)`, over) })
		default:
			throw new AuthoringError({ message: `find ${name}: unknown aggregate ${entry.agg}` })
	}
}

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
		throw new AuthoringError({
			message: `find ${name}: a ${entry[term]} is not projectable — find takes variables, aggregates or Compute expressions`
		})
	}
	if (isComputeExpr(entry)) {
		return Object.freeze({
			name,
			entry: Object.freeze({ kind: "compute" as const, expr: entry, result: entry.result }),
			closed: undefined,
			slot: undefined
		})
	}
	if (isAggregateEntry(entry)) {
		return Object.freeze({
			name,
			entry: Object.freeze({ kind: "aggregate" as const, agg: aggDataOf(name, entry) }),
			closed: undefined,
			slot: undefined
		})
	}
	throw new AuthoringError({
		message: `find ${name}: not a find entry — find takes variables, aggregates or Compute expressions`
	})
}

/**
 * The orderable ban's pointed refusal
 * § orderability): a closed reference is equality-and-membership only.
 */
function closedOrderError(context: string, position: string, vocabulary: string): Error {
	return new AuthoringError({
		message: `${context}: ${position} is a ${vocabulary} reference — declaration order is an accident, not semantics: vocabularies do not order (equality, membership, and counting remain)`
	})
}

function isOrderOp(op: CmpKind | "binding"): op is "lt" | "le" | "gt" | "ge" | "pointIn" {
	return op === "lt" || op === "le" || op === "gt" || op === "ge" || op === "pointIn"
}

function assertBound(where: string, bound: ReadonlySet<AnyVar>, ref: AnyVar): void {
	if (!bound.has(ref)) {
		throw new AuthoringError({
			message: `${where}: the variable ${ref.label} is not bound by a relation atom of the rule`
		})
	}
}

function assertInterval(where: string, ref: AnyVar): void {
	if (ref.field.kind !== "interval") {
		throw new AuthoringError({
			message: `${where}: ${ref.label} is not interval-typed — the measure is defined over interval-typed variables only`
		})
	}
}

function assertNotClosed(where: string, position: string, ref: AnyVar): void {
	const roster = rosterOf(ref.field)
	if (roster !== undefined) {
		throw closedOrderError(where, `${position} ${ref.label}`, roster.name)
	}
}

function assertNumeric(where: string, position: string, ref: AnyVar): void {
	if (ref.field.kind !== "u64" && ref.field.kind !== "i64" && ref.field.kind !== "f64") {
		throw new AuthoringError({
			message: `${where}: ${position} ${ref.label} is ${ref.field.kind}, not numeric — a fold reads u64/i64/f64 only`
		})
	}
}

/**
 * The head column's slot: a projected variable keeps its mint slot; an
 * aggregate output is a NEW derived scalar — its descriptor follows the
 * aggregate's typing (count is u64, mean is f64, sum/min/max keep the
 * input's scalar kind, pack keeps the interval kind) and its class is
 * BARE: a derived value is not the carrier the class law tracked.
 */
function findColumnSlotOf(context: ChainContext, column: FindColumn): ClassedField | undefined {
	const entry = column.entry
	if (entry.kind === "var") {
		return mintSlotOf(context, entry.over)
	}
	if (entry.kind === "compute") {
		return { field: computeFieldOf(entry.result), class: undefined }
	}
	const agg = entry.agg
	switch (agg.op) {
		case "count":
			return { field: u64Field, class: undefined }
		case "fold": {
			if (agg.fold === "mean") {
				return { field: f64Field, class: undefined }
			}
			return { field: agg.over.field, class: undefined }
		}
		case "pack":
			return { field: agg.over.field, class: undefined }
	}
}

function validateColumn(context: ChainContext, bound: ReadonlySet<AnyVar>, column: FindColumn): void {
	const where = `${contextLabel(context)} find ${column.name}`
	const entry = column.entry
	if (entry.kind === "var") {
		assertBound(where, bound, entry.over)
		return
	}
	if (entry.kind === "compute") {
		// Kind agreement, closed-reference and depth walls hold at
		// construction (#query/compute.ts); boundness is a RULE property,
		// judged here like every other term position.
		for (const ref of computeVarsOf(entry.expr)) {
			assertBound(where, bound, ref)
		}
		return
	}
	const agg = entry.agg
	switch (agg.op) {
		case "count":
			return
		case "fold": {
			assertBound(where, bound, agg.over)
			assertNotClosed(where, `the ${agg.fold} input`, agg.over)
			assertNumeric(where, `the ${agg.fold} input`, agg.over)
			if (agg.fold === "mean" && agg.over.field.kind !== "f64") {
				throw new AuthoringError({ message: `${where}: mean requires an f64 input; cast integers explicitly` })
			}
			return
		}
		case "pack":
			assertBound(where, bound, agg.over)
			assertInterval(where, agg.over)
			return
	}
}

function validateCond(context: ChainContext, bound: ReadonlySet<AnyVar>, cond: CondData): void {
	const label = contextLabel(context)
	if (cond.kind === "cmp") {
		for (const side of [cond.lhs, cond.rhs]) {
			if (side.kind === "var") {
				assertBound(label, bound, side.ref)
				const roster = rosterOf(side.ref.field)
				if (isOrderOp(cond.op.kind) && roster !== undefined) {
					throw closedOrderError(label, `the ${cond.op.kind} side ${side.ref.label}`, roster.name)
				}
			}
		}
		if ((cond.op.kind === "eq" || cond.op.kind === "ne") && cond.lhs.kind === "var" && cond.rhs.kind === "var") {
			assertBound(label, bound, cond.lhs.ref)
			assertBound(label, bound, cond.rhs.ref)
			const lhs = mintSlotOf(context, cond.lhs.ref)
			const rhs = mintSlotOf(context, cond.rhs.ref)
			if (!fieldJoins(lhs, rhs)) {
				throw new AuthoringError({
					message: `${label}: ${cond.op.kind}(${cond.lhs.ref.label}, ${cond.rhs.ref.label}) unifies domain-unequal fields — ${cond.lhs.ref.label} bound at ${renderFieldKind(lhs)}, ${cond.rhs.ref.label} at ${renderFieldKind(rhs)} (a var joins only class-equal slots; bare pairs only with bare)`
				})
			}
		}
		return
	}
	for (const child of cond.children) {
		validateCond(context, bound, child)
	}
}

function validateInterior(
	context: ChainContext,
	bound: ReadonlySet<AnyVar>,
	item: {
		readonly kind: "interior" | "negatedInterior"
		readonly target: DerivedTable
		readonly bindings: ReadonlyArray<{ readonly key: string; readonly ref: AnyVar }>
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
			throw new AuthoringError({
				message: `${label}: interior ${item.target.name} binds ${key}, not a head column of ${item.target.name} (head columns: ${headNames.join(", ")})`
			})
		}
	}
	for (const name of headNames) {
		if (!keys.includes(name)) {
			throw new AuthoringError({
				message: `${label}: interior ${item.target.name} omits the head column ${name} — an interior join binds every head column of ${item.target.name}`
			})
		}
	}
	for (const binding of item.bindings) {
		if (item.kind === "negatedInterior" && !bound.has(binding.ref)) {
			throw new AuthoringError({
				message: `${label}: negated interior ${item.target.name} names the variable ${binding.ref.label}, but no positive atom of the rule binds it — a negated atom binds nothing, only rejects (the safety rule)`
			})
		}
		const headColumn = headColumns.find(function byName(column) {
			return column.name === binding.key
		})
		if (headColumn === undefined || headColumn.slot === undefined) {
			continue
		}
		const mint = mintSlotOf(context, binding.ref)
		if (!fieldJoins(headColumn.slot, mint)) {
			throw new AuthoringError({
				message: `${label}: interior ${item.target.name} joins the variable ${binding.ref.label} (${renderFieldKind(mint)}) at head column ${binding.key} (${renderFieldKind(headColumn.slot)}) — a var joins only class-equal slots; bare pairs only with bare`
			})
		}
	}
}

function completeRule(context: ChainContext, state: RuleBuildState, rawColumns: readonly FindColumn[]): RuleData {
	const label = contextLabel(context)
	if (rawColumns.length === 0) {
		throw new AuthoringError({ message: `${label}: a find needs at least one entry` })
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
					throw new AuthoringError({
						message: `${label}: negated ${item.atom.relation.name} atom binds the variable ${binding.term.ref.label} at position ${binding.field}, but no positive atom of the rule binds it — a negated atom binds nothing, only rejects (the safety rule)`
					})
				}
			}
		}
		if (item.kind === "interior" || item.kind === "negatedInterior") {
			validateInterior(context, state.bound, item, columns)
		}
		if (item.kind === "cond") {
			validateCond(context, state.bound, item.cond)
		}
	}
	return Object.freeze({ items: state.items, finds: Object.freeze(columns), paramUses: state.paramUses })
}

function makeRuleValue<Row, P extends ParamsRecord>(rule: RuleData): RuleValue<Row, P> {
	return Object.freeze({ rule })
}

interface RawChain {
	match(relation: MatchOwner | AnyQuery, bindings: Readonly<Record<string, unknown>>): RawChain
	where(cond: AnyCond): RawChain
	interior(name: string, bindings: Readonly<Record<string, unknown>>): RawChain
	find(entries: Readonly<Record<string, unknown>>): RuleValue<never, never>
}

interface RawScope extends TermOps {
	match(relation: MatchOwner | AnyQuery, bindings: Readonly<Record<string, unknown>>): RawChain
	interior(name: string, bindings: Readonly<Record<string, unknown>>): RawChain
}

type DerivedEnv =
	| { readonly interiors: readonly InteriorData[] }
	| { readonly interiors: readonly InteriorData[]; readonly rec: RecHandle | RecHead | RecData }

type ChainContext = { readonly classes: SchemaClasses; readonly theory: AnySchema } & DerivedEnv &
	(
		| { readonly kind: "query" }
		| { readonly kind: "interior"; readonly self: string }
		| { readonly kind: "rec-base"; readonly self: RecHandle }
		| { readonly kind: "rec-arm"; readonly self: RecHead }
	)

function contextLabel(context: ChainContext): string {
	switch (context.kind) {
		case "query":
			return "query rule"
		case "interior":
			return `interior ${context.self} rule`
		case "rec-base":
			return `rec ${context.self.name} base`
		case "rec-arm":
			return `rec ${context.self.name} rec`
	}
}

function isRecHead(rec: RecHandle | RecHead | RecData): rec is RecHead {
	return Array.isArray((rec as RecHead).finds)
}

function lookupDerived(context: ChainContext, name: string): DerivedTable {
	const interior = context.interiors.find(function byName(candidate) {
		return candidate.name === name
	})
	if (interior !== undefined) {
		if (context.kind === "interior" && name === context.self) {
			throw new AuthoringError({
				message: `interior ${name}: an interior does not read itself — declaration order is topological (a self-read is InteriorNotPrior)`
			})
		}
		return interior
	}
	const rec = "rec" in context ? context.rec : undefined
	if (rec !== undefined && rec.name === name) {
		if (context.kind === "interior") {
			throw new AuthoringError({
				message: `interior ${context.self}: interiors cannot read the rec — this cut's interiors are a prefix`
			})
		}
		if (context.kind === "rec-base") {
			throw new AuthoringError({
				message: `rec ${rec.name}: a base arm does not read the rec — self-atoms belong on rec arms`
			})
		}
		if (!isRecHead(rec)) {
			throw new AuthoringError({ message: `rec ${rec.name}: rec arms resolve the rec head after base arms seal it` })
		}
		return rec
	}
	throw new AuthoringError({ message: `${contextLabel(context)}: no derived table named ${name} is in scope` })
}

function interiorAdvance(
	context: ChainContext,
	state: RuleBuildState,
	name: string,
	bindings: Readonly<Record<string, unknown>>
): RuleBuildState {
	return advanceInterior(state, lookupDerived(context, name), bindings, "interior")
}

/**
 * Imported query templates as relation-expression sources (chapter 34's
 * nonrecursive composition). The imported CQ's whole body — its inner
 * interiors plus its main rules as one derived table — is SPLICED into the
 * consuming query's interior roster at assembly ({@link makeRawQuery});
 * naming does not materialize anything, and the imported template keeps its
 * owned immutable IR. One entry is minted per imported query VALUE, so two
 * references to the same template read one derived table.
 */
interface ImportEntry {
	readonly table: InteriorData
	readonly inner: readonly InteriorData[]
}

const importEntries = new WeakMap<object, ImportEntry>()
const importTables = new WeakSet<InteriorData>()

function importEntryOf(context: ChainContext, imported: AnyQuery): ImportEntry {
	const cached = importEntries.get(imported)
	if (cached !== undefined) {
		if (imported.schema !== context.theory) {
			throw new AuthoringError({
				message: `${contextLabel(context)}: the imported query belongs to schema ${imported.schema.name}, not ${context.theory.name} — templates compose within one schema`
			})
		}
		return cached
	}
	if (imported.schema !== context.theory) {
		throw new AuthoringError({
			message: `${contextLabel(context)}: the imported query belongs to schema ${imported.schema.name}, not ${context.theory.name} — templates compose within one schema`
		})
	}
	if (imported.data.kind === "reach") {
		throw new AuthoringError({
			message: `${contextLabel(context)}: a recursive query is not importable as a relation expression yet — declare the recursion on the consuming query (C05 boundary, recorded)`
		})
	}
	const userParams = imported.data.params.filter(function userSupplied(entry) {
		return entry.membership === undefined
	})
	if (userParams.length > 0) {
		throw new AuthoringError({
			message: `${contextLabel(context)}: the imported query takes parameters (${userParams
				.map((entry) => entry.name)
				.join(", ")}) — bind an import's meaning at declaration; parameterized imports are not supported`
		})
	}
	const facade = importFacadeOf(imported)
	const table: InteriorData = Object.freeze({
		name: facade.owner.name,
		finds: imported.data.finds,
		rules: imported.data.rules
	})
	const entry: ImportEntry = Object.freeze({ table, inner: imported.data.interiors })
	importEntries.set(imported, entry)
	importTables.add(table)
	importInner.set(table, entry.inner)
	return entry
}

const importInner = new WeakMap<InteriorData, readonly InteriorData[]>()

/**
 * Every import table the assembled query references, in first-reference
 * order (declared interiors first, then rec arms, then main rules — the
 * lowering walk), deduplicated by table identity.
 */
function referencedImports(
	interiors: readonly InteriorData[],
	rec: RecData | undefined,
	rules: readonly RuleData[]
): readonly InteriorData[] {
	const seen = new Set<InteriorData>()
	const out: InteriorData[] = []
	function walk(ruleList: readonly RuleData[]): void {
		for (const rule of ruleList) {
			for (const item of rule.items) {
				if (item.kind !== "interior" && item.kind !== "negatedInterior") {
					continue
				}
				const target = item.target as InteriorData
				if (importTables.has(target) && !seen.has(target)) {
					seen.add(target)
					out.push(target)
				}
			}
		}
	}
	for (const interior of interiors) {
		walk(interior.rules)
	}
	if (rec !== undefined) {
		walk(rec.base)
		walk(rec.rec)
	}
	walk(rules)
	return out
}

/**
 * Splices referenced imports ahead of the declared interiors: each import's
 * inner interiors first (its rules read them), then the import's own table.
 * Interior prior-ness is preserved — declared interiors never read imports
 * they could not have named, and main/rec rules may read anything earlier.
 * Duplicate names across the final roster refuse.
 */
function spliceImports(
	interiors: readonly InteriorData[],
	rec: RecData | undefined,
	rules: readonly RuleData[]
): readonly InteriorData[] {
	const imports = referencedImports(interiors, rec, rules)
	if (imports.length === 0) {
		return interiors
	}
	const spliced: InteriorData[] = []
	const names = new Set<string>()
	const added = new Set<InteriorData>()
	function add(table: InteriorData): void {
		if (added.has(table)) {
			return
		}
		if (names.has(table.name)) {
			throw new AuthoringError({
				message: `query: the derived-table name ${table.name} appears twice after import splicing — rename the colliding interior (names are unique)`
			})
		}
		added.add(table)
		names.add(table.name)
		spliced.push(table)
	}
	for (const table of imports) {
		for (const inner of importInner.get(table) ?? []) {
			add(inner)
		}
		add(table)
	}
	for (const declared of interiors) {
		add(declared)
	}
	return Object.freeze(spliced)
}

function importAdvance(
	context: ChainContext,
	state: RuleBuildState,
	imported: AnyQuery,
	bindings: Readonly<Record<string, unknown>>
): RuleBuildState {
	const entry = importEntryOf(context, imported)
	return advanceInterior(state, entry.table, bindings, "interior")
}

function notInteriorAdvance(
	context: ChainContext,
	state: RuleBuildState,
	name: string,
	bindings: Readonly<Record<string, unknown>>
): RuleBuildState {
	if (context.kind === "rec-base" || context.kind === "rec-arm") {
		throw new AuthoringError({
			message: `rec ${context.self.name}: a rec rule negates no table — self-negation is negation through the cycle (a finished set is what keeps the operator monotone), and a finished table's fold belongs in the main rules`
		})
	}
	return advanceInterior(state, lookupDerived(context, name), bindings, "negatedInterior")
}

function findColumns(context: ChainContext, entries: Readonly<Record<string, unknown>>): FindColumn[] {
	const columns: FindColumn[] = []
	// Nonrecursive derived stages (interiors) may emit aggregate/computed
	// outputs (C05: "do not leave Interior.rules projection only"); ONLY the
	// recursive feedback cycle stays projection-only — no aggregate,
	// arithmetic-created value or negation flows through it.
	const recName = context.kind === "rec-base" || context.kind === "rec-arm" ? context.self.name : undefined
	for (const [name, entry] of Object.entries(entries)) {
		if (entry === undefined) {
			continue
		}
		if (recName !== undefined && !(isTerm(entry) && entry[term] === "var")) {
			throw new AuthoringError({
				message: `rec ${recName}: a rec head projects bound variables only — no aggregate or arithmetic-created value flows through the feedback cycle (compute over the finished set in the main rules)`
			})
		}
		columns.push(findColumnOf(name, entry))
	}
	return columns
}

function makeRawChain(context: ChainContext, state: RuleBuildState): RawChain {
	const chain: RawChain = {
		match(relation, bindings) {
			if (isImportedSource(relation)) {
				return makeRawChain(context, importAdvance(context, state, relation as unknown as AnyQuery, bindings))
			}
			return makeRawChain(context, advanceMatch(context, state, relation as MatchOwner, bindings))
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

function makeRawScope(context: ChainContext): RawScope {
	const scope: RawScope = {
		...termOps,
		match(relation, bindings) {
			if (isImportedSource(relation)) {
				return makeRawChain(context, importAdvance(context, EMPTY_RULE, relation as unknown as AnyQuery, bindings))
			}
			return makeRawChain(context, advanceMatch(context, EMPTY_RULE, relation as MatchOwner, bindings))
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

function makeQueryRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: AnySchema,
	env: DerivedEnv
): QueryRuleScope<Rels, Classes> {
	const raw = makeRawScope({ kind: "query", classes: theory.classes, theory, ...env })
	if (!isTypedScope<QueryRuleScope<Rels, Classes>>(raw)) {
		throw new SdkInvariantError({ message: "query rule builder construction incomplete" })
	}
	return raw
}

function makeInteriorRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: AnySchema,
	env: DerivedEnv,
	self: string
): InteriorRuleScope<Rels, Classes> {
	const raw = makeRawScope({ kind: "interior", self, classes: theory.classes, theory, ...env })
	if (!isTypedScope<InteriorRuleScope<Rels, Classes>>(raw)) {
		throw new SdkInvariantError({ message: "interior rule builder construction incomplete" })
	}
	return raw
}

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
		throw new SdkInvariantError({ message: "rec rule builder construction incomplete" })
	}
	return raw
}

function renderClosedSlice(closed: AnyClosedRoster | undefined): string {
	return closed === undefined ? "a bare value" : `a ${closed.name} reference`
}

function headSignature(column: FindColumn): string {
	const entry = column.entry
	if (entry.kind === "var") {
		return `${column.name}:var`
	}
	if (entry.kind === "compute") {
		return `${column.name}:compute`
	}
	const agg = entry.agg
	if (agg.op === "fold") {
		return `${column.name}:${agg.fold}`
	}
	return `${column.name}:${agg.op}`
}

function renderParamAnchor(roster: AnyClosedRoster | undefined): string {
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
	rec: RecData | undefined,
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
				throw new AuthoringError({
					message: `query param ${use.name} collides with a membership array's registry entry — name the param differently`
				})
			}
			if (existing.shape !== use.shape) {
				throw new AuthoringError({
					message: `query param ${use.name} is used both as a ${existing.shape} param and a ${use.shape} param — one name, one shape`
				})
			}
			if (existing.anchor !== undefined && use.anchor !== undefined) {
				const registered = rosterOf(existing.anchor)
				const anchored = rosterOf(use.anchor)
				if (registered !== anchored) {
					throw new AuthoringError({
						message: `query param ${use.name} is anchored at ${renderParamAnchor(registered)} and at ${renderParamAnchor(anchored)} — a closed-anchored param translates handle names through ONE roster (one name, one domain); name the params differently`
					})
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
	if (rec !== undefined) {
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
				throw new AuthoringError({ message: `query param ${name} lost its registry entry` })
			}

			let membership: QueryParam | undefined
			if (entry.members !== undefined) {
				const anchor = entry.anchor
				if (anchor === undefined) {
					throw new AuthoringError({ message: `query param ${name} lost its membership anchor` })
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

interface RawQuery {
	readonly schema: AnySchema
	readonly data: QueryData
	rule(build: (r: RawScope) => RuleValue<never, never>): RawQuery
	named(label: string): RawQuery
	interior(name: string, ...builds: never[]): never
	reach(name: string, arms: never): never
}

/**
 * Verifies every rule derives the same head and returns the MERGED head:
 * the meet of the rules' column slots. A head column joins class-equal
 * slots, and — the anti-join law's head twin — same-identity u64 wire
 * with at least one side bare; when the bare arm admits the pair, the
 * merged column's class claim demotes to bare, so a consumer joining
 * the head cannot inherit provenance a rule never proved.
 */
function alignedHeadOf(label: string, rules: readonly RuleData[]): readonly FindColumn[] {
	const first = rules[0]
	if (first === undefined) {
		throw new AuthoringError({ message: `${label} needs at least one rule` })
	}
	const signature = first.finds.map(headSignature).join(", ")
	rules.forEach(function verifyHead(rule, index) {
		const candidate = rule.finds.map(headSignature).join(", ")
		if (candidate !== signature) {
			throw new AuthoringError({
				message: `every rule of ${label} derives the same head — rule 0 finds (${signature}), rule ${index} finds (${candidate})`
			})
		}
		rule.finds.forEach(function verifyClosedSlice(column, position) {
			const lead = first.finds[position]
			if (lead !== undefined && column.closed !== lead.closed) {
				throw new AuthoringError({
					message: `every rule of ${label} derives the same head — the head column ${lead.name} is ${renderClosedSlice(lead.closed)} in rule 0 but ${renderClosedSlice(column.closed)} in rule ${index} (one column decodes through one roster)`
				})
			}
			if (lead === undefined) {
				return
			}
			if (lead.slot !== undefined && column.slot !== undefined && !headFieldJoins(lead.slot, column.slot)) {
				throw new AuthoringError({
					message: `every rule of ${label} derives the same head — the head column ${lead.name} is bound at ${renderFieldKind(lead.slot)} in rule 0 but at ${renderFieldKind(column.slot)} in rule ${index} (a head column joins class-equal slots; same-wire u64 admits a bare side, and the merged head is bare)`
				})
			}
		})
	})
	const merged = first.finds.map(function meetColumn(lead, position) {
		const slot = lead.slot
		if (slot === undefined || slot.class === undefined) {
			return lead
		}
		const demoted = rules.some(function bareElsewhere(rule) {
			const column = rule.finds[position]
			return column !== undefined && column.slot !== undefined && column.slot.class !== slot.class
		})
		if (!demoted) {
			return lead
		}
		return Object.freeze({
			name: lead.name,
			entry: lead.entry,
			closed: lead.closed,
			slot: Object.freeze({ field: slot.field, class: undefined })
		})
	})
	return Object.freeze(merged)
}

function afterMainError(what: string): Error {
	return new AuthoringError({
		message: `query: ${what} after a main rule is unwritable — declaration order is interiors, then rec, then main`
	})
}

function makeRawQuery(
	theory: AnySchema,
	interiors: readonly InteriorData[],
	rec: RecData | undefined,
	rules: readonly RuleData[]
): RawQuery {
	const mergedFinds = alignedHeadOf("a query", rules)
	const first = rules[0]
	if (first === undefined) {
		throw new AuthoringError({ message: "a query needs at least one rule" })
	}
	const env: DerivedEnv = rec === undefined ? { interiors } : { interiors, rec }
	const allInteriors = spliceImports(interiors, rec, rules)
	const frozenInteriors = Object.freeze([...allInteriors])
	const frozenRules = Object.freeze([...rules])
	const params = paramRegistryOf(allInteriors, rec, rules)
	const data: QueryData =
		rec === undefined
			? Object.freeze({
					kind: "cq" as const,
					interiors: frozenInteriors,
					rules: frozenRules,
					finds: mergedFinds,
					params
				})
			: Object.freeze({
					kind: "reach" as const,
					interiors: frozenInteriors,
					rec,
					rules: frozenRules,
					finds: mergedFinds,
					params
				})
	const value: RawQuery = {
		schema: theory,
		data,
		rule(build) {
			const built = build(makeRawScope({ kind: "query", classes: theory.classes, theory, ...env }))
			return makeRawQuery(theory, interiors, rec, [...rules, built.rule])
		},
		named(label) {
			// A diagnostic name for composition/tracing — not a schema
			// relation, a persistent view or a separate CTE type.
			labelImport(value, label)
			return value
		},
		interior() {
			throw afterMainError("interior")
		},
		reach() {
			throw afterMainError("reach")
		}
	}
	Object.freeze(value)
	return value
}

function makeQuery<Rels extends SchemaRelations, Row, P extends ParamsRecord, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>,
	interiors: readonly InteriorData[],
	rec: RecData | undefined,
	rules: readonly RuleData[]
): Query<Rels, Row, P, Classes> {
	return makeRawQuery(theory, interiors, rec, rules) as unknown as Query<Rels, Row, P, Classes>
}

function collectInterior<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>,
	env: DerivedEnv,
	name: string,
	builds: readonly InteriorBuild<Rels, Classes>[]
): InteriorData {
	if (builds.length === 0) {
		throw new AuthoringError({ message: `query: interior ${name} needs at least one rule` })
	}
	const rules = builds.map(function buildRule(buildOne) {
		return buildOne(makeInteriorRuleScope<Rels, Classes>(theory, env, name)).rule
	})
	const mergedFinds = alignedHeadOf(`interior ${name}`, rules)
	return Object.freeze({ name, finds: mergedFinds, rules: Object.freeze(rules) })
}

function collectRec<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>,
	interiors: readonly InteriorData[],
	name: string,
	baseBuilds: readonly RecBuild<Rels, Classes>[],
	recBuilds: readonly RecBuild<Rels, Classes>[]
): RecData {
	if (baseBuilds.length === 0) {
		throw new AuthoringError({ message: `query: rec ${name} has no base arms` })
	}
	if (recBuilds.length === 0) {
		throw new AuthoringError({ message: `query: rec ${name} has no rec arms` })
	}
	const handle: RecHandle = Object.freeze({ name })
	const baseEnv: DerivedEnv = { interiors, rec: handle }
	const base = baseBuilds.map(function buildBase(buildOne) {
		return buildOne(makeRecRuleScope<Rels, Classes>(theory, baseEnv, handle, "rec-base")).rule
	})
	const baseFinds = alignedHeadOf(`rec ${name}`, base)
	const first = base[0]
	if (first === undefined) {
		throw new AuthoringError({ message: `query: rec ${name} has no base arms` })
	}
	const firstFind = baseFinds[0]
	if (firstFind === undefined) {
		throw new AuthoringError({ message: `query: rec ${name} has no head` })
	}
	const finds: RecHead["finds"] = [firstFind, ...baseFinds.slice(1)]
	const head: RecHead = Object.freeze({ name, finds })
	const recEnv: DerivedEnv = { interiors, rec: head }
	const rec = recBuilds.map(function buildRec(buildOne) {
		return buildOne(makeRecRuleScope<Rels, Classes>(theory, recEnv, head, "rec-arm")).rule
	})
	const mergedAll = alignedHeadOf(`rec ${name}`, [...base, ...rec])
	const mergedFirst = mergedAll[0]
	if (mergedFirst === undefined) {
		throw new AuthoringError({ message: `query: rec ${name} has no head` })
	}
	const sealedFinds: RecHead["finds"] = [mergedFirst, ...mergedAll.slice(1)]
	const firstRec = rec[0]
	if (firstRec === undefined) {
		throw new AuthoringError({ message: `query: rec ${name} has no rec arms` })
	}
	const sealedBase: RecData["base"] = [first, ...base.slice(1)]
	const sealedRec: RecData["rec"] = [firstRec, ...rec.slice(1)]
	const recData: RecData = Object.freeze({
		name,
		finds: sealedFinds,
		base: sealedBase,
		rec: sealedRec
	})
	return recData
}

function makeQueryStart<Rels extends SchemaRelations, Classes extends SchemaClasses, P extends ParamsRecord>(
	theory: Schema<Rels, Classes>,
	interiors: readonly InteriorData[]
): QueryStart<Rels, Classes, P> {
	const env: DerivedEnv = { interiors }
	const start = {
		interior<const Builds extends readonly InteriorBuild<Rels, Classes>[]>(
			name: string,
			...builds: Builds
		): QueryStart<Rels, Classes, Flatten<P & BuildsParams<Builds>>> {
			if (
				interiors.some(function sameName(interior) {
					return interior.name === name
				})
			) {
				throw new AuthoringError({ message: `query: interior ${name} is already declared — names are unique` })
			}
			if (name.length === 0) {
				throw new AuthoringError({ message: "query: an interior needs a name" })
			}
			const data = collectInterior(theory, env, name, builds)
			return makeQueryStart<Rels, Classes, Flatten<P & BuildsParams<Builds>>>(theory, [...interiors, data])
		},
		reach<const Base extends readonly RecBuild<Rels, Classes>[], const Step extends readonly RecBuild<Rels, Classes>[]>(
			name: string,
			arms: { readonly base: Base; readonly rec: Step }
		): QueryReachStart<Rels, Classes, Flatten<P & BuildsParams<Base> & BuildsParams<Step>>> {
			if (
				interiors.some(function sameName(interior) {
					return interior.name === name
				})
			) {
				throw new AuthoringError({ message: `query: interior and rec share the name ${name}` })
			}
			if (name.length === 0) {
				throw new AuthoringError({ message: "query: reach needs a name" })
			}
			const data = collectRec(theory, interiors, name, arms.base, arms.rec)
			return makeQueryReachStart<Rels, Classes, Flatten<P & BuildsParams<Base> & BuildsParams<Step>>>(
				theory,
				interiors,
				data
			)
		},
		rule<RV extends AnyRuleValue>(
			build: (r: QueryRuleScope<Rels, Classes>) => RV
		): Query<Rels, RowOf<RV>, Flatten<P & ParamsOf<RV>>, Classes> {
			const built = build(makeQueryRuleScope<Rels, Classes>(theory, env))
			return makeQuery<Rels, RowOf<RV>, Flatten<P & ParamsOf<RV>>, Classes>(theory, interiors, undefined, [built.rule])
		}
	}
	Object.freeze(start)
	return start as unknown as QueryStart<Rels, Classes, P>
}

function makeQueryReachStart<Rels extends SchemaRelations, Classes extends SchemaClasses, P extends ParamsRecord>(
	theory: Schema<Rels, Classes>,
	interiors: readonly InteriorData[],
	rec: RecData
): QueryReachStart<Rels, Classes, P> {
	const env: DerivedEnv = { interiors, rec }
	const start = {
		rule<RV extends AnyRuleValue>(
			build: (r: QueryRuleScope<Rels, Classes>) => RV
		): Query<Rels, RowOf<RV>, Flatten<P & ParamsOf<RV>>, Classes> {
			const built = build(makeQueryRuleScope<Rels, Classes>(theory, env))
			return makeQuery<Rels, RowOf<RV>, Flatten<P & ParamsOf<RV>>, Classes>(theory, interiors, rec, [built.rule])
		}
	}
	Object.freeze(start)
	return start as unknown as QueryReachStart<Rels, Classes, P>
}

function query<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>
): QueryStart<Rels, Classes> {
	return makeQueryStart<Rels, Classes, Record<never, never>>(theory, [])
}

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
		throw new AuthoringError({
			message: `${context}: "${value}" is not a handle of ${closed.name} — the roster is ${closed.handles.join(", ")}`
		})
	}
	return { kind: "u64", value: BigInt(id) }
}

function taggedAtElementDomain(context: string, element: "u64" | "i64" | "f64", value: unknown): TaggedValue {
	if (element === "f64") {
		if (typeof value === "number") {
			return { kind: "f64", value }
		}
		if (isFloatIntervalValue(value)) {
			if (Number.isNaN(value.start) || Number.isNaN(value.end) || !(value.start < value.end)) {
				throw literalShapeError(context, "a nonempty float interval with non-NaN endpoints", value)
			}
			return {
				kind: "intervalF64",
				start: Object.is(value.start, -0) ? 0 : value.start,
				end: Object.is(value.end, -0) ? 0 : value.end
			}
		}
		throw literalShapeError(context, "number (point) or { start, end } numbers (interval)", value)
	}
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
		case "f64": {
			if (typeof value !== "number") {
				throw literalShapeError(context, "number", value)
			}
			return { kind: "f64", value }
		}
		case "id128": {
			if (!Id128.isId128(value)) {
				throw literalShapeError(context, "an Id128 (32 lowercase hex characters)", value)
			}
			return { kind: "id128", value }
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
function taggedCmpLiteral(context: string, sibling: AnyField, value: unknown, op: CmpKind | "binding"): TaggedValue {
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
	if (op === "pointIn" && rosterOf(sibling) === undefined && sibling.kind === "f64" && isFloatIntervalValue(value)) {
		return taggedAtElementDomain(context, "f64", value)
	}
	return taggedLiteral(context, sibling, value)
}

interface LowerContext {
	readonly theory: AnySchema
	readonly relationIds: ReadonlyMap<string, number>
	readonly interiorIds: ReadonlyMap<string, number>
	readonly paramIds: ReadonlyMap<string, number>
	readonly params: ReadonlyMap<string, ParamEntry>
}

interface VarIds {
	of(ref: AnyVar): number
}

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

function paramIdOf(ctx: LowerContext, name: string): number {
	const id = ctx.paramIds.get(name)
	if (id === undefined) {
		throw new AuthoringError({ message: `query lowering: param ${name} is not in the query's registry` })
	}
	return id
}

function lowerAtom(ctx: LowerContext, atom: AtomData, ids: VarIds): AtomIr {
	const member = ctx.theory.relations[atom.relation.name]
	if (member !== atom.relation) {
		throw new AuthoringError({
			message: `query lowering: relation ${atom.relation.name} is not the relation value schema ${ctx.theory.name} declares`
		})
	}
	const relationId = ctx.relationIds.get(atom.relation.name)
	if (relationId === undefined) {
		throw new AuthoringError({ message: `query lowering: relation ${atom.relation.name} has no ordinal` })
	}
	const ordered = sealedFieldsOf(atom.relation)
	const bindings: Array<readonly [number, TermIr]> = atom.bindings.map(function lowerBinding(binding) {
		const ordinal = ordered.findIndex(function byName(candidate) {
			return candidate.name === binding.field
		})
		if (ordinal < 0) {
			throw new AuthoringError({
				message: `query lowering: relation ${atom.relation.name} has no field ${binding.field}`
			})
		}
		return [ordinal, lowerBindingTerm(ctx, `${atom.relation.name}.${binding.field}`, binding, ids)] as const
	})
	return { source: { kind: "edb", relation: relationId }, bindings }
}

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
		throw new AuthoringError({ message: `query lowering: derived table ${target.name} was not declared on this query` })
	}
	if (target.finds.length === 0) {
		throw new AuthoringError({ message: `query lowering: derived table ${target.name} has no head` })
	}
	const irBindings: Array<readonly [number, TermIr]> = target.finds.map(function lowerPosition(column, position) {
		const binding = bindings.find(function byKey(candidate) {
			return candidate.key === column.name
		})
		if (binding === undefined) {
			throw new AuthoringError({ message: `query lowering: interior ${target.name} omits head column ${column.name}` })
		}
		return [position, { kind: "var", var: ids.of(binding.ref) } as const] as const
	})
	return { source: { kind: "interior", interior }, bindings: irBindings }
}

function lowerCmpTerm(ctx: LowerContext, side: CmpTermData, sibling: CmpTermData, ids: VarIds, op: CmpKind): TermIr {
	switch (side.kind) {
		case "var":
			return { kind: "var", var: ids.of(side.ref) }
		case "param":
			return { kind: "param", param: paramIdOf(ctx, side.name) }
		case "setParam":
			return { kind: "paramSet", param: paramIdOf(ctx, side.name) }
		case "literal": {
			const anchor = cmpAnchorOf(ctx, sibling)
			if (anchor === undefined) {
				throw new AuthoringError({
					message: "query lowering: a comparison literal needs a bound-variable or anchored-param sibling to type it"
				})
			}
			return { kind: "literal", value: taggedCmpLiteral("comparison literal", anchor, side.value, op) }
		}
	}
}

function cmpAnchorOf(ctx: LowerContext, sibling: CmpTermData): AnyField | undefined {
	if (sibling.kind === "var") {
		return sibling.ref.field
	}
	if (sibling.kind === "param" || sibling.kind === "setParam") {
		return ctx.params.get(sibling.name)?.anchor
	}
	return undefined
}

function lowerComparison(ctx: LowerContext, cmp: CmpData, ids: VarIds): ComparisonIr {
	if (cmp.op.kind === "allen") {
		return {
			op: { kind: "allen", mask: cmp.op.mask },
			lhs: lowerCmpTerm(ctx, cmp.lhs, cmp.rhs, ids, "allen"),
			rhs: lowerCmpTerm(ctx, cmp.rhs, cmp.lhs, ids, "allen")
		}
	}
	return {
		op: { kind: cmp.op.kind },
		lhs: lowerCmpTerm(ctx, cmp.lhs, cmp.rhs, ids, cmp.op.kind),
		rhs: lowerCmpTerm(ctx, cmp.rhs, cmp.lhs, ids, cmp.op.kind)
	}
}

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

/**
 * Lowers one computed scalar node to the C05 wire (`#native.ts`'s
 * `ScalarExprIr`): the shared grammar arms pass through 1:1. Cached
 * construction depth is the bound; this walk emits wire only and does
 * not re-judge kinds (L14/native compile owns binding).
 */
function lowerComputeNode(node: QueryNode, ids: VarIds): ScalarExprIr {
	if (node.depth > MAX_COMPUTE_DEPTH) {
		throw new AuthoringError({
			message: `query lowering: a compute expression is deeper than ${MAX_COMPUTE_DEPTH} nodes (the engine's scalar depth bound)`
		})
	}
	return lowerComputeGrammar(node, ids)
}

function lowerComputeGrammar(node: QueryNode, ids: VarIds): ScalarExprIr {
	switch (node.kind) {
		case "var":
			return { kind: "var", var: ids.of(node.ref as AnyVar) }
		case "literal":
			return { kind: "literal", value: literalWireOf(node.value) }
		case "negate":
		case "isNaN":
		case "isFinite":
			return { kind: node.kind, expr: lowerComputeGrammar(node.expr, ids) }
		case "cast":
			return { kind: "cast", cast: node.cast, expr: lowerComputeGrammar(node.expr, ids) }
		case "add":
		case "subtract":
		case "multiply":
		case "divide":
			return {
				kind: node.kind,
				left: lowerComputeGrammar(node.left, ids),
				right: lowerComputeGrammar(node.right, ids)
			}
		default:
			throw new AuthoringError({
				message: "query lowering: a query-var tree cannot carry a source-field leaf — use Compute over bound variables"
			})
	}
}

function lowerFind(entry: FindEntryData, ids: VarIds): FindTermIr {
	if (entry.kind === "var") {
		return { kind: "var", var: ids.of(entry.over) }
	}
	if (entry.kind === "compute") {
		return { kind: "compute", expr: lowerComputeNode(entry.expr, ids) }
	}
	const agg = entry.agg
	switch (agg.op) {
		case "count":
			return { kind: "count" }
		case "fold":
			return { kind: "aggregate", op: { kind: agg.fold }, over: ids.of(agg.over) }
		case "pack":
			return { kind: "pack", over: ids.of(agg.over) }
	}
}

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

function headTermOf(column: FindColumn): HeadTermIr {
	const entry = column.entry
	if (entry.kind === "var") {
		return { kind: "var" }
	}
	if (entry.kind === "compute") {
		return { kind: "compute" }
	}
	return { kind: "aggregate", op: headOpOf(entry.agg) }
}

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
				atoms.push(lowerInteriorAtom(ctx, item.target, item.bindings, ids))
				break
			}
			case "negatedInterior": {
				negated.push(lowerInteriorAtom(ctx, item.target, item.bindings, ids))
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
	if (q.data.kind === "reach") {
		interiorIds.set(q.data.rec.name, q.data.interiors.length)
	}
	const paramIds = new Map<string, number>()
	const params = new Map<string, ParamEntry>()
	q.data.params.forEach(function assignParamId(entry, index) {
		if (entry.anchor === undefined) {
			throw new AuthoringError({
				message: `query param ${entry.name} has no field-anchored use — bind it in an atom or compare it against a bound variable`
			})
		}
		paramIds.set(entry.name, index)
		params.set(entry.name, entry)
	})
	const ctx: LowerContext = { theory, relationIds, interiorIds, paramIds, params }
	const interiors = q.data.interiors.map(function lowerInterior(interior) {
		return {
			head: interior.finds.map(headTermOf),
			rules: interior.rules.map(function lowerInteriorRule(rule) {
				return lowerRule(ctx, rule)
			})
		}
	})
	const head = q.data.finds.map(headTermOf)
	const rules = q.data.rules.map(function lowerMainRule(rule) {
		return lowerRule(ctx, rule)
	})
	if (q.data.kind === "cq") {
		return parseQueryIr({ kind: "cq", interiors, head, rules })
	}
	return parseQueryIr({
		kind: "reach",
		interiors,
		rec: {
			head: q.data.rec.finds.map(headTermOf),
			base: q.data.rec.base.map(function lowerBase(rule) {
				return lowerRule(ctx, rule)
			}),
			rec: q.data.rec.rec.map(function lowerRecArm(rule) {
				return lowerRule(ctx, rule)
			})
		},
		head,
		rules
	})
}

export type {
	AnyQuery,
	AnyRuleValue,
	Query,
	QueryData,
	QueryParams,
	QueryReachStart,
	QueryRelation,
	QueryRow,
	QueryRuleChain,
	QueryRuleScope,
	QueryStart,
	RecRuleChain,
	RecRuleScope,
	RuleValue,
	TermOps
}
export { lowerQuery, query, taggedCmpLiteral }
