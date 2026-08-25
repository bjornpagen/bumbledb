import { regex } from "arkregex"

type ValueTypeSpec =
	| { readonly kind: "bool" }
	| { readonly kind: "u64" }
	| { readonly kind: "i64" }
	| { readonly kind: "string" }
	| { readonly kind: "fixedBytes"; readonly len: number }
	| {
			readonly kind: "interval"
			readonly element: "u64" | "i64"
			readonly width: bigint | undefined
	  }

type ValueSpec =
	| { readonly kind: "bool"; readonly value: boolean }
	| { readonly kind: "u64"; readonly value: bigint }
	| { readonly kind: "i64"; readonly value: bigint }
	| { readonly kind: "string"; readonly value: string }
	| { readonly kind: "fixedBytes"; readonly value: Uint8Array }
	| { readonly kind: "intervalU64"; readonly start: bigint; readonly end: bigint }
	| { readonly kind: "intervalI64"; readonly start: bigint; readonly end: bigint }

type LiteralSpec =
	| { readonly kind: "value"; readonly value: ValueSpec }
	| { readonly kind: "handle"; readonly handle: string }

type LiteralSetSpec =
	| { readonly kind: "one"; readonly literal: LiteralSpec }
	| { readonly kind: "many"; readonly literals: readonly LiteralSpec[] }

interface SideSpec {
	readonly relation: string
	readonly projection: readonly string[]
	readonly selection: ReadonlyArray<readonly [string, LiteralSetSpec]>
}

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

interface FieldSpec {
	readonly name: string
	readonly valueType: ValueTypeSpec
	readonly newtype: string | undefined
	readonly fresh: boolean
}

interface RowSpec {
	readonly handle: string
	readonly values: readonly LiteralSpec[]
}

/**
 * A relation's closedness as ONE sum (ruled 2026-07-23, R7): the handle
 * newtype and the ground axioms travel together — the two illegal states
 * (a roster without its newtype, a newtype without its roster) are
 * unspellable on the wire exactly as they are unrepresentable in the
 * fused Rust `RelationSpec`. `newtype` is the id's law-computed generator
 * class (`` `${name}.id` `` — the same label every referencing field
 * carries by law), which is how the engine resolves a handle literal back
 * to its roster.
 */
interface ClosedSpec {
	readonly newtype: string
	readonly rows: readonly RowSpec[]
}

interface RelationSpec {
	readonly name: string
	readonly fields: readonly FieldSpec[]
	readonly closed: ClosedSpec | undefined
}

type StatementSpec =
	| { readonly kind: "fd"; readonly relation: string; readonly projection: readonly string[] }
	| {
			readonly kind: "containment"
			readonly source: SideSpec
			readonly target: SideSpec
			readonly bidirectional: boolean
	  }
	| {
			readonly kind: "capacity"
			readonly target: SideSpec
			readonly weight: WeightSpec
			readonly window: CapacityWindowSpec
			readonly source: SideSpec
	  }

interface SchemaSpec {
	readonly relations: readonly RelationSpec[]
	readonly statements: readonly StatementSpec[]
}

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
	}
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
	ClosedSpec,
	FieldSpec,
	LiteralSetSpec,
	LiteralSpec,
	RelationSpec,
	RowSpec,
	SchemaSpec,
	SideSpec,
	StatementSpec,
	ValueSpec,
	ValueTypeSpec,
	WeightSpec
}
export { renderCapacityWindow, renderLiteral, renderLiteralSet, renderWeight }
