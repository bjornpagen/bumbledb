/**
 * The marshal layer (PRD-07): fact object ⇄ positional `FactValue[]` by
 * field ordinal, schema-directed, in ONE place only. The write side lowers
 * named host objects to rows in the relation's field-declaration order
 * (declaration order = ordinal ids, the macro's law); the read side decodes
 * rows back to named objects with brands applied by assertion — the store
 * is the proof carrier: a row the engine admitted IS a legal fact of its
 * relation, so readback asserts the brand instead of re-judging the value
 * (the same trust direction as Rust's typed readback). Shape mismatches
 * here are genuine failures and THROW typed; they are never domain data.
 */

import * as errors from "@superbuilders/errors"
import type { FieldData } from "#fields.ts"
import type { FactValue } from "#native.ts"
import type { AnyRelation, Fact, FreshKeys, RelationData } from "#relation.ts"

/**
 * The inferred object type `tx.insert` returns: one property per
 * fresh-marked field of `R`, carrying the minted (or resupplied) branded
 * id. A relation with no fresh field returns the empty object.
 */
type Minted<R extends AnyRelation> = { [K in FreshKeys<R>]: Fact<R>[K] }

/**
 * The key object `get` reads through. THE PRIMARY-KEY RULE: `get` always
 * reads through the PRIMARY candidate key — the first-declared one in the
 * engine's materialized statement order (fresh-implied keys first, closed
 * auto-keys second, declared `key()` statements last), so a fresh-bearing
 * relation's primary key is always its fresh field. When `R` carries a
 * fresh field the type demands exactly that field; otherwise the primary
 * key lives in the schema's statement list, which the type system cannot
 * see (statements are untyped values), so the type admits any partial fact
 * and the projection is verified at runtime — a missing key field throws
 * naming the projection.
 */
type KeyFact<R extends AnyRelation> = [FreshKeys<R>] extends [never]
	? Partial<Fact<R>>
	: { [K in FreshKeys<R>]: Fact<R>[K] }

/** The typed shape refusal of the row marshaler — a genuine failure, never data. */
function cellShapeError(context: string, expected: string, value: unknown): Error {
	return errors.new(`${context}: expected ${expected}, got ${typeof value}`)
}

/** Narrows an interval cell: a plain object with bigint start/end. */
function isIntervalCell(value: unknown): value is { readonly start: bigint; readonly end: bigint } {
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
 * Reprojects any host object to a string-indexed record — the boundary
 * through which generic fact objects (whose type parameters carry no index
 * signature) enter the name-directed marshaling below, without a cast.
 */
function recordOf(fact: object): Record<string, unknown> {
	return Object.fromEntries(Object.entries(fact))
}

/**
 * Marshals one host cell at its field position to the natural wire value,
 * schema-directed by the field's structural type (never guessed). Brands
 * are phantoms, so the runtime values are exactly the wire's natural JS
 * values; only widths and domains are left to the engine's own judge.
 */
function cellOf(context: string, field: FieldData, value: unknown): FactValue {
	switch (field.type.kind) {
		case "bool": {
			if (typeof value !== "boolean") {
				throw cellShapeError(context, "boolean", value)
			}
			return value
		}
		case "u64":
		case "i64": {
			if (typeof value !== "bigint") {
				throw cellShapeError(context, "bigint", value)
			}
			return value
		}
		case "string": {
			if (typeof value !== "string") {
				throw cellShapeError(context, "string", value)
			}
			/**
			 * A lone surrogate would be lossily replaced with U+FFFD at the
			 * bridge's UTF-8 crossing — the stored fact would differ from the
			 * written one, and distinct JS strings would collapse to one fact.
			 * The bijection law refuses it here, the one seam every write and
			 * lookup lowers through.
			 */
			if (!value.isWellFormed()) {
				throw cellShapeError(context, "well-formed string", value)
			}
			return value
		}
		case "fixedBytes": {
			if (!(value instanceof Uint8Array)) {
				throw cellShapeError(context, "Uint8Array", value)
			}
			return value
		}
		case "interval": {
			if (!isIntervalCell(value)) {
				throw cellShapeError(context, "interval ({ start, end } bigints)", value)
			}
			return { start: value.start, end: value.end }
		}
	}
}

/**
 * Marshals one complete fact object to its positional row, in field
 * declaration order (= ordinal ids). Every declared field must be present;
 * fresh minting happens BEFORE this point (the transaction fills omitted
 * fresh cells via the engine's alloc lane).
 */
function rowOf(relation: RelationData, fact: Readonly<Record<string, unknown>>): FactValue[] {
	return relation.fields.map(function marshalCell(declared) {
		const value = fact[declared.name]
		if (value === undefined) {
			throw errors.new(`relation ${relation.name}: fact is missing field ${declared.name}`)
		}
		return cellOf(`relation ${relation.name} field ${declared.name}`, declared.field, value)
	})
}

/**
 * Marshals a key object through a key statement's projection, in the
 * statement's projection order (what the engine's keyed point reads take).
 * A key field absent from the object throws naming the primary projection
 * (the {@link KeyFact} rule's runtime half).
 */
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

/**
 * The read-side trusted seam: a decoded row carrying every declared field
 * IS a fact of its relation — the engine admitted it, so the brand is
 * asserted, not re-derived. The checkable half (completeness) is verified;
 * the phantom half (brands) is carried by the store's own judgment.
 */
function isBrandedFact<R extends AnyRelation>(
	relation: R,
	decoded: Readonly<Record<string, FactValue>>
): decoded is Readonly<Record<string, FactValue>> & Fact<R> {
	return relation.data.fields.every(function present(declared) {
		return decoded[declared.name] !== undefined
	})
}

/**
 * The insert-return trusted seam: the collected fresh cells of one insert
 * (minted by the engine or resupplied by the caller) are the relation's
 * branded fresh ids — same assertion direction as {@link isBrandedFact}.
 */
function isMintedFresh<R extends AnyRelation>(
	relation: R,
	minted: Readonly<Record<string, FactValue>>
): minted is Readonly<Record<string, FactValue>> & Minted<R> {
	return relation.data.fields.every(function presentWhenFresh(declared) {
		return !declared.field.minted || minted[declared.name] !== undefined
	})
}

/**
 * Unmarshals one positional row to the relation's named, branded, frozen
 * fact object — the inverse of {@link rowOf}, ordinal-directed by the same
 * declaration order.
 */
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
		decoded[declared.name] = cell
	})
	Object.freeze(decoded)
	if (!isBrandedFact(relation, decoded)) {
		throw errors.new(`relation ${data.name}: decoded row is not a complete fact`)
	}
	return decoded
}

export type { KeyFact, Minted }
export { cellOf, factOf, isMintedFresh, keyRowOf, recordOf, rowOf }
