/**
 * `schema()` — assembles relations and statements into a theory value (the
 * `Theory` analog; what `Db.create`/`Db.open` take). Construction-time
 * validation is the macro-EXPANSION-boundary analog and nothing more:
 * membership, implied-key duplicates, duplicate statements, and a
 * belt-and-braces handle re-verification. Everything semantically deeper
 * (containment targets resolving declared keys, fresh-on-u64, …) is
 * DELIBERATELY left to the engine's `SchemaError` at `Db.create` — the
 * same judge, the same two-boundary split as Rust.
 */

import * as errors from "@superbuilders/errors"
import type { AnyClosed } from "#closed.ts"
import type { FaceData } from "#face.ts"
import { assertDeclarationOrderKey, type FieldData } from "#fields.ts"
import type { AnyRelation } from "#relation.ts"
import type { LiteralSetSpec, LiteralSpec } from "#spec.ts"
import { renderStatement, type Statement } from "#statements.ts"

/**
 * Validates the relation record and collects the implied keys: the
 * fresh-implied `R(field) -> R` per minted field and the closed auto-key
 * `R(id) -> R` per closed relation, each rendered canonically so an
 * explicit duplicate is caught by string identity with the renderer as the
 * single spelling authority.
 */
function collectImplied(name: string, relations: SchemaRelations): Set<string> {
	const implied = new Set<string>()
	for (const [recordKey, member] of Object.entries(relations)) {
		assertDeclarationOrderKey(`schema ${name} relation`, recordKey)
		if (member.name !== recordKey) {
			throw errors.new(
				`schema ${name}: record key ${recordKey} holds relation ${member.name} — the key must equal the relation's declared name`
			)
		}
		if ("handles" in member.data) {
			implied.add(`${member.name}(id) -> ${member.name}`)
			continue
		}
		for (const declared of member.data.fields) {
			if (declared.field.minted) {
				implied.add(`${member.name}(${declared.name}) -> ${member.name}`)
			}
		}
	}
	return implied
}

/** The relation values a statement addresses, for membership checking. */
function statementOwners(statement: Statement): readonly SchemaRelation[] {
	const data = statement.data
	if (data.kind === "key") {
		return [data.owner]
	}
	return [data.source.owner, data.target.owner]
}

/**
 * Requires every relation a statement addresses to be the IDENTICAL value
 * the schema record declares — same-name-different-value is a forgery, not
 * a membership.
 */
function verifyMembership(name: string, relations: SchemaRelations, statement: Statement, rendered: string): void {
	for (const owner of statementOwners(statement)) {
		const member = relations[owner.name]
		if (member === undefined) {
			throw errors.new(`schema ${name}: relation ${owner.name} is not declared in this schema — ${rendered}`)
		}
		if (member !== owner) {
			throw errors.new(
				`schema ${name}: statement references a different relation value named ${owner.name} than the one this schema declares — ${rendered}`
			)
		}
	}
}

/** Finds a face's field description by name, across both relation kinds. */
function faceField(face: FaceData, fieldName: string): FieldData | undefined {
	const data = face.owner.data
	if ("handles" in data) {
		const column = data.columns.find(function byName(candidate) {
			return candidate.name === fieldName
		})
		return column?.field
	}
	const declared = data.fields.find(function byName(candidate) {
		return candidate.name === fieldName
	})
	return declared?.field
}

/** Flattens one binding's literal set into its literals. */
function bindingLiterals(set: LiteralSetSpec): readonly LiteralSpec[] {
	if (set.kind === "one") {
		return [set.literal]
	}
	return set.literals
}

/**
 * Re-verifies one binding's handle literals against the field's roster —
 * belt-and-braces over what `where()` already resolved and the type level
 * already blocked, so a forged binding fails here rather than at the
 * engine boundary with a colder message.
 */
function verifyBindingHandles(
	name: string,
	face: FaceData,
	binding: { readonly field: string; readonly set: LiteralSetSpec },
	rendered: string
): void {
	const field = faceField(face, binding.field)
	for (const literal of bindingLiterals(binding.set)) {
		if (literal.kind !== "handle") {
			continue
		}
		if (field?.closed === undefined) {
			throw errors.new(
				`schema ${name}: ${face.owner.name}.${binding.field} is not a closed-relation reference — the handle literal ${literal.handle} is legal only on a field whose newtype is a closed relation's handle newtype — ${rendered}`
			)
		}
		if (!field.closed.handles.includes(literal.handle)) {
			throw errors.new(
				`schema ${name}: closed relation ${field.closed.name} has no handle ${literal.handle} — ${rendered}`
			)
		}
	}
}

/** Walks every face of a statement through the handle re-verification. */
function verifyHandles(name: string, statement: Statement, rendered: string): void {
	const data = statement.data
	if (data.kind === "key") {
		return
	}
	for (const face of [data.source, data.target]) {
		for (const binding of face.selection) {
			verifyBindingHandles(name, face, binding, rendered)
		}
	}
}

/**
 * Resolves the closed relation a `(relation, field)` pair references
 * through the DECLARED containments — the identical walk the engine's
 * canonical renderer performs (`schema/render.rs` `closed_target_of`): one
 * hop, source projecting exactly `[field]`, target projecting exactly the
 * closed relation's `[id]`, first declared match wins; a `mirrors`
 * contributes both of its materialized orientations. `undefined` = the
 * engine would render the field's selection literals as raw row ids.
 */
