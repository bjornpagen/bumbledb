/**
 * Field descriptors — the value half of the `schema!` field grammar
 * (`docs/architecture/70-api.md`), MINIMAL edition: `bool`, `u64`, `i64`,
 * `str`, `bytes(n)`, `interval(u64|i64[, width])`, each a plain frozen value
 * that IS its own descriptor type — `{ kind, width?, element?, fresh? }` —
 * honest at runtime and in the type alike. A field's VALUE type is its bare
 * structural type (`u64` → `bigint`, `str` → `string`, `bytes(n)` →
 * `Uint8Array`, intervals → `{ start, end }`): no brands, no phantoms, no
 * minting casts. A descriptor carries STRUCTURE ONLY — domains are never
 * declared anywhere (the owner ruling: THE LAWS TYPE THE COLUMNS): a
 * field's domain is COMPUTED by `schema()` from the statement list, where
 * the dependencies themselves induce the equivalence classes. The macro's
 * refusals are reproduced representationally: `.fresh` exists only on u64,
 * and no field-level constraint vocabulary of any kind exists —
 * `unique`/`fk` are unwritable, not rejected.
 */

import * as errors from "@superbuilders/errors"
import type { LiteralSpec } from "#spec.ts"

/**
 * A half-open interval `[start, end)` as a plain value object — the ONE
 * interval value type, whatever the field's element type or width label.
 * The ray is representable (`end` = the element type's MAX_END); widths
 * and signedness are NOT modeled on the value — they are descriptor-type
 * labels the engine judges at the typed write boundary. Interval fields
 * derive no order (the Rust refusal, `docs/architecture/10-data-model.md`),
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

/**
 * A closed relation's roster as seen from a referencing field: the handle
 * namespace `where()` selections and ground axioms resolve bare handle ids
 * through (the macro's own rule: a handle is legal exactly on a field that
 * references a closed relation). The handle union is PRECISE — `H` carries
 * the literal handle names in declaration order (the unbound `string`
 * default exists only as the fallback where no roster is in scope); the
 * runtime twin is the same frozen declaration-order array that was always
 * there.
 */
interface ClosedRoster<H extends string = string> {
	readonly name: string
	readonly handles: readonly H[]
}

/** The `bool` field descriptor: value type `boolean`. No `.fresh` (macro parity). */
interface BoolField {
	readonly kind: "bool"
}

/** The `str` field descriptor: value type `string`. No `.fresh` (macro parity). */
interface StrField {
	readonly kind: "str"
}

/**
 * A `fresh`-marked u64 field descriptor — `id: u64.fresh`. The mark is a
 * structural label (`fresh: true`) in the descriptor type AND on the
 * runtime value; it implies the key `R(field) -> R`, which the ENGINE
 * materializes (`SchemaDescriptor::materialized_statements`), and it makes
 * the field a GENERATOR — `schema()` names its equivalence class by the
 * declaration coordinate (`"Account.id"`). Terminal: no builder property
 * survives the mark.
 */
interface FreshU64Field {
	readonly kind: "u64"
	readonly fresh: true
}

/**
 * The `u64` field descriptor. `.fresh` marks the field as engine-minted —
 * the property doubles as the mark itself: on an unmarked descriptor it
 * holds the marked descriptor, on a marked one it IS the literal `true`
 * (one structural property, read either way).
 */
interface U64Field {
	readonly kind: "u64"
	readonly fresh: FreshU64Field
}

/** The `i64` field descriptor. Terminal: `.fresh` is legal on u64 only. */
interface I64Field {
	readonly kind: "i64"
}

/**
 * A `bytes<N>` field descriptor. The width is a descriptor-type label
 * (load-bearing: the engine enforces it at the write boundary) and the
 * value type is bare `Uint8Array`. No order is derived — no comparators
 * exist on the value type (the engine refuses order on bytes).
 */
interface BytesField<Width extends number = number> {
	readonly kind: "bytes"
	readonly width: Width
}

/**
 * An interval field descriptor — `interval(i64)` general (rays
 * representable), `interval(u64, w)` the fixed-width family. Element and
 * width are descriptor-type labels; the value type is always the bare
 * {@link IntervalValue}.
 */
