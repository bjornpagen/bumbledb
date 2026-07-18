/**
 * Field descriptors — the value half of the `schema!` field grammar
 * (`docs/architecture/70-api.md`), STRUCTURAL edition: `bool`, `u64`, `i64`,
 * `str`, `bytes(n)`, `interval(u64|i64[, width])`, each a plain frozen value
 * that IS its own descriptor type — `{ kind, domain, fresh?, width?,
 * element? }` — honest at runtime and in the type alike. A field's VALUE
 * type is its bare structural type (`u64` → `bigint`, `str` → `string`,
 * `bytes(n)` → `Uint8Array`, intervals → `{ start, end }`): no brands, no
 * phantoms, no minting casts. The domain is a string LABEL in the
 * descriptor type, attached by `.as("HolderId")` (the mirror of Rust's
 * `as HolderId`); same-string domains link fields, and the relational
 * builders (statements, queries) compare the labels structurally — the
 * domain wall lives in the builders and the engine, never on the value.
 * The macro's refusals are reproduced representationally: `.as` exists only
 * where Rust's `as` is legal (u64, i64, bytes, intervals — never bool/str),
 * `.fresh` exists only on u64 (bare or after `.as`), and no field-level
 * constraint vocabulary of any kind exists — `unique`/`fk` are unwritable,
 * not rejected.
 */

import * as errors from "@superbuilders/errors"
import type { LiteralSpec } from "#spec.ts"

/**
 * A half-open interval `[start, end)` as a plain value object — the ONE
 * interval value type, whatever the field's element domain or width label.
 * The ray is representable (`end` = the element domain's MAX_END); widths
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
 * through (the macro's own rule: a handle is legal exactly on a field whose
 * domain is a closed relation's handle domain).
 */
interface ClosedRoster {
	readonly name: string
	readonly handles: readonly string[]
}

/** The `bool` field descriptor: value type `boolean`. No `.as`, no `.fresh` (macro parity). */
interface BoolField {
	readonly kind: "bool"
	readonly domain: undefined
}

/** The `str` field descriptor: value type `string`. No `.as`, no `.fresh` (macro parity). */
interface StrField {
	readonly kind: "str"
	readonly domain: undefined
}

/**
 * A `fresh`-marked u64 field descriptor — `id: u64.as("AccountId").fresh`
 * (Rust: `id: u64 as AccountId, fresh`). The mark is a structural label
 * (`fresh: true`) in the descriptor type AND on the runtime value; it
 * implies the key `R(field) -> R`, which the ENGINE materializes
 * (`SchemaDescriptor::materialized_statements`). Terminal: no builder
 * property survives the mark.
 */
interface FreshU64Field<Domain extends string | undefined = undefined> {
	readonly kind: "u64"
	readonly domain: Domain
	readonly fresh: true
}

/**
 * A domain-labeled u64 field descriptor — `const HolderId =
 * u64.as("HolderId")` (Rust: `u64 as HolderId`). The label lives in the
 * descriptor type only; the value type stays bare `bigint`. `.fresh` marks
 * the field as engine-minted — the property doubles as the mark itself:
 * on an unmarked descriptor it holds the marked descriptor, on a marked
 * one it IS the literal `true` (one structural property, read either way).
 */
interface U64Field<Domain extends string | undefined = undefined> {
	readonly kind: "u64"
	readonly domain: Domain
	readonly fresh: FreshU64Field<Domain>
}

/** The `u64` constructor value: a bare u64 descriptor plus `.as` (one application — `.as` is absent on the result). */
interface U64Ctor extends U64Field<undefined> {
	as<const Domain extends string>(domain: Domain): U64Field<Domain>
}

/** A domain-labeled i64 field descriptor. Terminal: `.fresh` is legal on u64 only. */
interface I64Field<Domain extends string | undefined = undefined> {
	readonly kind: "i64"
	readonly domain: Domain
}

/** The `i64` constructor value: a bare i64 descriptor plus `.as`. */
interface I64Ctor extends I64Field<undefined> {
	as<const Domain extends string>(domain: Domain): I64Field<Domain>
}

/**
 * A `bytes<N>` field descriptor. The width is a descriptor-type label
 * (load-bearing: the engine enforces it at the write boundary) and the
 * value type is bare `Uint8Array`. No order is derived — no comparators
 * exist on the value type (the engine refuses order on bytes).
 */
interface BytesField<Width extends number = number, Domain extends string | undefined = undefined> {
	readonly kind: "bytes"
	readonly width: Width
	readonly domain: Domain
}

/** A `bytes(n)` constructor value: a bare bytes descriptor plus `.as`. */
interface BytesCtor<Width extends number = number> extends BytesField<Width, undefined> {
	as<const Domain extends string>(domain: Domain): BytesField<Width, Domain>
}

