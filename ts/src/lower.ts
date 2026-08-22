/**
 * Descriptor lowering: SDK values down to the PRD-01 `SchemaSpec` plain
 * data (`#spec.ts`), which the napi bridge marshals verbatim. Lowering is
 * TOTAL on well-typed inputs — no validation lives here beyond what the
 * types and the construction boundaries already guarantee — and it is the
 * only place statement internals are read for the wire. In particular,
 * lowering never emits an engine-refused containment target: it accepts
 * only `schema` outputs, and `schema`'s target-key wall already
 * refused any non-key target projection (60-containment-parity —
 * totality INHERITED, not re-checked). Ordering is declaration order
 * throughout, and every output object is built with one fixed key order,
 * so serialization is deterministic (byte-stable).
 */

import * as errors from "@superbuilders/errors"
import type { AnyClosed } from "#closed.ts"
import { isClosedMember } from "#closed.ts"
import type { FaceData } from "#face.ts"
import type { AnyField } from "#fields.ts"
import type { RelationClasses } from "#law.ts"
import type { AnyRelation } from "#relation.ts"
import type { AnySchema } from "#schema.ts"
import type {
	FieldSpec,
	LiteralSetSpec,
	RelationSpec,
	SchemaSpec,
	SideSpec,
	StatementSpec,
	ValueTypeSpec
} from "#spec.ts"
import type { Statement } from "#statements.ts"

function valueTypeOf(field: AnyField): ValueTypeSpec {
	switch (field.kind) {
		case "bool":
			return { kind: "bool" }
		case "u64":
			return { kind: "u64" }
		case "i64":
			return { kind: "i64" }
		case "str":
			return { kind: "string" }
		case "bytes":
			return { kind: "fixedBytes", len: field.width }
		case "interval":
			return { kind: "interval", element: field.element, width: field.width }
	}
}

function lowerField(name: string, field: AnyField, newtype: string | undefined): FieldSpec {
	return {
		name,
		valueType: valueTypeOf(field),
		newtype,
		fresh: "fresh" in field && field.fresh === true
	}
}

function lowerFace(face: FaceData): SideSpec {
	return {
		relation: face.owner.name,
		projection: [...face.projection],
		selection: face.selection.map(function lowerBinding(binding): readonly [string, LiteralSetSpec] {
			return [binding.field, binding.set]
		})
	}
}

function lowerStatement(statement: Statement): StatementSpec {
	const data = statement.data
	switch (data.kind) {
		case "key":
			return { kind: "fd", relation: data.owner.name, projection: [...data.projection] }
		case "containment":
			return {
				kind: "containment",
				source: lowerFace(data.source),
				target: lowerFace(data.target),
				bidirectional: false
			}
		case "mirrors":
			return {
				kind: "containment",
				source: lowerFace(data.source),
				target: lowerFace(data.target),
				bidirectional: true
			}
		case "capacity":
			return {
				kind: "capacity",
				target: lowerFace(data.target),
				weight: data.weight,
				window: data.window,
				source: lowerFace(data.source)
			}
	}
}

function lowerRelation(relation: AnyRelation, classes: RelationClasses): RelationSpec {
	const fields: FieldSpec[] = relation.data.fields.map(function lowerDeclared(declared) {
		return lowerField(declared.name, declared.field, classes[declared.name])
	})
	return { name: relation.name, fields, closed: undefined }
}

function lowerClosed(member: AnyClosed, classes: RelationClasses): RelationSpec {
	const fields: FieldSpec[] = member.data.columns.map(function lowerColumn(column) {
		return lowerField(column.name, column.field, classes[column.name])
	})
	const rows = member.data.rows.map(function lowerRow(row) {
		return { handle: row.handle, values: row.values }
	})
	const newtype = classes.id
	if (newtype === undefined) {
		throw errors.new(`closed relation ${member.name}: the id's generator class is missing from the class map`)
	}
	return { name: member.name, fields, closed: { newtype, rows } }
}

const noClasses: RelationClasses = Object.freeze({})

function lower(theory: AnySchema): SchemaSpec {
	const relations: RelationSpec[] = Object.entries(theory.relations).map(function lowerMember([name, member]) {
		const classes = theory.classes[name] ?? noClasses
		if (isClosedMember(member)) {
			return lowerClosed(member, classes)
		}
		return lowerRelation(member, classes)
	})
	return { relations, statements: theory.statements.map(lowerStatement) }
}

export { lower }
