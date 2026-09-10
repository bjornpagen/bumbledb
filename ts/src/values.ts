import { type AuthoringDiagnostic, AuthoringError } from "#errors.ts"
import type { AnyField, Infer } from "#fields.ts"
import type { ValueSpec } from "#spec.ts"
import { Uuid } from "#uuid.ts"

const U64_MAX = 0xffffffffffffffffn
const I64_MIN = -0x8000000000000000n
const I64_MAX = 0x7fffffffffffffffn

function invalid(context: string, expected: string, code: AuthoringDiagnostic["code"] = "InvalidValue"): never {
	throw new AuthoringError({ message: `${context}: expected ${expected}`, diagnostic: { code, context, expected } })
}

/** Validate the complete record shape without reading any field getter. */
function recordValue(context: string, input: unknown, fields: readonly string[]): Readonly<Record<string, unknown>> {
	if (typeof input !== "object" || input === null) {
		return invalid(context, "a plain record", "InvalidRecord")
	}
	const prototype = Object.getPrototypeOf(input)
	if (prototype !== Object.prototype && prototype !== null) {
		return invalid(context, "a plain record", "InvalidRecord")
	}
	for (const name of Reflect.ownKeys(input)) {
		if (typeof name !== "string" || !fields.includes(name)) {
			return invalid(`${context}.${String(name)}`, "only the declared fields", "UnknownField")
		}
		const property = Object.getOwnPropertyDescriptor(input, name)
		if (property?.enumerable !== true || !("value" in property)) {
			return invalid(`${context}.${name}`, "enumerable own data fields", "InvalidRecord")
		}
	}
	for (const name of fields) {
		if (!Object.hasOwn(input, name)) {
			return invalid(`${context}.${name}`, "a required own field", "MissingField")
		}
	}
	return input as Readonly<Record<string, unknown>>
}

/** Own a dense data array without invoking caller element accessors or methods. */
function arrayValue<A>(context: string, input: unknown, parse: (context: string, input: unknown) => A): readonly A[] {
	if (!Array.isArray(input)) return invalid(context, "an array", "InvalidRecord")
	for (const name of Reflect.ownKeys(input)) {
		if (name === "length") continue
		if (typeof name !== "string" || !/^(0|[1-9][0-9]*)$/.test(name) || Number(name) >= input.length)
			return invalid(context, "only array elements", "InvalidRecord")
		const property = Object.getOwnPropertyDescriptor(input, name)
		if (property?.enumerable !== true || !("value" in property))
			return invalid(context, "own data elements", "InvalidRecord")
	}
	const result: A[] = []
	for (let index = 0; index < input.length; index++) {
		if (!Object.hasOwn(input, index)) return invalid(`${context}[${index}]`, "a present element", "MissingField")
		result.push(parse(`${context}[${index}]`, input[index]))
	}
	return Object.freeze(result)
}

function integerValue(context: string, kind: "u64" | "i64", input: unknown): bigint {
	if (typeof input !== "bigint") return invalid(context, `${kind} bigint`)
	const minimum = kind === "u64" ? 0n : I64_MIN
	const maximum = kind === "u64" ? U64_MAX : I64_MAX
	if (input < minimum || input > maximum) return invalid(context, `${kind} bigint in range`)
	return input
}

function floatValue(context: string, input: unknown): number {
	if (typeof input !== "number") return invalid(context, "number")
	if (Number.isNaN(input)) return Number.NaN
	return Object.is(input, -0) ? 0 : input
}

/** Read typed-array internal slots through the intrinsic getters, never a
 * caller's shadowed buffer/length properties or custom iterator. */
function bytesValue(context: string, input: unknown): Uint8Array {
	if (!(input instanceof Uint8Array)) return invalid(context, "Uint8Array")
	const prototype = Object.getPrototypeOf(Uint8Array.prototype)
	const buffer = Reflect.get(prototype, "buffer", input)
	if (!(buffer instanceof ArrayBuffer))
		throw new AuthoringError({
			message: `${context}: SharedArrayBuffer-backed views are refused`,
			diagnostic: { code: "InvalidValue", context, expected: "ArrayBuffer-backed bytes" }
		})
	const offset = Reflect.get(prototype, "byteOffset", input)
	const length = Reflect.get(prototype, "byteLength", input)
	try {
		return new Uint8Array(new Uint8Array(buffer, offset, length))
	} catch {
		return invalid(context, "non-detached bytes")
	}
}

/**
 * The host interpreter of a field descriptor. All ingress paths use the
 * same range, width, ownership and canonicalization rules. Native admission
 * remains authoritative; this interpreter mirrors its value domain.
 */
