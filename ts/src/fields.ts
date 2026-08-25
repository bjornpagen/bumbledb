import * as errors from "@superbuilders/errors"
import { regex } from "arkregex"
import type { LiteralSpec } from "#spec.ts"

const INTEGER_INDEX_NAME = regex("^(?:0|[1-9][0-9]*)$")

/**
 * A half-open interval `[start, end)` as a plain value object — the ONE
 * interval value type, whatever the field's element type or width label.
 * The ray is representable (`end` = the element type's MAX_END); widths
 * and signedness are NOT modeled on the value — they are descriptor-type
 * labels the engine judges at the typed write boundary. Interval fields
 * derive no order (the Rust refusal,,
 * so no comparators exist on the value type.
 */
interface IntervalValue {
	readonly start: bigint
	readonly end: bigint
}

/**
 * Constructs an interval literal — the `start..end` spelling. Half-open
 * and nonempty by construction: `start >= end` is a typed construction
 * error (parse, don't validate — the same invariant Rust's
 * `Interval::new` enforces at the host boundary). The value is bare and
 * structural: it is assignable to any interval field.
 */
function span(start: bigint, end: bigint): IntervalValue {
	if (start >= end) {
		throw errors.new(`interval is half-open and nonempty: start must be < end (got ${start}..${end})`)
	}
	return Object.freeze({ start, end })
}

interface ClosedRoster<Name extends string = string, H extends string = string> {
	readonly name: Name
	readonly handles: readonly H[]
}

interface BoolField {
	readonly kind: "bool"
}

interface StrField {
	readonly kind: "str"
}

interface FreshU64Field {
	readonly kind: "u64"
	readonly fresh: true
}

interface U64Field {
	readonly kind: "u64"
	readonly fresh: FreshU64Field
}

interface I64Field {
	readonly kind: "i64"
}

interface BytesField<Width extends number = number> {
	readonly kind: "bytes"
	readonly width: Width
}

interface IntervalField<
	Element extends "u64" | "i64" = "u64" | "i64",
	Width extends bigint | undefined = bigint | undefined
> {
	readonly kind: "interval"
	readonly element: Element
	readonly width: Width
}

interface ClosedIdField<Name extends string = string, H extends string = string> {
	readonly kind: "u64"
	readonly closed: ClosedRoster<Name, H>
}

type AnyField = BoolField | StrField | U64Field | FreshU64Field | I64Field | BytesField | IntervalField | ClosedIdField

type Infer<F extends AnyField> = F extends { readonly kind: "bool" }
	? boolean
	: F extends { readonly kind: "str" }
		? string
		: F extends { readonly closed: { readonly handles: readonly (infer H extends string)[] } }
			? H
			: F extends { readonly kind: "u64" }
				? bigint
				: F extends { readonly kind: "i64" }
					? bigint
					: F extends { readonly kind: "bytes" }
						? Uint8Array
						: F extends { readonly kind: "interval" }
							? IntervalValue
							: never

/**
 * The typed shape refusal shared by every literal machine — the selection
 * lowering here, the row marshaler (`marshal.ts`), and the query-literal
 * tagger (`query/lower.ts`) all throw through this ONE voice; reached only
 * through ill-typed input (the well-typed surfaces make it unrepresentable).
 */
function literalShapeError(context: string, expected: string, value: unknown): Error {
	return errors.new(`${context}: expected ${expected}, got ${typeof value}`)
}

function rosterOf(field: AnyField | undefined): ClosedRoster | undefined {
	if (field !== undefined && "closed" in field) {
		return field.closed
	}
	return undefined
}

function isIntervalValue(value: unknown): value is IntervalValue {
	return (
		typeof value === "object" &&
		value !== null &&
		"start" in value &&
		"end" in value &&
		typeof value.start === "bigint" &&
		typeof value.end === "bigint"
	)
}

function handleLiteral(closed: ClosedRoster, value: unknown): LiteralSpec {
	if (typeof value !== "string") {
		throw literalShapeError("selection literal", `a ${closed.name} handle name (string)`, value)
	}
	if (!closed.handles.includes(value)) {
		throw errors.new(`"${value}" is not a handle of ${closed.name} — the roster is ${closed.handles.join(", ")}`)
	}
	return { kind: "handle", handle: value }
}

function intervalLiteral(element: "u64" | "i64", value: unknown): LiteralSpec {
	if (!isIntervalValue(value)) {
		throw literalShapeError("selection literal", "interval ({ start, end } bigints)", value)
	}
	if (element === "u64") {
		return { kind: "value", value: { kind: "intervalU64", start: value.start, end: value.end } }
	}
	return { kind: "value", value: { kind: "intervalI64", start: value.start, end: value.end } }
}

