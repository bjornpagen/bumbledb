import { AuthoringError, SdkInvariantError } from "#errors.ts"
/**
 * The row codec: fact object ⇄ positional cell array by field
 * ordinal, schema-directed, in ONE place. The write side lowers named host
 * objects to rows in the relation's field-declaration order (declaration
 * order = ordinal ids); the read side decodes owned rows back to named
 * objects of BARE structural values. This module is the one fact⇄row
 * projector. It covers all value types: bool, u64, i64,
 * f64, uuid, str, bytes<N>, discrete intervals and dense float intervals,
 * plus the closed-handle bijection (name ⇄ declaration-order row id).
 *
 * Cells here are OWNED plain host values; the native boundary re-judges
 * every crossing against its resident sealed roster. Shape misuse throws
 * the pure {@link AuthoringError}; the Effect ingestion boundary catches
 * and types it (never an untracked partial draft). SharedArrayBuffer-backed
 * views are refused before any copy. Host length is judged before string
 * scans and byte copies.
 */
import type { AnyClosedRoster, AnyField } from "#fields.ts"
import { literalShapeError, rosterOf } from "#fields.ts"
import { type AnyRelation, type Fact, relationFields } from "#relation.ts"
import { fieldValue, recordValue } from "#values.ts"

/**
 * One owned cell at the private bridge boundary. The declared sealed field
 * type disambiguates the union: `string` is text, an `uuid` in its
 * canonical hyphenated UUID spelling, or a closed handle
 * before lowering (closed handles cross as `bigint` row ids),
 * `Uint8Array` is `bytes<N>`, `{ start, end }` bigints are a discrete
 * interval and numbers a dense float interval.
 */
type CellValue =
	| boolean
	| bigint
	| number
	| string
	| Uint8Array
	| { readonly start: bigint; readonly end: bigint }
	| { readonly start: number; readonly end: number }

interface FlatRows {
	readonly rows: bigint
	readonly cells: readonly CellValue[]
}

function recordOf(fact: object): Readonly<Record<string, unknown>> {
	if (typeof fact !== "object" || fact === null) {
		throw new AuthoringError({ message: "expected a fact record" })
	}
	return fact as Readonly<Record<string, unknown>>
}

/**
 * Approximate payload size for host batching, not a memory allowance.
 * Strings use two bytes per UTF-16 code unit, not their UTF-8 wire size.
 */
function hostCellCharge(value: unknown): bigint {
	if (typeof value === "string") {
		return BigInt(value.length) * 2n
	}
	if (value instanceof Uint8Array) {
		return BigInt(value.byteLength)
	}
	if (typeof value === "boolean") {
		return 1n
	}
	if (typeof value === "bigint" || typeof value === "number") {
		return 8n
	}
	if (value !== null && typeof value === "object" && "start" in value && "end" in value) {
		return 16n
	}
	return 0n
}

function handleOf(context: string, closed: AnyClosedRoster, cell: unknown): string {
	if (typeof cell !== "bigint") {
		throw literalShapeError(context, `a ${closed.name} handle id (bigint)`, cell)
	}
	const handle = cell < 0n || cell >= BigInt(closed.handles.length) ? undefined : closed.handles[Number(cell)]
	if (handle === undefined) {
		throw new AuthoringError({
			message: `${context}: id ${cell} is outside the ${closed.name} roster (${closed.handles.join(", ")})`
		})
	}
	return handle
}

function cellOf(context: string, field: AnyField, value: unknown): CellValue {
	if ("closed" in field) {
		return BigInt(field.closed.handles.indexOf(fieldValue(context, field, value)))
	}
	return fieldValue(context, field, value)
}

function factCellsOf(data: AnyRelation, fact: unknown): CellValue[] {
	const record = recordValue(
		`relation ${data.name}`,
		fact,
		relationFields(data).map((declared) => declared.name)
	)
	return relationFields(data).map((declared) =>
		cellOf(`relation ${data.name} field ${declared.name}`, declared.field, record[declared.name])
	)
}

/**
 * The flat projector: every fact's cells land in ONE row-major cell array
 * (length rows × arity), counting rows while projecting.
 * Missing-field refusal and per-cell judgment are
 * {@link cellOf}'s, byte for byte.
 */
function flatRowsOf(data: AnyRelation, facts: Iterable<object>): FlatRows {
	const cells: CellValue[] = []
	let rows = 0n
	for (const fact of facts) {
		for (const cell of factCellsOf(data, fact)) cells.push(cell)
		rows += 1n
	}
	return { rows, cells }
}

function keyCellsOf(
	data: AnyRelation,
	projection: readonly string[],
	input: Readonly<Record<string, unknown>>
): CellValue[] {
	const key = recordValue(`relation ${data.name} key`, input, projection)
	return projection.map(function marshalKeyCell(fieldName) {
		const declared = relationFields(data).find((candidate) => candidate.name === fieldName)
		if (declared === undefined) {
			throw new AuthoringError({ message: `relation ${data.name}: key projection cites unknown field ${fieldName}` })
		}
		return cellOf(`relation ${data.name} key field ${fieldName}`, declared.field, key[fieldName])
	})
}

function decodeCell(context: string, field: AnyField, cell: unknown): unknown {
	const roster = rosterOf(field)
	if (roster !== undefined) return handleOf(context, roster, cell)
	try {
		return fieldValue(context, field, cell)
	} catch (cause) {
		if (cause instanceof AuthoringError) throw new SdkInvariantError({ message: cause.message })
		throw cause
	}
}

function isCompleteFact<R extends AnyRelation>(
	relation: R,
	decoded: Readonly<Record<string, unknown>>
): decoded is Readonly<Record<string, unknown>> & Fact<R> {
	return relationFields(relation).every(function present(declared) {
		return decoded[declared.name] !== undefined
	})
}

/**
 * The read half: one owned positional row into a plain frozen record in
 * declared field order: stable field names and shapes, no Proxy or per-cell closures.
 */
function factOfCells<R extends AnyRelation>(relation: R, row: readonly unknown[]): Fact<R> {
	const data = relation
	if (row.length !== relationFields(data).length) {
		throw new SdkInvariantError({
			message: `relation ${data.name}: row arity ${row.length} does not match the ${relationFields(data).length} declared fields`
		})
	}
	const decoded: Record<string, unknown> = {}
	relationFields(data).forEach(function decodeOne(declared, ordinal) {
		const cell = row[ordinal]
		if (cell === undefined) {
			throw new SdkInvariantError({
				message: `relation ${data.name}: row cell ${ordinal} (${declared.name}) is absent`
			})
		}
		const value = decodeCell(`relation ${data.name} field ${declared.name}`, declared.field, cell)
		setOwnField(decoded, declared.name, value)
	})
	Object.freeze(decoded)
	if (!isCompleteFact(relation, decoded)) {
		throw new SdkInvariantError({ message: `relation ${data.name}: decoded row is not a complete fact` })
	}
	return decoded
}

/** Preserve every declared name as data, including JavaScript's prototype setter. */
function setOwnField(record: Record<string, unknown>, name: string, value: unknown): void {
	if (name === "__proto__") {
		Object.defineProperty(record, name, { value, enumerable: true, writable: true, configurable: true })
	} else {
		record[name] = value
	}
}

export type { CellValue, FlatRows }
export {
	cellOf,
	decodeCell,
	factCellsOf,
	factOfCells,
	flatRowsOf,
	handleOf,
	hostCellCharge,
	keyCellsOf,
	recordOf,
	setOwnField
}
