/**
 * `query()` and the IR lowering, STRUCTURAL edition. A query is built
 * kysely-shaped — `query(S).rule(r => r.match(Rel, { f: r.var("x") })
 * .where(r.eq(r.var("x"), r.param("p"))).select("x"))` — and is an INERT
 * value: `Query<Rels, Row, Params>` with `Row` inferred from each rule's
 * `.select` and `Params` inferred to be EXACTLY the params the rules use
 * (params are typed BY USE; a param value no rule uses never registers, so
 * every query executes under its own inferred type). Vars are string
 * names, domain-typed by the field they first bind and joined by reuse —
 * the rule builder's environment carries name → field descriptor through
 * the chain, checked structurally at every reuse (`JoinOk`), so the old
 * brand-equal join is now the domain-equal compile error. Lowering is a
 * pure function of the query value down to the bridge's `ProgramIr`
 * (`bumbledb/crates/bumbledb/src/ir.rs`, the bijection target): relations
 * by declaration ordinal (the declaration-order-is-ids law the engine's
 * manifest pins), variables by dense per-rule first-occurrence ids
 * (rule-scoped, exactly as the IR scopes them), params by first-use order
 * across the program walk. Lowering is STABLE — the same query value
 * lowers to deeply-equal IR every time, and two identically-written
 * queries lower identically. Construction validates negation safety and
 * name-boundness (typed, naming the variable — earlier and warmer than
 * the engine's refusal); everything else (strata, types, aggregate
 * rosters, rule caps) is the ENGINE's judge, surfacing its typed errors
 * at prepare. No invented limits: rule and predicate counts are never
 * pre-checked here.
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
	RuleIr,
	TaggedValue,
	TermIr
} from "#native.ts"
import type {
	AggData,
	AnyCond,
	AtomData,
	BindEnv,
	BindingEntry,
	BindParamsShape,
	CheckBindings,
	CheckCond,
	CmpData,
	CmpKind,
	CmpTermData,
	CondData,
	CondParamsShape,
	MaskData,
	MatchFields,
	MatchOwner,
	MatchShape,
	ParamUse,
	RecData,
	RuleData,
	RuleItem,
	SelectColumn,
	SelectEntryData,
	TreeData
} from "#query/atom.ts"
import { allen, and, eq, ge, gt, le, lt, ne, not, or, pointIn } from "#query/atom.ts"
import type {
	ClassedField,
	EnvShape,
	Flatten,
	InferredOf,
	JoinOk,
	ParamEntry,
	ParamsRecord,
	Var
} from "#query/scope.ts"
import {
	fieldJoins,
	inferred,
	isTerm,
	makeDuration,
	makeMaskParam,
	makeParam,
	makeSetParam,
	makeVar,
	makeVars,
	renderFieldKind,
	term
} from "#query/scope.ts"
import type { CheckNameSelect, CheckSelect, RowOfNameSelect, RowOfSelect, SelectEntry } from "#query/select.ts"
import { argMax, argMin, count, countDistinct, max, min, pack, sum } from "#query/select.ts"
import type { FieldsShape } from "#relation.ts"
import type { AnySchema, Schema, SchemaRelations } from "#schema.ts"

/**
 * The matchable members of a schema's record — ordinary relations AND
 * closed vocabularies (ψ query atoms: a closed atom is an ordinary EDB
 * atom over the sealed extension; the ENGINE decides whether it folds to a
 * plan-constant member set or joins the L1-resident virtual image — the
 * SDK lowers pass-through and never knows which).
 */
type QueryRelation<Rels extends SchemaRelations> = Extract<Rels[keyof Rels], MatchOwner>

/** The environment after one bindings record: the incoming env plus every var the record binds (as classed slots). */
type EnvOfMatch<Env extends EnvShape, F extends FieldsShape, CR, B> =
	Flatten<Env & BindEnv<F, CR, B>> extends infer E extends EnvShape ? E : never

/** Reads an inferred-params carrier off a rec reference or rule value. */
type ParamsOf<T> = InferredOf<T> extends { readonly params: infer P extends ParamsRecord } ? P : Record<never, never>

/** Reads an inferred-row carrier off a rule value or query. */
type RowOf<T> = InferredOf<T> extends { readonly row: infer R } ? R : never

/**
 * A recursive predicate's HEAD signature as classed slots (descriptor +
 * law-computed class), position for position — carried on the rec
 * reference so an `idb` join can be judged against it; `undefined` on
 * values that carry no head (a plain query rule, or an unthreaded rec
 * handle before its first rule).
 */
type HeadShape = readonly ClassedField[] | undefined

/**
 * One finished rule as a plain value: the runtime data plus the inferred
 * row/params carrier (and, for a RECURSIVE rule, the head's positional
 * field descriptors — the signature `idb` joins pair against).
 * `.rule(...)` consumes it; hosts never build one by hand.
 */
interface RuleValue<Row, P extends ParamsRecord, Head extends HeadShape = undefined> {
	readonly rule: RuleData
	readonly [inferred]?: { readonly row: Row; readonly params: P; readonly head: Head }
}

/** Any finished rule value. */
type AnyRuleValue = RuleValue<unknown, ParamsRecord, HeadShape>

/** The positional head-field tuple of a recursive rule's names-only select. */
type HeadFieldsOf<Env extends EnvShape, S extends readonly string[]> = {
	readonly [I in keyof S]: Env[S[I] & keyof Env]
}

/** Reads an inferred-head carrier off a rule value or rec reference. */
type HeadOf<T> = InferredOf<T> extends { readonly head: infer H extends readonly ClassedField[] } ? H : undefined

/**
 * A recursive predicate REFERENCE — the shape `idb()` targets carry: the
 * name (type-level identity: a recursive rule's own `idb` accepts only its
 * own name — the self-recursion cut), the runtime data (value identity),
 * the params its attached rules have used so far, and the head signature
 * its FIRST rule sealed (thread the value `.rule(...)` returns into an
 * `idb` and the program's `Params` type stays exact AND the idb join is
 * arity- and domain-checked against the head).
 */
interface RecRef<Name extends string, P extends ParamsRecord, Head extends HeadShape = HeadShape> {
	readonly name: Name
	readonly data: RecData
	readonly [inferred]?: { readonly params: P; readonly head: Head }
}

/** One `idb` position's judgment: the var must be bound by a relation atom, class-equal to the head slot when the head is carried. */
type IdbVarOk<Env extends EnvShape, T, F> =
	T extends Var<infer N extends string>
		? N extends keyof Env
			? F extends ClassedField
				? JoinOk<Env[N], F>
				: true
			: false
		: false

/**
 * The validated `idb` variable tuple: every var must already be bound by a
 * relation atom of the rule; and when the target carries its head
 * signature (the threaded rec handle), the tuple must match the head's
 * arity and every position must be class-equal to its head slot — the
 * same wall `JoinOk` holds for EDB atoms. An unthreaded handle carries no
 * head; its joins stay boundness-checked here and arity/class-judged at
 * prepare (the engine's law stands behind both tiers).
 */
