/**
 * Spellings for the addon's JSON inputs. 64-bit integers cross as decimal strings, `f64` as its
 * 16-hex-digit IEEE bit pattern (so -0, NaN and the infinities stay exact) and bytes as lowercase
 * hex; JSON numbers only ever carry ordinals.
 */
import type {
	CapacityBoundSpec,
	CapacityWindowSpec,
	LiteralSetSpec,
	LiteralSpec,
	ValueSpec,
	WeightSpec
} from "../spec.ts"
import { f64BitsHex } from "../spec.ts"
import type {
	BoundSpecIn,
	CapacityWindowSpecIn,
	LiteralSetSpecIn,
	LiteralSpecIn,
	ValueIn,
	WeightSpecIn
} from "./binding.d.ts"

function hex(bytes: Uint8Array): string {
	let out = ""
	for (const byte of bytes) out += byte.toString(16).padStart(2, "0")
	return out
}

function valueIn(value: ValueSpec): ValueIn {
	switch (value.kind) {
		case "bool":
			return { kind: "Bool", value: value.value }
		case "u64":
			return { kind: "U64", value: value.value.toString() }
		case "i64":
			return { kind: "I64", value: value.value.toString() }
		case "f64":
			return { kind: "F64", value: f64BitsHex(value.value) }
		case "string":
			return { kind: "String", value: value.value }
		case "uuid":
			return { kind: "Uuid", value: value.value }
		case "fixedBytes":
			return { kind: "FixedBytes", value: hex(value.value) }
		case "intervalU64":
			return { kind: "IntervalU64", start: value.start.toString(), end: value.end.toString() }
		case "intervalI64":
			return { kind: "IntervalI64", start: value.start.toString(), end: value.end.toString() }
		case "intervalF64":
			return { kind: "IntervalF64", start: f64BitsHex(value.start), end: f64BitsHex(value.end) }
	}
}

function literalIn(literal: LiteralSpec): LiteralSpecIn {
	return literal.kind === "handle"
		? { kind: "Handle", handle: literal.handle }
		: { kind: "Value", value: valueIn(literal.value) }
}

function literalSetIn(set: LiteralSetSpec): LiteralSetSpecIn {
	return set.kind === "one"
		? { kind: "One", literal: literalIn(set.literal) }
		: { kind: "Many", literals: set.literals.map(literalIn) }
}

function boundIn(bound: CapacityBoundSpec): BoundSpecIn {
	switch (bound.kind) {
		case "lit":
			return { kind: "Lit", value: bound.value.toString() }
		case "field":
			return { kind: "Field", field: bound.field }
		case "durationField":
			return { kind: "Duration", field: bound.field }
	}
}

function windowIn(window: CapacityWindowSpec): CapacityWindowSpecIn {
	switch (window.kind) {
		case "exact":
			return { kind: "Exact", n: boundIn(window.n) }
		case "range":
			return { kind: "Range", lo: boundIn(window.lo), hi: boundIn(window.hi) }
		case "floor":
			return { kind: "Floor", lo: boundIn(window.lo) }
	}
}

function weightIn(weight: WeightSpec): WeightSpecIn {
	switch (weight.kind) {
		case "unit":
			return { kind: "Unit" }
		case "field":
			return { kind: "Field", field: weight.field }
		case "durationField":
			return { kind: "Duration", field: weight.field }
	}
}

export { literalIn, literalSetIn, valueIn, weightIn, windowIn }
