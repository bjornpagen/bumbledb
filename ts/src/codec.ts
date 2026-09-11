import { Effect, Schema as EffectSchema, Result } from "effect"
import { membersAgree } from "#closed.ts"
import { schemaTables } from "#compile.ts"
import { dbNative } from "#db-native.ts"
import { AuthoringError } from "#errors.ts"
import { type AnyField, fieldDescriptor, type Infer } from "#fields.ts"
import { lower } from "#lower.ts"
import { type AnyRelation, type Fact, relationDescriptor, relationFields } from "#relation.ts"
import type { CellValue } from "#rows.ts"
import { factOfCells, flatRowsOf } from "#rows.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError, DbError } from "#runtime-errors.ts"
import type { AnySchema } from "#schema.ts"
import { schemaDescriptor } from "#schema.ts"
import type { Rel } from "#shape.ts"
import { f64BitsHex } from "#spec.ts"
import { fieldValue, recordValue } from "#values.ts"

/**
 * Boundary row codecs, derived from the core relation descriptors — never a
 * second hand-maintained field roster.
 *
 * Two layers:
 *
 * 1. The CANONICAL native row codec: `encodeRows`/`decodeRows` run the
 *    same native implementation the engine, log and migrations
 *    share (owned bytes in, owned typed rows out; untrusted input cannot
 *    inject native capabilities). Effect-only, `NativeRuntime` required.
 * 2. The schema-tagged JSON VALUE form for HTTP/export boundaries
 *    (chapter 30): every `f64` — finite included — is
 *    `{"$f64":"<16 lowercase hex digits>"}` of canonical binary64 bits;
 *    integers are canonical decimal strings; `Uuid` is canonical UUID;
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
	const ownedSchema = schemaDescriptor(schema)
	const ownedRelation = relationDescriptor(relation)
	return Object.freeze({ schema: ownedSchema, relation: ownedRelation })
}

function checkedRowShape<R extends AnyRelation>(operation: string, input: RowShape<R>) {
	recordValue(`${operation} shape`, input, ["schema", "relation"])
	const schema = schemaDescriptor(input.schema)
	const relation = relationDescriptor(input.relation)
	const relationId = schemaTables(schema).relationIds.get(relation.name)
	if (relationId === undefined || !membersAgree(schema.relations[relation.name], relation)) {
		throw new AuthoringError({ message: `${operation}: relation is not declared in the schema` })
	}
	return { relation, relationId, spec: lower(schema) }
}

function invalid(operation: string): DbError {
	return new DbError({ operation, reason: { _tag: "InvalidArgument" } })
}

const DECIMAL = /^(?:0|-?[1-9][0-9]*)$/
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

/** One host value to its canonical, field-directed JSON representation. */
function encodeBoundaryValue(context: string, field: AnyField, input: unknown): unknown {
	if ("closed" in field) return fieldValue(context, field, input)
	switch (field.kind) {
		case "bool":
		case "str":
		case "uuid":
			return fieldValue(context, field, input)
		case "u64":
		case "i64":
			return fieldValue(context, field, input).toString(10)
		case "f64":
			return { $f64: f64BitsHex(fieldValue(context, field, input)) }
		case "bytes":
			return hexOfBytes(fieldValue(context, field, input))
		case "interval": {
			if (field.element === "f64") {
				const value = fieldValue(context, { ...field, element: "f64" }, input)
				return { start: { $f64: f64BitsHex(value.start) }, end: { $f64: f64BitsHex(value.end) } }
			}
			const value = fieldValue(context, { ...field, element: field.element }, input)
			return { start: value.start.toString(10), end: value.end.toString(10) }
		}
	}
}

function decodeInteger(value: unknown): bigint | undefined {
	// No accepted integer spelling is longer than 20 ASCII characters.
	if (typeof value !== "string" || value.length > 20 || !DECIMAL.test(value)) return undefined
	return BigInt(value)
}

function decodeTaggedF64(value: unknown): number | undefined {
	const record = recordValue("binary64 image", value, ["$f64"])
	return f64OfHex(record.$f64)
}

/** Decode representation, then judge the value through the shared interpreter. */
function decodeBoundaryValue<F extends AnyField>(context: string, field: F, input: unknown): Infer<F>
function decodeBoundaryValue(context: string, field: AnyField, input: unknown): Infer<AnyField> {
	if ("closed" in field) return fieldValue(context, field, input)
	let decoded: unknown = input
	switch (field.kind) {
		case "u64":
		case "i64":
			decoded = decodeInteger(input)
			break
		case "f64":
			decoded = decodeTaggedF64(input)
			break
		case "bytes":
			decoded = typeof input === "string" && input.length === field.width * 2 ? bytesOfHex(input) : undefined
			break
		case "interval": {
			const record = recordValue(context, input, ["start", "end"])
			decoded =
				field.element === "f64"
					? { start: decodeTaggedF64(record.start), end: decodeTaggedF64(record.end) }
					: { start: decodeInteger(record.start), end: decodeInteger(record.end) }
			break
		}
	}
	return fieldValue(context, field, decoded)
}

/** The host-value schema, using the same interpreter as facts and row codecs. */
function fieldSchema<F extends AnyField>(field: F) {
	const descriptor = fieldDescriptor("fieldSchema", field)
	return EffectSchema.declare((value: unknown): value is Infer<F> => {
		try {
			fieldValue("fieldSchema", descriptor, value)
			return true
		} catch {
			return false
		}
	})
}