function assertDeclarationOrderKey(where: string, name: string): void {
	if (INTEGER_INDEX_NAME.test(name)) {
		throw errors.new(
			`${where}: name ${name} is an integer index — JavaScript object keys re-order integer indices, breaking the declaration-order law; use a non-numeric name`
		)
	}
	if (name.includes(".")) {
		throw errors.new(
			`${where}: name ${name} contains a dot — the law classes key on the \`relation.field\` coordinate, so a dotted name would alias unrelated slots (macro parity: Rust identifiers cannot contain dots); use a dot-free name`
		)
	}
}

function assertDeclarationRecord(where: string, record: object): void {
	const proto = Object.getPrototypeOf(record)
	if (proto !== Object.prototype && proto !== null) {
		throw errors.new(
			`${where}: the declaration record's prototype was replaced — a plain \`__proto__: {...}\` entry is the prototype setter, so its key silently vanishes from the declaration; spell it computed (["__proto__"]: {...}) to declare it as data`
		)
	}
}

const freshU64: FreshU64Field = Object.freeze({ kind: "u64", fresh: true })

const u64: U64Field = Object.freeze({ kind: "u64", fresh: freshU64 })

const i64: I64Field = Object.freeze({ kind: "i64" })

const bool: BoolField = Object.freeze({ kind: "bool" })

const str: StrField = Object.freeze({ kind: "str" })

function bytes<const Width extends number>(width: Width): BytesField<Width> {
	if (!Number.isInteger(width) || width < 1 || width > 64) {
		throw errors.new(`bytes width must be an integer in 1..=64 (got ${width}) — the range is pinned at declaration`)
	}
	return Object.freeze({ kind: "bytes", width })
}

function interval<Element extends U64Field | I64Field>(element: Element): IntervalField<Element["kind"], undefined>
function interval<Element extends U64Field | I64Field, const Width extends bigint>(
	element: Element,
	width: Width
): IntervalField<Element["kind"], Width>
function interval(element: U64Field | I64Field, width?: bigint): IntervalField<"u64" | "i64", bigint | undefined> {
	const elementKind = element.kind
	if (elementKind !== "u64" && elementKind !== "i64") {
		throw errors.new(`interval element must be the u64 or i64 field constructor (got ${elementKind})`)
	}
	if (width !== undefined && width < 1n) {
		throw errors.new(`interval width must be >= 1 (got ${width}) — w >= 1 is pinned at declaration`)
	}
	return Object.freeze({ kind: "interval", element: elementKind, width })
}

function literalOf(field: AnyField, value: unknown): LiteralSpec {
	const roster = rosterOf(field)
	if (roster !== undefined) {
		return handleLiteral(roster, value)
	}
	switch (field.kind) {
		case "bool": {
			if (typeof value !== "boolean") {
				throw literalShapeError("selection literal", "boolean", value)
			}
			return { kind: "value", value: { kind: "bool", value } }
		}
		case "u64": {
			if (typeof value !== "bigint") {
				throw literalShapeError("selection literal", "bigint", value)
			}
			return { kind: "value", value: { kind: "u64", value } }
		}
		case "i64": {
			if (typeof value !== "bigint") {
				throw literalShapeError("selection literal", "bigint", value)
			}
			return { kind: "value", value: { kind: "i64", value } }
		}
		case "str": {
			if (typeof value !== "string") {
				throw literalShapeError("selection literal", "string", value)
			}

			if (!value.isWellFormed()) {
				throw literalShapeError("selection literal", "well-formed string", value)
			}
			return { kind: "value", value: { kind: "string", value } }
		}
		case "bytes": {
			if (!(value instanceof Uint8Array)) {
				throw literalShapeError("selection literal", "Uint8Array", value)
			}
			return { kind: "value", value: { kind: "fixedBytes", value } }
		}
		case "interval":
			return intervalLiteral(field.element, value)
	}
}

export type {
	AnyField,
	BoolField,
	BytesField,
	ClosedIdField,
	ClosedRoster,
	FreshU64Field,
	I64Field,
	Infer,
	IntervalField,
	IntervalValue,
	StrField,
	U64Field
}
export {
	assertDeclarationOrderKey,
	assertDeclarationRecord,
	bool,
	bytes,
	i64,
	interval,
	isIntervalValue,
	literalOf,
	literalShapeError,
	rosterOf,
	span,
	str,
	u64
}
