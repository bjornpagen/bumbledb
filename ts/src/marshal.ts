/**
 * The marshal layer: fact object ⇄ positional `FactValue[]` by field
 * ordinal, schema-directed, in ONE place only. The write side lowers named
 * host objects to rows in the relation's field-declaration order
 * (declaration order = ordinal ids, the macro's law); the read side decodes
 * rows back to named objects of BARE structural values — the marshal
 * boundary is pure both ways. CAST-FREE, LITERALLY: with structural values
 * there is no brand to assert on the way out (the historical "one
 * sanctioned marshal cast" died with the brand era), so product code
 * carries zero casts — the only trusted seams are the completeness
 * PREDICATES below, which verify the checkable half (every declared field
 * present) and rely on the store as the proof carrier for the rest: a row
 * the engine admitted IS a legal fact of its relation (the same trust
 * direction as Rust's typed readback). Shape mismatches here are genuine
 * failures and THROW typed; they are never domain data.
 *
 * THE CLOSED BIJECTION (0.4.0): a closed-referencing cell crosses this
 * boundary as its handle NAME — the write side lowers name → u64 row id
 * (declaration order = row ids, the sealed roster's own law, ≤ 256 rows),
 * the read side lifts id → name, and both directions are total and static
 * over the roster. An unknown name is a pointed THROW at the write seam —
 * a deliberate UPGRADE over 0.3.0, where any bigint sailed through the
 * marshal to a commit-time containment violation; the wrong spelling now
 * dies here, before the engine ever sees the row. An out-of-roster id on
 * the read side (reachable only in a store whose closed-typed column was
 * never pinned by its containment law) is equally pointed — never a
 * silent fallback, never `undefined`.
 */

import * as errors from "@superbuilders/errors"
import type { AnyField, ClosedRoster } from "#fields.ts"
import { isIntervalValue, literalShapeError, rosterOf } from "#fields.ts"
import type { FactValue } from "#native.ts"
import type { AnyRelation, Fact, FreshKeys, RelationData } from "#relation.ts"

function isFreshField(field: AnyField): boolean {
	return "fresh" in field && field.fresh === true
}

type KeyFact<R extends AnyRelation> = [FreshKeys<R>] extends [never]
	? Partial<Fact<R>>
	: { [K in FreshKeys<R>]: Fact<R>[K] }

function recordOf(fact: object): Readonly<Record<string, unknown>> {
	if (!isStringIndexed(fact)) {
		throw errors.new("fact object is not string-indexable")
	}
	return fact
}

function isStringIndexed(value: object): value is Readonly<Record<string, unknown>> {
	return typeof value === "object" || typeof value === "function"
}

/**
 * The write half of the closed bijection: one handle NAME to its u64 row
 * id (declaration order = row ids — the engine's own minting of the
 * sealed extension). An unknown name is a pointed refusal naming the
 * vocabulary and its roster — the 0.4.0 upgrade over any-bigint-compiles:
 * the wrong spelling dies at the marshal, never as a commit-time
 * violation. `indexOf` is the whole machine (the roster is ≤ 256 rows,
 * engine law — no map is warranted).
 */
function closedCellOf(context: string, closed: ClosedRoster, name: string): FactValue {
	const id = closed.handles.indexOf(name)
	if (id === -1) {
		throw errors.new(
			`${context}: "${name}" is not a handle of ${closed.name} — the roster is ${closed.handles.join(", ")}`
		)
	}
	return BigInt(id)
}

function handleOf(context: string, closed: ClosedRoster, cell: FactValue): string {
	if (typeof cell !== "bigint") {
		throw literalShapeError(context, `a ${closed.name} handle id (bigint)`, cell)
	}
	const handle = closed.handles[Number(cell)]
	if (handle === undefined) {
		throw errors.new(
			`${context}: id ${cell} is outside the ${closed.name} roster (${closed.handles.join(", ")}) — the column types ${closed.name} but no law pins it — a containment statement is the missing piece`
		)
	}
	return handle
}

