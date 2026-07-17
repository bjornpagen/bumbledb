/**
 * Field type constructors — the value half of the `schema!` field grammar
 * (`docs/architecture/70-api.md`): `bool`, `u64`, `i64`, `str`, `bytes(n)`,
 * `interval(u64|i64[, width])`, each a plain frozen value carrying its
 * structural type at runtime and its host value type in a phantom generic.
 * Newtypes are DECLARATION-FIRST, one spelling only (owner ruling
 * 2026-07-16): `const AccountId = u64.newtype("AccountId")` declares the
 * brand ONCE as a value that IS the field, paired with
 * `type AccountId = Infer<typeof AccountId>` for signatures; every field
 * position references the declared value (`holder: HolderId`). The macro's
 * refusals are reproduced representationally: `.newtype` exists only where
 * Rust's `as` is legal (u64, i64, bytes, intervals — never bool/str),
 * `.fresh` exists only on declared u64 newtypes (the macro demands `as
 * NewType` on fresh fields), and no field-level constraint vocabulary of
 * any kind exists — `unique`/`fk` are unwritable, not rejected.
 */

import * as errors from "@superbuilders/errors"
import type { Brand, Interval, IntervalValue } from "#brand.ts"
import { phantom } from "#brand.ts"
import type { LiteralSpec, ValueTypeSpec } from "#spec.ts"

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
 * Resolves one closed-handle literal: the branded id (a bigint at runtime)
 * back to its handle NAME through the roster — an out-of-roster id is a
 * construction error, the belt the type level cannot provide against
 * forged brands.
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
 * A closed relation's roster as seen from a referencing field: the handle
 * namespace `where()` selections and ground axioms resolve bare handles
 * through (the macro's own rule: a handle is legal exactly on a field whose
 * newtype is a closed relation's handle newtype).
 */
interface ClosedRoster {
	readonly name: string
	readonly handles: readonly string[]
}

/**
 * One field's runtime description: structural type, host newtype name, the
 * `fresh` mint mark (`minted` — the property name `fresh` is taken by the
 * builder surface), and the closed-relation reference when the field is a
 * closed relation's id type.
 */
interface FieldData<Minted extends boolean = boolean> {
	readonly type: ValueTypeSpec
	readonly newtype: string | undefined
	readonly minted: Minted
	readonly closed: ClosedRoster | undefined
}

/**
 * The base shape every field value shares: runtime data plus the phantom
 * host value type `V` (never present at runtime).
 */
interface Field<V> {
	readonly data: FieldData
	readonly [phantom]?: V
}

/** Extracts a field value's host value type from its phantom. */
type FieldValue<F> = F extends Field<infer V> ? V : never

/** Any field value, whatever its host value type. */
type AnyField = Field<unknown>

/** The `bool` field: host type `boolean`. No `.newtype`, no `.fresh` (macro parity). */
interface BoolField extends Field<boolean> {
	readonly data: FieldData<false>
}

/** The `str` field: host type `string`. No `.newtype`, no `.fresh` (macro parity). */
interface StrField extends Field<string> {
	readonly data: FieldData<false>
}

/**
 * A `fresh`-marked u64 newtype field — `id: AccountId.fresh` (Rust: `id:
 * u64 as AccountId, fresh`). Terminal: the mark implies the key
 * `R(field) -> R`, which the ENGINE materializes
 * (`SchemaDescriptor::materialized_statements`); `schema()` rejects an
 * explicit duplicate of it (macro parity).
 */
interface FreshU64Newtype<Name extends string> extends Field<Brand<bigint, Name>> {
	readonly data: FieldData<true>
}

/**
 * A declared u64 newtype — `const AccountId = u64.newtype("AccountId")`
 * (Rust: `u64 as AccountId`). The value IS the field: relation blocks
 * reference it (`holder: HolderId`). `.fresh` marks it as minted; the
 * property exists ONLY here (the macro demands `as NewType` on fresh
 * fields, so bare-u64 fresh is unwritable).
 */
interface U64Newtype<Name extends string> extends Field<Brand<bigint, Name>> {
	readonly data: FieldData<false>
	readonly fresh: FreshU64Newtype<Name>
}

/** The bare `u64` field; `.newtype(name)` declares a branded u64 newtype. */
interface U64Field extends Field<bigint> {
	readonly data: FieldData<false>
	newtype<const Name extends string>(name: Name): U64Newtype<Name>
}

/** A declared i64 newtype. Terminal: `fresh` is legal on u64 only. */
interface I64Newtype<Name extends string> extends Field<Brand<bigint, Name>> {
	readonly data: FieldData<false>
}

/** The bare `i64` field; `.newtype(name)` declares a branded i64 newtype. */
interface I64Field extends Field<bigint> {
	readonly data: FieldData<false>
	newtype<const Name extends string>(name: Name): I64Newtype<Name>
}

/** A declared `bytes<N>` newtype (no order derived — no comparators exist). */
interface BytesNewtype<Name extends string> extends Field<Brand<Uint8Array, Name>> {
	readonly data: FieldData<false>
}

/** A `bytes<N>` field; `.newtype(name)` declares a branded bytes newtype. */
interface BytesField extends Field<Uint8Array> {
	readonly data: FieldData<false>
	newtype<const Name extends string>(name: Name): BytesNewtype<Name>
}

/** A declared interval newtype — Rust: `interval<i64> as ActiveDuring`. */
interface IntervalNewtype<Name extends string> extends Field<Interval<Name>> {
	readonly data: FieldData<false>
}

/** An interval field; `.newtype(name)` brands the whole `{ start, end }` object. */
interface IntervalField extends Field<IntervalValue> {
	readonly data: FieldData<false>
	newtype<const Name extends string>(name: Name): IntervalNewtype<Name>
}

/**
 * The branded value type of a declared newtype — the type half of the
 * declaration-first pairing (owner ruling 2026-07-16): `const AccountId =
 * u64.newtype("AccountId")` + `type AccountId = Infer<typeof AccountId>`.
 * Reads the phantom, so it works on any field value (a closed relation's
 * `id` field infers its handle brand the same way).
 */
type Infer<F extends AnyField> = F extends Field<infer V> ? V : never

/**
 * A closed relation's id field constructor (`Kind.id`) — a u64 field
 * pre-branded with the closed relation's handle newtype, for use in other
 * relations' field blocks (`kind: Kind.id`). Terminal: its newtype IS the
 * closed relation, so `.newtype` and `.fresh` do not exist.
 */
interface ClosedIdField<Name extends string> extends Field<Brand<bigint, Name>> {
	readonly data: FieldData<false>
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

/**
 * Builds one frozen {@link FieldData}. Internal seam shared by the field
 * constructors here and by `closed()` (which mints {@link ClosedIdField}
 * values against its own roster).
 */
function fieldData<Minted extends boolean>(
	type: ValueTypeSpec,
	newtype: string | undefined,
	minted: Minted,
	closed: ClosedRoster | undefined
): FieldData<Minted> {
	return Object.freeze({ type: Object.freeze(type), newtype, minted, closed })
}

/** The one `u64` field constructor value. */
const u64: U64Field = Object.freeze({
	data: fieldData({ kind: "u64" }, undefined, false, undefined),
	newtype<const Name extends string>(name: Name): U64Newtype<Name> {
		const fresh: FreshU64Newtype<Name> = Object.freeze({
			data: fieldData({ kind: "u64" }, name, true, undefined)
		})
		return Object.freeze({
			data: fieldData({ kind: "u64" }, name, false, undefined),
			fresh
		})
	}
})

/** The one `i64` field constructor value. */
const i64: I64Field = Object.freeze({
	data: fieldData({ kind: "i64" }, undefined, false, undefined),
	newtype<const Name extends string>(name: Name): I64Newtype<Name> {
		return Object.freeze({ data: fieldData({ kind: "i64" }, name, false, undefined) })
	}
})

/** The one `bool` field constructor value. */
const bool: BoolField = Object.freeze({
	data: fieldData({ kind: "bool" }, undefined, false, undefined)
})

/** The one `str` field constructor value. */
const str: StrField = Object.freeze({
	data: fieldData({ kind: "string" }, undefined, false, undefined)
})

/**
 * The `bytes<N>` field constructor. The width is mandatory and part of the
 * type; `len` is validated to 1..=64 here because the grammar pins that
 * range at declaration (`docs/architecture/70-api.md` § the `schema!`
 * grammar: N ∈ 1..=64 — bare `bytes` does not parse), the macro-expansion
 * boundary's analog being construction.
 */
function bytes(len: number): BytesField {
	if (!Number.isInteger(len) || len < 1 || len > 64) {
		throw errors.new(
			`bytes width must be an integer in 1..=64 (got ${len}) — docs/architecture/70-api.md pins the range at declaration`
		)
	}
	return Object.freeze({
		data: fieldData({ kind: "fixedBytes", len }, undefined, false, undefined),
		newtype<const Name extends string>(name: Name): BytesNewtype<Name> {
			return Object.freeze({ data: fieldData({ kind: "fixedBytes", len }, name, false, undefined) })
		}
	})
}

/**
 * The interval field constructor — `interval(u64)` / `interval(i64)` for
 * the general type (rays representable), `interval(u64, w)` for the
 * fixed-width family whose width IS the type. The element is spelled with
 * the u64/i64 field constructor values themselves, never a string. `width
 * >= 1` is validated here because the grammar pins it at declaration
 * (`docs/architecture/70-api.md`: w ≥ 1; `interval<u64, 0>` is an
 * expansion error naming the field).
 */
function interval(element: U64Field | I64Field, width?: bigint): IntervalField {
	const elementKind = element.data.type.kind
	if (elementKind !== "u64" && elementKind !== "i64") {
		throw errors.new(`interval element must be the u64 or i64 field constructor (got ${elementKind})`)
	}
	if (width !== undefined && width < 1n) {
		throw errors.new(
			`interval width must be >= 1 (got ${width}) — docs/architecture/70-api.md pins w >= 1 at declaration`
		)
	}
	const type: ValueTypeSpec = { kind: "interval", element: elementKind, width }
	return Object.freeze({
		data: fieldData(type, undefined, false, undefined),
		newtype<const Name extends string>(name: Name): IntervalNewtype<Name> {
			return Object.freeze({ data: fieldData(type, name, false, undefined) })
		}
	})
}

/**
 * Lowers one host literal at its field position to the wire
 * {@link LiteralSpec} — the selection-literal machine ground axioms and
 * `where()` bindings both ride (one machine, same errors — the macro's own
 * rule). A value on a closed-reference field resolves to its handle NAME
 * (the id is re-verified against the roster: an out-of-roster id is a
 * construction error); everything else lowers to a plain value tagged by
 * the field's structural type.
 */
function literalOf(field: FieldData, value: unknown): LiteralSpec {
	if (field.closed !== undefined) {
		return handleLiteral(field.closed, value)
	}
	switch (field.type.kind) {
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
		case "string": {
			if (typeof value !== "string") {
				throw literalShapeError("string", value)
			}
			return { kind: "value", value: { kind: "string", value } }
		}
		case "fixedBytes": {
			if (!(value instanceof Uint8Array)) {
				throw literalShapeError("Uint8Array", value)
			}
			return { kind: "value", value: { kind: "fixedBytes", value } }
		}
		case "interval":
			return intervalLiteral(field.type.element, value)
	}
}

export type {
	AnyField,
	BoolField,
	BytesField,
	BytesNewtype,
	ClosedIdField,
	ClosedRoster,
	Field,
	FieldData,
	FieldValue,
	FreshU64Newtype,
	I64Field,
	I64Newtype,
	Infer,
	IntervalField,
	IntervalNewtype,
	StrField,
	U64Field,
	U64Newtype
}
export { assertDeclarationOrderKey, bool, bytes, fieldData, i64, interval, literalOf, str, u64 }
