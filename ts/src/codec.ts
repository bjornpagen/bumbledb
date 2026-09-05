import { Effect, Result, Schema as EffectSchema } from "effect"
import { dbNative } from "#db-native.ts"
import type { AnyField } from "#fields.ts"
import { isFloatIntervalValue, isIntervalValue, rosterOf } from "#fields.ts"
import { Id128 } from "#id128.ts"
import { lower } from "#lower.ts"
import type { AnyRelation, Fact } from "#relation.ts"
import type { CellValue } from "#rows.ts"
import { factOfCells, flatRowsOf } from "#rows.ts"
import { schemaTables } from "#compile.ts"
import { DbError } from "#runtime-errors.ts"
import type { ExecutionPolicy } from "#runtime.ts"
import { nativeOperationWith, policyWire, runtimeHandle } from "#runtime.ts"
import type { AnySchema } from "#schema.ts"
import { f64BitsHex } from "#spec.ts"
import type { Rel } from "#shape.ts"

/**
 * Boundary row codecs, derived from the core relation descriptors — never a
 * second hand-maintained field roster (chapter 35).
 *
 * Two layers:
 *
 * 1. The CANONICAL native row codec: `encodeRows`/`decodeRows` run the
 *    same charged native implementation the engine, log and migrations
 *    share (owned bytes in, owned typed rows out; untrusted input cannot
 *    inject native capabilities). Effect-only, `NativeRuntime` required.
 * 2. The schema-tagged JSON VALUE form for HTTP/export boundaries
 *    (chapter 30): every `f64` — finite included — is
 *    `{"$f64":"<16 lowercase hex digits>"}` of canonical binary64 bits;
 *    integers are canonical decimal strings; `Id128` is 32 lowercase hex;
 *    bytes use ONE strict lowercase-hex encoding; intervals are
 *    `{start,end}` in their element encoding; closed references are handle
 *    names. `JSON.stringify` of raw numbers is NOT the database value
 *    wire codec — it loses infinities/NaN and cannot encode BigInt.
 *    Decoders reject malformed widths, unknown tags and noncanonical
 *    representations. These are pure bounded per-row functions.
 */

/** The typed row shape: a schema plus one of its relations. */
interface RowShape<R extends AnyRelation> {
	readonly schema: AnySchema
	readonly relation: R
}

function rowShape<S extends AnySchema, R extends Rel<S>>(schema: S, relation: R): RowShape<R> {
	return Object.freeze({ schema, relation })
}

function invalid(operation: string): DbError {
	return new DbError({ operation, reason: { _tag: "InvalidArgument" } })
}

const DECIMAL = /^-?(?:0|[1-9][0-9]*)$/
const HEX16 = /^[0-9a-f]{16}$/
const LOWER_HEX = /^(?:[0-9a-f]{2})*$/

function hexOfBytes(bytes: Uint8Array): string {
	let out = ""
	for (const byte of bytes) {
		out += byte.toString(16).padStart(2, "0")
	}
	return out
}

function bytesOfHex(text: string): Uint8Array | undefined {
	if (!LOWER_HEX.test(text)) {
		return undefined
	}
	const out = new Uint8Array(text.length / 2)
	for (let index = 0; index < out.length; index += 1) {
		out[index] = Number.parseInt(text.slice(index * 2, index * 2 + 2), 16)
	}
	return out
}

function f64OfHex(text: unknown): number | undefined {
	if (typeof text !== "string" || !HEX16.test(text)) {
		return undefined
	}
	const bits = BigInt(`0x${text}`)
	const image = new DataView(new ArrayBuffer(8))
	image.setBigUint64(0, bits)
	const value = image.getFloat64(0)
	// Canonical only: the one NaN bit pattern, and no negative zero image.
	if (f64BitsHex(value) !== text) {
		return undefined
	}
	return value
}

