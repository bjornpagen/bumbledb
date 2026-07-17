/**
 * `query()` and the IR lowering (PRD-08). A query is built inside a scope
 * callback (variable identity is lexical and typed) and is an INERT value:
 * `Query<Rels, Row, Params>` with `Row` inferred from `select` and
 * `Params` from every parameter the returned rules (and the predicates
 * they reach) use. Lowering is a pure function of the query value down to
 * the bridge's `ProgramIr` (`bumbledb/crates/bumbledb/src/ir.rs`, the
 * bijection target): relations and predicates by declaration ordinal (the
 * declaration-order-is-ids law the engine's manifest pins), variables by
 * dense per-rule first-occurrence ids (rule-scoped, exactly as the IR
 * scopes them), params by scope declaration order. Lowering is STABLE —
 * the same query value lowers to deeply-equal IR every time, and two
 * identically-written queries lower identically (prepared-query caching
 * upstream keys on this). Construction validates negation safety (typed,
 * naming the variable — earlier and warmer than the engine's refusal);
 * everything else (strata, types, aggregates rosters, rule caps) is the
 * ENGINE's judge, surfacing its typed errors at prepare. No invented
 * limits: rule counts and predicate counts are never pre-checked here.
 */

import * as errors from "@superbuilders/errors"
import { phantom } from "#brand.ts"
import { assertDeclarationOrderKey, type FieldData } from "#fields.ts"
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
	AnyBodyItem,
	AnyCondition,
	AtomSourceData,
	BindingEntry,
	CmpOpData,
	CmpTerm,
	ComparisonItem,
	MatchAtom
} from "#query/atom.ts"

import type {
	ColumnValues,
	Predicate,
	PredicateColumnsInput,
	PredicateData,
	PredicateParams,
	PredicateRuleInput,
	PredicateSelf
} from "#query/predicate.ts"
import { makePredicate } from "#query/predicate.ts"
import type {
	AnyTerm,
	AnyVar,
	ItemParams,
	MaskParam,
	Param,
	ParamSet,
	ParamsRecord,
	ParamsShape,
	QueryRegistry,
	Var
} from "#query/scope.ts"

import { createRegistry, isTerm, scopeAllenParam, scopeParam, scopeParamSet, scopeVar, term } from "#query/scope.ts"
import type { AggregateData, RowOf, SelectShape } from "#query/select.ts"
import type { FieldRef } from "#relation.ts"
import type { AnySchema, Schema, SchemaRelations } from "#schema.ts"

/**
 * The scope value the `query()` build callback receives: variable and
 * parameter declaration plus predicate declaration (engine recursion).
 */
interface Scope<Rels extends SchemaRelations> {
	/**
	 * Declares one query variable, typed by the field it is declared from
	 * — usable in ANY atom position whose field carries the same brand
	 * (the nominal join discipline). Two calls are two variables.
	 */
	var<V>(field: FieldRef<keyof Rels & string, string, V>): Var<V>
	/**
	 * Declares one scalar parameter under a mandatory name literal — the
	 * key of the typed params object `execute` takes.
	 */
	param<const Name extends string, V>(name: Name, field: FieldRef<keyof Rels & string, string, V>): Param<Name, V>
	/**
	 * Declares one set parameter (the IR's `ParamSet`): bound at execution
	 * to a readonly array; a binding position matches on set membership.
	 */
	paramSet<const Name extends string, V>(name: Name, field: FieldRef<keyof Rels & string, string, V>): ParamSet<Name, V>
	/**
	 * Declares one Allen-mask parameter (`MaskTerm::Param`): the temporal
	 * relation as a bind-time 13-bit mask argument.
	 */
	allenParam<const Name extends string>(name: Name): MaskParam<Name>
	/**
	 * Declares one IDB predicate with a named, field-typed head; its rules
	 * are given in the declaration through a callback receiving the
	 * predicate's own reference (self-recursion is `self.match({...})`).
	 * `.match` on the returned value uses it as a body atom elsewhere.
	 */
	predicate<
		const Cols extends PredicateColumnsInput,
		const Rules extends readonly PredicateRuleInput<ColumnValues<Cols>>[]
	>(
		name: string,
		columns: Cols,
		rules: (self: PredicateSelf<ColumnValues<Cols>>) => Rules
	): Predicate<ColumnValues<Cols>, PredicateParams<Rules>>
}