function fieldValue<F extends AnyField>(context: string, field: F, input: unknown): Infer<F>
function fieldValue(context: string, field: AnyField, input: unknown): Infer<AnyField> {
	if ("closed" in field) {
		if (typeof input !== "string" || !field.closed.handles.includes(input)) {
			return invalid(context, `a ${field.closed.name} handle`)
		}
		return input
	}
	switch (field.kind) {
		case "bool":
			return typeof input === "boolean" ? input : invalid(context, "boolean")
		case "str":
			return typeof input === "string" && input.isWellFormed() ? input : invalid(context, "well-formed string")
		case "u64":
		case "i64":
			return integerValue(context, field.kind, input)
		case "f64":
			return floatValue(context, input)
		case "uuid":
			return Uuid.isUuid(input) ? input : invalid(context, "canonical UUID text")
		case "bytes": {
			const owned = bytesValue(context, input)
			if (owned.byteLength !== field.width) return invalid(context, `bytes<${field.width}>`)
			return owned
		}
		case "interval": {
			const record = recordValue(context, input, ["start", "end"])
			if (field.element === "f64") {
				const start = floatValue(`${context}.start`, record.start)
				const end = floatValue(`${context}.end`, record.end)
				if (!(start < end)) return invalid(context, "a nonempty float interval with non-NaN endpoints")
				return Object.freeze({ start, end })
			}
			const start = integerValue(`${context}.start`, field.element, record.start)
			const end = integerValue(`${context}.end`, field.element, record.end)
			if (start >= end) return invalid(context, "a nonempty interval (start < end)")
			const maximumEnd = field.element === "u64" ? U64_MAX : I64_MAX
			if (field.width !== undefined && (end === maximumEnd || end - start !== field.width)) {
				return invalid(context, `a finite interval of width ${field.width}`)
			}
			return Object.freeze({ start, end })
		}
	}
}

/** One checked host value to the native logical scalar description. */
function taggedValueOf(context: string, field: AnyField, input: unknown): ValueSpec {
	if ("closed" in field) {
		const handle = fieldValue(context, field, input)
		return { kind: "u64", value: BigInt(field.closed.handles.indexOf(handle)) }
	}
	switch (field.kind) {
		case "bool":
			return { kind: "bool", value: fieldValue(context, field, input) }
		case "str":
			return { kind: "string", value: fieldValue(context, field, input) }
		case "u64":
			return { kind: "u64", value: fieldValue(context, field, input) }
		case "i64":
			return { kind: "i64", value: fieldValue(context, field, input) }
		case "f64":
			return { kind: "f64", value: fieldValue(context, field, input) }
		case "uuid":
			return { kind: "uuid", value: fieldValue(context, field, input) }
		case "bytes":
			return { kind: "fixedBytes", value: fieldValue(context, field, input) }
		case "interval": {
			if (field.element === "f64") {
				const value = fieldValue(context, { ...field, element: "f64" }, input)
				return { kind: "intervalF64", ...value }
			}
			const value = fieldValue(context, { ...field, element: field.element }, input)
			return { kind: field.element === "u64" ? "intervalU64" : "intervalI64", ...value }
		}
	}
}

/** Check a generated logical value through the same field interpreter. */
function valueDescriptor(context: string, input: unknown): ValueSpec {
	if (typeof input !== "object" || input === null) return invalid(context, "a tagged value", "InvalidRecord")
	const tag = recordValue(context, input, Object.keys(input)).kind
	const intervalElement = { intervalU64: "u64", intervalI64: "i64", intervalF64: "f64" } as const
	if (tag === "intervalU64" || tag === "intervalI64" || tag === "intervalF64") {
		const raw = recordValue(context, input, ["kind", "start", "end"])
		return Object.freeze(
			taggedValueOf(
				context,
				{ kind: "interval", element: intervalElement[tag], width: undefined },
				{ start: raw.start, end: raw.end }
			)
		)
	}
	const raw = recordValue(context, input, ["kind", "value"])
	if (tag === "fixedBytes") {
		const bytes = bytesValue(context, raw.value)
		if (bytes.byteLength < 1 || bytes.byteLength > 64) return invalid(context, "bytes with width 1 through 64")
		return Object.freeze({ kind: "fixedBytes", value: bytes })
	}
	if (tag === "string") return Object.freeze(taggedValueOf(context, { kind: "str" }, raw.value))
	switch (tag) {
		case "bool":
			return Object.freeze(taggedValueOf(context, { kind: tag }, raw.value))
		case "u64":
			return Object.freeze(taggedValueOf(context, { kind: tag }, raw.value))
		case "i64":
			return Object.freeze(taggedValueOf(context, { kind: tag }, raw.value))
		case "f64":
			return Object.freeze(taggedValueOf(context, { kind: tag }, raw.value))
		case "uuid":
			return Object.freeze(taggedValueOf(context, { kind: tag }, raw.value))
	}
	return invalid(context, "a supported value tag", "InvalidDeclaration")
}

export {
	arrayValue,
	bytesValue,
	fieldValue,
	I64_MAX,
	I64_MIN,
	integerValue,
	recordValue,
	taggedValueOf,
	U64_MAX,
	valueDescriptor
}