/** One host value to its schema-tagged JSON form (pure, bounded). */
function encodeBoundaryValue(field: AnyField, value: unknown): unknown {
	const roster = rosterOf(field)
	if (roster !== undefined) {
		if (typeof value !== "string" || !roster.handles.includes(value)) {
			throw invalid("encodeBoundaryRows")
		}
		return value
	}
	switch (field.kind) {
		case "bool": {
			if (typeof value !== "boolean") {
				throw invalid("encodeBoundaryRows")
			}
			return value
		}
		case "u64":
		case "i64": {
			if (typeof value !== "bigint") {
				throw invalid("encodeBoundaryRows")
			}
			return value.toString(10)
		}
		case "f64": {
			if (typeof value !== "number") {
				throw invalid("encodeBoundaryRows")
			}
			return { $f64: f64BitsHex(value) }
		}
		case "id128": {
			if (!Id128.isId128(value)) {
				throw invalid("encodeBoundaryRows")
			}
			return value
		}
		case "str": {
			if (typeof value !== "string" || !value.isWellFormed()) {
				throw invalid("encodeBoundaryRows")
			}
			return value
		}
		case "bytes": {
			if (!(value instanceof Uint8Array) || value.byteLength !== field.width) {
				throw invalid("encodeBoundaryRows")
			}
			return hexOfBytes(value)
		}
		case "interval": {
			if (field.element === "f64") {
				if (!isFloatIntervalValue(value)) {
					throw invalid("encodeBoundaryRows")
				}
				return { start: { $f64: f64BitsHex(value.start) }, end: { $f64: f64BitsHex(value.end) } }
			}
			if (!isIntervalValue(value)) {
				throw invalid("encodeBoundaryRows")
			}
			return { start: value.start.toString(10), end: value.end.toString(10) }
		}
	}
}

const U64_MAX = 0xffffffffffffffffn
const I64_MIN = -0x8000000000000000n
const I64_MAX = 0x7fffffffffffffffn

function decodeInteger(kind: "u64" | "i64", value: unknown): bigint | undefined {
	if (typeof value !== "string" || !DECIMAL.test(value)) {
		return undefined
	}
	const parsed = BigInt(value)
	if (kind === "u64" && (parsed < 0n || parsed > U64_MAX)) {
		return undefined
	}
	if (kind === "i64" && (parsed < I64_MIN || parsed > I64_MAX)) {
		return undefined
	}
	// Canonical decimal only (no leading zeros, no "-0") — DECIMAL enforces.
	return parsed
}

function decodeTaggedF64(value: unknown): number | undefined {
	if (typeof value !== "object" || value === null || !("$f64" in value)) {
		return undefined
	}
	const keys = Object.keys(value)
	if (keys.length !== 1) {
		return undefined
	}
	return f64OfHex((value as { readonly $f64: unknown }).$f64)
}

/** One schema-tagged JSON form back to the host value (strict, pure). */
function decodeBoundaryValue(field: AnyField, value: unknown): unknown | undefined {
	const roster = rosterOf(field)
	if (roster !== undefined) {
		return typeof value === "string" && roster.handles.includes(value) ? value : undefined
	}
	switch (field.kind) {
		case "bool":
			return typeof value === "boolean" ? value : undefined
		case "u64":
		case "i64":
			return decodeInteger(field.kind, value)
		case "f64":
			return decodeTaggedF64(value)
		case "id128": {
			if (typeof value !== "string") {
				return undefined
			}
			const parsed = Id128.fromHex(value)
			return Result.isSuccess(parsed) ? parsed.success : undefined
		}
		case "str":
			return typeof value === "string" && value.isWellFormed() ? value : undefined
		case "bytes": {
			if (typeof value !== "string" || value.length !== field.width * 2) {
				return undefined
			}
			return bytesOfHex(value)
		}
		case "interval": {
			if (typeof value !== "object" || value === null || !("start" in value) || !("end" in value)) {
				return undefined
			}
			const raw = value as { readonly start: unknown; readonly end: unknown }
			if (field.element === "f64") {
				const start = decodeTaggedF64(raw.start)
				const end = decodeTaggedF64(raw.end)
				if (start === undefined || end === undefined || Number.isNaN(start) || Number.isNaN(end) || !(start < end)) {
					return undefined
				}
				return Object.freeze({ start, end })
			}
			const start = decodeInteger(field.element, raw.start)
			const end = decodeInteger(field.element, raw.end)
			if (start === undefined || end === undefined || start >= end) {
				return undefined
			}
			return Object.freeze({ start, end })
		}
	}
}

/** Pure schema-tagged JSON encoding of owned rows (bounded per call). */
function encodeBoundaryRows<R extends AnyRelation>(
	relation: R,
	rows: Iterable<Fact<R>>
): Result.Result<ReadonlyArray<Readonly<Record<string, unknown>>>, DbError> {
	return Result.try({
		try: () => {
			const out: Array<Readonly<Record<string, unknown>>> = []
			for (const row of rows) {
				const record: Record<string, unknown> = {}
				for (const declared of relation.data.fields) {
					record[declared.name] = encodeBoundaryValue(
						declared.field,
						(row as Readonly<Record<string, unknown>>)[declared.name]
					)
				}
				out.push(Object.freeze(record))
			}
			return Object.freeze(out)
		},
		catch: (cause) => (cause instanceof DbError ? cause : invalid("encodeBoundaryRows"))
	})
}