/**
 * What the build callback returns: `rules` is an array of conjunctions
 * (multiple rules = set union — answers are SETS, no order/limit exists;
 * the host sorts); `select` is the head record — non-aggregate entries are
 * the implicit group key.
 */
interface QueryBuild {
	readonly rules: readonly (readonly AnyBodyItem[])[]
	readonly select: SelectShape
}

/** One classified select column as runtime data. */
type SelectEntryData =
	| { readonly kind: "var"; readonly over: AnyVar }
	| { readonly kind: "measure"; readonly over: AnyVar }
	| { readonly kind: "aggregate"; readonly aggregate: AggregateData }

/** One answer column: its name (the row object key) and its entry. */
interface SelectColumn {
	readonly name: string
	readonly entry: SelectEntryData
}

/** A query's runtime description — everything lowering and execution read. */
interface QueryData {
	readonly registry: QueryRegistry
	readonly rules: readonly (readonly AnyBodyItem[])[]
	readonly select: readonly SelectColumn[]
}

/**
 * An inert query value. `Row` is the inferred answer-row object type;
 * `Params` the inferred execute-params object type. Prepare with
 * `db.prepare(q)`; nothing here touches an engine.
 */
interface Query<Rels extends SchemaRelations, Row, Params extends ParamsRecord> {
	readonly schema: Schema<Rels>
	readonly data: QueryData
	readonly [phantom]?: { readonly row: Row; readonly params: Params }
}

/** Any query value, whatever its schema and inferred types. */
type AnyQuery = Query<SchemaRelations, unknown, ParamsRecord>

/** Extracts a query value's inferred answer-row type. */
type QueryRow<Q extends AnyQuery> = Exclude<Q[typeof phantom], undefined>["row"]

/** Extracts a query value's inferred execute-params type. */
type QueryParams<Q extends AnyQuery> = Exclude<Q[typeof phantom], undefined>["params"]

/** The params object type a build result implies (union over every rule item). */
type BuildParams<R extends QueryBuild> = ParamsShape<ItemParams<R["rules"][number][number]>>

/**
 * Narrows an aggregate select value — a trusted seam over the surface's
 * own constructors (only `#query/select.ts` produces `aggregate`-carrying
 * values), the same direction as every other constructor-owned shape here.
 */
function isAggregateEntry(value: object): value is { readonly aggregate: AggregateData } {
	return "aggregate" in value
}

/** Narrows a `duration()` select value (only `duration` produces `measure`). */
function isMeasureEntry(value: object): value is { readonly measure: AnyVar } {
	return "measure" in value
}

/** Classifies one select record value into its runtime entry. */
function selectEntryOf(name: string, value: unknown): SelectEntryData {
	if (isTerm(value)) {
		if (value[term] !== "var") {
			throw errors.new(
				`query select column ${name}: a ${value[term]} is not projectable — select takes variables, duration(v), or aggregates`
			)
		}
		return Object.freeze({ kind: "var" as const, over: value })
	}
	if (typeof value === "object" && value !== null) {
		if (isAggregateEntry(value)) {
			return Object.freeze({ kind: "aggregate" as const, aggregate: value.aggregate })
		}
		if (isMeasureEntry(value)) {
			return Object.freeze({ kind: "measure" as const, over: value.measure })
		}
	}
	throw errors.new(
		`query select column ${name}: not a select entry — select takes variables, duration(v), or aggregates`
	)
}

/** Renders an atom source's name for construction diagnostics. */
function sourceName(source: AtomSourceData): string {
	if (source.kind === "relation") {
		return source.relation.name
	}
	return `predicate ${source.pred.name}`
}

/** Collects every variable a rule's positive atoms bind. */
function positiveVarsOf(body: readonly AnyBodyItem[]): Set<AnyVar> {
	const positive = new Set<AnyVar>()
	for (const item of body) {
		if (item.item !== "atom" || item.negated) {
			continue
		}
		for (const binding of item.bindings) {
			if (binding.term.kind === "term" && binding.term.value[term] === "var") {
				positive.add(binding.term.value)
			}
		}
	}
	return positive
}