function cellOf(context: string, field: AnyField, value: unknown): FactValue {
	const roster = rosterOf(field)
	if (roster !== undefined) {
		if (typeof value !== "string") {
			throw literalShapeError(context, `a ${roster.name} handle name (string)`, value)
		}
		return closedCellOf(context, roster, value)
	}
	switch (field.kind) {
		case "bool": {
			if (typeof value !== "boolean") {
				throw literalShapeError(context, "boolean", value)
			}
			return value
		}
		case "u64":
		case "i64": {
			if (typeof value !== "bigint") {
				throw literalShapeError(context, "bigint", value)
			}
			return value
		}
		case "str": {
			if (typeof value !== "string") {
				throw literalShapeError(context, "string", value)
			}

			if (!value.isWellFormed()) {
				throw literalShapeError(context, "well-formed string", value)
			}
			return value
		}
		case "bytes": {
			if (!(value instanceof Uint8Array)) {
				throw literalShapeError(context, "Uint8Array", value)
			}
			return value
		}
		case "interval": {
			if (!isIntervalValue(value)) {
				throw literalShapeError(context, "interval ({ start, end } bigints)", value)
			}
			return { start: value.start, end: value.end }
		}
	}
}

function rowOf(relation: RelationData, fact: Readonly<Record<string, unknown>>): FactValue[] {
	return relation.fields.map(function marshalCell(declared) {
		const value = fact[declared.name]
		if (value === undefined) {
			throw errors.new(`relation ${relation.name}: fact is missing field ${declared.name}`)
		}
		return cellOf(`relation ${relation.name} field ${declared.name}`, declared.field, value)
	})
}

function keyRowOf(
	relation: RelationData,
	projection: readonly string[],
	key: Readonly<Record<string, unknown>>
): FactValue[] {
	return projection.map(function marshalKeyCell(fieldName) {
		const declared = relation.fields.find(function byName(candidate) {
			return candidate.name === fieldName
		})
		if (declared === undefined) {
			throw errors.new(`relation ${relation.name}: key projection cites unknown field ${fieldName}`)
		}
		const value = key[fieldName]
		if (value === undefined) {
			throw errors.new(
				`relation ${relation.name}: key object is missing field ${fieldName} — get reads through the primary (first-declared) key, whose projection is (${projection.join(", ")})`
			)
		}
		return cellOf(`relation ${relation.name} key field ${fieldName}`, declared.field, value)
	})
}

function isCompleteFact<R extends AnyRelation>(
	relation: R,
	decoded: Readonly<Record<string, FactValue>>
): decoded is Readonly<Record<string, FactValue>> & Fact<R> {
	return relation.data.fields.every(function present(declared) {
		return decoded[declared.name] !== undefined
	})
}

function factOf<R extends AnyRelation>(relation: R, row: readonly FactValue[]): Fact<R> {
	const data = relation.data
	if (row.length !== data.fields.length) {
		throw errors.new(
			`relation ${data.name}: row arity ${row.length} does not match the ${data.fields.length} declared fields`
		)
	}
	const decoded: Record<string, FactValue> = {}
	data.fields.forEach(function decodeCell(declared, ordinal) {
		const cell = row[ordinal]
		if (cell === undefined) {
			throw errors.new(`relation ${data.name}: row cell ${ordinal} (${declared.name}) is absent`)
		}
		const roster = rosterOf(declared.field)
		decoded[declared.name] =
			roster !== undefined ? handleOf(`relation ${data.name} field ${declared.name}`, roster, cell) : cell
	})
	Object.freeze(decoded)
	if (!isCompleteFact(relation, decoded)) {
		throw errors.new(`relation ${data.name}: decoded row is not a complete fact`)
	}
	return decoded
}

export type { KeyFact }
export { cellOf, factOf, handleOf, isFreshField, keyRowOf, recordOf, rowOf }