/**
 * An interval field descriptor — `interval(i64)` general (rays
 * representable), `interval(u64, w)` the fixed-width family. Element and
 * width are descriptor-type labels; the value type is always the bare
 * {@link IntervalValue}.
 */
interface IntervalField<
	Element extends "u64" | "i64" = "u64" | "i64",
	Width extends bigint | undefined = bigint | undefined,
	Domain extends string | undefined = undefined
> {
	readonly kind: "interval"
	readonly element: Element
	readonly width: Width
	readonly domain: Domain
}

/** An `interval(e[, w])` constructor value: a bare interval descriptor plus `.as`. */
interface IntervalCtor<
	Element extends "u64" | "i64" = "u64" | "i64",
	Width extends bigint | undefined = bigint | undefined
> extends IntervalField<Element, Width, undefined> {
	as<const Domain extends string>(domain: Domain): IntervalField<Element, Width, Domain>
}

/**
 * A closed relation's reference field descriptor (`Kind.id`) — a u64
 * descriptor whose domain is the closed relation's handle domain
 * (`"KindId"`, mirroring Rust's `closed relation Kind as KindId`) and
 * whose roster resolves bare handle ids in selections and ground axioms.
 * Terminal: no `.as`, no `.fresh` — its domain IS the closed relation's.
 */
interface ClosedIdField<Domain extends string = string> {
	readonly kind: "u64"
	readonly domain: Domain
	readonly closed: ClosedRoster
}

/** Any field descriptor, whatever its kind, domain label, or marks. */
type AnyField =
	| BoolField
	| StrField
	| U64Field<string | undefined>
	| FreshU64Field<string | undefined>
	| I64Field<string | undefined>
	| BytesField<number, string | undefined>
	| IntervalField<"u64" | "i64", bigint | undefined, string | undefined>
	| ClosedIdField

/**
 * The bare structural VALUE type of a field descriptor — the one total
 * definition every fact, result row, and query term reads: `bool` →
 * `boolean`, `str` → `string`, `u64`/`i64` → `bigint` (domain labels
 * included — the label never touches the value), `bytes<N>` →
 * `Uint8Array`, intervals → {@link IntervalValue}.
 */
type Infer<F extends AnyField> = F extends { readonly kind: "bool" }
	? boolean
	: F extends { readonly kind: "str" }
		? string
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
 * The typed shape refusal of the selection-literal machine — reached only
 * through ill-typed input (the well-typed surfaces make it unrepresentable).
 */
function literalShapeError(expected: string, value: unknown): Error {
	return errors.new(`selection literal shape mismatch: expected ${expected}, got ${typeof value}`)
}