/** Judges one negated atom against the rule's positively-bound variables. */
function assertNegatedAtomSafe(context: string, positive: ReadonlySet<AnyVar>, atom: MatchAtom<ParamsRecord>): void {
	for (const binding of atom.bindings) {
		if (binding.term.kind === "oneOf") {
			throw errors.new(
				`${context}: negated ${sourceName(atom.source)} atom binds ${binding.field} with oneOf(...) — its lowering mints a variable no positive atom binds (the safety rule); write one negated atom per literal, or bind a paramSet`
			)
		}
		if (binding.term.kind !== "term" || binding.term.value[term] !== "var") {
			continue
		}
		const variable = binding.term.value
		if (!positive.has(variable)) {
			throw errors.new(
				`${context}: negated ${sourceName(atom.source)} atom binds the variable declared from ${variable.relation}.${variable.field} at position ${binding.field}, but no positive atom of the rule binds it — a negated atom binds nothing, only rejects (the safety rule)`
			)
		}
	}
}

/**
 * The negation safety rule, judged at construction (earlier and warmer
 * than the engine's refusal, which also stands): every variable a negated
 * atom uses must be bound by a positive atom of the same rule — a negated
 * atom binds nothing, only rejects.
 */
function assertNegationSafety(context: string, body: readonly AnyBodyItem[]): void {
	const positive = positiveVarsOf(body)
	for (const item of body) {
		if (item.item === "atom" && item.negated) {
			assertNegatedAtomSafe(context, positive, item)
		}
	}
}

/**
 * Builds a query as a typed value: runs `build` inside a fresh scope,
 * classifies the select record (written order = answer column order),
 * validates negation safety across every rule (the predicates' rules
 * included), and freezes. Rule counts, strata legality, and every deeper
 * roster stay the engine's judge at prepare.
 */
function query<Rels extends SchemaRelations, const R extends QueryBuild>(
	theory: Schema<Rels>,
	build: ($: Scope<Rels>) => R
): Query<Rels, RowOf<R["select"]>, BuildParams<R>> {
	const registry = createRegistry(theory)
	const scope: Scope<Rels> = Object.freeze({
		var<V>(field: FieldRef<keyof Rels & string, string, V>): Var<V> {
			return scopeVar(registry, field)
		},
		param<const Name extends string, V>(name: Name, field: FieldRef<keyof Rels & string, string, V>): Param<Name, V> {
			return scopeParam(registry, name, field)
		},
		paramSet<const Name extends string, V>(
			name: Name,
			field: FieldRef<keyof Rels & string, string, V>
		): ParamSet<Name, V> {
			return scopeParamSet(registry, name, field)
		},
		allenParam<const Name extends string>(name: Name): MaskParam<Name> {
			return scopeAllenParam(registry, name)
		},
		predicate<
			const Cols extends PredicateColumnsInput,
			const Rules extends readonly PredicateRuleInput<ColumnValues<Cols>>[]
		>(
			name: string,
			columns: Cols,
			rules: (self: PredicateSelf<ColumnValues<Cols>>) => Rules
		): Predicate<ColumnValues<Cols>, PredicateParams<Rules>> {
			return makePredicate(registry, name, columns, rules)
		}
	})
	const built = build(scope)
	const select: SelectColumn[] = []
	for (const [name, value] of Object.entries(built.select)) {
		assertDeclarationOrderKey("query select column", name)
		select.push(Object.freeze({ name, entry: selectEntryOf(name, value) }))
	}
	for (const pred of registry.predicates) {
		pred.rules.forEach(function validateRule(rule, index) {
			assertNegationSafety(`query construction (predicate ${pred.name}, rule ${index})`, rule.body)
		})
	}
	const rules = built.rules.map(function freezeRule(rule, index) {
		assertNegationSafety(`query construction (rule ${index})`, rule)
		return Object.freeze([...rule])
	})
	Object.freeze(registry.params)
	Object.freeze(registry.predicates)
	Object.freeze(registry)
	return Object.freeze({
		schema: theory,
		data: Object.freeze({ registry, rules: Object.freeze(rules), select: Object.freeze(select) })
	})
}

/** The typed shape refusal of the literal tagger — a genuine failure, never data. */
function literalShapeError(context: string, expected: string, value: unknown): Error {
	return errors.new(`${context}: expected ${expected}, got ${typeof value}`)
}

