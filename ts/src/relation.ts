import { AuthoringError } from "#errors.ts"
/**
 * `relation` — the ordinary-relation half of the theory's signature. A
 * relation value is a frozen plain object carrying its name, its ordered
 * field descriptors (declaration order = ordinal ids, the macro's law),
 * and — since selections are the relation's own vocabulary — `where`,
 * which resolves a selection into lowered bindings eagerly (handles
 * verified against their roster at construction). Fields are addressed by
 * NAME everywhere — statements (`on(R, "holder")`), selections, and match
 * records all spell the field's own name, checked by type
 * (`FaceFields`/`MatchShape`). `Fact<>` is the inferred row object type
 * at BARE structural value types (no brands): every field is present,
 * including fresh cells. Mint with `tx.reserve` before insert.
 */

import { type AnyField, assertDeclarationOrderKey, assertDeclarationRecord, type Infer, literalOf } from "#fields.ts"
import { type LiteralSetSpec, type LiteralSpec, renderLiteral } from "#spec.ts"

function resolveEntry(context: string, field: AnyField, entry: unknown): LiteralSetSpec {
	if (Array.isArray(entry)) {
		if (entry.length < 2) {
			throw new AuthoringError({
				message:
					entry.length === 0
						? `${context}: an empty literal set selects nothing — write the selection you mean`
						: `${context}: a one-element literal set is the bare literal respelled — write the literal (the canonical-utterance law: one meaning, one spelling)`
			})
		}
		const seen = new Set<string>()
		const literals: LiteralSpec[] = entry.map(function lowerSetLiteral(literal: unknown) {
			const lowered = Object.freeze(literalOf(field, literal))
			const rendered = renderLiteral(lowered)
			if (seen.has(rendered)) {
				throw new AuthoringError({
					message: `${context}: the literal set spells ${rendered} twice — write it once (the canonical-utterance law: one meaning, one spelling)`
				})
			}
			seen.add(rendered)
			return lowered
		})
		return Object.freeze({ kind: "many", literals: Object.freeze(literals) })
	}
	return Object.freeze({ kind: "one", literal: Object.freeze(literalOf(field, entry)) })
}

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
			throw new AuthoringError({ message: `relation ${name} has no field ${fieldName}` })
		}
		bindings.push(
			Object.freeze({ field: fieldName, set: resolveEntry(`relation ${name}.${fieldName}`, declared.field, entry) })
		)
	}
	if (bindings.length === 0) {
		throw new AuthoringError({
			message: `relation ${name}: an empty selection is the bare relation respelled — pass the relation itself (the canonical-utterance law: one meaning, one spelling)`
		})
	}
	return Object.freeze(bindings)
}

type FieldsShape = Record<string, AnyField>

interface RelationField {
	readonly name: string
	readonly field: AnyField
}

interface RelationData {
	readonly name: string
	readonly fields: readonly RelationField[]
}

interface SelectionBinding {
	readonly field: string
	readonly set: LiteralSetSpec
}

type SelectionInput<Fields extends FieldsShape> = {
	readonly [K in keyof Fields]?: Infer<Fields[K]> | readonly Infer<Fields[K]>[]
}

interface Selected<Name extends string, Fields extends FieldsShape> {
	readonly relation: Relation<Name, Fields>
	readonly selection: readonly SelectionBinding[]
}

interface Relation<Name extends string, Fields extends FieldsShape> {
	readonly name: Name
	readonly data: RelationData
	where(selection: SelectionInput<Fields>): Selected<Name, Fields>
}

type AnyRelation = Relation<string, FieldsShape>

interface AnySelected {
	readonly relation: AnyRelation
	readonly selection: readonly SelectionBinding[]
}

type RelationFields<R extends AnyRelation> = R extends Relation<string, infer F extends FieldsShape> ? F : never

type Fact<R extends AnyRelation> = {
	[K in keyof RelationFields<R>]: Infer<RelationFields<R>[K]>
}

type FreshKeys<R extends AnyRelation> = {
	[K in keyof RelationFields<R>]: RelationFields<R>[K] extends { readonly fresh: true } ? K : never
}[keyof RelationFields<R>]

function relation<const Name extends string, Fields extends FieldsShape>(
	name: Name,
	fields: Fields
): Relation<Name, Fields> {
	assertDeclarationOrderKey("relation", name)
	assertDeclarationRecord(`relation ${name} fields`, fields)
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
			throw new AuthoringError({ message: `relation ${name}: self-reference read before construction completed` })
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
	Relation,
	RelationData,
	RelationField,
	RelationFields,
	Selected,
	SelectionBinding,
	SelectionInput
}
export { relation, resolveSelection }
