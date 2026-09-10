import { regex } from "arkregex"
import { AuthoringError } from "#errors.ts"
import type { LiteralSpec } from "#spec.ts"
import type { Uuid } from "#uuid.ts"
import { arrayValue, fieldValue, recordValue, taggedValueOf, U64_MAX } from "#values.ts"

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
 * A half-open dense float interval `[start, end)` as a plain value object:
 * two canonical binary64 bounds on the dense numeric line.
 * Field-directed validation rejects NaN endpoints and empty intervals and
 * normalizes signed zero. Infinite bounds denote a missing bound, not a
 * member point; a plain object alone is not evidence of validation.
 */
interface FloatIntervalValue {
	readonly start: number
	readonly end: number
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
 * The application-owned 128-bit identity scalar: sixteen
 * exact bytes, spelled as the canonical hyphenated UUID {@link Uuid}
 * host value. There is no `fresh` mark anywhere: the database issues no
 * identity, and key laws are declared statements.
 */
interface UuidField {
	readonly kind: "uuid"
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
	| UuidField
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
						: F extends { readonly kind: "uuid" }
							? Uuid
							: F extends { readonly kind: "bytes" }
								? Uint8Array
								: F extends { readonly kind: "interval"; readonly element: infer Element }
									? Element extends "f64"
										? FloatIntervalValue
										: IntervalValue
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

function assertDeclarationOrderKey(where: string, name: string): void {
	if (typeof name !== "string" || !name.isWellFormed()) {
		throw new AuthoringError({ message: `${where}: expected a well-formed Unicode name` })
	}
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
	if (typeof record !== "object" || record === null) {
		throw new AuthoringError({ message: `${where}: expected a plain declaration record` })
	}
	const proto = Object.getPrototypeOf(record)
	if (proto !== Object.prototype && proto !== null) {
		throw new AuthoringError({
			message: `${where}: the declaration record's prototype was replaced — a plain \`__proto__: {...}\` entry is the prototype setter, so its key silently vanishes from the declaration; spell it computed (["__proto__"]: {...}) to declare it as data`
		})
	}
	for (const key of Reflect.ownKeys(record)) {
		const property = Object.getOwnPropertyDescriptor(record, key)
		if (typeof key !== "string" || property?.enumerable !== true || !("value" in property)) {
			throw new AuthoringError({ message: `${where}: declarations require enumerable own data fields` })
		}
	}
}

/** Own a checked roster; never freeze the caller's array. */
function ownHandles<const H extends ClosedHandleTuple>(context: string, handles: H): H
function ownHandles(context: string, handles: unknown): ClosedHandleTuple
function ownHandles(context: string, handles: unknown): ClosedHandleTuple {
	if (!Array.isArray(handles) || handles.length === 0) {
		throw new AuthoringError({ message: `${context}: expected a nonempty handle tuple` })
	}
	const seen = new Set<string>()
	const values = arrayValue(context, handles, (_, handle) => {
		if (typeof handle !== "string") throw new AuthoringError({ message: `${context}: expected string handles` })
		if (!handle.isWellFormed()) throw new AuthoringError({ message: `${context}: expected well-formed string handles` })
		if (seen.has(handle)) throw new AuthoringError({ message: `${context}: duplicate handle ${handle}` })
		seen.add(handle)
		return handle
	})
	const first = values[0]
	if (first === undefined) throw new AuthoringError({ message: `${context}: expected a nonempty handle tuple` })
	return Object.freeze([first, ...values.slice(1)])
}

/** Checked structural field descriptors, shared by declaration boundaries. */
function fieldDescriptor<F extends AnyField>(context: string, input: F): F
function fieldDescriptor(context: string, input: unknown): AnyField
function fieldDescriptor(context: string, input: unknown): AnyField {
	if (typeof input !== "object" || input === null || !("kind" in input)) {
		throw new AuthoringError({ message: `${context}: expected a field descriptor` })
	}
	assertDeclarationRecord(context, input)
	switch (input.kind) {
		case "bool":
		case "str":
		case "i64":
		case "f64":
		case "uuid":
			recordValue(context, input, ["kind"])
			return Object.freeze({ kind: input.kind })
		case "u64": {
			if (!("closed" in input)) {
				recordValue(context, input, ["kind"])
				return u64
			}
			recordValue(context, input, ["kind", "closed"])
			const roster = recordValue(context, input.closed, ["name", "handles"])
			if (typeof roster.name !== "string") throw new AuthoringError({ message: `${context}: expected a roster name` })
			assertDeclarationOrderKey(context, roster.name)
			return Object.freeze({
				kind: "u64",
				closed: Object.freeze({ name: roster.name, handles: ownHandles(context, roster.handles) })
			})
		}
		case "bytes": {
			const value = recordValue(context, input, ["kind", "width"])
			if (typeof value.width !== "number") throw new AuthoringError({ message: `${context}: expected a bytes width` })
			return bytes(value.width)
		}
		case "interval": {
			const value = recordValue(context, input, ["kind", "element", "width"])
			if (value.element !== "u64" && value.element !== "i64" && value.element !== "f64") {
				throw new AuthoringError({ message: `${context}: expected an interval element kind` })
			}
			if (value.width === undefined) return interval({ kind: value.element })
			if (value.element === "f64" || typeof value.width !== "bigint") {
				throw new AuthoringError({ message: `${context}: only discrete intervals accept a bigint width` })
			}
			return interval({ kind: value.element }, value.width)
		}
		default:
			throw new AuthoringError({ message: `${context}: unknown field kind` })
	}
}

const u64: U64Field = Object.freeze({ kind: "u64" })

const i64: I64Field = Object.freeze({ kind: "i64" })

/** Binary64. The native value boundary canonicalizes NaN and signed zero. */
const f64: F64Field = Object.freeze({ kind: "f64" })

/** The application-owned 128-bit identity scalar; no fresh mark exists. */
const uuid: UuidField = Object.freeze({ kind: "uuid" })

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
	if (width !== undefined && (typeof width !== "bigint" || width < 1n || width > U64_MAX)) {
		throw new AuthoringError({
			message: "interval width must be a bigint in 1..=u64::MAX"
		})
	}
	return Object.freeze({ kind: "interval", element: elementKind, width })
}

function literalOf(field: AnyField, value: unknown): LiteralSpec {
	if ("closed" in field) {
		return { kind: "handle", handle: fieldValue("selection literal", field, value) }
	}
	return { kind: "value", value: taggedValueOf("selection literal", field, value) }
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
	Infer,
	IntervalElementKind,
	IntervalField,
	IntervalValue,
	SignatureOf,
	StrField,
	U64Field,
	UuidField
}
export {
	assertDeclarationOrderKey,
	assertDeclarationRecord,
	bool,
	bytes,
	f64,
	fieldDescriptor,
	i64,
	interval,
	isFloatIntervalValue,
	isIntervalValue,
	literalOf,
	literalShapeError,
	ownHandles,
	rosterOf,
	rostersAgree,
	signaturesAgree,
	str,
	u64,
	uuid
}