/** Narrows an interval-shaped literal (a plain `{ start, end }` bigint pair). */
function isIntervalShaped(value: unknown): value is { readonly start: bigint; readonly end: bigint } {
	return (
		typeof value === "object" &&
		value !== null &&
		"start" in value &&
		"end" in value &&
		typeof value.start === "bigint" &&
		typeof value.end === "bigint"
	)
}

/**
 * Tags one host literal at a FIELD position (atom bindings): the field's
 * structural type directs the tag, never a guess. At an interval field a
 * bigint literal tags as the ELEMENT type — the IR's membership typing
 * rule (point membership), an interval-shaped literal as the interval
 * (value equality). A closed-reference literal is its branded id, tagged
 * u64 after a roster re-verification (queries cross ids, never handle
 * names).
 */
function taggedLiteral(context: string, field: FieldData, value: unknown): TaggedValue {
	if (field.closed !== undefined) {
		return taggedHandleId(context, field.closed, value)
	}
	switch (field.type.kind) {
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
		case "string": {
			if (typeof value !== "string") {
				throw literalShapeError(context, "string", value)
			}
			return { kind: "string", value }
		}
		case "fixedBytes": {
			if (!(value instanceof Uint8Array)) {
				throw literalShapeError(context, "Uint8Array", value)
			}
			return { kind: "fixedBytes", value }
		}
		case "interval":
			return taggedAtElementDomain(context, field.type.element, value)
	}
}

/**
 * Tags one closed-reference literal: the branded id, re-verified against
 * the roster (the belt the type level cannot provide against forged
 * brands) and tagged u64 — queries cross ids, never handle names.
 */
function taggedHandleId(context: string, closed: NonNullable<FieldData["closed"]>, value: unknown): TaggedValue {
	if (typeof value !== "bigint") {
		throw literalShapeError(context, `a ${closed.name} handle id (bigint)`, value)
	}
	if (closed.handles[Number(value)] === undefined) {
		throw errors.new(
			`${context}: closed relation ${closed.name} has no handle with id ${value} (roster holds ${closed.handles.length})`
		)
	}
	return { kind: "u64", value }
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
	if (isIntervalShaped(value)) {
		if (element === "u64") {
			return { kind: "intervalU64", start: value.start, end: value.end }
		}
		return { kind: "intervalI64", start: value.start, end: value.end }
	}
	throw literalShapeError(context, "bigint (point) or { start, end } (interval)", value)
}

/**
 * Tags one host literal at a COMPARISON position, where no field position
 * directs the type: the sibling term's element domain does — a measure
 * sibling is u64, an interval-field sibling contributes its element type
 * (so both a point literal in `covers` and a `span` literal in `allen`
 * tag correctly), a scalar sibling its own type. At `pointIn` the operand
 * order is interval-left, point-right (`ir::CmpOp::PointIn`), so an
 * interval-shaped literal beside a scalar element-typed sibling is the
 * LEGAL lhs of `covers(span(...), t)` and tags as the interval of the
 * sibling's element domain; under every other operator an interval shape
 * against a scalar sibling stays refused (the engine's IllegalComparison
 * — eq/lt interval-vs-scalar is not a comparison).
 */
function taggedCmpLiteral(
	context: string,
	sibling: FieldData | "measure",
	value: unknown,
	op: CmpOpData["kind"]
): TaggedValue {
	if (sibling === "measure") {
		if (typeof value !== "bigint") {
			throw literalShapeError(context, "bigint (the measure is u64)", value)
		}
		return { kind: "u64", value }
	}
	if (sibling.type.kind === "interval") {
		return taggedAtElementDomain(context, sibling.type.element, value)
	}
	if (op === "pointIn" && (sibling.type.kind === "u64" || sibling.type.kind === "i64") && isIntervalShaped(value)) {
		return taggedAtElementDomain(context, sibling.type.kind, value)
	}
	return taggedLiteral(context, sibling, value)
}

/** The shared lowering context of one `lowerQuery` run. */
interface LowerContext {
	readonly theory: AnySchema
	readonly relationIds: ReadonlyMap<string, number>
	readonly predicateIds: ReadonlyMap<PredicateData, number>
	readonly registry: QueryRegistry
	/** Every dense param id some lowered term referenced — the usage census the dead-declaration refusal reads. */
	readonly usedParams: Set<number>
}