type CheckIdbVars<Env extends EnvShape, V, Head extends HeadShape = undefined> = Head extends readonly ClassedField[]
	? V extends readonly unknown[]
		? V["length"] extends Head["length"]
			? { readonly [I in keyof V]: IdbVarOk<Env, V[I], Head[I & keyof Head]> extends true ? V[I] : never }
			: { readonly [I in keyof V]: never }
		: never
	: { readonly [I in keyof V]: IdbVarOk<Env, V[I], undefined> extends true ? V[I] : never }

/**
 * The term/predicate/aggregate constructor vocabulary every rule builder
 * carries — pure value builders, environment-free: the chain's `.where`,
 * `.match`, and `.select` seams judge their output against the rule
 * environment.
 */
interface TermOps {
	/** Declares/names one variable: typed by the field it first binds; reuse joins. */
	readonly var: typeof makeVar
	/** Mints several variables at once — `const { service, w } = r.vars("service", "w")`: each name typed exactly; duplicates refuse. */
	readonly vars: typeof makeVars
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

/** The rule builder a `query(S).rule(...)` callback receives: the ops plus the first atom (`Classes` — the schema type's class map, the join judge's authority). */
interface QueryRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> extends TermOps {
	/** The first EDB atom of the rule: fields bind vars, params, ∈-sets, or bare literals; absence is the wildcard (same-named vars within the record join class-equal). */
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Record<never, never>, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): QueryRuleChain<
		Rels,
		EnvOfMatch<Record<never, never>, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>,
		BindParamsShape<MatchFields<R>, B>,
		Classes
	>
}

/** The chain of a plain query rule: more atoms, residual predicates, then the head. */
interface QueryRuleChain<
	Rels extends SchemaRelations,
	Env extends EnvShape,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	/** One more positive EDB atom — var reuse joins, class-equal by the environment check. */
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Env, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): QueryRuleChain<
		Rels,
		EnvOfMatch<Env, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>,
		Flatten<P & BindParamsShape<MatchFields<R>, B>>,
		Classes
	>
	/** One residual predicate: a comparison, an `and`/`or` tree, or a negated atom (`r.not`). */
	where<const C extends AnyCond>(
		cond: CheckCond<Env, Classes, C> & C
	): QueryRuleChain<Rels, Env, Flatten<P & CondParamsShape<Env, C>>, Classes>
	/** The head projection: var names, the measure, and aggregates; written order = answer column order. */
	select<const S extends readonly SelectEntry[]>(...entries: CheckSelect<Env, S> & S): RuleValue<RowOfSelect<Env, S>, P>
}

/** The rule builder an OUTPUT rule of a `program()` receives: a query rule plus finished-stratum `idb` atoms. */
interface OutputRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> extends TermOps {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Record<never, never>, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): OutputRuleChain<
		Rels,
		EnvOfMatch<Record<never, never>, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>,
		BindParamsShape<MatchFields<R>, B>,
		Classes
	>
}

/** The chain of an output rule: atoms, predicates, `idb` joins over the program's recs, then the head. */
interface OutputRuleChain<
	Rels extends SchemaRelations,
	Env extends EnvShape,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Env, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): OutputRuleChain<
		Rels,
		EnvOfMatch<Env, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>,
		Flatten<P & BindParamsShape<MatchFields<R>, B>>,
		Classes
	>
	where<const C extends AnyCond>(
		cond: CheckCond<Env, Classes, C> & C
	): OutputRuleChain<Rels, Env, Flatten<P & CondParamsShape<Env, C>>, Classes>
	/**
	 * One `idb` atom over a FINISHED stratum (any rec of this program): a
	 * positional join against the rec's head. An idb atom is a join
	 * position — every variable must already be bound by a relation atom of
	 * the rule (the theory's own domain relation; the rec's answers are
	 * theory values, so the join is identity). Threading the rec value the
	 * last `.rule(...)` returned carries its rules' params into `Params`
	 * AND its head signature, so the join is arity- and class-checked
	 * against the head at compile time.
	 */
	idb<Target extends RecRef<string, ParamsRecord>, const V extends readonly Var<string>[]>(
		target: Target,
		...vars: CheckIdbVars<Env, V, HeadOf<Target>> & V
	): OutputRuleChain<Rels, Env, Flatten<P & ParamsOf<Target>>, Classes>
	select<const S extends readonly SelectEntry[]>(...entries: CheckSelect<Env, S> & S): RuleValue<RowOfSelect<Env, S>, P>
}

/** The rule builder a RECURSIVE rule (`rec.rule(...)`) receives. */
interface RecRuleScope<Rels extends SchemaRelations, Self extends string, Classes extends SchemaClasses = SchemaClasses>
	extends TermOps {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Record<never, never>, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): RecRuleChain<
		Rels,
		Self,
		EnvOfMatch<Record<never, never>, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>,
		BindParamsShape<MatchFields<R>, B>,
		Classes
	>
}

/**
 * The chain of a recursive rule. Its `idb` accepts ONLY the rec itself —
 * the self-recursion cut as a type-level boundary (mutual recursion is
 * unwritable; a finished lower stratum is folded by the OUTPUT rules) —
 * and its `select` takes bound variable NAMES only: aggregates and the
 * measure are unrepresentable in a recursive head (the strata judge's
 * `AggregationThroughCycle`/`MeasureInRecursiveHead`, made unwritable).
 */
interface RecRuleChain<
	Rels extends SchemaRelations,
	Self extends string,
	Env extends EnvShape,
	P extends ParamsRecord,
	Classes extends SchemaClasses = SchemaClasses
> {
	match<R extends QueryRelation<Rels>, const B extends MatchShape<MatchFields<R>>>(
		relation: R,
		bindings: B & CheckBindings<Env, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>
	): RecRuleChain<
		Rels,
		Self,
		EnvOfMatch<Env, MatchFields<R>, ClassRecordOf<Classes, R["name"]>, B>,
		Flatten<P & BindParamsShape<MatchFields<R>, B>>,
		Classes
	>
	where<const C extends AnyCond>(
		cond: CheckCond<Env, Classes, C> & C
	): RecRuleChain<Rels, Self, Env, Flatten<P & CondParamsShape<Env, C>>, Classes>
	/** The self-recursive atom: `idb(self, ...boundVars)` — only this rec's own reference is accepted (threaded, its head arity- and class-checks the join). */
	idb<Target extends RecRef<Self, ParamsRecord>, const V extends readonly Var<string>[]>(
		target: Target,
		...vars: CheckIdbVars<Env, V, HeadOf<Target>> & V
	): RecRuleChain<Rels, Self, Env, P, Classes>
	/** The recursive head: bound variable names only (the creation quarantine, restated for fixpoint topology); the value carries the head's classed slots for `idb` pairing. */
	select<const S extends readonly string[]>(
		...names: CheckNameSelect<Env, S> & S
	): RuleValue<RowOfNameSelect<Env, S>, P, HeadFieldsOf<Env, S>>
}