function closedTargetOf(statements: readonly Statement[], owner: string, field: string): string | undefined {
	for (const statement of statements) {
		const data = statement.data
		if (data.kind !== "containment") {
			continue
		}
		const pairs: Array<readonly [FaceData, FaceData]> = [[data.source, data.target]]
		if (data.bidirectional) {
			pairs.push([data.target, data.source])
		}
		for (const [source, target] of pairs) {
			if (
				source.owner.name === owner &&
				source.projection.length === 1 &&
				source.projection[0] === field &&
				target.projection.length === 1 &&
				target.projection[0] === "id" &&
				"handles" in target.owner.data
			) {
				return target.owner.name
			}
		}
	}
	return undefined
}

/**
 * Admits a handle spelling only when the schema also declares the
 * containment the ENGINE's canonical renderer resolves it through
 * (`docs/architecture/10-data-model.md` § closed relations: a closed
 * reference is the plain u64 column PLUS a declared containment). Without
 * it the two renderers drift — `renderStatement` prints the handle name,
 * the engine's violation `canonical` prints the raw row id — and the
 * paste-back law (`violation.canonical === renderStatement(statement)`)
 * breaks. Runs over the COMPLETE statement list, so declaration order
 * never matters. The closed relation's own `id` field resolves directly
 * (the walk's field-0 case).
 */
function verifyClosedReferences(name: string, statements: readonly Statement[]): void {
	for (const statement of statements) {
		const data = statement.data
		if (data.kind === "key") {
			continue
		}
		const rendered = renderStatement(statement)
		for (const face of [data.source, data.target]) {
			for (const binding of face.selection) {
				verifyClosedReferenceBinding(name, statements, face, binding, rendered)
			}
		}
	}
}

/** One binding's closed-reference resolution check (the {@link verifyClosedReferences} leaf). */
function verifyClosedReferenceBinding(
	name: string,
	statements: readonly Statement[],
	face: FaceData,
	binding: { readonly field: string; readonly set: LiteralSetSpec },
	rendered: string
): void {
	const spellsHandle = bindingLiterals(binding.set).some(function isHandle(literal) {
		return literal.kind === "handle"
	})
	if (!spellsHandle) {
		return
	}
	const roster = faceField(face, binding.field)?.closed
	if (roster === undefined) {
		return
	}
	if ("handles" in face.owner.data && binding.field === "id") {
		return
	}
	const resolved = closedTargetOf(statements, face.owner.name, binding.field)
	if (resolved !== roster.name) {
		throw errors.new(
			`schema ${name}: ${face.owner.name}.${binding.field} spells a ${roster.name} handle, but no declared containment resolves the closed reference — a closed reference is the plain u64 column plus its declared containment; declare contained(on(${face.owner.name}, "${binding.field}"), on(${roster.name}, "id")) — ${rendered}`
		)
	}
}

/** One member of a schema's relation record. */
type SchemaRelation = AnyRelation | AnyClosed

/** The relation record a schema is generic over — what `Db` and queries key on. */
type SchemaRelations = Record<string, SchemaRelation>

/** A theory value: named relations plus the DECLARED dependency statements. */
interface Schema<Rels extends SchemaRelations> {
	readonly name: string
	readonly relations: Rels
	readonly statements: readonly Statement[]
}

/** Any schema value, whatever its relation record. */
type AnySchema = Schema<SchemaRelations>

/**
 * Assembles a theory:
 * `schema("Ledger", { Kind, Account, Holder }, [ ...statements ])`.
 *
 * Rejected here, each with the offending statement rendered canonically:
 * a record key differing from its relation's declared name; a statement
 * whose relation is not (identically) a member of the record; an explicit
 * duplicate of a fresh-implied or closedness-implied key (macro parity:
 * "redundant here — and rejected as a duplicate"); a duplicate statement
 * (two statements rendering to one canonical utterance ARE one judgment);
 * a handle selection that its roster does not hold (belt-and-braces —
 * the type level already blocks it); and a handle selection whose closed
 * reference no declared containment resolves (the engine's canonical
 * renderer would print the raw row id where `renderStatement` prints the
 * handle — the paste-back law demands the two spellings agree).
 *
 * The fresh-implied and closed auto-keys are NOT added to the statement
 * list: the engine materializes them itself, in its own pinned order
 * (`SchemaDescriptor::materialized_statements`), and restating them would
 * double them.
 */
function schema<const Rels extends SchemaRelations>(
	name: string,
	relations: Rels,
	statements: readonly Statement[]
): Schema<Rels> {
	const implied = collectImplied(name, relations)
	const seen = new Set<string>()
	for (const statement of statements) {
		const rendered = renderStatement(statement)
		verifyMembership(name, relations, statement, rendered)
		if (implied.has(rendered)) {
			throw errors.new(
				`schema ${name}: ${rendered} is redundant here (the fresh mark or closedness already implies it) — and rejected as a duplicate`
			)
		}
		if (seen.has(rendered)) {
			throw errors.new(`schema ${name}: duplicate statement — ${rendered}`)
		}
		seen.add(rendered)
		verifyHandles(name, statement, rendered)
	}
	verifyClosedReferences(name, statements)
	return Object.freeze({ name, relations, statements: Object.freeze([...statements]) })
}

export type { AnySchema, Schema, SchemaRelation, SchemaRelations }
export { schema }
