/**
 * `relation()` — the ordinary-relation half of the theory's signature. A
 * relation value is a frozen plain object carrying its name, its ordered
 * field metadata (declaration order = ordinal ids, the macro's law), typed
 * field references (`R.fields.holder`), and — since selections are the
 * relation's own vocabulary — `where()`, which resolves a selection into
 * lowered bindings eagerly (handles re-verified against their roster at
 * construction). `Fact<>`/`InsertFact<>` are the inferred row object
 * types: fresh fields are optional on insert input (omit-to-mint) and
 * present on read (resupply-to-preserve-identity), typed exactly.
 */

import * as errors from "@superbuilders/errors"
import { phantom } from "#brand.ts"
import type { OneOf } from "#face.ts"
import { type AnyField, assertDeclarationOrderKey, type FieldData, type FieldValue, literalOf } from "#fields.ts"
import type { LiteralSetSpec, LiteralSpec } from "#spec.ts"

/** Flattens an intersection into one displayed object type (hover legibility). */
type Flatten<T> = { [K in keyof T]: T[K] }

/**
 * The one nominal step of `relation()`: the reference record is built by
 * iterating the declared fields, and this guard verifies the checkable
 * facts — one reference per declared field, each carrying its own name —
 * before the record is admitted as the typed {@link FieldRefs}. The
 * phantom halves (brands) are carried by construction; this is the
 * module's single trusted seam, the macro-emission analog.
 */
function refsComplete<RName extends string, Fields extends FieldsShape>(
	refs: Record<string, unknown>,
	fields: Fields
): refs is FieldRefs<RName, Fields> {
	return Object.keys(fields).every(function hasRef(fieldName) {
		const ref = refs[fieldName]
		return typeof ref === "object" && ref !== null && "field" in ref && ref.field === fieldName
	})
}

/**
 * Resolves one selection entry to its lowered literal set: an `oneOf`
 * value (detected by its `literals` tuple — no field value is ever an
 * object carrying `literals`) becomes a disjunctive set (≥ 2 by the
 * `oneOf` signature — the one-element set is unwritable); anything else is
 * the bare literal.
 */
function resolveEntry(field: FieldData, entry: unknown): LiteralSetSpec {
	if (typeof entry === "object" && entry !== null && "literals" in entry && Array.isArray(entry.literals)) {
		const literals: LiteralSpec[] = entry.literals.map(function lowerSetLiteral(literal: unknown) {
			return Object.freeze(literalOf(field, literal))
		})
		return Object.freeze({ kind: "many", literals: Object.freeze(literals) })
	}
	return Object.freeze({ kind: "one", literal: Object.freeze(literalOf(field, entry)) })
}

/**
 * Resolves a whole `where()` selection against the declared fields, in the
 * selection's written order (macro parity: σ is spelled, not sorted). An
 * empty selection is the bare relation respelled and rejected (the
 * canonical-utterance law).
 */
function resolveSelection(
	name: string,
	ordered: readonly RelationField[],
	entries: ReadonlyArray<readonly [string, unknown]>
): readonly SelectionBinding[] {
	const bindings: SelectionBinding[] = []
	for (const [fieldName, entry] of entries) {
		if (entry === undefined) {
			continue
		}
		const declared = ordered.find(function byName(candidate) {
			return candidate.name === fieldName
		})
		if (declared === undefined) {
			throw errors.new(`relation ${name} has no field ${fieldName}`)
		}
		bindings.push(Object.freeze({ field: fieldName, set: resolveEntry(declared.field, entry) }))
	}
	if (bindings.length === 0) {
		throw errors.new(
			`relation ${name}: an empty selection is the bare relation respelled — pass the relation itself (the canonical-utterance law: one meaning, one spelling)`
		)
	}
	return Object.freeze(bindings)
}

/** The field block of a relation: field name to field constructor value. */
type FieldsShape = Record<string, AnyField>

/**
 * A typed field reference (`Account.fields.holder`) — the value statements,
 * selections, and queries address a field through; its hover shows the
 * field's brand in the phantom position.
 */
interface FieldRef<Rel extends string, Name extends string, V> {
	readonly relation: Rel
	readonly field: Name
	readonly [phantom]?: V
}

/** The typed field-reference record of a relation. */
type FieldRefs<RName extends string, Fields extends FieldsShape> = {
	readonly [K in keyof Fields & string]: FieldRef<RName, K, FieldValue<Fields[K]>>
}

/** One declared field: name plus its runtime description, in declaration order. */
interface RelationField {
	readonly name: string
	readonly field: FieldData
}

/** A relation's runtime description. */
interface RelationData {
	readonly name: string
	readonly fields: readonly RelationField[]
}