/** A query's runtime description — everything lowering, the wire marshal, and answer decode read. */
interface QueryData {
	/** The program's recursive predicates in declaration order (empty for a plain query); `PredId` = index. */
	readonly recs: readonly RecData[]
	/** The output rules in written order (multiple rules = set union). */
	readonly rules: readonly RuleData[]
	/** The head columns (every rule derives the same head; written order = answer column order). */
	readonly select: readonly SelectColumn[]
	/** The registered params in first-use order across the program walk (= dense `ParamId`s). */
	readonly params: readonly ParamEntry[]
}

/**
 * An inert query value. `Row` is the inferred answer-row object type;
 * `Params` the inferred execute-params object type — exactly the params
 * the rules use. Prepare with `db.prepare(q)`; nothing here touches an
 * engine.
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

/**
 * Any query value as lowering and the runtime consume it: the theory it
 * was built against and its runtime description — every `Query` (typed or
 * program-built) carries exactly this.
 */
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
	var: makeVar,
	vars: makeVars,
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

/** One rule under construction: immutable — every chain step is a fresh state. */
interface RuleBuildState {
	readonly items: readonly RuleItem[]
	readonly varFields: Readonly<Record<string, ClassedField>>
	readonly paramUses: readonly ParamUse[]
}

/** The empty rule state. */
const EMPTY_RULE: RuleBuildState = Object.freeze({
	items: Object.freeze([]),
	varFields: Object.freeze({}),
	paramUses: Object.freeze([])
})

/** One resolved bindings record: the atom entries, the vars it binds (as classed slots), and the params it uses. */
interface ResolvedBindings {
	readonly atom: AtomData
	readonly vars: ReadonlyArray<{ readonly name: string; readonly slot: ClassedField }>
	readonly uses: readonly ParamUse[]
}

/**
 * Judges one membership ARRAY at a binding position — legal exactly at a
 * CLOSED-reference field (the owner ruling: ordinary u64/str membership is
 * spelled through `r.inSet` params; literal arrays are the closed
 * vocabulary's spelling), holding ≥ 2 DISTINCT handle names (the
 * degenerate sets are refusals: empty selects nothing, one element is the
 * bare literal respelled, and a duplicate member is the same respelling in
 * disguise — write each member once). The returned name is
 * CONTENT-ADDRESSED (vocabulary + the member SET — the key sorts a copy,
 * so two spellings of one set, reordered or not, share one dense
 * `ParamId`); the members are shape-checked strings here and
 * roster-verified at the one verification point (`taggedHandleId`) when
 * the SDK supplies the set at execute — the same moment a bound `r.inSet`
 * param's members are judged.
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
 * Resolves a bindings record against an atom owner's matchable fields (a
 * relation's declared fields; a closed relation's sealed id + columns), in
 * the record's written order: terms classify by their runtime tag,
 * everything else is a bare literal (typed by the FIELD at lowering — the
 * membership typing rule included). Every bound field carries its
 * law-computed class, read off the schema value's frozen class map — the
 * runtime twin of the type tier's `SlotAt` lookups.
 */