interface IntervalField<
	Element extends "u64" | "i64" = "u64" | "i64",
	Width extends bigint | undefined = bigint | undefined
> {
	readonly kind: "interval"
	readonly element: Element
	readonly width: Width
}

/**
 * A closed relation's reference field descriptor (`Kind.id`) — a u64
 * descriptor carrying the closed linkage: the roster resolves handle
 * literals in selections and ground axioms, and `schema()` names the id's
 * generator class `"Kind.id"`. The handle union `H` is the field's VALUE
 * TYPE (see {@link Infer}); `kind: "u64"` stays load-bearing for the class
 * map and JoinOk, which compare kind/class/width/element. Terminal: no
 * `.fresh` — a vocabulary's rows are ground axioms, never minted.
 */
interface ClosedIdField<H extends string = string> {
	readonly kind: "u64"
	readonly closed: ClosedRoster<H>
}

/** Any field descriptor, whatever its kind or marks. */
type AnyField = BoolField | StrField | U64Field | FreshU64Field | I64Field | BytesField | IntervalField | ClosedIdField

/**
 * The bare structural VALUE type of a field descriptor — the one total
 * definition every fact, result row, and query term reads: `bool` →
 * `boolean`, `str` → `string`, `u64`/`i64` → `bigint`, `bytes<N>` →
 * `Uint8Array`, intervals → {@link IntervalValue}, and a closed reference →
 * its PRECISE handle union (`"DirectPass" | "Failed"` — the string-literal
 * union IS the value type at the TS surface; the engine keeps u64 row ids
 * and the marshal owns the bijection). The closed arm precedes the `u64`
 * arm because a closed reference is structurally a u64 descriptor plus the
 * roster.
 */
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

/**
 * The roster a field descriptor carries — THE one reader: present exactly
 * on a closed-reference descriptor (the structural `closed` property of
 * {@link ClosedIdField}), absent on every other field kind. Tolerates
 * `undefined` so name-lookup misses flow through without a re-spelled
 * probe at every call site.
 */
function rosterOf(field: AnyField | undefined): ClosedRoster | undefined {
	if (field !== undefined && "closed" in field) {
		return field.closed
	}
	return undefined
}

/** Narrows an interval-shaped value: a plain object with bigint start/end — THE one interval predicate. */
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

/**
 * Resolves one closed-handle literal: the handle NAME, verified against the
 * roster — an unknown name is a construction error, the belt the wide
 * fallback type deliberately does not provide (structural values make any
 * string spellable here; the roster judges). The name IS the value at the
 * TS surface (the drizzle law); the wire literal already crossed as
 * `{ kind: "handle", handle }`, so the output — and every fingerprint
 * derived from it — is untouched.
 */
function handleLiteral(closed: ClosedRoster, value: unknown): LiteralSpec {
	if (typeof value !== "string") {
		throw literalShapeError("selection literal", `a ${closed.name} handle name (string)`, value)
	}
	if (!closed.handles.includes(value)) {
		throw errors.new(`"${value}" is not a handle of ${closed.name} — the roster is ${closed.handles.join(", ")}`)
	}
	return { kind: "handle", handle: value }
}

/** Lowers one interval literal at its element type. */
function intervalLiteral(element: "u64" | "i64", value: unknown): LiteralSpec {
	if (!isIntervalValue(value)) {
		throw literalShapeError("selection literal", "interval ({ start, end } bigints)", value)
	}
	if (element === "u64") {
		return { kind: "value", value: { kind: "intervalU64", start: value.start, end: value.end } }
	}
	return { kind: "value", value: { kind: "intervalI64", start: value.start, end: value.end } }
}

/**
 * Rejects a declaration name that JavaScript would re-order, and a name
 * that would break the class map's coordinate encoding. Declaration order =
 * ordinal ids is the law relations, columns, and schemas all lean on, and
 * it is carried by object-literal key order — which ECMA-262's
 * OrdinaryOwnPropertyKeys breaks for integer-index keys (they enumerate
 * first, ascending, regardless of where they were written). A `.` in a name
 * would make the law engine's `${relation}.${field}` coordinate template
 * non-injective at BOTH tiers (relation `"A.B"` field `"x"` and relation
 * `"A"` field `"B.x"` are one coordinate), silently merging unrelated law
 * classes — banned here, which is exact macro parity: Rust identifiers
 * cannot contain dots. Both are construction errors, exactly as an
 * unparseable name is a macro expansion error.
 */
