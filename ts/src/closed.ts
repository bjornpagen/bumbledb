/**
 * Closed relations (`docs/architecture/10-data-model.md` § closed
 * relations): a vocabulary whose extension is declared in the schema — two
 * tiers, one function. The emission per closed relation mirrors the
 * macro's (host-enum analog): handle CONSTANTS on the value
 * (`Kind.Checking`, ids = declaration order, each a BARE `bigint` — no
 * brand), the `fromId` weld, an `id` field descriptor carrying the handle
 * DOMAIN (`"KindId"`, mirroring Rust's `closed relation Kind as KindId`)
 * for other relations' field blocks (`kind: Kind.id`), and payload
 * readback (`Kind.axioms`). Bare tier: `closed("Kind", ["Checking",
 * "Savings"])`. Payload tier: `closed("Sev", { pages: bool })({ Critical:
 * { pages: true }, ... })` — the axioms record IS the handle declaration,
 * every handle carrying every column exactly once (type-enforced). No fact
 * type and no insert surface exist — closed relations are unwritable by
 * construction: the value simply lacks the writable relation shape.
 */

import * as errors from "@superbuilders/errors"
import {
	type AnyField,
	assertDeclarationOrderKey,
	type ClosedIdField,
	type ClosedRoster,
	type Infer,
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
	"columns",
	"fromId",
	"relation",
	"selection"
])

/**
 * Reads one handle's axiom row, refusing absence loudly: the payload tier's
 * axioms record is keyed by the handles themselves (a missing row is a
 * missing handle — unrepresentable), and the bare tier never reaches here
 * (it declares no columns).
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
 * A payload column of a closed relation: any field descriptor except a
 * fresh-marked one (a vocabulary's rows are ground axioms, never minted).
 */
type PayloadField = Exclude<AnyField, { readonly fresh: true }>

/** One declared payload column: name plus its field descriptor. */
interface ClosedColumn {
	readonly name: string
	readonly field: PayloadField
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

/** One axiom row as the host writes and reads it: column name to bare structural value. */
type AxiomRow<Cols extends Record<string, PayloadField>> = { readonly [C in keyof Cols]: Infer<Cols[C]> }

/**
 * The whole axiom record: every handle exactly once, every column exactly
 * once per row — a missing or extra column is a TYPE error (each row is
 * contextually checked against the declared columns), and the handle set
 * IS the record's key set.
 */
type Axioms<Handles extends string, Cols extends Record<string, PayloadField>> = {
	readonly [H in Handles]: AxiomRow<Cols>
}

/**
 * The named surface of a closed relation value, minus the handle constants
 * (which {@link Closed} intersects in).
 */
interface ClosedCore<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>> {
	readonly name: Name
	/**
	 * The handle-domain reference descriptor: `kind: Kind.id` in another
	 * relation's field block is the reference through which bare handle ids
	 * become legal in that relation's selections. Its domain is the closed
	 * relation's handle domain (`"KindId"` — Rust's `as KindId`).
	 */
	readonly id: ClosedIdField<`${Name}Id`>
	readonly data: ClosedData
	/** Payload readback: handle to its declared column values, bare and structural. */
	readonly axioms: Axioms<Handles, Cols>
	/**
	 * The declared payload columns, name → S1 field descriptor — carried in
	 * the TYPE so a projected payload column's domain label is recoverable
	 * off the schema type (the face layer's `ProjectedDomain` reads it; the
	 * runtime twin is `data.columns`).
	 */
	readonly columns: Cols
	/** The weld: declaration-order id back to its handle, or undefined beyond the roster. */
	fromId(id: bigint): Handles | undefined
}

/**
 * A closed relation value: the core surface plus one BARE constant per
 * handle (`Kind.Checking: bigint`, ids = declaration order — the value is
 * structural; the roster judges out-of-vocabulary ids at construction and
 * the engine at commit).
 */
type Closed<Name extends string, Handles extends string, Cols extends Record<string, PayloadField>> = ClosedCore<
	Name,
	Handles,
	Cols
> & { readonly [H in Handles]: bigint }

/** Any closed relation value, whatever its roster and columns. */
interface AnyClosed {
	readonly name: string
	readonly id: ClosedIdField
	readonly data: ClosedData
	readonly axioms: Readonly<Record<string, object>>
	readonly columns: Readonly<Record<string, PayloadField>>
}

/** Narrows the two-tier second argument: a handle tuple (bare tier) or a column block (payload tier). */
function isHandleTuple(
	shape: readonly [string, ...string[]] | Record<string, PayloadField>
): shape is readonly [string, ...string[]] {
	return Array.isArray(shape)
}

/** Bare tier: `closed("Kind", ["Checking", "Savings"])` — handles only. */
function closed<const Name extends string, const Handles extends readonly [string, ...string[]]>(
	name: Name,
	handles: Handles
): Closed<Name, Handles[number], Record<never, never>>

/**
 * Payload tier: declared columns, then ground axioms — `closed("Grade",
 * { mastered: bool })({ DirectPass: { mastered: true }, Failed: { mastered:
 * false } })`. The axioms record's keys ARE the handles (declaration order
 * = key order, integer-index names rejected); every row carries every
 * column exactly once (type-enforced by {@link Axioms}).
 */
function closed<const Name extends string, const Cols extends Record<string, PayloadField>>(
	name: Name,
	columns: Cols
): <Handles extends string>(axioms: Axioms<Handles, Cols>) => Closed<Name, Handles, Cols>

function closed(name: string, shape: readonly [string, ...string[]] | Record<string, PayloadField>): unknown {
	if (isHandleTuple(shape)) {
		return mintClosed(name, shape, [], undefined)
	}
	const cols: ClosedColumn[] = []
	for (const [columnName, field] of Object.entries(shape)) {
		assertDeclarationOrderKey(`closed relation ${name} column`, columnName)
		cols.push(Object.freeze({ name: columnName, field }))
	}
	Object.freeze(cols)
	return function withAxioms(axioms: Readonly<Record<string, Readonly<Record<string, unknown>>>>): unknown {
		const handles = Object.keys(axioms)
		for (const handle of handles) {
			assertDeclarationOrderKey(`closed relation ${name} handle`, handle)
		}
		return mintClosed(name, handles, cols, axioms)
	}
}

/**
 * Mints one closed relation value — the shared seam of both tiers: roster
 * checks, eager axiom lowering, the domain-labeled `id` descriptor, and
 * the handle constants.
 */
function mintClosed(
	name: string,
	handles: readonly string[],
	cols: readonly ClosedColumn[],
	axioms: Readonly<Record<string, Readonly<Record<string, unknown>>>> | undefined
): unknown {
	if (handles.length === 0) {
		throw errors.new(`closed relation ${name}: at least one handle is required (an empty vocabulary declares nothing)`)
	}
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
		columns: Object.freeze([...cols]),
		rows: Object.freeze(rows)
	})
	const id: ClosedIdField = Object.freeze({
		kind: "u64",
		domain: `${name}Id`,
		closed: roster
	})
	/**
	 * Handle names are arbitrary identifiers, so rows and constants are
	 * minted with OWN-property definition, never assignment: a handle named
	 * "__proto__" would otherwise ride the Object.prototype accessor —
	 * silently swapping the record's prototype instead of creating the row,
	 * and no-oping the constant (a primitive through the setter) — minting a
	 * value whose type claims a bigint constant but reads back an object.
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