/** Encode one field's host value in the canonical JSON-boundary form. */
function encodeBoundaryField<F extends AnyField>(field: F, value: Infer<F>): Result.Result<unknown, DbError> {
	return Result.try({
		try: () => encodeBoundaryValue("encodeBoundaryField", fieldDescriptor("encodeBoundaryField", field), value),
		catch: (cause) => argumentError("encodeBoundaryField", cause)
	})
}

/** Strict JSON-boundary decoding, returning the field's canonical host value. */
function decodeBoundaryField<F extends AnyField>(field: F, input: unknown): Result.Result<Infer<F>, DbError> {
	return Result.try({
		try: () => decodeBoundaryValue("decodeBoundaryField", fieldDescriptor("decodeBoundaryField", field), input),
		catch: (cause) => argumentError("decodeBoundaryField", cause)
	})
}

/** Pure schema-tagged JSON encoding of complete rows. */
function encodeBoundaryRows<R extends AnyRelation>(
	relation: R,
	rows: Iterable<Fact<R>>
): Result.Result<ReadonlyArray<Readonly<Record<string, unknown>>>, DbError> {
	return Result.try({
		try: () => {
			const out: Array<Readonly<Record<string, unknown>>> = []
			const fields = relationFields(relation)
			const names = fields.map((declared) => declared.name)
			for (const row of rows) {
				const input = recordValue(`relation ${relation.name}`, row, names)
				out.push(
					Object.freeze(
						Object.fromEntries(
							fields.map((declared) => [
								declared.name,
								encodeBoundaryValue(`relation ${relation.name}.${declared.name}`, declared.field, input[declared.name])
							])
						)
					)
				)
			}
			return Object.freeze(out)
		},
		catch: (cause) => argumentError("encodeBoundaryRows", cause)
	})
}

/** Pure strict decoding; malformed records, getters and values fail as data. */
function decodeBoundaryRows<R extends AnyRelation>(
	relation: R,
	input: unknown
): Result.Result<ReadonlyArray<Fact<R>>, DbError> {
	return Result.try({
		try: () => {
			if (!Array.isArray(input)) throw invalid("decodeBoundaryRows")
			const fields = relationFields(relation)
			const names = fields.map((declared) => declared.name)
			return Object.freeze(
				input.map((row) => {
					const raw = recordValue(`relation ${relation.name}`, row, names)
					return Object.freeze(
						Object.fromEntries(
							fields.map((declared) => [
								declared.name,
								decodeBoundaryValue(`relation ${relation.name}.${declared.name}`, declared.field, raw[declared.name])
							])
						)
					) as Fact<R>
				})
			)
		},
		catch: (cause) => argumentError("decodeBoundaryRows", cause)
	})
}

/** An Effect Schema derived from the same complete-row value interpreter. */
function rowSchema<R extends AnyRelation>(relation: R) {
	const fields = relationFields(relation)
	const names = fields.map((declared) => declared.name)
	return EffectSchema.declare((value: unknown): value is Fact<R> => {
		try {
			const record = recordValue(`relation ${relation.name}`, value, names)
			for (const declared of fields)
				fieldValue(`relation ${relation.name}.${declared.name}`, declared.field, record[declared.name])
			return true
		} catch {
			return false
		}
	})
}

/**
 * The canonical native row codec (chapter 35 roster): owned bytes
 * out, owned typed rows back in — the same implementation log sealing and
 * migrations use. Binding parameters is ingestion: input must stay stable
 * through execution, and no native work starts before the checked owned
 * chunk exists.
 */
const encodeRows = Effect.fn("encodeRows")(function* <R extends AnyRelation>(
	shape: RowShape<R>,
	rows: Iterable<Fact<R>>
) {
	const runtime = yield* runtimeHandle()
	const { relation, relationId, spec } = yield* Effect.try({
		try: () => checkedRowShape("encodeRows", shape),
		catch: (cause) => argumentError("encodeRows", cause)
	})
	const flat = yield* Effect.try({
		try: () => flatRowsOf(relation, rows as Iterable<object>),
		catch: (cause) => argumentError("encodeRows", cause)
	})
	return yield* nativeOperationWith(
		"encodeRows",
		(callback) => dbNative.runtimeEncodeRows(runtime, spec, relationId, flat.rows, flat.cells, callback),
		dbNative.runtimeBytesTake,
		(bytes) => bytes
	)
})

const decodeRows = Effect.fn("decodeRows")(function* <R extends AnyRelation>(shape: RowShape<R>, input: Uint8Array) {
	const runtime = yield* runtimeHandle()
	const { relation, relationId, spec } = yield* Effect.try({
		try: () => checkedRowShape("decodeRows", shape),
		catch: (cause) => argumentError("decodeRows", cause)
	})
	if (!(input instanceof Uint8Array) || !(input.buffer instanceof ArrayBuffer)) {
		return yield* Effect.fail(invalid("decodeRows"))
	}
	return yield* nativeOperationWith(
		"decodeRows",
		(callback) => dbNative.runtimeDecodeRows(runtime, spec, relationId, input, callback),
		dbNative.runtimeRowsTake,
		(rows) => Object.freeze(rows.map((row) => factOfCells(relation, row as readonly CellValue[])))
	)
})

export type { RowShape }
export {
	decodeBoundaryField,
	decodeBoundaryRows,
	decodeRows,
	encodeBoundaryField,
	encodeBoundaryRows,
	encodeRows,
	fieldSchema,
	rowSchema,
	rowShape
}
