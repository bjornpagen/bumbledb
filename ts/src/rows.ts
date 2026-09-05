import { Result } from "effect"
import { AuthoringError, SdkInvariantError } from "#errors.ts"
/**
 * The successor row codec: fact object ⇄ positional cell array by field
 * ordinal, schema-directed, in ONE place. The write side lowers named host
 * objects to rows in the relation's field-declaration order (declaration
 * order = ordinal ids); the read side decodes owned rows back to named
 * objects of BARE structural values. This module supersedes the fact⇄row
 * half of the legacy `marshal.ts` (P06R's cutover deletes that file's
 * copy); it covers the COMPLETE successor value roster: bool, u64, i64,
 * f64, id128, str, bytes<N>, discrete intervals and dense float intervals,
 * plus the closed-handle bijection (name ⇄ declaration-order row id).
 *
 * Cells here are OWNED plain host values; the native boundary re-judges
 * every crossing against its resident sealed roster. Shape misuse throws
 * the pure {@link AuthoringError}; the Effect ingestion boundary catches
 * and types it (never an untracked partial draft). SharedArrayBuffer-backed
 * views are refused before any copy (chapter 30).
 */
import type { AnyClosedRoster, AnyField } from "#fields.ts"
import { isFloatIntervalValue, isIntervalValue, literalShapeError, rosterOf } from "#fields.ts"
import { Id128 } from "#id128.ts"
import type { AnyRelation, Fact, RelationData } from "#relation.ts"

/**
 * One owned cell at the private bridge boundary. The declared sealed field
 * type disambiguates the union: `string` is text (or a closed handle after
 * lowering — those cross as `bigint` row ids), `Uint8Array` is `bytes<N>`
 * or the sixteen `id128` bytes, `{ start, end }` bigints are a discrete
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
	/** Conservative charged byte size of the owned cells (input accounting). */
	readonly bytes: bigint
}

function recordOf(fact: object): Readonly<Record<string, unknown>> {
	if (typeof fact !== "object" && typeof fact !== "function") {
		throw new AuthoringError({ message: "fact object is not string-indexable" })
	}
	return fact as Readonly<Record<string, unknown>>
}

function refuseShared(context: string, value: Uint8Array): void {
	if (!(value.buffer instanceof ArrayBuffer)) {
		throw new AuthoringError({
			message: `${context}: SharedArrayBuffer-backed views are refused — make a synchronized stable copy into ordinary unshared input first`
		})
	}
}

/**
 * The write half of the closed bijection: one handle NAME to its u64 row
 * id (declaration order = row ids). An unknown name is a pointed refusal
 * naming the vocabulary and its roster.
 */
function closedCellOf(context: string, closed: AnyClosedRoster, name: string): CellValue {
	const id = closed.handles.indexOf(name)
	if (id === -1) {
		throw new AuthoringError({
			message: `${context}: "${name}" is not a handle of ${closed.name} — the roster is ${closed.handles.join(", ")}`
		})
	}
	return BigInt(id)
}

function handleOf(context: string, closed: AnyClosedRoster, cell: unknown): string {
	if (typeof cell !== "bigint") {
		throw literalShapeError(context, `a ${closed.name} handle id (bigint)`, cell)
	}
	const handle = closed.handles[Number(cell)]
	if (handle === undefined) {
		throw new AuthoringError({
			message: `${context}: id ${cell} is outside the ${closed.name} roster (${closed.handles.join(", ")})`
		})
	}
	return handle
}

/** Conservative owned-byte charge of one cell (host accounting, not wire). */
function cellBytes(cell: CellValue): bigint {
	if (typeof cell === "boolean") {
		return 1n
	}
	if (typeof cell === "bigint" || typeof cell === "number") {
		return 8n
	}
	if (typeof cell === "string") {
		// UTF-16 code units × 2 is a cheap safe upper bound before exact
		// UTF-8 conversion is charged natively (chapter 35: reject oversize
		// cells with cheap length bounds before costly conversion).
		return BigInt(cell.length) * 2n
	}
	if (cell instanceof Uint8Array) {
		return BigInt(cell.byteLength)
	}
	return 16n
}

function cellOf(context: string, field: AnyField, value: unknown): CellValue {
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
		case "f64": {
			if (typeof value !== "number") {
				throw literalShapeError(context, "number", value)
			}
			return value
		}
		case "id128": {
			if (!Id128.isId128(value)) {
				throw literalShapeError(context, "an Id128 (32 lowercase hex characters)", value)
			}
			const bytes = Id128.toBytes(value)
			return bytes
		}
		case "bytes": {
			if (!(value instanceof Uint8Array)) {
				throw literalShapeError(context, "Uint8Array", value)
			}
			refuseShared(context, value)
			if (value.byteLength !== field.width) {
				throw new AuthoringError({
					message: `${context}: bytes<${field.width}> takes exactly ${field.width} bytes (got ${value.byteLength})`
				})
			}
			// Owned copy at the ownership boundary: later caller mutation
			// cannot change an accepted cell.
			return Uint8Array.from(value)
		}
		case "interval": {
			if (field.element === "f64") {
				if (!isFloatIntervalValue(value)) {
					throw literalShapeError(context, "float interval ({ start, end } numbers)", value)
				}
				if (Number.isNaN(value.start) || Number.isNaN(value.end)) {
					throw new AuthoringError({ message: `${context}: a float interval endpoint cannot be NaN` })
				}
				const start = Object.is(value.start, -0) ? 0 : value.start
				const end = Object.is(value.end, -0) ? 0 : value.end
				if (!(start < end)) {
					throw new AuthoringError({
						message: `${context}: a float interval is half-open and nonempty (start < end strictly)`
					})
				}
				return { start, end }
			}
			if (!isIntervalValue(value)) {
				throw literalShapeError(context, "interval ({ start, end } bigints)", value)
			}
			return { start: value.start, end: value.end }
		}
	}
}

