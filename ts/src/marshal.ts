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

/**
 * The fresh-mark probe: `true` exactly for a `.fresh`-marked u64 descriptor
 * (the S1 kernel's one structural mark — an unmarked u64's `fresh` property
 * holds the MARKED descriptor, so the probe compares against the literal
 * `true`, never truthiness).
 */
function isFreshField(field: AnyField): boolean {
	return "fresh" in field && field.fresh === true
}

/**
 * The inferred object type `tx.insert` returns: one property per
 * fresh-marked field of `R`, carrying the minted (or resupplied) id as a
 * bare `bigint`. A relation with no fresh field returns the empty object.
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
 * see (the schema's statement list is not carried in the schema's type,
 * only in the KeyStatement values themselves), so the type admits any partial fact
 * and the projection is verified at runtime — a missing key field throws
 * naming the projection.
 */
type KeyFact<R extends AnyRelation> = [FreshKeys<R>] extends [never]
	? Partial<Fact<R>>
	: { [K in FreshKeys<R>]: Fact<R>[K] }

/**
 * Reprojects any host object to a string-indexed record — the boundary
 * through which generic fact objects (whose type parameters carry no index
 * signature) enter the name-directed marshaling below, without a cast. An
 * ALLOCATION-FREE IDENTITY (the admission predicate is the type
 * reprojection; the value passes through untouched): every consumer
 * downstream — `rowOf`, `keyRowOf`, the query param marshal — only READS
 * properties, so no copy is warranted. The one mutating consumer
 * (`mintFreshCells` on the insert path) takes its own spread copy at the
 * call site, so the caller's fact object is never written through this
 * seam.
 */
function recordOf(fact: object): Readonly<Record<string, unknown>> {
	if (!isStringIndexed(fact)) {
		throw errors.new("fact object is not string-indexable")
	}
	return fact
}

/**
 * The trusted admission seam of {@link recordOf}: every JS object IS
 * string-indexable (property reads on absent names yield `undefined`,
 * which every consumer already guards), so the predicate verifies the one
 * checkable fact — objecthood — and admits the value at the indexed type
 * without a copy or a cast.
 */
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

/**
 * The read half of the closed bijection: one u64 row id back to its handle
 * NAME (`Number(cell)` is safe — the sealed extension holds at most 256
 * rows, engine law). An id outside the roster THROWS pointed, never a
 * silent fallback and never `undefined`: the state is reachable only in a
 * store whose closed-typed column was never pinned by its containment law,
 * and the error names that missing piece.
 */
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

/**
 * Marshals one host cell at its field position to the natural wire value,
 * schema-directed by the field descriptor's structural kind (never
 * guessed). A closed-referencing cell arrives as its handle NAME and
 * lowers through {@link closedCellOf} — the arm precedes the switch
 * because a closed reference is structurally a u64 descriptor plus the
 * roster (the same precedence `Infer` pins at the type level). Everything
 * else is bare, so the runtime values ARE the wire's natural JS values;
 * widths and domain labels are the engine's own judgment at the write
 * boundary.
 */
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
			/**
			 * A lone surrogate would be lossily replaced with U+FFFD at the
			 * bridge's UTF-8 crossing — the stored fact would differ from the
			 * written one, and distinct JS strings would collapse to one fact.
			 * The bijection law refuses it here, the one seam every write and
			 * lookup lowers through.
			 */
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
 * IS a fact of its relation — the engine admitted the row, and the values
 * are BARE structural values, so nothing is asserted beyond presence (no
 * brand exists to re-derive; the store is the proof carrier).
 */
function isCompleteFact<R extends AnyRelation>(
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
 * fresh ids as bare bigints — same presence-only direction as
 * {@link isCompleteFact}.
 */
function isMintedFresh<R extends AnyRelation>(
	relation: R,
	minted: Readonly<Record<string, FactValue>>
): minted is Readonly<Record<string, FactValue>> & Minted<R> {
	return relation.data.fields.every(function presentWhenFresh(declared) {
		return !isFreshField(declared.field) || minted[declared.name] !== undefined
	})
}

/**
 * Unmarshals one positional row to the relation's named, frozen fact object
 * of bare structural values — the inverse of {@link rowOf},
 * ordinal-directed by the same declaration order. Closed-referencing cells
 * lift id → handle NAME through {@link handleOf} (the read half of the
 * bijection), so every fact a user sees speaks the roster's vocabulary.
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

export type { KeyFact, Minted }
export { cellOf, factOf, handleOf, isFreshField, isMintedFresh, keyRowOf, recordOf, rowOf }
