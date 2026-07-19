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

/**
 * Lowers one field descriptor's structural type to the wire
 * {@link ValueTypeSpec}: the S1 kind tags map 1:1 onto the `ValueType`
 * vocabulary (`str` spells `string`, `bytes` spells `fixedBytes` with its
 * width label as `len`; intervals carry element and width labels through).
 */
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

/**
 * Lowers one field descriptor to its {@link FieldSpec}: the structural
 * type, the structural fresh mark (`fresh` is the literal `true` exactly
 * on a fresh-marked u64 — on an unmarked one the property holds the marked
 * descriptor, never `true`), and the wire's `newtype` — the COMPUTED class
 * name `schema()` derived from the statement list (the laws type the
 * columns), `undefined` on a bare field. The engine reads newtypes for
 * handle resolution and the coherence check only and DROPS them at
 * descriptor lowering — class names are never fingerprinted.
 */
function lowerField(name: string, field: AnyField, newtype: string | undefined): FieldSpec {
	return {
		name,
		valueType: valueTypeOf(field),
		newtype,
		fresh: "fresh" in field && field.fresh === true
	}
}

/** Lowers one face to a `SideSpec`: names only, σ as (field, set) pairs. */
function lowerFace(face: FaceData): SideSpec {
	return {
		relation: face.owner.name,
		projection: [...face.projection],
		selection: face.selection.map(function lowerBinding(binding): readonly [string, LiteralSetSpec] {
			return [binding.field, binding.set]
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
 * declaration order, each carrying its law-computed class name as the
 * `newtype` (`classes` — the schema's class record for this relation;
 * bare fields carry `undefined`), `extension: undefined` (the option is
 * the kind).
 */
function lowerRelation(relation: AnyRelation, classes: RelationClasses): RelationSpec {
	const fields: FieldSpec[] = relation.data.fields.map(function lowerDeclared(declared) {
		return lowerField(declared.name, declared.field, classes[declared.name])
	})
	return { name: relation.name, newtype: undefined, fields, extension: undefined }
}

/**
 * Lowers one closed relation to its `RelationSpec` fragment: declared
 * intrinsic columns only (the engine materializes the synthetic `id`),
 * the handle newtype — the COMPUTED class name of the id's generator
 * class (`"Kind.id"`, always present: a closed id is a generator), which
 * every referencing field shares by law (how the engine resolves a handle
 * literal back to its roster) — and the ground axioms in declaration
 * order (row id = index); the literals were already lowered at `closed()`
 * construction.
 */
function lowerClosed(member: AnyClosed, classes: RelationClasses): RelationSpec {
	const fields: FieldSpec[] = member.data.columns.map(function lowerColumn(column) {
		return lowerField(column.name, column.field, classes[column.name])
	})
	const extension = member.data.rows.map(function lowerRow(row) {
		return { handle: row.handle, values: row.values }
	})
	return { name: member.name, newtype: classes.id, fields, extension }
}

/** The frozen empty class record a relation outside the schema's map lowers under (nothing classed). */
const noClasses: RelationClasses = Object.freeze({})

/**
 * Lowers a whole theory to the `SchemaSpec` the bridge takes: relations in
 * record declaration order, DECLARED statements only in written order (the
 * engine materializes the fresh-implied and closed auto-keys itself), and
 * every field's `newtype` slot fed from the schema's law-computed class
 * map — the ONE domain authority (fingerprint-neutral: the engine drops
 * newtypes at descriptor lowering).
 */
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

export { lower, lowerClosed, lowerRelation }