/** Narrows an interval literal: a plain object with bigint start/end. */
function isIntervalLiteral(value: unknown): value is IntervalValue {
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
 * Resolves one closed-handle literal: the handle id (a bare bigint) back to
 * its handle NAME through the roster — an out-of-roster id is a
 * construction error, the belt the type level deliberately does not provide
 * (structural values make any bigint spellable here; the roster judges).
 */
function handleLiteral(closed: ClosedRoster, value: unknown): LiteralSpec {
	if (typeof value !== "bigint") {
		throw literalShapeError(`a ${closed.name} handle id (bigint)`, value)
	}
	const handle = closed.handles[Number(value)]
	if (handle === undefined) {
		throw errors.new(
			`closed relation ${closed.name} has no handle with id ${value} (roster holds ${closed.handles.length})`
		)
	}
	return { kind: "handle", handle }
}

/** Lowers one interval literal at its element type. */
function intervalLiteral(element: "u64" | "i64", value: unknown): LiteralSpec {
	if (!isIntervalLiteral(value)) {
		throw literalShapeError("interval ({ start, end } bigints)", value)
	}
	if (element === "u64") {
		return { kind: "value", value: { kind: "intervalU64", start: value.start, end: value.end } }
	}
	return { kind: "value", value: { kind: "intervalI64", start: value.start, end: value.end } }
}

/**
 * Rejects a declaration name that JavaScript would re-order. Declaration
 * order = ordinal ids is the law relations, columns, and schemas all lean
 * on, and it is carried by object-literal key order — which ECMA-262's
 * OrdinaryOwnPropertyKeys breaks for integer-index keys (they enumerate
 * first, ascending, regardless of where they were written). An
 * integer-index name would silently reorder its declaration, so it is a
 * construction error, exactly as an unparseable name is a macro expansion
 * error.
 */
function assertDeclarationOrderKey(where: string, name: string): void {
	if (/^(?:0|[1-9][0-9]*)$/.test(name)) {
		throw errors.new(
			`${where}: name ${name} is an integer index — JavaScript object keys re-order integer indices, breaking the declaration-order law; use a non-numeric name`
		)
	}
}

/** Builds one fresh-marked u64 descriptor (the `.fresh` property of an unmarked one). */
function freshU64<Domain extends string | undefined>(domain: Domain): FreshU64Field<Domain> {
	return Object.freeze({ kind: "u64", domain, fresh: true })
}

/** The one `u64` constructor value. */
const u64: U64Ctor = Object.freeze({
	kind: "u64",
	domain: undefined,
	fresh: freshU64(undefined),
	as<const Domain extends string>(domain: Domain): U64Field<Domain> {
		return Object.freeze({ kind: "u64", domain, fresh: freshU64(domain) })
	}
})

/** The one `i64` constructor value. */
const i64: I64Ctor = Object.freeze({
	kind: "i64",
	domain: undefined,
	as<const Domain extends string>(domain: Domain): I64Field<Domain> {
		return Object.freeze({ kind: "i64", domain })
	}
})

/** The one `bool` constructor value. */
const bool: BoolField = Object.freeze({ kind: "bool", domain: undefined })

/** The one `str` constructor value. */
const str: StrField = Object.freeze({ kind: "str", domain: undefined })

/**
 * The `bytes<N>` field constructor. The width is mandatory and a
 * descriptor-type label; `width` is validated to 1..=64 here because the
 * grammar pins that range at declaration (`docs/architecture/70-api.md`
 * § the `schema!` grammar: N ∈ 1..=64 — bare `bytes` does not parse), the
 * macro-expansion boundary's analog being construction.
 */
function bytes<const Width extends number>(width: Width): BytesCtor<Width> {
	if (!Number.isInteger(width) || width < 1 || width > 64) {
		throw errors.new(
			`bytes width must be an integer in 1..=64 (got ${width}) — docs/architecture/70-api.md pins the range at declaration`
		)
	}
	return Object.freeze({
		kind: "bytes",
		width,
		domain: undefined,
		as<const Domain extends string>(domain: Domain): BytesField<Width, Domain> {
			return Object.freeze({ kind: "bytes", width, domain })
		}
	})
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
function interval<Element extends U64Ctor | I64Ctor>(element: Element): IntervalCtor<Element["kind"], undefined>
function interval<Element extends U64Ctor | I64Ctor, const Width extends bigint>(
	element: Element,
	width: Width
): IntervalCtor<Element["kind"], Width>
function interval(element: U64Ctor | I64Ctor, width?: bigint): IntervalCtor<"u64" | "i64", bigint | undefined> {
	const elementKind = element.kind
	if (elementKind !== "u64" && elementKind !== "i64") {
		throw errors.new(`interval element must be the u64 or i64 field constructor (got ${elementKind})`)
	}
	if (width !== undefined && width < 1n) {
		throw errors.new(
			`interval width must be >= 1 (got ${width}) — docs/architecture/70-api.md pins w >= 1 at declaration`
		)
	}
	return Object.freeze({
		kind: "interval",
		element: elementKind,
		width,
		domain: undefined,
		as<const Domain extends string>(domain: Domain): IntervalField<"u64" | "i64", bigint | undefined, Domain> {
			return Object.freeze({ kind: "interval", element: elementKind, width, domain })
		}
	})
}

/**
 * Lowers one host literal at its field position to the wire
 * {@link LiteralSpec} — the selection-literal machine ground axioms and
 * `where()` bindings both ride (one machine, same errors — the macro's own
 * rule). A value on a closed-reference field resolves to its handle NAME
 * (the id is verified against the roster: an out-of-roster id is a
 * construction error); everything else lowers to a plain value tagged by
 * the field's structural kind.
 */
function literalOf(field: AnyField, value: unknown): LiteralSpec {
	if ("closed" in field) {
		return handleLiteral(field.closed, value)
	}
	switch (field.kind) {
		case "bool": {
			if (typeof value !== "boolean") {
				throw literalShapeError("boolean", value)
			}
			return { kind: "value", value: { kind: "bool", value } }
		}
		case "u64": {
			if (typeof value !== "bigint") {
				throw literalShapeError("bigint", value)
			}
			return { kind: "value", value: { kind: "u64", value } }
		}
		case "i64": {
			if (typeof value !== "bigint") {
				throw literalShapeError("bigint", value)
			}
			return { kind: "value", value: { kind: "i64", value } }
		}
		case "str": {
			if (typeof value !== "string") {
				throw literalShapeError("string", value)
			}
			return { kind: "value", value: { kind: "string", value } }
		}
		case "bytes": {
			if (!(value instanceof Uint8Array)) {
				throw literalShapeError("Uint8Array", value)
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
	BytesCtor,
	BytesField,
	ClosedIdField,
	ClosedRoster,
	FreshU64Field,
	I64Ctor,
	I64Field,
	Infer,
	IntervalCtor,
	IntervalField,
	IntervalValue,
	StrField,
	U64Ctor,
	U64Field
}
export { assertDeclarationOrderKey, bool, bytes, i64, interval, literalOf, span, str, u64 }