function assertDeclarationOrderKey(where: string, name: string): void {
	if (/^(?:0|[1-9][0-9]*)$/.test(name)) {
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

/**
 * Rejects a declaration record whose prototype was replaced. A plain
 * `__proto__: {...}` property in an object literal is ECMA-262 Annex B's
 * prototype SETTER, not a data property — the entry never becomes an own
 * enumerable key, so the declared handle/field/relation would silently
 * vanish from every `Object.keys`/`Object.entries` walk while the type
 * tier still admits its name. A non-default prototype on a declaration
 * literal proves exactly that spelling, so it is a construction error; the
 * computed spelling `["__proto__"]: {...}` creates an own data property
 * and is admitted (no name is reserved). `Object.create(null)` records
 * stay admissible.
 */
function assertDeclarationRecord(where: string, record: object): void {
	const proto = Object.getPrototypeOf(record)
	if (proto !== Object.prototype && proto !== null) {
		throw errors.new(
			`${where}: the declaration record's prototype was replaced — a plain \`__proto__: {...}\` entry is the prototype setter, so its key silently vanishes from the declaration; spell it computed (["__proto__"]: {...}) to declare it as data`
		)
	}
}

/** The one fresh-marked u64 descriptor (the `.fresh` property of the unmarked one). */
const freshU64: FreshU64Field = Object.freeze({ kind: "u64", fresh: true })

/** The one `u64` constructor value. */
const u64: U64Field = Object.freeze({ kind: "u64", fresh: freshU64 })

/** The one `i64` constructor value. */
const i64: I64Field = Object.freeze({ kind: "i64" })

/** The one `bool` constructor value. */
const bool: BoolField = Object.freeze({ kind: "bool" })

/** The one `str` constructor value. */
const str: StrField = Object.freeze({ kind: "str" })

/**
 * The `bytes<N>` field constructor. The width is mandatory and a
 * descriptor-type label; `width` is validated to 1..=64 here because the
 * grammar pins that range at declaration (`docs/architecture/70-api.md`
 * § the `schema!` grammar: N ∈ 1..=64 — bare `bytes` does not parse), the
 * macro-expansion boundary's analog being construction.
 */
function bytes<const Width extends number>(width: Width): BytesField<Width> {
	if (!Number.isInteger(width) || width < 1 || width > 64) {
		throw errors.new(
			`bytes width must be an integer in 1..=64 (got ${width}) — docs/architecture/70-api.md pins the range at declaration`
		)
	}
	return Object.freeze({ kind: "bytes", width })
}

/**
 * The interval field constructor — `interval(u64)` / `interval(i64)` for
 * the general type (rays representable), `interval(u64, w)` for the
 * fixed-width family whose width IS a descriptor-type label. The element is
 * spelled with the u64/i64 constructor values themselves, never a string.
 * `width >= 1` is validated here because the grammar pins it at declaration
 * (`docs/architecture/70-api.md`: w ≥ 1; `interval<u64, 0>` is an
 * expansion error naming the field).
 */
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
		throw errors.new(
			`interval width must be >= 1 (got ${width}) — docs/architecture/70-api.md pins w >= 1 at declaration`
		)
	}
	return Object.freeze({ kind: "interval", element: elementKind, width })
}

/**
 * Lowers one host literal at its field position to the wire
 * {@link LiteralSpec} — the selection-literal machine ground axioms and
 * `where()` bindings both ride (one machine, same errors — the macro's own
 * rule). A value on a closed-reference field IS its handle NAME (verified
 * against the roster: an unknown name is a construction error); everything
 * else lowers to a plain value tagged by the field's structural kind.
 */
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
			/**
			 * The marshal's bijection law at the schema-literal seam
			 * (`marshal.ts` cellOf): a lone surrogate would cross dbCreate
			 * lossily (stored as U+FFFD engine-side), collapsing two
			 * distinct TS schema values into one stored theory/fingerprint
			 * and splitting the canonical statement rendering from the
			 * SDK's. All three string-admission seams — fact row, query
			 * literal/param, schema literal — enforce the one law.
			 */
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