/**
 * One resolved σ binding: the field name and its lowered literal set —
 * handles already resolved to names, values already tagged by structural
 * type.
 */
interface SelectionBinding {
	readonly field: string
	readonly set: LiteralSetSpec
}

/**
 * The `where()` argument: per field, a branded literal of that field's
 * type (a closed handle constant IS such a literal — it carries the closed
 * relation's brand, so it is legal exactly where the field is that closed
 * relation's id type), an `oneOf(a, b, ...)` literal set, or a `span(start,
 * end)` interval literal. Equality-only by construction: no operator
 * parameter exists anywhere.
 */
type SelectionInput<Fields extends FieldsShape> = {
	readonly [K in keyof Fields]?: FieldValue<Fields[K]> | OneOf<FieldValue<Fields[K]>>
}

/** A relation with a selection applied — what `on()` consumes as a σ-carrying source. */
interface Selected<Name extends string, Fields extends FieldsShape> {
	readonly relation: Relation<Name, Fields>
	readonly selection: readonly SelectionBinding[]
}

/** A relation value. */
interface Relation<Name extends string, Fields extends FieldsShape> {
	readonly name: Name
	readonly data: RelationData
	readonly fields: FieldRefs<Name, Fields>
	where(selection: SelectionInput<Fields>): Selected<Name, Fields>
}

/** Any relation value, whatever its name and field block. */
type AnyRelation = Relation<string, FieldsShape>

/** Any selected relation value. */
interface AnySelected {
	readonly relation: AnyRelation
	readonly selection: readonly SelectionBinding[]
}

/** Extracts a relation's field block type. */
type RelationFields<R extends AnyRelation> = R extends Relation<string, infer F> ? F : never

/**
 * The inferred row object type of a relation as READ: every field present,
 * branded. Closed relations have no `Fact` — they are unwritable, and the
 * type constraint refuses them because a closed value lacks the relation
 * shape.
 */
type Fact<R extends AnyRelation> = {
	[K in keyof RelationFields<R>]: FieldValue<RelationFields<R>[K]>
}

/** The field names of `R` that carry the fresh mint mark. */
type FreshKeys<R extends AnyRelation> = {
	[K in keyof RelationFields<R>]: RelationFields<R>[K] extends {
		readonly data: { readonly minted: true }
	}
		? K
		: never
}[keyof RelationFields<R>]

/**
 * The inferred row object type of a relation as INSERTED: fresh fields
 * optional — omitted, the engine mints; supplied, identity is preserved
 * (the ETL resupply idiom).
 */
type InsertFact<R extends AnyRelation> = Flatten<Omit<Fact<R>, FreshKeys<R>> & Partial<Pick<Fact<R>, FreshKeys<R>>>>

/**
 * Declares one relation: `relation("Account", { id: AccountId.fresh,
 * holder: HolderId, ... })` — every field references a declared newtype
 * (`const AccountId = u64.newtype("AccountId")`) or a bare constructor.
 * Field declaration order is ordinal-id order (macro parity); the returned
 * value is frozen and side-effect free.
 */
function relation<const Name extends string, Fields extends FieldsShape>(
	name: Name,
	fields: Fields
): Relation<Name, Fields> {
	const ordered: RelationField[] = []
	for (const [fieldName, field] of Object.entries(fields)) {
		assertDeclarationOrderKey(`relation ${name} field`, fieldName)
		ordered.push(Object.freeze({ name: fieldName, field: field.data }))
	}
	const data: RelationData = Object.freeze({ name, fields: Object.freeze(ordered) })
	const refs: Record<string, unknown> = {}
	for (const declared of ordered) {
		refs[declared.name] = Object.freeze({ relation: name, field: declared.name })
	}
	Object.freeze(refs)
	if (!refsComplete<Name, Fields>(refs, fields)) {
		throw errors.new(`relation ${name}: field-reference construction incomplete`)
	}
	const holder: { value: Relation<Name, Fields> | undefined } = { value: undefined }
	function where(selection: SelectionInput<Fields>): Selected<Name, Fields> {
		const owner = holder.value
		if (owner === undefined) {
			throw errors.new(`relation ${name}: self-reference read before construction completed`)
		}
		return Object.freeze({
			relation: owner,
			selection: resolveSelection(name, ordered, Object.entries(selection))
		})
	}
	const value: Relation<Name, Fields> = Object.freeze({ name, data, fields: refs, where })
	holder.value = value
	return value
}

export type {
	AnyRelation,
	AnySelected,
	Fact,
	FieldRef,
	FieldRefs,
	FieldsShape,
	FreshKeys,
	InsertFact,
	Relation,
	RelationData,
	RelationField,
	RelationFields,
	Selected,
	SelectionBinding,
	SelectionInput
}
export { relation }