/** One rule's dense variable numbering: first occurrence in written order. */
interface VarIds {
	of(variable: AnyVar): number
	synthetic(): number
}

/** Creates one rule-scoped variable numberer. */
function makeVarIds(registry: QueryRegistry): VarIds {
	const assigned = new Map<AnyVar, number>()
	const state = { next: 0 }
	return {
		of(variable) {
			const existing = assigned.get(variable)
			if (existing !== undefined) {
				return existing
			}
			if (!registry.vars.has(variable)) {
				throw errors.new(
					`query lowering: the variable declared from ${variable.relation}.${variable.field} belongs to a different query scope`
				)
			}
			const id = state.next
			state.next += 1
			assigned.set(variable, id)
			return id
		},
		synthetic() {
			const id = state.next
			state.next += 1
			return id
		}
	}
}

/** Resolves a param term to its dense positional id, recording the use. */
function paramIdOf(ctx: LowerContext, value: AnyTerm, name: string): number {
	const index = ctx.registry.paramIndex.get(value)
	if (index === undefined) {
		throw errors.new(`query lowering: param ${name} belongs to a different query scope`)
	}
	ctx.usedParams.add(index)
	return index
}

/** Lowers one atom (either polarity) and appends any `oneOf` disjunctions. */
function lowerAtom(
	ctx: LowerContext,
	atom: MatchAtom<ParamsRecord>,
	ids: VarIds,
	extraConditions: ConditionTreeIr[]
): AtomIr {
	const source = lowerSource(ctx, atom.source)
	const bindings: Array<readonly [number, TermIr]> = atom.bindings.map(function lowerBinding(binding) {
		return [
			fieldOrdinal(atom.source, binding),
			lowerBindingTerm(ctx, atom.source, binding, ids, extraConditions)
		] as const
	})
	return { source, bindings }
}

/** Lowers an atom source to its numeric id, verifying scope membership. */
function lowerSource(ctx: LowerContext, source: AtomSourceData): AtomIr["source"] {
	if (source.kind === "relation") {
		const member = ctx.theory.relations[source.relation.name]
		if (member !== source.relation) {
			throw errors.new(
				`query lowering: relation ${source.relation.name} is not the relation value schema ${ctx.theory.name} declares`
			)
		}
		const id = ctx.relationIds.get(source.relation.name)
		if (id === undefined) {
			throw errors.new(`query lowering: relation ${source.relation.name} has no ordinal`)
		}
		return { kind: "edb", relation: id }
	}
	const pred = ctx.predicateIds.get(source.pred)
	if (pred === undefined) {
		throw errors.new(`query lowering: predicate ${source.pred.name} was declared in a different query scope`)
	}
	return { kind: "idb", pred }
}

/** A binding's field ordinal: declaration index (relations) or head position (predicates). */
function fieldOrdinal(source: AtomSourceData, binding: BindingEntry): number {
	if (source.kind === "relation") {
		const ordinal = source.relation.data.fields.findIndex(function byName(candidate) {
			return candidate.name === binding.field
		})
		if (ordinal < 0) {
			throw errors.new(`query lowering: relation ${source.relation.name} has no field ${binding.field}`)
		}
		return ordinal
	}
	const ordinal = source.pred.columns.findIndex(function byName(candidate) {
		return candidate.name === binding.field
	})
	if (ordinal < 0) {
		throw errors.new(`query lowering: predicate ${source.pred.name} has no column ${binding.field}`)
	}
	return ordinal
}

/** Lowers one binding term; an `oneOf` mints a fresh variable + disjunction. */
function lowerBindingTerm(
	ctx: LowerContext,
	source: AtomSourceData,
	binding: BindingEntry,
	ids: VarIds,
	extraConditions: ConditionTreeIr[]
): TermIr {
	const context = `${sourceName(source)}.${binding.field}`
	const bound = binding.term
	if (bound.kind === "term") {
		const value = bound.value
		switch (value[term]) {
			case "var":
				return { kind: "var", var: ids.of(value) }
			case "param":
				return { kind: "param", param: paramIdOf(ctx, value, value.name) }
			case "paramSet":
				return { kind: "paramSet", param: paramIdOf(ctx, value, value.name) }
			case "maskParam":
				throw errors.new(`${context}: an Allen-mask param is not a field-typed binding`)
		}
	}
	if (bound.kind === "oneOf") {
		const minted = ids.synthetic()
		const leaves: ConditionTreeIr[] = bound.values.map(function equalityLeaf(candidate) {
			return {
				kind: "leaf",
				cmp: {
					op: { kind: "eq" },
					lhs: { kind: "var", var: minted },
					rhs: { kind: "literal", value: taggedLiteral(context, binding.data, candidate) }
				}
			}
		})
		extraConditions.push({ kind: "or", children: leaves })
		return { kind: "var", var: minted }
	}
	return { kind: "literal", value: taggedLiteral(context, binding.data, bound.value) }
}