/** Pure strict decoding of schema-tagged JSON rows; any refusal is typed. */
function decodeBoundaryRows<R extends AnyRelation>(
	relation: R,
	input: unknown
): Result.Result<ReadonlyArray<Fact<R>>, DbError> {
	if (!Array.isArray(input)) {
		return Result.fail(invalid("decodeBoundaryRows"))
	}
	const out: Array<Fact<R>> = []
	for (const raw of input) {
		if (typeof raw !== "object" || raw === null) {
			return Result.fail(invalid("decodeBoundaryRows"))
		}
		const record: Record<string, unknown> = {}
		for (const declared of relation.data.fields) {
			const decoded = decodeBoundaryValue(declared.field, (raw as Readonly<Record<string, unknown>>)[declared.name])
			if (decoded === undefined) {
				return Result.fail(invalid("decodeBoundaryRows"))
			}
			record[declared.name] = decoded
		}
		// Unknown extra keys refuse: wrong-schema values never pass.
		for (const key of Object.keys(raw)) {
			if (!(key in record)) {
				return Result.fail(invalid("decodeBoundaryRows"))
			}
		}
		out.push(Object.freeze(record) as Fact<R>)
	}
	return Result.succeed(Object.freeze(out))
}

/**
 * An Effect Schema for one relation's typed row, DERIVED from the core
 * descriptors (a validation-only declaration; the canonical `$f64`/decimal
 * wire form stays this module's explicit codec — Effect Schema's generic
 * JSON number encoding is not Bumbledb's `$f64` codec).
 */
function rowSchema<R extends AnyRelation>(relation: R) {
	return EffectSchema.declare((value: unknown): value is Fact<R> => {
		if (typeof value !== "object" || value === null) {
			return false
		}
		const record = value as Readonly<Record<string, unknown>>
		return relation.data.fields.every(function checkField(declared) {
			const cell = record[declared.name]
			if (cell === undefined) {
				return false
			}
			return Result.isSuccess(
				Result.try({
					try: () => encodeBoundaryValue(declared.field, cell),
					catch: () => invalid("rowSchema")
				})
			)
		})
	})
}

/**
 * The charged CANONICAL native row codec (chapter 35 roster): owned bytes
 * out, owned typed rows back in — the same implementation log sealing and
 * migrations use. Binding parameters is ingestion: input must stay stable
 * through execution, and no native work starts before the checked owned
 * chunk exists.
 */
const encodeRows = Effect.fn("encodeRows")(function* <R extends AnyRelation>(
	shape: RowShape<R>,
	rows: Iterable<Fact<R>>,
	work: ExecutionPolicy
) {
	const runtime = yield* runtimeHandle()
	const tables = schemaTables(shape.schema)
	const relationId = tables.relationIds.get(shape.relation.name)
	if (relationId === undefined || shape.schema.relations[shape.relation.name] !== shape.relation) {
		return yield* Effect.fail(invalid("encodeRows"))
	}
	const flat = yield* Effect.try({
		try: () => flatRowsOf(shape.relation.data, rows as Iterable<object>),
		catch: (cause) => (cause instanceof DbError ? cause : invalid("encodeRows"))
	})
	const spec = lower(shape.schema)
	return yield* nativeOperationWith(
		"encodeRows",
		(callback) =>
			dbNative.runtimeEncodeRows(runtime, policyWire(work, "encodeRows"), spec, relationId, flat.rows, flat.cells, callback),
		dbNative.runtimeBytesTake,
		(bytes) => bytes
	)
})

const decodeRows = Effect.fn("decodeRows")(function* <R extends AnyRelation>(
	shape: RowShape<R>,
	input: Uint8Array,
	work: ExecutionPolicy
) {
	const runtime = yield* runtimeHandle()
	const tables = schemaTables(shape.schema)
	const relationId = tables.relationIds.get(shape.relation.name)
	if (relationId === undefined || shape.schema.relations[shape.relation.name] !== shape.relation) {
		return yield* Effect.fail(invalid("decodeRows"))
	}
	if (!(input instanceof Uint8Array) || !(input.buffer instanceof ArrayBuffer)) {
		return yield* Effect.fail(invalid("decodeRows"))
	}
	const spec = lower(shape.schema)
	return yield* nativeOperationWith(
		"decodeRows",
		(callback) => dbNative.runtimeDecodeRows(runtime, policyWire(work, "decodeRows"), spec, relationId, input, callback),
		dbNative.runtimeRowsTake,
		(rows) => Object.freeze(rows.map((row) => factOfCells(shape.relation, row as readonly CellValue[])))
	)
})

export type { RowShape }
export { decodeBoundaryRows, decodeRows, encodeBoundaryRows, encodeRows, rowSchema, rowShape }
