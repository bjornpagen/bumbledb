import { regex } from "arkregex"

type ValueSpec =
	| { readonly kind: "bool"; readonly value: boolean }
	| { readonly kind: "u64"; readonly value: bigint }
	| { readonly kind: "i64"; readonly value: bigint }
	| { readonly kind: "f64"; readonly value: number }
	| { readonly kind: "uuid"; readonly value: string }
	| { readonly kind: "string"; readonly value: string }
	| { readonly kind: "fixedBytes"; readonly value: Uint8Array }
	| { readonly kind: "intervalU64"; readonly start: bigint; readonly end: bigint }
	| { readonly kind: "intervalI64"; readonly start: bigint; readonly end: bigint }
	| { readonly kind: "intervalF64"; readonly start: number; readonly end: number }

/** A tagged value on the data plane: a query param, or a member of a set param. */
type TaggedValue = ValueSpec

type QueryParam = TaggedValue | { readonly kind: "set"; readonly values: readonly TaggedValue[] }

type LiteralSpec =
	| { readonly kind: "value"; readonly value: ValueSpec }
	| { readonly kind: "handle"; readonly handle: string }

type LiteralSetSpec =
	| { readonly kind: "one"; readonly literal: LiteralSpec }
	| { readonly kind: "many"; readonly literals: readonly LiteralSpec[] }

type CapacityBoundSpec =
	| { readonly kind: "lit"; readonly value: bigint }
	| { readonly kind: "field"; readonly field: string }
	| { readonly kind: "durationField"; readonly field: string }

type WeightSpec =
	| { readonly kind: "unit" }
	| { readonly kind: "field"; readonly field: string }
	| { readonly kind: "durationField"; readonly field: string }

type CapacityWindowSpec =
	| { readonly kind: "exact"; readonly n: CapacityBoundSpec }
	| { readonly kind: "range"; readonly lo: CapacityBoundSpec; readonly hi: CapacityBoundSpec }
	| { readonly kind: "floor"; readonly lo: CapacityBoundSpec }

const NON_PRINTABLE = regex("[\\p{C}\\p{Z}]", "u")

const GRAPHEME_EXTEND = regex("\\p{Grapheme_Extend}", "u")

function escapeDebugChar(ch: string): string {
	if (ch === "\0") {
		return "\\0"
	}
	if (ch === "\t") {
		return "\\t"
	}
	if (ch === "\r") {
		return "\\r"
	}
	if (ch === "\n") {
		return "\\n"
	}
	if (ch === "\\" || ch === "'" || ch === '"') {
		return `\\${ch}`
	}
	if (GRAPHEME_EXTEND.test(ch) || (ch !== " " && NON_PRINTABLE.test(ch))) {
		const codePoint = ch.codePointAt(0)
		if (codePoint === undefined) {
			return ch
		}
		return `\\u{${codePoint.toString(16)}}`
	}
	return ch
}

function escapeAsciiByte(byte: number): string {
	if (byte === 0x09) {
		return "\\t"
	}
	if (byte === 0x0d) {
		return "\\r"
	}
	if (byte === 0x0a) {
		return "\\n"
	}
	if (byte === 0x5c) {
		return "\\\\"
	}
	if (byte === 0x27) {
		return "\\'"
	}
	if (byte === 0x22) {
		return '\\"'
	}
	if (byte >= 0x20 && byte <= 0x7e) {
		return String.fromCharCode(byte)
	}
	return `\\x${byte.toString(16).padStart(2, "0")}`
}

function renderLiteral(literal: LiteralSpec): string {
	if (literal.kind === "handle") {
		return literal.handle
	}
	const value = literal.value
	switch (value.kind) {
		case "bool":
			return value.value ? "true" : "false"
		case "u64":
		case "i64":
			return value.value.toString()
		case "f64":
			return `f64:0x${f64BitsHex(value.value)}`
		case "uuid":
			return `uuid:${value.value}`
		case "string": {
			let out = '"'
			for (const ch of value.value) {
				out += escapeDebugChar(ch)
			}
			return `${out}"`
		}
		case "fixedBytes": {
			let out = 'b"'
			for (const byte of value.value) {
				out += escapeAsciiByte(byte)
			}
			return `${out}"`
		}
		case "intervalU64":
		case "intervalI64":
			return `${value.start}..${value.end}`
		case "intervalF64":
			return `f64:0x${f64BitsHex(value.start)}..f64:0x${f64BitsHex(value.end)}`
	}
}

/**
 * The canonical binary64 bit image as sixteen lowercase hex digits — the
 * one f64 rendering: every NaN is the quiet canonical NaN and
 * `-0` renders as `+0`, mirroring the engine's `f64:0x{bits:016x}`.
 */
function f64BitsHex(value: number): string {
	const image = new DataView(new ArrayBuffer(8))
	if (Number.isNaN(value)) {
		image.setBigUint64(0, 0x7ff8000000000000n)
	} else {
		image.setFloat64(0, value === 0 ? 0 : value)
	}
	return image.getBigUint64(0).toString(16).padStart(16, "0")
}

function renderLiteralSet(set: LiteralSetSpec): string {
	if (set.kind === "one") {
		return renderLiteral(set.literal)
	}
	return `{${set.literals.map(renderLiteral).join(", ")}}`
}

function renderCapacityBound(bound: CapacityBoundSpec): string {
	switch (bound.kind) {
		case "lit":
			return bound.value.toString()
		case "field":
			return bound.field
		case "durationField":
			return `Duration(${bound.field})`
	}
}

function renderCapacityWindow(window: CapacityWindowSpec): string {
	switch (window.kind) {
		case "exact":
			return `{${renderCapacityBound(window.n)}}`
		case "range":
			return `{${renderCapacityBound(window.lo)}..${renderCapacityBound(window.hi)}}`
		case "floor":
			return `{${renderCapacityBound(window.lo)}..*}`
	}
}

function renderWeight(weight: WeightSpec): string {
	switch (weight.kind) {
		case "unit":
			return ""
		case "field":
			return `[${weight.field}]`
		case "durationField":
			return `[Duration(${weight.field})]`
	}
}

export type {
	CapacityBoundSpec,
	CapacityWindowSpec,
	LiteralSetSpec,
	LiteralSpec,
	QueryParam,
	TaggedValue,
	ValueSpec,
	WeightSpec
}
export { f64BitsHex, renderCapacityWindow, renderLiteral, renderLiteralSet, renderWeight }