/** Lowers one comparison side; literals tag by the sibling's element domain (op-aware at `pointIn`). */
function lowerCmpTerm(ctx: LowerContext, side: CmpTerm, sibling: CmpTerm, ids: VarIds, op: CmpOpData["kind"]): TermIr {
	if (side.kind === "term") {
		const value = side.value
		switch (value[term]) {
			case "var":
				return { kind: "var", var: ids.of(value) }
			case "param":
				return { kind: "param", param: paramIdOf(ctx, value, value.name) }
			case "paramSet":
				return { kind: "paramSet", param: paramIdOf(ctx, value, value.name) }
			case "maskParam":
				throw errors.new(
					"query lowering: an Allen-mask param is not a comparison term — masks live in allen()'s mask position"
				)
		}
	}
	if (side.kind === "measure") {
		return { kind: "measure", var: ids.of(side.over) }
	}
	if (sibling.kind === "term") {
		const value = sibling.value
		if (value[term] === "maskParam") {
			throw errors.new("query lowering: an Allen-mask param cannot type a literal side")
		}
		return {
			kind: "literal",
			value: taggedCmpLiteral("comparison literal", value.data, side.value, op)
		}
	}
	if (sibling.kind === "measure") {
		return {
			kind: "literal",
			value: taggedCmpLiteral("comparison literal", "measure", side.value, op)
		}
	}
	throw errors.new("query lowering: a comparison without a variable or parameter side is constant-valued")
}

/** Lowers one comparison. */
function lowerComparison(ctx: LowerContext, cmp: ComparisonItem<ParamsRecord>, ids: VarIds): ComparisonIr {
	const op = cmp.op
	if (op.kind === "allen") {
		const mask =
			op.mask.kind === "literal"
				? ({ kind: "literal", mask: op.mask.mask } as const)
				: ({ kind: "param", param: paramIdOf(ctx, op.mask.param, op.mask.param.name) } as const)
		return {
			op: { kind: "allen", mask },
			lhs: lowerCmpTerm(ctx, cmp.lhs, cmp.rhs, ids, "allen"),
			rhs: lowerCmpTerm(ctx, cmp.rhs, cmp.lhs, ids, "allen")
		}
	}
	return {
		op: { kind: op.kind },
		lhs: lowerCmpTerm(ctx, cmp.lhs, cmp.rhs, ids, op.kind),
		rhs: lowerCmpTerm(ctx, cmp.rhs, cmp.lhs, ids, op.kind)
	}
}

