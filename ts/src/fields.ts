import { regex } from "arkregex"
import { Result } from "effect"
import { AuthoringError } from "#errors.ts"
import type { Id128 } from "#id128.ts"
import { Id128 as Id128Value } from "#id128.ts"
import { DbError } from "#runtime-errors.ts"
import type { LiteralSpec } from "#spec.ts"

const INTEGER_INDEX_NAME = regex("^(?:0|[1-9][0-9]*)$")

/**
 * A half-open integer interval `[start, end)` as a plain value object —
 * the ONE discrete interval value type, whatever the field's element type
 * or width label. The ray is representable (`end` = the element type's
 * MAX_END); widths and signedness are NOT modeled on the value — they are
 * descriptor-type labels the engine judges at the typed write boundary.
 * Interval fields derive no order, so no comparators exist on the value.
 */
interface IntervalValue {
	readonly start: bigint
	readonly end: bigint
}

/**
 * A half-open dense float interval `[start, end)` as a plain value object
 * (chapter 11): two canonical binary64 bounds on the dense numeric line.
 * NaN is never an endpoint, signed zero is normalized at the checked
 * constructor and again by the native boundary, and strict `start < end`
 * makes empty spans unrepresentable through {@link span}. Infinite bounds
 * denote a missing bound, not a member point.
 */
interface FloatIntervalValue {
	readonly start: number
	readonly end: number
}

/**
 * Constructs a checked interval literal — the `start..end` spelling.
 * Half-open and nonempty by construction. This is one of chapter 35's
 * "checked small interval constructors": genuinely fallible pure parsing
 * returns `Result` (use `Effect.fromResult(span(...))` inside a generator),
 * never hidden I/O and never a thrown domain outcome.
 *
 * Two element domains, selected by argument type: `span(0n, 60n)` is a
 * discrete integer interval; `span(0.5, 1.5)` is a dense float interval
 * with canonical endpoints (NaN refused, `-0` normalized to `+0`,
 * `-Infinity` legal only as the lower bound and `+Infinity` only as the
 * upper bound — both enforced by strict numeric `start < end`).
 */
function span(start: bigint, end: bigint): Result.Result<IntervalValue, DbError>
function span(start: number, end: number): Result.Result<FloatIntervalValue, DbError>
function span(
	start: bigint | number,
	end: bigint | number
): Result.Result<IntervalValue, DbError> | Result.Result<FloatIntervalValue, DbError> {
	if (typeof start === "bigint" && typeof end === "bigint") {
		if (start >= end) {
			return Result.fail(new DbError({ operation: "span", reason: { _tag: "InvalidArgument" } }))
		}
		return Result.succeed(Object.freeze({ start, end }))
	}
	if (typeof start === "number" && typeof end === "number") {
		if (Number.isNaN(start) || Number.isNaN(end)) {
			return Result.fail(new DbError({ operation: "span", reason: { _tag: "InvalidArgument" } }))
		}
		const lo = Object.is(start, -0) ? 0 : start
		const hi = Object.is(end, -0) ? 0 : end
		if (!(lo < hi)) {
			return Result.fail(new DbError({ operation: "span", reason: { _tag: "InvalidArgument" } }))
		}
		return Result.succeed(Object.freeze({ start: lo, end: hi }))
	}
	return Result.fail(new DbError({ operation: "span", reason: { _tag: "InvalidArgument" } }))
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

interface U64Field {
	readonly kind: "u64"
}

interface I64Field {
	readonly kind: "i64"
}

interface F64Field {
	readonly kind: "f64"
}

/**
 * The application-owned 128-bit identity scalar (chapter 30/34): sixteen
 * exact bytes, spelled as the canonical 32-lowercase-hex {@link Id128}
 * host value. There is no `fresh` mark anywhere: the database issues no
 * identity, and key laws are declared statements.
 */
interface Id128Field {
	readonly kind: "id128"
}

interface BytesField<Width extends number = number> {
	readonly kind: "bytes"
	readonly width: Width
}

type IntervalElementKind = "u64" | "i64" | "f64"

interface IntervalField<
	Element extends IntervalElementKind = IntervalElementKind,
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
	| I64Field
	| F64Field
	| Id128Field
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
						: F extends { readonly kind: "id128" }
							? Id128
							: F extends { readonly kind: "bytes" }
								? Uint8Array
								: F extends { readonly kind: "interval"; readonly element: "f64" }
									? FloatIntervalValue
									: F extends { readonly kind: "interval" }
										? IntervalValue
										: never

/**
 * The typed shape refusal shared by every literal machine — the selection
 * lowering here, the row codec (`rows.ts`), and the query-literal
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

function isFloatIntervalValue(value: unknown): value is FloatIntervalValue {
	return (
		typeof value === "object" &&
		value !== null &&
		"start" in value &&
		"end" in value &&
		typeof value.start === "number" &&
		typeof value.end === "number"
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

function intervalLiteral(element: IntervalElementKind, value: unknown): LiteralSpec {
	if (element === "f64") {
		if (!isFloatIntervalValue(value)) {
			throw literalShapeError("selection literal", "float interval ({ start, end } numbers)", value)
		}
		if (Number.isNaN(value.start) || Number.isNaN(value.end) || !(value.start < value.end)) {
			throw new AuthoringError({
				message: "selection literal: a float interval is half-open and nonempty with non-NaN canonical endpoints"
			})
		}
		return {
			kind: "value",
			value: {
				kind: "intervalF64",
				start: Object.is(value.start, -0) ? 0 : value.start,
				end: Object.is(value.end, -0) ? 0 : value.end
			}
		}
	}
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

const u64: U64Field = Object.freeze({ kind: "u64" })

const i64: I64Field = Object.freeze({ kind: "i64" })

/** Binary64. The native value boundary canonicalizes NaN and signed zero. */
const f64: F64Field = Object.freeze({ kind: "f64" })

/** The application-owned 128-bit identity scalar; no fresh mark exists. */
const id128: Id128Field = Object.freeze({ kind: "id128" })

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

function interval<Element extends U64Field | I64Field | F64Field>(
	element: Element
): IntervalField<Element["kind"], undefined>
function interval<Element extends U64Field | I64Field, const Width extends bigint>(
	element: Element,
	width: Width
): IntervalField<Element["kind"], Width>
function interval(
	element: U64Field | I64Field | F64Field,
	width?: bigint
): IntervalField<IntervalElementKind, bigint | undefined> {
	const elementKind = element.kind
	if (elementKind !== "u64" && elementKind !== "i64" && elementKind !== "f64") {
		throw new AuthoringError({
			message: `interval element must be the u64, i64 or f64 field constructor (got ${elementKind})`
		})
	}
	if (width !== undefined && elementKind === "f64") {
		throw new AuthoringError({
			message:
				"interval(f64) takes no width — a fixed-width float interval is unrepresentable (rounded start + width is not an exact fixed length on the dense line); applications supply two checked bounds"
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
		case "id128": {
			if (!Id128Value.isId128(value)) {
				throw literalShapeError("selection literal", "an Id128 (32 lowercase hex characters)", value)
			}
			return { kind: "value", value: { kind: "id128", value } }
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
	FloatIntervalValue,
	I64Field,
	Id128Field,
	Infer,
	IntervalElementKind,
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
	id128,
	interval,
	isFloatIntervalValue,
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
