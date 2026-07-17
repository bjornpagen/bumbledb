/**
 * IDB predicates (PRD-08) — engine recursion as values, mirroring the IR's
 * cut exactly (`ir::Program`/`PredicateDef`/`AtomSource::Idb`;
 * `docs/architecture/20-query-ir.md` § engine recursion): `$.predicate`
 * declares a predicate with a named, field-typed head; its rules are given
 * IN the declaration (a callback receiving the predicate's own reference,
 * so self-recursion is writable and the rules are data the type system can
 * see — the params they use flow into the query's inferred `Params`); and
 * `.match({...})` uses it as a body atom, bindings addressing head
 * positions by the declared column names (lowered positionally —
 * `FieldId(i)` is head position i, exactly as the IR reads it). Strata
 * legality (no negation/aggregation through a cycle) is the ENGINE's
 * judge; its typed error surfaces at prepare.
 */

import * as errors from "@superbuilders/errors"
import { phantom } from "#brand.ts"
import { assertDeclarationOrderKey, type FieldData } from "#fields.ts"
import type { AnyBodyItem, BindingsParams, MatchAtom, TermInput } from "#query/atom.ts"
import { resolveBindings } from "#query/atom.ts"
import type { AnyVar, Flatten, ItemParams, ParamsRecord, ParamsShape, QueryRegistry, Var } from "#query/scope.ts"
import { isTerm, resolveFieldData, term } from "#query/scope.ts"
import type { FieldRef } from "#relation.ts"

/** One declared head column: name plus the field description that types it. */
interface PredicateColumn {
	readonly name: string
	readonly data: FieldData
}

/** One clause of a predicate: head projections (column order) and the body. */
interface PredicateRuleData {
	readonly finds: readonly AnyVar[]
	readonly body: readonly AnyBodyItem[]
}

/** A predicate's runtime description; identity keys the dense `PredId` at lowering. */
interface PredicateData {
	readonly name: string
	readonly columns: readonly PredicateColumn[]
	readonly rules: readonly PredicateRuleData[]
}

/** Any field reference, whatever its brand — the column-declaration position. */
type AnyFieldRef = FieldRef<string, string, unknown>

/** The columns record `$.predicate` takes: column name to typing field reference. */
type PredicateColumnsInput = Readonly<Record<string, AnyFieldRef>>

/** Extracts a field reference's host value type. */
type RefValue<T> = T extends { readonly [phantom]?: infer V } ? Exclude<V, undefined> : never

/** The head's typed column record, derived from the declaration. */
type ColumnValues<Cols> = { [K in keyof Cols]: RefValue<Cols[K]> }

/**
 * The `.match` bindings of a predicate atom: per head column, a term of
 * the column's brand; unmentioned columns are wildcards, exactly as
 * relation atoms.
 */
type PredicateBindings<ColsV> = { readonly [K in keyof ColsV]?: TermInput<ColsV[K]> }

/**
 * One rule of a predicate as the declaration callback returns it: `finds`
 * names the projected variable per head column (interior heads project
 * bound variables only — the creation quarantine; the engine's strata
 * judge enforces it), `body` is the clause's conjunction.
 */
interface PredicateRuleInput<ColsV> {
	readonly finds: { readonly [K in keyof ColsV]: Var<ColsV[K]> }
	readonly body: readonly AnyBodyItem[]
}

/**
 * The predicate's own reference, passed INTO its rules callback — the
 * fixpoint spelling: `self.match({...})` inside a rule of the same
 * predicate is the recursive atom.
 */
interface PredicateSelf<ColsV> {
	readonly data: PredicateData
	match<const B extends PredicateBindings<ColsV>>(bindings: B): MatchAtom<BindingsParams<B>>
}

/**
 * A declared predicate. `.match({...})` uses it as a body atom — in
 * another predicate's rules or in the output rules; the phantom `P`
 * carries the params its own rules contributed, so a query's `Params`
 * type sees through predicates it only reaches transitively.
 */
interface Predicate<ColsV, P extends ParamsRecord> {
	readonly data: PredicateData
	match<const B extends PredicateBindings<ColsV>>(bindings: B): MatchAtom<Flatten<P & BindingsParams<B>>>
}

/** The params contributed by a predicate's declared rules. */
type PredicateParams<Rules extends readonly { readonly body: readonly AnyBodyItem[] }[]> = ParamsShape<
	ItemParams<Rules[number]["body"][number]>
>

/** Builds one predicate atom (shared by `self.match` and `.match`). */
function predicateAtom(data: PredicateData, bindings: Readonly<Record<string, unknown>>): MatchAtom<never> {
	return Object.freeze({
		item: "atom" as const,
		negated: false,
		source: Object.freeze({ kind: "predicate" as const, pred: data }),
		bindings: resolveBindings(
			`predicate ${data.name}`,
			data.columns.map(function asField(column) {
				return { name: column.name, field: column.data }
			}),
			bindings
		)
	})
}

/**
 * Declares one predicate in the scope (the `$.predicate` implementation):
 * resolves the head columns, runs the rules callback against the
 * predicate's own reference, verifies every rule projects a declared
 * scope variable per column, and registers the predicate in declaration
 * order (= its dense `PredId`).
 */
function makePredicate<
	const Cols extends PredicateColumnsInput,
	const Rules extends readonly PredicateRuleInput<ColumnValues<Cols>>[]
>(
	registry: QueryRegistry,
	name: string,
	columns: Cols,
	rules: (self: PredicateSelf<ColumnValues<Cols>>) => Rules
): Predicate<ColumnValues<Cols>, PredicateParams<Rules>> {
	const ordered: PredicateColumn[] = []
	for (const [columnName, ref] of Object.entries(columns)) {
		assertDeclarationOrderKey(`predicate ${name} column`, columnName)
		ordered.push(
			Object.freeze({
				name: columnName,
				data: resolveFieldData(registry.theory, ref.relation, ref.field)
			})
		)
	}
	if (ordered.length === 0) {
		throw errors.new(`predicate ${name}: a predicate head needs at least one column`)
	}
	const ruleSlots: PredicateRuleData[] = []
	const data: PredicateData = Object.freeze({
		name,
		columns: Object.freeze(ordered),
		rules: ruleSlots
	})
	const self: PredicateSelf<ColumnValues<Cols>> = Object.freeze({
		data,
		match(bindings: Readonly<Record<string, unknown>>) {
			return predicateAtom(data, bindings)
		}
	})
	const declared = rules(self)
	for (const rule of declared) {
		const finds: AnyVar[] = []
		const record: Readonly<Record<string, unknown>> = Object.fromEntries(Object.entries(rule.finds))
		for (const column of ordered) {
			const found = record[column.name]
			if (!isTerm(found) || found[term] !== "var") {
				throw errors.new(
					`predicate ${name}: rule finds must project a scope variable for column ${column.name} (interior heads project bound variables only)`
				)
			}
			finds.push(found)
		}
		ruleSlots.push(Object.freeze({ finds: Object.freeze(finds), body: Object.freeze([...rule.body]) }))
	}
	Object.freeze(ruleSlots)
	registry.predicates.push(data)
	return Object.freeze({
		data,
		match(bindings: Readonly<Record<string, unknown>>) {
			return predicateAtom(data, bindings)
		}
	})
}

export type {
	AnyFieldRef,
	ColumnValues,
	Predicate,
	PredicateBindings,
	PredicateColumn,
	PredicateColumnsInput,
	PredicateData,
	PredicateParams,
	PredicateRuleData,
	PredicateRuleInput,
	PredicateSelf
}
export { makePredicate }