/** Lowers one condition node (comparison leaf or and/or tree). */
function lowerCondition(ctx: LowerContext, condition: AnyCondition, ids: VarIds): ConditionTreeIr {
	if (condition.item === "cmp") {
		return { kind: "leaf", cmp: lowerComparison(ctx, condition, ids) }
	}
	return {
		kind: condition.op,
		children: condition.children.map(function lowerChild(child) {
			return lowerCondition(ctx, child, ids)
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
	const aggregate = entry.aggregate
	switch (aggregate.op) {
		case "count":
			return { kind: "aggregate", op: { kind: "count" } }
		case "countDistinct":
			return { kind: "aggregate", op: { kind: "countDistinct" }, over: ids.of(aggregate.over) }
		case "fold": {
			const over = aggregate.over
			if (isTerm(over)) {
				return { kind: "aggregate", op: { kind: aggregate.fold }, over: ids.of(over) }
			}
			return { kind: "aggregateMeasure", op: { kind: aggregate.fold }, over: ids.of(over.measure) }
		}
		case "arg":
			return {
				kind: "aggregate",
				op: { kind: aggregate.direction, key: ids.of(aggregate.key) },
				over: ids.of(aggregate.over)
			}
		case "pack":
			return { kind: "aggregate", op: { kind: "pack" }, over: ids.of(aggregate.over) }
	}
}

/** One aggregate's var-free head-op kind (`AggOp::head_op`). */
function headOpOf(aggregate: AggregateData): HeadOpIr {
	switch (aggregate.op) {
		case "count":
			return "count"
		case "countDistinct":
			return "countDistinct"
		case "fold":
			return aggregate.fold
		case "arg":
			return aggregate.direction
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
	return { kind: "aggregate", op: headOpOf(entry.aggregate) }
}

/** Lowers one rule: body walked in written order, finds supplied per shape. */
function lowerRule(ctx: LowerContext, body: readonly AnyBodyItem[], finds: (ids: VarIds) => FindTermIr[]): RuleIr {
	const ids = makeVarIds(ctx.registry)
	const atoms: AtomIr[] = []
	const negated: AtomIr[] = []
	const conditions: ConditionTreeIr[] = []
	const extraConditions: ConditionTreeIr[] = []
	for (const item of body) {
		if (item.item === "atom") {
			const lowered = lowerAtom(ctx, item, ids, extraConditions)
			if (item.negated) {
				negated.push(lowered)
			} else {
				atoms.push(lowered)
			}
			continue
		}
		conditions.push(lowerCondition(ctx, item, ids))
	}
	return {
		finds: finds(ids),
		atoms,
		negated,
		conditions: [...conditions, ...extraConditions]
	}
}

/**
 * Lowers a query value to the bridge's `ProgramIr` — pure and stable: the
 * declared predicates in declaration order (`PredId` = index), the output
 * predicate (built from `rules` + `select`) appended last. Relations lower
 * by declaration ordinal, the law the engine's own manifest pins;
 * `db.prepare` re-verifies the alignment against the live manifest before
 * sending.
 */
function lowerQuery(q: AnyQuery): ProgramIr {
	const theory = q.schema
	const relationIds = new Map<string, number>()
	Object.keys(theory.relations).forEach(function assignOrdinal(name, index) {
		relationIds.set(name, index)
	})
	const registry = q.data.registry
	const predicateIds = new Map<PredicateData, number>()
	registry.predicates.forEach(function assignPredId(pred, index) {
		predicateIds.set(pred, index)
	})
	const ctx: LowerContext = { theory, relationIds, predicateIds, registry, usedParams: new Set() }
	const predicates: PredicateDefIr[] = registry.predicates.map(function lowerPredicate(pred) {
		return {
			head: pred.columns.map(function boundHead(): HeadTermIr {
				return { kind: "var" }
			}),
			rules: pred.rules.map(function lowerClause(rule) {
				return lowerRule(ctx, rule.body, function clauseFinds(ids) {
					return rule.finds.map(function findVar(variable): FindTermIr {
						return { kind: "var", var: ids.of(variable) }
					})
				})
			})
		}
	})
	predicates.push({
		head: q.data.select.map(headTermOf),
		rules: q.data.rules.map(function lowerOutputRule(body) {
			return lowerRule(ctx, body, function outputFinds(ids) {
				return q.data.select.map(function findOf(column) {
					return lowerFind(column.entry, ids)
				})
			})
		})
	})
	/**
	 * The dead-declaration refusal: a declared param no rule reached is
	 * unexecutable BOTH ways — its inferred `Params` contribution is nothing
	 * (params ride item phantoms), yet the wire marshal is the full registry
	 * in declaration order, while the ENGINE's arity is usage-derived. The
	 * contradiction is refused here, the earliest seam that knows usage, so
	 * every query that lowers has registry == used set (and the dense-id
	 * hole a skipped middle declaration would open never exists).
	 */
	registry.params.forEach(function assertUsed(entry, index) {
		if (!ctx.usedParams.has(index)) {
			throw errors.new(
				`query declares param ${entry.name} but no rule uses it — remove the declaration or reference it`
			)
		}
	})
	return { predicates, output: registry.predicates.length }
}

export type {
	AnyQuery,
	BuildParams,
	Query,
	QueryBuild,
	QueryData,
	QueryParams,
	QueryRow,
	Scope,
	SelectColumn,
	SelectEntryData
}
export { lowerQuery, query, taggedLiteral }
