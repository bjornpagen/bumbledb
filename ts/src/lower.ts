/**
 * Lowers a schema to the addon's `SchemaSpecIn`, in declaration order with a fixed key order, so
 * the JSON is byte-stable. Lowering judges nothing: the engine compiles and judges the spec.
 */
import type { AnyClosed } from "./closed.ts"
import { isClosedMember } from "./closed.ts"
import { AuthoringError } from "./errors.ts"
import type { AnyFace } from "./face.ts"
import { type AnyField, literalOf } from "./fields.ts"
import type { RelationClasses } from "./law.ts"
import type {
	FieldSpecIn,
	RelationSpecIn,
	SchemaSpecIn,
	SideSpecIn,
	StatementSpecIn,
	ValueTypeIn
} from "./native/binding.d.ts"
import { literalIn, literalSetIn, weightIn, windowIn } from "./native/json.ts"
import { type AnyRelation, relationFields } from "./relation.ts"
import type { AnySchema } from "./schema.ts"
import type { Statement } from "./statements.ts"

const ELEMENT = { u64: "U64", i64: "I64", f64: "F64" } as const

function valueTypeOf(field: AnyField): ValueTypeIn {
	switch (field.kind) {
		case "bool":
			return { kind: "Bool" }
		case "u64":
			return { kind: "U64" }
		case "i64":
			return { kind: "I64" }
		case "f64":
			return { kind: "F64" }
		case "uuid":
			return { kind: "Uuid" }
		case "str":
			return { kind: "String" }
		case "bytes":
			return { kind: "FixedBytes", len: field.width }
		case "interval":
			if (field.width === undefined) return { kind: "Interval", element: ELEMENT[field.element] }
			if (field.element === "f64")
				throw new AuthoringError({ message: "a fixed-width interval needs an integer element" })
			return { kind: "FixedInterval", element: ELEMENT[field.element], width: field.width.toString() }
	}
}

function lowerField(name: string, field: AnyField, newtype: string | undefined): FieldSpecIn {
	return newtype === undefined
		? { name, valueType: valueTypeOf(field) }
		: { name, valueType: valueTypeOf(field), newtype }
}

function lowerFace(face: AnyFace): SideSpecIn {
	return {
		relation: face.owner.name,
		projection: [...face.projection],
		selection: face.selection.map((binding) => ({ field: binding.field, set: literalSetIn(binding.set) }))
	}
}

function lowerStatement(statement: Statement): StatementSpecIn {
	const data = statement
	switch (data.kind) {
		case "key":
			return { kind: "Fd", relation: data.owner.name, projection: [...data.projection] }
		case "containment":
			return {
				kind: "Containment",
				source: lowerFace(data.source),
				target: lowerFace(data.target),
				bidirectional: false
			}
		case "mirrors":
			return {
				kind: "Containment",
				source: lowerFace(data.source),
				target: lowerFace(data.target),
				bidirectional: true
			}
		case "capacity":
			return {
				kind: "Capacity",
				target: lowerFace(data.target),
				weight: weightIn(data.weight),
				window: windowIn(data.window),
				source: lowerFace(data.source)
			}
	}
}

function lowerRelation(relation: AnyRelation, classes: RelationClasses): RelationSpecIn {
	const fields = relationFields(relation).map((declared) =>
		lowerField(declared.name, declared.field, classes[declared.name])
	)
	return { name: relation.name, fields }
}

function lowerClosed(member: AnyClosed, classes: RelationClasses): RelationSpecIn {
	const fields = Object.entries(member.columns).map(([name, field]) => lowerField(name, field, classes[name]))
	const rows = member.handles.map((handle) => ({
		handle,
		values: Object.entries(member.columns).map(([name, field]) =>
			literalIn(literalOf(field, member.axioms[handle]?.[name]))
		)
	}))
	const newtype = classes.id
	if (newtype === undefined) {
		throw new AuthoringError({
			message: `closed relation ${member.name}: the id's generator class is missing from the class map`
		})
	}
	return { name: member.name, fields, closed: { newtype, rows } }
}

const noClasses: RelationClasses = Object.freeze({})

function lower(theory: AnySchema): SchemaSpecIn {
	const relations: RelationSpecIn[] = Object.entries(theory.relations).map(function lowerMember([name, member]) {
		const classes = theory.classes[name] ?? noClasses
		if (isClosedMember(member)) {
			return lowerClosed(member, classes)
		}
		return lowerRelation(member, classes)
	})
	return { relations, statements: theory.statements.map(lowerStatement) }
}

export { lower }
