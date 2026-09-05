import { regex } from "arkregex"
import { AuthoringError } from "#errors.ts"
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
		throw new AuthoringError({
			message: `interval is half-open and nonempty: start must be < end (got ${start}..${end})`
		})
	}
	return Object.freeze({ start, end })
}

/**
 * Nonempty declaration-order handle vector — the ONE roster carrier. The
 * handle union is `Handles[number]`, the ordinal of a handle is its tuple
 * position, and the roster size is the tuple length. An empty vocabulary
 * is unspellable.
 */
type ClosedHandleTuple = readonly [string, ...string[]]

interface ClosedRoster<Name extends string, Handles extends ClosedHandleTuple> {
	readonly name: Name
	readonly handles: Handles
}

/** The roster top type — what an erased carrier knows about any roster. */
type AnyClosedRoster = ClosedRoster<string, ClosedHandleTuple>

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

interface F64Field {
	readonly kind: "f64"
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

interface ClosedIdField<Name extends string, Handles extends ClosedHandleTuple> {
	readonly kind: "u64"
	readonly closed: ClosedRoster<Name, Handles>
}

/** The closed-id top type — the erased carrier's view of any closed id. */
type AnyClosedIdField = ClosedIdField<string, ClosedHandleTuple>

type AnyField =
	| BoolField
	| StrField
	| U64Field
	| FreshU64Field
	| I64Field
	| F64Field
	| BytesField
	| IntervalField
	| AnyClosedIdField

/**
 * The ONE structural interpreter of a field descriptor — the positional
 * signature every equality judgment reads (the positive join wall in
 * `#query/scope.ts` and the face pairing wall in `#face.ts`). Two fields
 * are one shape exactly when their signatures are the same tuple: kind,
 * width, interval element, and the roster as name plus the handle VECTOR
 * (order and length carry meaning; a set would forget both).
 */
type SignatureOf<F extends AnyField> = readonly [
	F["kind"],
	F extends { readonly width: infer W } ? W : undefined,
	F extends { readonly element: infer E } ? E : undefined,
	F extends {
		readonly closed: { readonly name: infer N extends string; readonly handles: infer H extends ClosedHandleTuple }
	}
		? readonly [N, H]
		: undefined
]

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
					: F extends { readonly kind: "f64" }
						? number
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
	return new AuthoringError({ message: `${context}: expected ${expected}, got ${typeof value}` })
}

function rosterOf(field: AnyField | undefined): AnyClosedRoster | undefined {
	if (field !== undefined && "closed" in field) {
		return field.closed
	}
	return undefined
}

/**
 * The runtime twin of roster equality inside {@link SignatureOf}: same
 * vocabulary name, same handle vector (order and length). Two absent
 * rosters agree; a roster never agrees with a bare field.
 */
function rostersAgree(a: AnyClosedRoster | undefined, b: AnyClosedRoster | undefined): boolean {
	if (a === undefined || b === undefined) {
		return a === b
	}
	return (
		a.name === b.name &&
		a.handles.length === b.handles.length &&
		a.handles.every(function sameHandle(handle, index) {
			return handle === b.handles[index]
		})
	)
}

/**
 * The runtime twin of {@link SignatureOf} equality — the ONE spelling of
 * "these two descriptors are one shape". Kind, width, interval element,
 * and the roster vector must all agree.
 */
function signaturesAgree(a: AnyField, b: AnyField): boolean {
	const widthA = "width" in a ? a.width : undefined
	const widthB = "width" in b ? b.width : undefined
	const elementA = "element" in a ? a.element : undefined
	const elementB = "element" in b ? b.element : undefined
	return a.kind === b.kind && widthA === widthB && elementA === elementB && rostersAgree(rosterOf(a), rosterOf(b))
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

function handleLiteral(closed: AnyClosedRoster, value: unknown): LiteralSpec {
	if (typeof value !== "string") {
		throw literalShapeError("selection literal", `a ${closed.name} handle name (string)`, value)
	}
	if (!closed.handles.includes(value)) {
		throw new AuthoringError({
			message: `"${value}" is not a handle of ${closed.name} — the roster is ${closed.handles.join(", ")}`
		})
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
		throw new AuthoringError({
			message: `${where}: name ${name} is an integer index — JavaScript object keys re-order integer indices, breaking the declaration-order law; use a non-numeric name`
		})
	}
	if (name.includes(".")) {
		throw new AuthoringError({
			message: `${where}: name ${name} contains a dot — the law classes key on the \`relation.field\` coordinate, so a dotted name would alias unrelated slots (macro parity: Rust identifiers cannot contain dots); use a dot-free name`
		})
	}
}

function assertDeclarationRecord(where: string, record: object): void {
	const proto = Object.getPrototypeOf(record)
	if (proto !== Object.prototype && proto !== null) {
		throw new AuthoringError({
			message: `${where}: the declaration record's prototype was replaced — a plain \`__proto__: {...}\` entry is the prototype setter, so its key silently vanishes from the declaration; spell it computed (["__proto__"]: {...}) to declare it as data`
		})
	}
}

const freshU64: FreshU64Field = Object.freeze({ kind: "u64", fresh: true })

const u64: U64Field = Object.freeze({ kind: "u64", fresh: freshU64 })

const i64: I64Field = Object.freeze({ kind: "i64" })

/** Binary64. The native value boundary canonicalizes NaN and signed zero. */
const f64: F64Field = Object.freeze({ kind: "f64" })

const bool: BoolField = Object.freeze({ kind: "bool" })

const str: StrField = Object.freeze({ kind: "str" })

function bytes<const Width extends number>(width: Width): BytesField<Width> {
	if (!Number.isInteger(width) || width < 1 || width > 64) {
		throw new AuthoringError({
			message: `bytes width must be an integer in 1..=64 (got ${width}) — the range is pinned at declaration`
		})
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
		throw new AuthoringError({
			message: `interval element must be the u64 or i64 field constructor (got ${elementKind})`
		})
	}
	if (width !== undefined && width < 1n) {
		throw new AuthoringError({
			message: `interval width must be >= 1 (got ${width}) — w >= 1 is pinned at declaration`
		})
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
		case "f64": {
			if (typeof value !== "number") {
				throw literalShapeError("selection literal", "number", value)
			}
			return { kind: "value", value: { kind: "f64", value } }
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
	AnyClosedIdField,
	AnyClosedRoster,
	AnyField,
	BoolField,
	BytesField,
	ClosedHandleTuple,
	ClosedIdField,
	ClosedRoster,
	F64Field,
	FreshU64Field,
	I64Field,
	Infer,
	IntervalField,
	IntervalValue,
	SignatureOf,
	StrField,
	U64Field
}
export {
	assertDeclarationOrderKey,
	assertDeclarationRecord,
	bool,
	bytes,
	f64,
	i64,
	interval,
	isIntervalValue,
	literalOf,
	literalShapeError,
	rosterOf,
	rostersAgree,
	signaturesAgree,
	span,
	str,
	u64
}
