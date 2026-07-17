/**
 * Descriptor lowering: SDK values down to the PRD-01 `SchemaSpec` plain
 * data (`#spec.ts`), which the napi bridge marshals verbatim. Lowering is
 * TOTAL on well-typed inputs — no validation lives here beyond what the
 * types and the construction boundaries already guarantee — and it is the
 * only place statement internals are read for the wire. Ordering is
 * declaration order throughout, and every output object is built with one
 * fixed key order, so serialization is deterministic (byte-stable).
 */

import type { AnyClosed } from "#closed.ts"
import type { FaceData } from "#face.ts"
import type { AnyRelation } from "#relation.ts"
import type { AnySchema, SchemaRelation } from "#schema.ts"
import type { FieldSpec, RelationSpec, SchemaSpec, SideSpec, StatementSpec } from "#spec.ts"
import type { Statement } from "#statements.ts"

/**
 * The relation-kind discriminant: a closed relation's runtime description
 * carries its handle roster, an ordinary relation's never does.
 */
function isClosedMember(member: SchemaRelation): member is AnyClosed {
	return "handles" in member.data
}

/** Lowers one face to a `SideSpec`: names only, σ as (field, set) pairs. */
function lowerFace(face: FaceData): SideSpec {
	return {
		relation: face.owner.name,
		projection: [...face.projection],
		selection: face.selection.map(function lowerBinding(binding) {
			return [binding.field, binding.set] as const
		})
	}
}

/**
 * Lowers one statement. `mirrors` stays ONE spec statement
 * (`bidirectional: true`) — the engine performs the `==` lowering to two
 * adjacent containments, `source <= target` first, exactly as the macro
 * does.
 */
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
				bidirectional: data.bidirectional
			}
		case "window":
			return {
				kind: "cardinality",
				target: lowerFace(data.target),
				window: data.window,
				source: lowerFace(data.source)
			}
	}
}

/**
 * Lowers one ordinary relation to its `RelationSpec` fragment: fields in
 * declaration order, `extension: undefined` (the option is the kind).
 */
function lowerRelation(relation: AnyRelation): RelationSpec {
	const fields: FieldSpec[] = relation.data.fields.map(function lowerField(declared) {
		return {
			name: declared.name,
			valueType: declared.field.type,
			newtype: declared.field.newtype,
			fresh: declared.field.minted
		}
	})
	return { name: relation.name, newtype: undefined, fields, extension: undefined }
}

/**
 * Lowers one closed relation to its `RelationSpec` fragment: declared
 * intrinsic columns only (the engine materializes the synthetic `id`),
 * the relation's own name as its handle newtype, and the ground axioms in
 * declaration order (row id = index) — the literals were already lowered
 * at `closed()` construction.
 */
function lowerClosed(member: AnyClosed): RelationSpec {
	const fields: FieldSpec[] = member.data.columns.map(function lowerColumn(column) {
		return {
			name: column.name,
			valueType: column.field.type,
			newtype: column.field.newtype,
			fresh: false
		}
	})
	const extension = member.data.rows.map(function lowerRow(row) {
		return { handle: row.handle, values: row.values }
	})
	return { name: member.name, newtype: member.name, fields, extension }
}

/**
 * Lowers a whole theory to the `SchemaSpec` the bridge takes: relations in
 * record declaration order, DECLARED statements only in written order (the
 * engine materializes the fresh-implied and closed auto-keys itself).
 */
function lower(theory: AnySchema): SchemaSpec {
	const relations: RelationSpec[] = Object.values(theory.relations).map(function lowerMember(member) {
		if (isClosedMember(member)) {
			return lowerClosed(member)
		}
		return lowerRelation(member)
	})
	return { relations, statements: theory.statements.map(lowerStatement) }
}

export { lower, lowerClosed, lowerRelation }
