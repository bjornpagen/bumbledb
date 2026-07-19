/**
 * `relation()` — the ordinary-relation half of the theory's signature. A
 * relation value is a frozen plain object carrying its name, its ordered
 * field descriptors (declaration order = ordinal ids, the macro's law),
 * and — since selections are the relation's own vocabulary — `where()`,
 * which resolves a selection into lowered bindings eagerly (handles
 * verified against their roster at construction). Fields are addressed by
 * NAME everywhere — statements (`on(R, "holder")`), selections, and match
 * records all spell the field's own name, checked by type
 * (`FaceFields`/`MatchShape`). `Fact<>`/`InsertFact<>` are the inferred
 * row object types at BARE structural value types (no brands): fresh
 * fields are optional on insert input (omit-to-mint) and present on read
 * (resupply-to-preserve-identity), typed exactly.
 */

import * as errors from "@superbuilders/errors"
import { type AnyField, assertDeclarationOrderKey, type Infer, literalOf } from "#fields.ts"
import { type LiteralSetSpec, type LiteralSpec, renderLiteral } from "#spec.ts"

/** Flattens an intersection into one displayed object type (hover legibility). */
type Flatten<T> = { [K in keyof T]: T[K] }

/**
 * Resolves one selection entry to its lowered literal set: a plain ARRAY
 * (detected by `Array.isArray` — no field's value type is an array;
 * `Uint8Array` is not one) becomes a disjunctive set, anything else the
 * bare literal. The degenerate sets are construction errors, each
 * self-locating (`context` names the relation and field) — the empty set
 * selects nothing, the one-element set is the bare literal respelled, and
 * a DUPLICATE literal (judged on the canonical rendering — the engine's
 * own duplicate test, reached here first so its index-speak twin at
 * `Db.create` stays unreachable from this surface) is the same respelling
 * in disguise (the canonical-utterance law; the old set combinator's
 * signature made the length degenerates unwritable, and the refusals here
 * are that law's runtime seat). The lowered set — `{ kind: "many",
 * literals }` — is byte-identical to what the combinator produced, so no
 * fingerprint moves.
 */
function resolveEntry(context: string, field: AnyField, entry: unknown): LiteralSetSpec {
	if (Array.isArray(entry)) {
		if (entry.length < 2) {
			throw errors.new(
				entry.length === 0
					? `${context}: an empty literal set selects nothing — write the selection you mean`
					: `${context}: a one-element literal set is the bare literal respelled — write the literal (the canonical-utterance law: one meaning, one spelling)`
			)
		}
		const seen = new Set<string>()
		const literals: LiteralSpec[] = entry.map(function lowerSetLiteral(literal: unknown) {
			const lowered = Object.freeze(literalOf(field, literal))
			const rendered = renderLiteral(lowered)
			if (seen.has(rendered)) {
				throw errors.new(
					`${context}: the literal set spells ${rendered} twice — write it once (the canonical-utterance law: one meaning, one spelling)`
				)
			}
			seen.add(rendered)
			return lowered
		})
		return Object.freeze({ kind: "many", literals: Object.freeze(literals) })
	}
	return Object.freeze({ kind: "one", literal: Object.freeze(literalOf(field, entry)) })
}

/**
 * Resolves a whole `where()` selection against the declared fields, in the
 * selection's written order (macro parity: σ is spelled, not sorted). An
 * empty selection is the bare relation respelled and rejected (the
 * canonical-utterance law). THE one selection resolver — `closed()`'s
 * `where()` resolves its payload columns through this same machine (a
 * `ClosedColumn` is structurally a {@link RelationField}), so both surfaces
 * share one vocabulary and one error voice.
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
		bindings.push(
			Object.freeze({ field: fieldName, set: resolveEntry(`relation ${name}.${fieldName}`, declared.field, entry) })
		)
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
 * field's value type (a closed reference's literal IS its handle name —
 * `"Savings"`, verified against the roster at construction), a plain
 * ARRAY of such literals read disjunctively — `kind: ["Checking",
 * "Savings"]` — or a `span(start, end)` interval literal. Membership is
 * an array, never an operator (the drizzle law); equality-only by
 * construction: no operator parameter exists anywhere.
 */
type SelectionInput<Fields extends FieldsShape> = {
	readonly [K in keyof Fields]?: Infer<Fields[K]> | readonly Infer<Fields[K]>[]
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
	const value: Relation<Name, Fields> = Object.freeze({ name, data, where })
	holder.value = value
	return value
}

export type {
	AnyRelation,
	AnySelected,
	Fact,
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
export { relation, resolveSelection }
