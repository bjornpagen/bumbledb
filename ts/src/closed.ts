/**
 * Closed relations (`docs/architecture/10-data-model.md` § closed
 * relations): a vocabulary whose extension is declared in the schema — two
 * tiers, one function. The emission per closed relation mirrors the macro's
 * (host-enum analog): handle CONSTANTS on the value (`Kind.Checking`, ids =
 * declaration order), the `fromId` weld, an `id` field constructor
 * pre-branded with the handle newtype for other relations' field blocks
 * (`kind: Kind.id`), and payload readback (`Kind.axioms`). No fact type and
 * no insert surface exist — closed relations are unwritable by
 * construction: the value simply lacks the writable relation shape.
 */

import * as errors from "@superbuilders/errors"
import type { Brand } from "#brand.ts"
import {
	type AnyField,
	assertDeclarationOrderKey,
	type ClosedIdField,
	type ClosedRoster,
	type FieldData,
	type FieldValue,
	fieldData,
	literalOf
} from "#fields.ts"
import type { LiteralSpec } from "#spec.ts"

/**
 * The value-surface property names a handle may not shadow — the macro's
 * name-collision diagnostic, here over the closed value's own properties
 * (`relation`/`selection` are reserved so a closed value can never be
 * mistaken for a selected relation by `on()`'s discriminant).
 */
const reservedHandleNames: readonly string[] = Object.freeze([
	"name",
	"id",
	"data",
	"axioms",
	"fromId",
	"relation",
	"selection"
])

/**
 * Reads one handle's axiom row, refusing absence loudly: the payload tier's
 * overload types the record exhaustively, so a missing row is ill-typed
 * input, and the bare tier never reaches here (it declares no columns).
 */
function axiomRow(
	name: string,
	axioms: Readonly<Record<string, Readonly<Record<string, unknown>>>> | undefined,
	handle: string
): Readonly<Record<string, unknown>> {
	if (axioms === undefined) {
		throw errors.new(`closed relation ${name}: payload columns declared without ground axioms`)
	}
	const row = axioms[handle]
	if (row === undefined) {
		throw errors.new(`closed relation ${name}: no ground axiom for handle ${handle}`)
	}
	return row
}

/**
 * A payload column of a closed relation: any field constructor except a
 * fresh-marked one (a vocabulary's rows are ground axioms, never minted).
 */
type PayloadField = AnyField & { readonly data: FieldData<false> }

/** One declared payload column: name plus its field description. */
interface ClosedColumn {
	readonly name: string
	readonly field: FieldData
}

/**
 * One ground axiom, already lowered: the handle plus one wire literal per
 * declared column in column-declaration order (row id = index). Lowered
 * EAGERLY at construction so axiom literals ride the same selection-literal
 * machine as `where()` bindings, with the same errors (the macro's rule).
 */
interface ClosedRow {
	readonly handle: string
	readonly values: readonly LiteralSpec[]
}

/** A closed relation's runtime description. */
interface ClosedData {
	readonly name: string
	readonly handles: readonly string[]
	readonly columns: readonly ClosedColumn[]
	readonly rows: readonly ClosedRow[]
}

/** One axiom row as the host writes and reads it: column name to host value. */
type AxiomRow<Cols> = { readonly [C in keyof Cols]: FieldValue<Cols[C]> }

/**
 * The whole axiom record: every handle exactly once, every column exactly
 * once per row — a missing or extra axiom, column, or handle is a TYPE
 * error (mapped over the handle tuple).
 */
type Axioms<Handles extends readonly string[], Cols> = {
	readonly [H in Handles[number]]: AxiomRow<Cols>
}

/**
 * The named surface of a closed relation value, minus the handle constants
 * (which {@link Closed} intersects in).
 */
interface ClosedCore<Name extends string, Handles extends readonly string[], Cols> {
	readonly name: Name
	/**
	 * The pre-branded u64 field constructor: `kind: Kind.id` in another
	 * relation's field block is the reference through which bare handles
	 * become legal in that relation's selections.
	 */
	readonly id: ClosedIdField<Name>
	readonly data: ClosedData
	/** Payload readback: handle to its declared column values. */
	readonly axioms: Axioms<Handles, Cols>
	/** The weld: declaration-order id back to its handle, or undefined beyond the roster. */
	fromId(id: Brand<bigint, Name>): Handles[number] | undefined
}