function resolveBindings(
	context: string,
	relation: MatchOwner,
	bindings: Readonly<Record<string, unknown>>,
	classes: SchemaClasses
): ResolvedBindings {
	const entries: BindingEntry[] = []
	const vars: Array<{ readonly name: string; readonly slot: ClassedField }> = []
	const uses: ParamUse[] = []
	const relationClasses = classes[relation.name]
	const ordered = sealedFieldsOf(relation)
	for (const [fieldName, value] of Object.entries(bindings)) {
		if (value === undefined) {
			continue
		}
		const declared = ordered.find(function byName(candidate) {
			return candidate.name === fieldName
		})
		if (declared === undefined) {
			throw errors.new(`${context} has no field ${fieldName}`)
		}
		const fieldClass = relationClasses?.[fieldName]
		let bound: BindingEntry["term"]
		if (isTerm(value)) {
			switch (value[term]) {
				case "var": {
					bound = Object.freeze({ kind: "var" as const, name: value.name })
					vars.push(
						Object.freeze({ name: value.name, slot: Object.freeze({ field: declared.field, class: fieldClass }) })
					)
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
						`${context}.${fieldName}: an Allen-mask param is not a field-typed value — masks live in allen() conditions only`
					)
				case "duration":
					throw errors.new(
						`${context}.${fieldName}: the measure is not a field-typed value — it lives in comparisons and select entries`
					)
			}
		} else if (Array.isArray(value)) {
			const set = membershipSet(`${context}.${fieldName}`, declared.field, value)
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
	return {
		atom: Object.freeze({ relation, bindings: Object.freeze(entries) }),
		vars,
		uses
	}
}

/**
 * Extends a rule state with one positive atom. Vars bind on first
 * occurrence; every LATER occurrence (a later atom's field or a same-record
 * sibling) is a join and must be class-equal — the construction-time twin
 * of the type tier's `JoinOk` (bare pairs only with bare), so the domain
 * wall holds for untyped callers too.
 */
function advanceMatch(
	state: RuleBuildState,
	relation: MatchOwner,
	bindings: Readonly<Record<string, unknown>>,
	classes: SchemaClasses
): RuleBuildState {
	const resolved = resolveBindings(`relation ${relation.name}`, relation, bindings, classes)
	const varFields: Record<string, ClassedField> = { ...state.varFields }
	for (const bound of resolved.vars) {
		const existing = varFields[bound.name]
		if (existing === undefined) {
			varFields[bound.name] = bound.slot
		} else if (!fieldJoins(existing, bound.slot)) {
			throw errors.new(
				`relation ${relation.name}: the variable ${bound.name} joins domain-unequal fields — first bound at ${renderFieldKind(existing)}, reused at ${renderFieldKind(bound.slot)} (a var joins only class-equal slots; bare pairs only with bare)`
			)
		}
	}
	return {
		items: Object.freeze([...state.items, Object.freeze({ kind: "atom" as const, atom: resolved.atom })]),
		varFields: Object.freeze(varFields),
		paramUses: Object.freeze([...state.paramUses, ...resolved.uses])
	}
}

/** Resolves one comparison side to its runtime term. */
function cmpTermDataOf(op: string, value: unknown): CmpTermData {
	if (isTerm(value)) {
		switch (value[term]) {
			case "var":
				return Object.freeze({ kind: "var" as const, name: value.name })
			case "param":
				return Object.freeze({ kind: "param" as const, name: value.name })
			case "setParam":
				return Object.freeze({ kind: "setParam" as const, name: value.name })
			case "duration":
				return Object.freeze({ kind: "measure" as const, name: value.name })
			case "maskParam":
				throw errors.new(`${op}: an Allen-mask param is not a comparison term — masks live in allen()'s mask position`)
		}
	}
	return Object.freeze({ kind: "literal" as const, value })
}

/**
 * One comparison side's contribution to the param census: a param/set side
 * anchors to its SIBLING — a bound variable's field descriptor or the
 * measure; an unanchorable use (literal or param sibling) records with no
 * anchor and must be anchored by some other use of the same name.
 */
function sideUses(
	op: CmpKind,
	side: CmpTermData,
	sibling: CmpTermData,
	varFields: Readonly<Record<string, ClassedField>>,
	uses: ParamUse[]
): void {
	if (side.kind !== "param" && side.kind !== "setParam") {
		return
	}
	let anchor: AnyField | "measure" | undefined
	if (sibling.kind === "var") {
		anchor = varFields[sibling.name]?.field
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
function condDataOf(cond: AnyCond, varFields: Readonly<Record<string, ClassedField>>, uses: ParamUse[]): CondData {
	if (cond.cond === "cmp") {
		const lhs = cmpTermDataOf(cond.op, cond.lhs)
		const rhs = cmpTermDataOf(cond.op, cond.rhs)
		sideUses(cond.op, lhs, rhs, varFields, uses)
		sideUses(cond.op, rhs, lhs, varFields, uses)
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
		const data: CmpData = Object.freeze({ kind: "cmp" as const, op: cond.op, mask, lhs, rhs })
		return data
	}
	if (cond.cond === "tree") {
		const children = cond.children.map(function lowerChild(child) {
			return condDataOf(child, varFields, uses)
		})
		const data: TreeData = Object.freeze({ kind: "tree" as const, op: cond.op, children: Object.freeze(children) })
		return data
	}
	throw errors.new(
		"a negated atom is not a condition-tree node — pass not(...) to where() directly, never inside and()/or()"
	)
}

/** Extends a rule state with one `.where` item (a condition or a negated atom). */
function advanceWhere(state: RuleBuildState, cond: AnyCond, classes: SchemaClasses): RuleBuildState {
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
		const resolved = resolveBindings(`negated relation ${relation.name}`, relation, bindings, classes)
		return {
			items: Object.freeze([...state.items, Object.freeze({ kind: "negated" as const, atom: resolved.atom })]),
			varFields: state.varFields,
			paramUses: Object.freeze([...state.paramUses, ...resolved.uses])
		}
	}
	const uses: ParamUse[] = []
	const data = condDataOf(cond, state.varFields, uses)
	return {
		items: Object.freeze([...state.items, Object.freeze({ kind: "cond" as const, cond: data })]),
		varFields: state.varFields,
		paramUses: Object.freeze([...state.paramUses, ...uses])
	}
}

/** Extends a rule state with one `idb` atom (vars must be bound — validated at completion). */
function advanceIdb(state: RuleBuildState, rec: RecData, vars: readonly Var<string>[]): RuleBuildState {
	const names = vars.map(function nameOf(variable) {
		if (!isTerm(variable) || variable[term] !== "var") {
			throw errors.new(`idb ${rec.name}: positions take variables — bind literals and params through where()/match()`)
		}
		return variable.name
	})
	return {
		items: Object.freeze([...state.items, Object.freeze({ kind: "idb" as const, rec, vars: Object.freeze(names) })]),
		varFields: state.varFields,
		paramUses: state.paramUses
	}
}

/** Narrows a select entry to an aggregate value. */
function isAggregateEntry(
	value: unknown
): value is { readonly agg: string; readonly over: unknown; readonly key: unknown } {
	return typeof value === "object" && value !== null && "agg" in value
}

/**
 * Classifies one select entry into its named answer column. The `closed`
 * slice is resolved LATER, at rule completion (`completeRule`), where the
 * rule's `varFields` are in hand — until then every column is provisionally
 * bare.
 */
function selectColumnOf(entry: unknown): SelectColumn {
	if (typeof entry === "string") {
		return Object.freeze({
			name: entry,
			entry: Object.freeze({ kind: "var" as const, over: entry }),
			closed: undefined
		})
	}
	if (isTerm(entry)) {
		if (entry[term] === "duration") {
			return Object.freeze({
				name: entry.name,
				entry: Object.freeze({ kind: "measure" as const, over: entry.name }),
				closed: undefined
			})
		}
		throw errors.new(
			`query select: a ${entry[term]} is not projectable — select takes variable names, duration(v), or aggregates`
		)
	}
	if (isAggregateEntry(entry)) {
		return aggregateColumnOf(entry)
	}
	throw errors.new("query select: not a select entry — select takes variable names, duration(v), or aggregates")
}

/** Classifies one aggregate select entry. */
function aggregateColumnOf(entry: {
	readonly agg: string
	readonly over: unknown
	readonly key: unknown
}): SelectColumn {
	function column(name: string, agg: AggData): SelectColumn {
		return Object.freeze({
			name,
			entry: Object.freeze({ kind: "aggregate" as const, agg: Object.freeze(agg) }),
			closed: undefined
		})
	}
	const over = entry.over
	switch (entry.agg) {
		case "count":
			return column("count", { op: "count" })
		case "countDistinct": {
			if (typeof over !== "string") {
				throw errors.new("countDistinct takes a variable name")
			}
			return column(over, { op: "countDistinct", over })
		}
		case "sum":
		case "min":
		case "max": {
			if (typeof over === "string") {
				return column(over, { op: "fold", fold: entry.agg, over })
			}
			if (isTerm(over) && over[term] === "duration") {
				return column(over.name, { op: "fold", fold: entry.agg, over: Object.freeze({ duration: over.name }) })
			}
			throw errors.new(`${entry.agg} takes a variable name or duration(v)`)
		}
		case "argMax":
		case "argMin": {
			if (typeof over !== "string" || typeof entry.key !== "string") {
				throw errors.new(`${entry.agg} takes a carried variable name and an orderable key variable name`)
			}
			return column(over, { op: "arg", direction: entry.agg, over, key: entry.key })
		}
		case "pack": {
			if (typeof over !== "string") {
				throw errors.new("pack takes a variable name")
			}
			return column(over, { op: "pack", over })
		}
		default:
			throw errors.new(`unknown aggregate ${entry.agg}`)
	}
}

/**
 * The orderable ban's pointed refusal (`docs/architecture/10-data-model.md`
 * § orderability): a closed reference is equality-and-membership only —
 * its declaration-id order is an encoding accident, so every
 * order-comparison and fold position refuses it. The construction-time
 * twin of the type tier's `OrderVarOk` exclusion, so the wall holds for
 * untyped callers too (the engine cannot backstop this one: the wire IR
 * carries plain u64s, no rosters).
 */
function closedOrderError(context: string, position: string, vocabulary: string): Error {
	return errors.new(
		`${context}: ${position} is a ${vocabulary} reference — declaration order is an accident, not semantics: vocabularies do not order (docs/architecture/10-data-model.md; equality, membership, and counting remain)`
	)
}

/** The comparison ops under the orderable ban (order roster + point membership — every order-comparison position). */
function isOrderOp(op: CmpKind | "binding"): op is "lt" | "le" | "gt" | "ge" | "pointIn" {
	return op === "lt" || op === "le" || op === "gt" || op === "ge" || op === "pointIn"
}

/** Requires a var name to be bound by a relation atom of the rule. */
function assertBound(context: string, varFields: Readonly<Record<string, ClassedField>>, name: string): ClassedField {
	const slot = varFields[name]
	if (slot === undefined) {
		throw errors.new(`${context}: the variable ${name} is not bound by a relation atom of the rule`)
	}
	return slot
}

/** Requires a var name to be bound at an interval field (the measure's and pack's domain). */
function assertIntervalBound(context: string, varFields: Readonly<Record<string, ClassedField>>, name: string): void {
	const slot = assertBound(context, varFields, name)
	if (slot.field.kind !== "interval") {
		throw errors.new(
			`${context}: ${name} is not interval-typed — the measure is defined over interval-typed variables only`
		)
	}
}

/**
 * Validates one condition's variable references against the rule's bound
 * names — and, for `eq`/`ne` over two variables, holds the class wall: the
 * unification IS a join, so the two slots must be class-equal exactly as a
 * match-reuse join must be (the construction-time twin of the type tier's
 * `EqOk` → `JoinOk`; bare pairs only with bare). The engine cannot backstop
 * this one — the query IR carries no domains — so the wall lives here for
 * untyped callers too.
 */
function validateCond(context: string, varFields: Readonly<Record<string, ClassedField>>, cond: CondData): void {
	if (cond.kind === "cmp") {
		for (const side of [cond.lhs, cond.rhs]) {
			if (side.kind === "var") {
				const slot = assertBound(context, varFields, side.name)
				const roster = rosterOf(slot.field)
				if (isOrderOp(cond.op) && roster !== undefined) {
					throw closedOrderError(context, `the ${cond.op} side ${side.name}`, roster.name)
				}
			}
			if (side.kind === "measure") {
				assertIntervalBound(context, varFields, side.name)
			}
		}
		if ((cond.op === "eq" || cond.op === "ne") && cond.lhs.kind === "var" && cond.rhs.kind === "var") {
			const lhs = assertBound(context, varFields, cond.lhs.name)
			const rhs = assertBound(context, varFields, cond.rhs.name)
			if (!fieldJoins(lhs, rhs)) {
				throw errors.new(
					`${context}: ${cond.op}(${cond.lhs.name}, ${cond.rhs.name}) unifies domain-unequal fields — ${cond.lhs.name} bound at ${renderFieldKind(lhs)}, ${cond.rhs.name} at ${renderFieldKind(rhs)} (a var joins only class-equal slots; bare pairs only with bare)`
				)
			}
		}
		return
	}
	for (const child of cond.children) {
		validateCond(context, varFields, child)
	}
}

/** Validates one select column's variable references. */
function validateColumn(
	context: string,
	varFields: Readonly<Record<string, ClassedField>>,
	column: SelectColumn
): void {
	const entry = column.entry
	if (entry.kind === "var") {
		assertBound(`${context} select ${column.name}`, varFields, entry.over)
		return
	}
	if (entry.kind === "measure") {
		assertIntervalBound(`${context} select ${column.name}`, varFields, entry.over)
		return
	}
	const agg = entry.agg
	switch (agg.op) {
		case "count":
			return
		case "countDistinct":
			assertBound(`${context} select ${column.name}`, varFields, agg.over)
			return
		case "fold": {
			if (typeof agg.over === "string") {
				const slot = assertBound(`${context} select ${column.name}`, varFields, agg.over)
				const roster = rosterOf(slot.field)
				if (roster !== undefined) {
					throw closedOrderError(`${context} select ${column.name}`, `the ${agg.fold} input ${agg.over}`, roster.name)
				}
				return
			}
			assertIntervalBound(`${context} select ${column.name}`, varFields, agg.over.duration)
			return
		}
		case "arg": {
			assertBound(`${context} select ${column.name}`, varFields, agg.over)
			const key = assertBound(`${context} select ${column.name}`, varFields, agg.key)
			const keyRoster = rosterOf(key.field)
			if (keyRoster !== undefined) {
				throw closedOrderError(
					`${context} select ${column.name}`,
					`the ${agg.direction} key ${agg.key}`,
					keyRoster.name
				)
			}
			return
		}
		case "pack":
			assertIntervalBound(`${context} select ${column.name}`, varFields, agg.over)
			return
	}
}

/**
 * Resolves the roster one select column decodes through: a projected var,
 * or an Arg-carried payload, bound at a closed-referencing field carries
 * that field's roster (read off `varFields` — the same slot the domain
 * machinery reads), and `decodeAnswers` lifts the column's row ids back to
 * handle NAMES through it — the runtime twin of the row type's `Infer`
 * claim. Every other entry decodes bare: counts are counts, the measure
 * and `pack` are never closed, and a closed FOLD is banned outright
 * ({@link closedOrderError}) before this resolution runs.
 */
function selectClosedOf(
	varFields: Readonly<Record<string, ClassedField>>,
	entry: SelectEntryData
): ClosedRoster | undefined {
	let over: string | undefined
	if (entry.kind === "var") {
		over = entry.over
	} else if (entry.kind === "aggregate" && entry.agg.op === "arg") {
		over = entry.agg.over
	} else {
		over = undefined
	}
	if (over === undefined) {
		return undefined
	}
	return rosterOf(varFields[over]?.field)
}

/**
 * Completes one rule: classifies the select record (written order = answer
 * column order, names must be declaration-order-safe keys), and validates
 * boundness — every condition/select/idb variable bound by a relation atom,
 * and every NEGATED atom's variable positively bound (the safety rule: a
 * negated atom binds nothing, only rejects).
 */
function completeRule(context: string, state: RuleBuildState, columns: readonly SelectColumn[]): RuleData {
	if (columns.length === 0) {
		throw errors.new(`${context}: a select needs at least one entry`)
	}
	const seen = new Set<string>()
	for (const column of columns) {
		assertDeclarationOrderKey(`${context} select column`, column.name)
		if (seen.has(column.name)) {
			throw errors.new(`${context}: select names the answer column ${column.name} twice`)
		}
		seen.add(column.name)
		validateColumn(context, state.varFields, column)
	}
	for (const item of state.items) {
		if (item.kind === "negated") {
			for (const binding of item.atom.bindings) {
				if (binding.term.kind === "var") {
					const bound = state.varFields[binding.term.name]
					if (bound === undefined) {
						throw errors.new(
							`${context}: negated ${item.atom.relation.name} atom binds the variable ${binding.term.name} at position ${binding.field}, but no positive atom of the rule binds it — a negated atom binds nothing, only rejects (the safety rule)`
						)
					}
					const negatedSlot: ClassedField = { field: binding.data, class: binding.class }
					if (!fieldJoins(bound, negatedSlot)) {
						throw errors.new(
							`${context}: negated ${item.atom.relation.name} atom reuses the variable ${binding.term.name} at ${binding.field} (${renderFieldKind(negatedSlot)}), but the rule binds it at ${renderFieldKind(bound)} — a var joins only class-equal slots; bare pairs only with bare`
						)
					}
				}
			}
		}
		if (item.kind === "idb") {
			const head = item.rec.rules[0]
			item.vars.forEach(function checkIdbVar(name, position) {
				const bound = state.varFields[name]
				if (bound === undefined) {
					throw errors.new(
						`${context}: idb ${item.rec.name} names the variable ${name}, but no relation atom of the rule binds it — an idb atom is a join position; bind the variable through the theory's own relation first`
					)
				}
				const column = head?.select[position]
				if (column === undefined || column.entry.kind !== "var") {
					return
				}
				const headSlot = head?.varFields[column.entry.over]
				if (headSlot !== undefined && !fieldJoins(headSlot, bound)) {
					throw errors.new(
						`${context}: idb ${item.rec.name} joins the variable ${name} (${renderFieldKind(bound)}) at head position ${position} (${column.name}: ${renderFieldKind(headSlot)}) — a var joins only class-equal slots; bare pairs only with bare`
					)
				}
			})
		}
		if (item.kind === "cond") {
			validateCond(context, state.varFields, item.cond)
		}
	}
	return Object.freeze({
		items: state.items,
		select: Object.freeze(
			columns.map(function enrichColumn(column): SelectColumn {
				return Object.freeze({
					name: column.name,
					entry: column.entry,
					closed: selectClosedOf(state.varFields, column.entry)
				})
			})
		),
		varFields: state.varFields,
		paramUses: state.paramUses
	})
}

/** Builds one typed rule value over completed rule data. */
function makeRuleValue<Row, P extends ParamsRecord>(rule: RuleData): RuleValue<Row, P> {
	return Object.freeze({ rule })
}

/**
 * The one runtime chain every context shares — non-generic on purpose: the
 * typed chain interfaces (`QueryRuleChain`/`OutputRuleChain`/`RecRuleChain`)
 * apply at the scope factories' boundaries, and the runtime beneath them is
 * one plain value walk. Context gates the two context-bound verbs: `idb`
 * (a program construct — self-only inside a rec, any rec of the program in
 * the output, refused in a plain query) and the recursive `select`
 * (bound variable names only — the creation quarantine).
 */
interface RawChain {
	match(relation: MatchOwner, bindings: Readonly<Record<string, unknown>>): RawChain
	where(cond: AnyCond): RawChain
	idb(target: RecRef<string, ParamsRecord>, ...vars: readonly Var<string>[]): RawChain
	select(...entries: readonly SelectEntry[]): RuleValue<never, never>
}

/** The runtime rule-builder shape beneath every typed scope. */
interface RawScope extends TermOps {
	match(relation: MatchOwner, bindings: Readonly<Record<string, unknown>>): RawChain
}

/** Which rule family a chain builds — gates `idb` and the recursive select — plus the schema's runtime class map (the join judge's authority). */
type ChainContext = { readonly classes: SchemaClasses } & (
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
	vars: readonly Var<string>[]
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
		return advanceIdb(state, context.self, vars)
	}
	if (!context.program.recs.includes(target.data)) {
		throw errors.new(
			`idb ${target.name}: the rec was declared by a different program — rec identity is the membership rule`
		)
	}
	return advanceIdb(state, target.data, vars)
}

/** Classifies one select tuple per the context (a recursive head projects bound NAMES only). */
function selectColumns(context: ChainContext, entries: readonly SelectEntry[]): SelectColumn[] {
	return entries.map(function columnOf(entry): SelectColumn {
		if (context.kind === "rec" && typeof entry !== "string") {
			throw errors.new(
				`rec ${context.self.name}: a recursive head projects bound variable NAMES only — aggregates and the measure read finished sets (the strata judge's quarantine, unwritable here)`
			)
		}
		return selectColumnOf(entry)
	})
}

/** Builds one runtime chain (immutably — every step is a fresh chain over fresh state). */
function makeRawChain(context: ChainContext, state: RuleBuildState): RawChain {
	const chain: RawChain = {
		match(relation, bindings) {
			return makeRawChain(context, advanceMatch(state, relation, bindings, context.classes))
		},
		where(cond) {
			return makeRawChain(context, advanceWhere(state, cond, context.classes))
		},
		idb(target, ...vars) {
			return makeRawChain(context, idbAdvance(context, state, target, vars))
		},
		select(...entries) {
			return makeRuleValue<never, never>(completeRule(contextLabel(context), state, selectColumns(context, entries)))
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
			return makeRawChain(context, advanceMatch(EMPTY_RULE, relation, bindings, context.classes))
		}
	}
	Object.freeze(scope)
	return scope
}

/**
 * The rule builders' trusted admission seam — THE home of the
 * trusted-admission-seam pattern the other mint guards cite (the face,
 * class-map, axiom-readback, rec-handle, and query-value seams): the raw
 * builder is one runtime shape for every context, and this guard verifies
 * the checkable fact — the builder verbs exist — before the value is
 * admitted at its TYPED face. The type-level
 * judgments (domain-equal joins, boundness, the recursion cut) live in the
 * interfaces themselves; the runtime twin of every one of them is a
 * construction-time validation in this module.
 */
function isTypedScope<S>(scope: RawScope): scope is RawScope & S {
	return typeof scope.match === "function"
}

/** Builds one query-rule builder (the typed face of the raw builder). */
function makeQueryRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	classes: SchemaClasses
): QueryRuleScope<Rels, Classes> {
	const raw = makeRawScope({ kind: "query", classes })
	if (!isTypedScope<QueryRuleScope<Rels, Classes>>(raw)) {
		throw errors.new("query rule builder construction incomplete")
	}
	return raw
}

/** Builds one output-rule builder over a program's recs. */
function makeOutputRuleScope<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	program: ProgramState
): OutputRuleScope<Rels, Classes> {
	const raw = makeRawScope({ kind: "output", program, classes: program.classes })
	if (!isTypedScope<OutputRuleScope<Rels, Classes>>(raw)) {
		throw errors.new("program output rule builder construction incomplete")
	}
	return raw
}