/**
 * The flat projector: every fact's cells land in ONE row-major cell array
 * (length rows × arity) with the row count and charged byte size counted
 * while projecting. Missing-field refusal and per-cell judgment are
 * {@link cellOf}'s, byte for byte.
 */
function flatRowsOf(data: RelationData, facts: Iterable<object>): FlatRows {
	const cells: CellValue[] = []
	let rows = 0n
	let bytes = 0n
	for (const fact of facts) {
		rows += 1n
		const record = recordOf(fact)
		for (const declared of data.fields) {
			const value = record[declared.name]
			if (value === undefined) {
				throw new AuthoringError({ message: `relation ${data.name}: fact is missing field ${declared.name}` })
			}
			const cell = cellOf(`relation ${data.name} field ${declared.name}`, declared.field, value)
			bytes += cellBytes(cell)
			cells.push(cell)
		}
	}
	return { rows, cells, bytes }
}

function keyCellsOf(
	data: RelationData,
	projection: readonly string[],
	key: Readonly<Record<string, unknown>>
): CellValue[] {
	for (const supplied of Object.keys(key)) {
		if (key[supplied] !== undefined && !projection.includes(supplied)) {
			throw new AuthoringError({
				message: `relation ${data.name}: key object carries field ${supplied} outside the primary key projection (${projection.join(", ")})`
			})
		}
	}
	return projection.map(function marshalKeyCell(fieldName) {
		const declared = data.fields.find(function byName(candidate) {
			return candidate.name === fieldName
		})
		if (declared === undefined) {
			throw new AuthoringError({
				message: `relation ${data.name}: key projection cites unknown field ${fieldName}`
			})
		}
		const value = key[fieldName]
		if (value === undefined) {
			throw new AuthoringError({
				message: `relation ${data.name}: key object is missing field ${fieldName} — get reads through the primary (first-declared) key, whose projection is (${projection.join(", ")})`
			})
		}
		return cellOf(`relation ${data.name} key field ${fieldName}`, declared.field, value)
	})
}

function decodeCell(context: string, field: AnyField, cell: unknown): unknown {
	const roster = rosterOf(field)
	if (roster !== undefined) {
		return handleOf(context, roster, cell)
	}
	switch (field.kind) {
		case "bool": {
			if (typeof cell !== "boolean") {
				throw new SdkInvariantError({ message: `${context}: expected boolean cell` })
			}
			return cell
		}
		case "u64":
		case "i64": {
			if (typeof cell !== "bigint") {
				throw new SdkInvariantError({ message: `${context}: expected bigint cell` })
			}
			return cell
		}
		case "f64": {
			if (typeof cell !== "number") {
				throw new SdkInvariantError({ message: `${context}: expected number cell` })
			}
			return cell
		}
		case "str": {
			if (typeof cell !== "string") {
				throw new SdkInvariantError({ message: `${context}: expected string cell` })
			}
			return cell
		}
		case "id128": {
			if (!(cell instanceof Uint8Array) || cell.length !== 16) {
				throw new SdkInvariantError({ message: `${context}: expected 16 id128 bytes` })
			}
			const decoded = Id128.fromBytes(cell)
			if (!Result.isSuccess(decoded)) {
				throw new SdkInvariantError({ message: `${context}: id128 bytes did not decode` })
			}
			return decoded.success
		}
		case "bytes": {
			if (!(cell instanceof Uint8Array)) {
				throw new SdkInvariantError({ message: `${context}: expected owned bytes` })
			}
			return cell
		}
		case "interval": {
			if (field.element === "f64") {
				if (!isFloatIntervalValue(cell)) {
					throw new SdkInvariantError({ message: `${context}: expected a float interval cell` })
				}
				return Object.freeze({ start: cell.start, end: cell.end })
			}
			if (!isIntervalValue(cell)) {
				throw new SdkInvariantError({ message: `${context}: expected an integer interval cell` })
			}
			return Object.freeze({ start: cell.start, end: cell.end })
		}
	}
}

function isCompleteFact<R extends AnyRelation>(
	relation: R,
	decoded: Readonly<Record<string, unknown>>
): decoded is Readonly<Record<string, unknown>> & Fact<R> {
	return relation.data.fields.every(function present(declared) {
		return decoded[declared.name] !== undefined
	})
}

/**
 * The read half: one owned positional row into a plain frozen record in
 * declared field order — the SAME fields and shapes on every row (chapter
 * 35's stable row shape rule; no Proxy, no per-cell closures).
 */
function factOfCells<R extends AnyRelation>(relation: R, row: readonly unknown[]): Fact<R> {
	const data = relation.data
	if (row.length !== data.fields.length) {
		throw new SdkInvariantError({
			message: `relation ${data.name}: row arity ${row.length} does not match the ${data.fields.length} declared fields`
		})
	}
	const decoded: Record<string, unknown> = {}
	data.fields.forEach(function decodeOne(declared, ordinal) {
		const cell = row[ordinal]
		if (cell === undefined) {
			throw new SdkInvariantError({ message: `relation ${data.name}: row cell ${ordinal} (${declared.name}) is absent` })
		}
		decoded[declared.name] = decodeCell(`relation ${data.name} field ${declared.name}`, declared.field, cell)
	})
	Object.freeze(decoded)
	if (!isCompleteFact(relation, decoded)) {
		throw new SdkInvariantError({ message: `relation ${data.name}: decoded row is not a complete fact` })
	}
	return decoded
}

export type { CellValue, FlatRows }
export { cellBytes, cellOf, decodeCell, factOfCells, flatRowsOf, handleOf, keyCellsOf, recordOf }