/**
 * A closed relation value: the core surface plus one branded constant per
 * handle (`Kind.Checking: Brand<bigint, "Kind">`, ids = declaration order).
 */
type Closed<Name extends string, Handles extends readonly string[], Cols> = ClosedCore<Name, Handles, Cols> & {
	readonly [H in Handles[number]]: Brand<bigint, Name>
}

/** Any closed relation value, whatever its roster and columns. */
interface AnyClosed {
	readonly name: string
	readonly id: ClosedIdField<string>
	readonly data: ClosedData
	readonly axioms: Readonly<Record<string, object>>
}

/** Bare tier: `closed("Kind", ["Checking", "Savings"])` — handles only. */
function closed<const Name extends string, const Handles extends readonly [string, ...string[]]>(
	name: Name,
	handles: Handles
): Closed<Name, Handles, Record<never, never>>

/**
 * Payload tier: declared columns plus ground axioms, every handle with
 * every column exactly once (type-enforced by {@link Axioms}).
 */
function closed<
	const Name extends string,
	const Handles extends readonly [string, ...string[]],
	const Cols extends Record<string, PayloadField>
>(name: Name, handles: Handles, columns: Cols, axioms: Axioms<Handles, Cols>): Closed<Name, Handles, Cols>

function closed(
	name: string,
	handles: readonly [string, ...string[]],
	columns?: Record<string, PayloadField>,
	axioms?: Readonly<Record<string, Readonly<Record<string, unknown>>>>
): unknown {
	const seen = new Set<string>()
	for (const handle of handles) {
		if (seen.has(handle)) {
			throw errors.new(`closed relation ${name}: duplicate handle ${handle}`)
		}
		seen.add(handle)
		if (reservedHandleNames.includes(handle)) {
			throw errors.new(
				`closed relation ${name}: handle ${handle} collides with the closed value's own surface (${reservedHandleNames.join(", ")})`
			)
		}
	}
	const roster: ClosedRoster = Object.freeze({ name, handles: Object.freeze([...handles]) })
	const cols: ClosedColumn[] = []
	if (columns !== undefined) {
		for (const [columnName, field] of Object.entries(columns)) {
			assertDeclarationOrderKey(`closed relation ${name} column`, columnName)
			cols.push(Object.freeze({ name: columnName, field: field.data }))
		}
	}
	const rows: ClosedRow[] = handles.map(function lowerRow(handle) {
		const values = cols.map(function lowerAxiomLiteral(column) {
			const row = axiomRow(name, axioms, handle)
			return Object.freeze(literalOf(column.field, row[column.name]))
		})
		return Object.freeze({ handle, values: Object.freeze(values) })
	})
	const data: ClosedData = Object.freeze({
		name,
		handles: roster.handles,
		columns: Object.freeze(cols),
		rows: Object.freeze(rows)
	})
	const id: ClosedIdField<string> = Object.freeze({
		data: fieldData({ kind: "u64" }, name, false, roster)
	})
	/**
	 * Handle names are arbitrary identifiers, so rows and constants are
	 * minted with OWN-property definition, never assignment: a handle named
	 * "__proto__" would otherwise ride the Object.prototype accessor —
	 * silently swapping the record's prototype instead of creating the row,
	 * and no-oping the constant (a primitive through the setter) — minting a
	 * value whose type claims Brand<bigint, Name> but reads back an object.
	 */
	const axiomsOut: Record<string, object> = {}
	for (const handle of handles) {
		const row = axioms === undefined ? Object.freeze({}) : Object.freeze({ ...axiomRow(name, axioms, handle) })
		Object.defineProperty(axiomsOut, handle, { value: row, enumerable: true })
	}
	const value: Record<string, unknown> = {
		name,
		id,
		data,
		axioms: Object.freeze(axiomsOut),
		fromId(idValue: bigint): string | undefined {
			return roster.handles[Number(idValue)]
		}
	}
	handles.forEach(function mintHandleConstant(handle, index) {
		Object.defineProperty(value, handle, { value: BigInt(index), enumerable: true })
	})
	return Object.freeze(value)
}

export type { AnyClosed, AxiomRow, Axioms, Closed, ClosedColumn, ClosedCore, ClosedData, ClosedRow, PayloadField }
export { closed }