/** One program's build-time registry: its recs in declaration order (sealed when the output is declared) and the theory's class map. */
interface ProgramState {
	readonly recs: RecData[]
	readonly classes: SchemaClasses
	sealed: boolean
}

/** Renders one head column's closed slice for the rule-alignment check's diagnostics. */
function renderClosedSlice(closed: ClosedRoster | undefined): string {
	return closed === undefined ? "a bare value" : `a ${closed.name} reference`
}

/** Renders one head column's signature for the rule-alignment check. */
function headSignature(column: SelectColumn): string {
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

/**
 * The classed slot one answer column's VALUES flow from, resolved through
 * the rule's own binding environment: a projected var's first-binding slot,
 * or an Arg-carried payload's (`argMax`/`argMin` carry `over` verbatim —
 * the same two shapes the closed slice lifts). Counts, folds, `pack` and
 * the measure derive numbers/intervals rather than carrying a slot's ids,
 * so they resolve no slot (`undefined`).
 */
function headSlotOf(rule: RuleData, column: SelectColumn): ClassedField | undefined {
	const entry = column.entry
	if (entry.kind === "var") {
		return rule.varFields[entry.over]
	}
	if (entry.kind === "aggregate" && entry.agg.op === "arg") {
		return rule.varFields[entry.agg.over]
	}
	return undefined
}

/** The roster a param anchor carries: present exactly on a closed-reference field anchor (rides THE one `rosterOf` reader). */
function anchorRosterOf(anchor: AnyField | "measure" | undefined): ClosedRoster | undefined {
	return anchor === "measure" ? undefined : rosterOf(anchor)
}

/** Renders one param anchor's closedness for the registry's coherence diagnostics. */
function renderParamAnchor(roster: ClosedRoster | undefined): string {
	return roster === undefined ? "a non-closed position" : `a ${roster.name} reference`
}

/**
 * Folds every rule's param uses (recs in declaration order first, output
 * rules last — exactly the lowering walk) into the query's registry:
 * first use mints the dense `ParamId`, the first FIELD-ANCHORED use types
 * the wire, and one name must keep one shape AND one closedness — every
 * anchored use of one name must agree on the roster (value identity), so a
 * param anchored at a closed reference is GUARANTEED to ride the one
 * roster-verification point (`taggedHandleId`) at execute; a name anchored
 * both at a closed reference and at a non-closed position (or at two
 * vocabularies) is refused here, because the wire would translate only the
 * first anchor's reading (the type tier intersects the uses to `never`;
 * this is its runtime twin for untyped callers). A param whose anchor is a
 * CLOSED reference must never sit in an order-comparison position — the
 * anchor types its value a handle name and the engine would order the
 * translated row ids, so the pairing is refused here too (the registry is
 * the one place a name's every use and its anchoring field meet).
 */
function paramRegistryOf(recs: readonly RecData[], rules: readonly RuleData[]): readonly ParamEntry[] {
	const order: string[] = []
	const byName = new Map<
		string,
		{
			shape: ParamEntry["shape"]
			anchor: ParamEntry["anchor"]
			op: ParamEntry["op"]
			members: ParamEntry["members"]
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
			return Object.freeze({ name, shape: entry.shape, anchor: entry.anchor, op: entry.op, members: entry.members })
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
 * the decode labels and the engine's alignment rule agree by
 * construction), and the param registry folds in program-walk order.
 */
function makeRawQuery(theory: AnySchema, recs: readonly RecData[], rules: readonly RuleData[]): RawQuery {
	const first = rules[0]
	if (first === undefined) {
		throw errors.new("a query needs at least one rule")
	}
	const signature = first.select.map(headSignature).join(", ")
	rules.forEach(function verifyHead(rule, index) {
		const candidate = rule.select.map(headSignature).join(", ")
		if (candidate !== signature) {
			throw errors.new(
				`every rule of a query derives the same head — rule 0 selects (${signature}), rule ${index} selects (${candidate})`
			)
		}
		// The closed slice is part of the head too: one answer column decodes
		// through one roster, so a union whose rules bind a column at
		// different vocabularies (or one closed, one bare — the ids would
		// mistranslate silently) is refused pointed. Vocabulary identity is
		// value identity, the SDK's membership rule everywhere.
		rule.select.forEach(function verifyClosedSlice(column, position) {
			const lead = first.select[position]
			if (lead !== undefined && column.closed !== lead.closed) {
				throw errors.new(
					`every rule of a query derives the same head — the answer column ${lead.name} is ${renderClosedSlice(lead.closed)} in rule 0 but ${renderClosedSlice(column.closed)} in rule ${index} (one column decodes through one roster)`
				)
			}
			// The law-class wall on the union head: one answer column is one
			// value space, so the classed slot each rule binds the column at
			// must join across rules — the SAME fieldJoins judgment every
			// join/eq/negated-atom position enforces. The SDK holds this wall
			// because the wire IR carries no domains: the engine cannot
			// backstop it, and without it a union mixes (say) Holder ids and
			// Account ids in one column the consumer reads as one id space.
			if (lead === undefined) {
				return
			}
			const leadSlot = headSlotOf(first, lead)
			const slot = headSlotOf(rule, column)
			if (leadSlot !== undefined && slot !== undefined && !fieldJoins(leadSlot, slot)) {
				throw errors.new(
					`every rule of a query derives the same head — the answer column ${lead.name} unions domain-unequal fields: bound at ${renderFieldKind(leadSlot)} in rule 0 but at ${renderFieldKind(slot)} in rule ${index} (a union column joins only class-equal slots; bare pairs only with bare)`
				)
			}
		})
	})
	const data: QueryData = Object.freeze({
		recs: Object.freeze([...recs]),
		rules: Object.freeze([...rules]),
		select: first.select,
		params: paramRegistryOf(recs, rules)
	})
	const value: RawQuery = {
		schema: theory,
		data,
		rule(build) {
			const built = build(makeRawScope({ kind: "query", classes: theory.classes }))
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
 * Opens a query over a schema: `query(S).rule(r => ...)`. Each `.rule`
 * adds one conjunctive rule; multiple rules are the set union (answers are
 * SETS — no order or limit exists anywhere; hosts sort). The schema's
 * law-computed class map rides into every rule builder — the join walls
 * compare class names off it, at the type level and at construction alike.
 */
function query<Rels extends SchemaRelations, Classes extends SchemaClasses>(
	theory: Schema<Rels, Classes>
): QueryStart<Rels, Classes> {
	const start: QueryStart<Rels, Classes> = {
		rule<RV extends AnyRuleValue>(
			build: (r: QueryRuleScope<Rels, Classes>) => RV
		): Query<Rels, RowOf<RV>, ParamsOf<RV>, Classes> {
			const built = build(makeQueryRuleScope<Rels, Classes>(theory.classes))
			return makeQuery<Rels, RowOf<RV>, ParamsOf<RV>, Classes>(theory, [], [built.rule])
		}
	}
	Object.freeze(start)
	return start
}

/**
 * Tags one closed-reference literal: the handle NAME, verified against the
 * roster (the belt the wide fallback type cannot provide — structural
 * values make any string spellable here) and translated to its
 * declaration-order row id, tagged u64 — queries cross ids, never handle
 * names; the wire is untouched. THE single roster-verification point of
 * the query surface: atom-binding literals, comparison literals,
 * execute-time params, and membership-array members all reach it (never
 * duplicate the check per call site).
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
 * structural kind directs the tag, never a guess. At an interval field a
 * bigint literal tags as the ELEMENT type — the IR's membership typing
 * rule (point membership), an interval-shaped literal as the interval
 * (value equality). A closed-reference literal is its bare handle id,
 * tagged u64 after a roster verification.
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
			/**
			 * The marshal's bijection law at the query seam (`marshal.ts`
			 * cellOf): a lone surrogate would be lossily replaced with
			 * U+FFFD at the bridge's UTF-8 crossing and silently match a
			 * fact the typed write surface can never store — distinct JS
			 * strings collapsing to one wire query. This is the single
			 * seam every query string literal, string param
			 * (`taggedCmpLiteral`), and membership member lowers through.
			 */
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
 * Tags one host literal at a COMPARISON or PARAM position, where the
 * SIBLING anchors the type: a measure sibling is u64, an interval-field
 * sibling contributes its element domain (so both a point literal in
 * `pointIn` and a `span` literal in `allen` tag correctly), a scalar
 * sibling its own type. At `pointIn` the operand order is interval-left,
 * point-right (`ir::CmpOp::PointIn`), so an interval-shaped literal
 * beside a scalar element-typed sibling is the LEGAL interval operand of
 * `pointIn(t, span(...))` and tags as the interval of the sibling's
 * element domain; under every other operator an interval shape against a
 * scalar sibling stays refused (the engine's IllegalComparison — the
 * bug-hunt fix, preserved op-aware).
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

/** One rule's dense variable numbering: first occurrence in written order. */
interface VarIds {
	of(name: string): number
}

/** Creates one rule-scoped variable numberer. */
function makeVarIds(): VarIds {
	const assigned = new Map<string, number>()
	return {
		of(name) {
			const existing = assigned.get(name)
			if (existing !== undefined) {
				return existing
			}
			const id = assigned.size
			assigned.set(name, id)
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
 * same edb source — its ordinal is its record-declaration slot exactly like
 * an ordinary relation's — with field ordinals over the SEALED shape: `id`
 * at 0, each payload column at its declared index + 1 (`sealedFieldsOf`
 * carries the shift; the lowering golden pins it).
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

/**
 * Lowers one binding term. A membership ARRAY (`literalSet`) lowers to the
 * existing param-set term over its content-addressed registry entry — the
 * program IR is byte-identical to the same set spelled `r.inSet`; the SDK
 * supplies the translated member set itself at execute (`wireParams`).
 */
function lowerBindingTerm(ctx: LowerContext, context: string, binding: BindingEntry, ids: VarIds): TermIr {
	const bound = binding.term
	switch (bound.kind) {
		case "var":
			return { kind: "var", var: ids.of(bound.name) }
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

/** Lowers one idb atom: positional head bindings, `FieldId(i)` = head position i. */
function lowerIdbAtom(ctx: LowerContext, rec: RecData, vars: readonly string[], ids: VarIds): AtomIr {
	const pred = ctx.recIds.get(rec)
	if (pred === undefined) {
		throw errors.new(`query lowering: rec ${rec.name} was declared by a different program`)
	}
	const arity = rec.rules[0]?.select.length
	if (arity !== undefined && vars.length !== arity) {
		throw errors.new(`query lowering: idb ${rec.name} takes ${arity} positions, got ${vars.length}`)
	}
	const bindings: Array<readonly [number, TermIr]> = vars.map(function lowerPosition(name, position) {
		return [position, { kind: "var", var: ids.of(name) } as const] as const
	})
	return { source: { kind: "idb", pred }, bindings }
}

/** Lowers one comparison side; literals tag by the sibling's anchor (op-aware at `pointIn`). */
function lowerCmpTerm(
	ctx: LowerContext,
	rule: RuleData,
	side: CmpTermData,
	sibling: CmpTermData,
	ids: VarIds,
	op: CmpKind
): TermIr {
	switch (side.kind) {
		case "var":
			return { kind: "var", var: ids.of(side.name) }
		case "param":
			return { kind: "param", param: paramIdOf(ctx, side.name) }
		case "setParam":
			return { kind: "paramSet", param: paramIdOf(ctx, side.name) }
		case "measure":
			return { kind: "measure", var: ids.of(side.name) }
		case "literal": {
			const anchor = cmpAnchorOf(ctx, rule, sibling)
			if (anchor === undefined) {
				throw errors.new(
					"query lowering: a comparison literal needs a bound-variable, measure, or anchored-param sibling to type it"
				)
			}
			return { kind: "literal", value: taggedCmpLiteral("comparison literal", anchor, side.value, op) }
		}
	}
}

/** Resolves the anchor a comparison literal tags by: the sibling's field, the measure, or an anchored param. */
function cmpAnchorOf(ctx: LowerContext, rule: RuleData, sibling: CmpTermData): AnyField | "measure" | undefined {
	if (sibling.kind === "var") {
		return rule.varFields[sibling.name]?.field
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
function lowerComparison(ctx: LowerContext, rule: RuleData, cmp: CmpData, ids: VarIds): ComparisonIr {
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
			lhs: lowerCmpTerm(ctx, rule, cmp.lhs, cmp.rhs, ids, "allen"),
			rhs: lowerCmpTerm(ctx, rule, cmp.rhs, cmp.lhs, ids, "allen")
		}
	}
	return {
		op: { kind: cmp.op },
		lhs: lowerCmpTerm(ctx, rule, cmp.lhs, cmp.rhs, ids, cmp.op),
		rhs: lowerCmpTerm(ctx, rule, cmp.rhs, cmp.lhs, ids, cmp.op)
	}
}

/** Lowers one condition node (comparison leaf or and/or tree). */
function lowerCondition(ctx: LowerContext, rule: RuleData, cond: CondData, ids: VarIds): ConditionTreeIr {
	if (cond.kind === "cmp") {
		return { kind: "leaf", cmp: lowerComparison(ctx, rule, cond, ids) }
	}
	return {
		kind: cond.op,
		children: cond.children.map(function lowerChild(child) {
			return lowerCondition(ctx, rule, child, ids)
		})
	}
}

/** Lowers one select entry to its per-rule find term. */
function lowerFind(entry: SelectEntryData, ids: VarIds): FindTermIr {
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
			if (typeof agg.over === "string") {
				return { kind: "aggregate", op: { kind: agg.fold }, over: ids.of(agg.over) }
			}
			return { kind: "aggregateMeasure", op: { kind: agg.fold }, over: ids.of(agg.over.duration) }
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

/** One select entry's var-free head shape. */
function headTermOf(column: SelectColumn): HeadTermIr {
	const entry = column.entry
	if (entry.kind === "var" || entry.kind === "measure") {
		return { kind: "var" }
	}
	return { kind: "aggregate", op: headOpOf(entry.agg) }
}

/** Lowers one rule: body walked in written order (var ids by first occurrence), finds last. */
function lowerRule(ctx: LowerContext, rule: RuleData): RuleIr {
	const ids = makeVarIds()
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
				atoms.push(lowerIdbAtom(ctx, item.rec, item.vars, ids))
				break
			}
			case "cond": {
				conditions.push(lowerCondition(ctx, rule, item.cond, ids))
				break
			}
		}
	}
	return {
		finds: rule.select.map(function findOf(column) {
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
 * (rules + head) appended last. Relations lower by declaration ordinal,
 * the law the engine's own manifest pins; `db.prepare` re-verifies the
 * alignment against the live manifest before sending. Every registered
 * param must carry a field anchor by now — an unanchorable param (its
 * every use beside a literal) is refused here, naming it.
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
			head: head.select.map(headTermOf),
			rules: rec.rules.map(function lowerRecRule(rule) {
				return lowerRule(ctx, rule)
			})
		}
	})
	predicates.push({
		head: q.data.select.map(headTermOf),
		rules: q.data.rules.map(function lowerOutputRule(rule) {
			return lowerRule(ctx, rule)
		})
	})
	return { predicates, output: q.data.recs.length }
}

export type {
	AnyQuery,
	AnyRuleValue,
	HeadFieldsOf,
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
