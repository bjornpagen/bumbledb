import { AuthoringError } from "#errors.ts"
import {
	type AnyField,
	assertDeclarationOrderKey,
	assertDeclarationRecord,
	fieldDescriptor,
	type Infer
} from "#fields.ts"
import { descriptorCache } from "#immutable.ts"
import { recordValue } from "#values.ts"

/** Ordinary relations are structural declarations. Field order defines native ordinals. */
interface Relation<Name extends string = string, Fields extends FieldsShape = FieldsShape> {
	readonly kind: "relation"
	readonly name: Name
	readonly fields: Fields
}
type FieldsShape = Readonly<Record<string, AnyField>>
type AnyRelation = Relation
type RelationFields<R extends AnyRelation> = R["fields"]
type Fact<R extends AnyRelation> = { -readonly [K in keyof R["fields"]]: Infer<R["fields"][K]> }

interface RelationField {
	readonly name: string
	readonly field: AnyField
}

/** Validate and own every field, without freezing or retaining caller records. */
function ownFields<F extends FieldsShape>(context: string, input: F): F
function ownFields(context: string, input: unknown): FieldsShape
function ownFields(context: string, input: unknown): FieldsShape {
	if (typeof input !== "object" || input === null)
		throw new AuthoringError({ message: `${context}: expected a field record` })
	assertDeclarationRecord(context, input)
	return Object.freeze(
		Object.fromEntries(
			Object.entries(input).map(([name, field]) => {
				assertDeclarationOrderKey(context, name)
				return [name, fieldDescriptor(`${context}.${name}`, field)]
			})
		)
	)
}

function relationDescriptor<R extends AnyRelation>(input: R): R
function relationDescriptor(input: unknown): AnyRelation
function relationDescriptor(input: unknown): AnyRelation {
	return checkedRelation(input)
}

const checkedRelation = descriptorCache((input): AnyRelation => {
	const raw = recordValue("relation", input, ["kind", "name", "fields"])
	if (raw.kind !== "relation" || typeof raw.name !== "string")
		throw new AuthoringError({ message: "relation: expected a relation declaration" })
	assertDeclarationOrderKey("relation", raw.name)
	return Object.freeze({
		kind: "relation",
		name: raw.name,
		fields: ownFields(`relation ${raw.name} fields`, raw.fields)
	})
})

function relation<const Name extends string, const Fields extends FieldsShape>(
	name: Name,
	fields: Fields
): Relation<Name, Fields> {
	return relationDescriptor({ kind: "relation", name, fields })
}

/** Ordered field view derived from the one declaration record. */
function relationFields(relation: AnyRelation): readonly RelationField[] {
	return Object.entries(relation.fields).map(([name, field]) => ({ name, field }))
}

export type { AnyRelation, Fact, FieldsShape, Relation, RelationField, RelationFields }
export { ownFields, relation, relationDescriptor, relationFields }
