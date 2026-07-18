/**
 * `relation()` — the ordinary-relation half of the theory's signature. A
 * relation value is a frozen plain object carrying its name, its ordered
 * field descriptors (declaration order = ordinal ids, the macro's law),
 * typed field references (`R.fields.holder`), and — since selections are
 * the relation's own vocabulary — `where()`, which resolves a selection
 * into lowered bindings eagerly (handles verified against their roster at
 * construction). `Fact<>`/`InsertFact<>` are the inferred row object
 * types at BARE structural value types (no brands): fresh fields are
 * optional on insert input (omit-to-mint) and present on read
 * (resupply-to-preserve-identity), typed exactly.
 */

import * as errors from "@superbuilders/errors"
import type { OneOf } from "#face.ts"
import { type AnyField, assertDeclarationOrderKey, type Infer, literalOf } from "#fields.ts"
import type { LiteralSetSpec, LiteralSpec } from "#spec.ts"

/** Flattens an intersection into one displayed object type (hover legibility). */
type Flatten<T> = { [K in keyof T]: T[K] }

/**
 * The one trusted seam of `relation()`: the reference record is built by
 * iterating the declared fields, and this guard verifies the checkable
 * facts — one reference per declared field, each carrying its own name —
 * before the record is admitted as the typed {@link FieldRefs} (the
 * macro-emission analog).
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
function resolveEntry(field: AnyField, entry: unknown): LiteralSetSpec {
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

/** The field block of a relation: field name to field descriptor. */
type FieldsShape = Record<string, AnyField>

/**
 * A typed field reference (`Account.fields.holder`) — the value statements,
 * selections, and queries address a field through. Purely positional
 * (relation name + field name); the field's descriptor is read off the
 * relation's schema type structurally.
 */
interface FieldRef<Rel extends string, Name extends string> {
	readonly relation: Rel
	readonly field: Name
}

/** The typed field-reference record of a relation. */
type FieldRefs<RName extends string, Fields extends FieldsShape> = {
	readonly [K in keyof Fields & string]: FieldRef<RName, K>
}

/** One declared field: name plus its descriptor, in declaration order. */
interface RelationField {
	readonly name: string
	readonly field: AnyField
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
 * The `where()` argument: per field, a bare structural literal of that
 * field's value type (a closed handle constant IS such a literal — a
 * bigint verified against the roster at construction), an `oneOf(a, b,
 * ...)` literal set, or a `span(start, end)` interval literal.
 * Equality-only by construction: no operator parameter exists anywhere.
 */
type SelectionInput<Fields extends FieldsShape> = {
	readonly [K in keyof Fields]?: Infer<Fields[K]> | OneOf<Infer<Fields[K]>>
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
type RelationFields<R extends AnyRelation> = R extends Relation<string, infer F extends FieldsShape> ? F : never

/**
 * The inferred row object type of a relation as READ: every field present,
 * at its BARE structural value type ({@link Infer}). Closed relations have
 * no `Fact` — they are unwritable, and the type constraint refuses them
 * because a closed value lacks the relation shape.
 */
type Fact<R extends AnyRelation> = {
	[K in keyof RelationFields<R>]: Infer<RelationFields<R>[K]>
}

/** The field names of `R` whose descriptor type carries the fresh mint mark. */
type FreshKeys<R extends AnyRelation> = {
	[K in keyof RelationFields<R>]: RelationFields<R>[K] extends { readonly fresh: true } ? K : never
}[keyof RelationFields<R>]

/**
 * The inferred row object type of a relation as INSERTED: fresh fields
 * optional — omitted, the engine mints; supplied, identity is preserved
 * (the ETL resupply idiom).
 */
type InsertFact<R extends AnyRelation> = Flatten<Omit<Fact<R>, FreshKeys<R>> & Partial<Pick<Fact<R>, FreshKeys<R>>>>

/**
 * Declares one relation: `relation("Account", { id: u64.fresh,
 * holder: u64, kind: Kind.id, ... })` — every field is a pure-structure
 * descriptor (the constructor values themselves; domains are never
 * declared: `schema()` computes them from the statements). Field
 * declaration order is ordinal-id order (macro parity), carried at BOTH
 * levels: the type level by the fields object, the value level by the
 * frozen `data.fields` list — the law the schema-level class naming leans
 * on. The returned value is frozen and side-effect free.
 */
function relation<const Name extends string, Fields extends FieldsShape>(
	name: Name,
	fields: Fields
): Relation<Name, Fields> {
	const ordered: RelationField[] = []
	for (const [fieldName, field] of Object.entries(fields)) {
		assertDeclarationOrderKey(`relation ${name} field`, fieldName)
		ordered.push(Object.freeze({ name: fieldName, field }))
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
