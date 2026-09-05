import { AuthoringError, SdkInvariantError } from "#errors.ts"
/**
 * `schema` — assembles relations and statements into a theory value (the
 * `Theory` analog; what `Db.create`/`Db.open` take). Construction-time
 * validation is the macro-EXPANSION-boundary analog: membership,
 * implied-key duplicates, duplicate statements, a belt-and-braces handle
 * re-verification — and the TARGET-KEY WALL ({@link verifyTargetKeys}),
 * the value tier of the two-tier containment law
 * (60-containment-parity): every containment/mirrors/capacity target
 * projection must set-match a key of its relation, judged HERE with the
 * engine's exact rule so `lower` never emits an engine-refused
 * containment. The type tier is `law.ts`'s `TargetKeyWall` (best effort,
 * statically known tuples); every OTHER semantic judgment (key-internal
 * legality, fresh-on-u64, …) stays the engine's `SchemaError` at
 * `Db.create` — the engine is the final authority for every boundary,
 * this wall just makes the SDK agree with it first.
 */

import type { AnyClosed } from "#closed.ts"
import { isClosedMember, sealedFieldOf } from "#closed.ts"
import type { FaceData } from "#face.ts"
import { assertDeclarationOrderKey, assertDeclarationRecord, rosterOf } from "#fields.ts"
import { type ClassesOf, classesComplete, computeClasses, type LawfulStatements, type SchemaClasses } from "#law.ts"
import type { AnyRelation } from "#relation.ts"
import type { LiteralSetSpec, LiteralSpec } from "#spec.ts"
import { isStatement, renderStatement, type Statement } from "#statements.ts"

interface ImpliedKeys {
	readonly rendered: ReadonlySet<string>
	readonly roster: ReadonlyMap<string, ReadonlyArray<readonly string[]>>
}

function collectImplied(name: string, relations: SchemaRelations): ImpliedKeys {
	assertDeclarationRecord(`schema ${name} relations`, relations)
	const rendered = new Set<string>()
	const roster = new Map<string, ReadonlyArray<readonly string[]>>()
	for (const [recordKey, member] of Object.entries(relations)) {
		assertDeclarationOrderKey(`schema ${name} relation`, recordKey)
		if (member.name !== recordKey) {
			throw new AuthoringError({
				message: `schema ${name}: record key ${recordKey} holds relation ${member.name} — the key must equal the relation's declared name`
			})
		}
		const projections: Array<readonly string[]> = []
		if (isClosedMember(member)) {
			projections.push(Object.freeze(["id"]))
		} else {
			for (const declared of member.data.fields) {
				if ("fresh" in declared.field && declared.field.fresh === true) {
					projections.push(Object.freeze([declared.name]))
				}
			}
		}
		for (const projection of projections) {
			rendered.add(`${member.name}(${projection.join(", ")}) -> ${member.name}`)
		}
		roster.set(member.name, Object.freeze(projections))
	}
	return { rendered, roster }
}

function statementOwners(statement: Statement): readonly SchemaRelation[] {
	const data = statement.data
	if (data.kind === "key") {
		return [data.owner]
	}
	return [data.source.owner, data.target.owner]
}

function verifyMembership(name: string, relations: SchemaRelations, statement: Statement, rendered: string): void {
	for (const owner of statementOwners(statement)) {
		const member = relations[owner.name]
		if (member === undefined) {
			throw new AuthoringError({
				message: `schema ${name}: relation ${owner.name} is not declared in this schema — ${rendered}`
			})
		}
		if (member !== owner) {
			throw new AuthoringError({
				message: `schema ${name}: statement references a different relation value named ${owner.name} than the one this schema declares — ${rendered}`
			})
		}
	}
}

function bindingLiterals(set: LiteralSetSpec): readonly LiteralSpec[] {
	if (set.kind === "one") {
		return [set.literal]
	}
	return set.literals
}

function verifyBindingHandles(
	name: string,
	face: FaceData,
	binding: { readonly field: string; readonly set: LiteralSetSpec },
	rendered: string
): void {
	const roster = rosterOf(sealedFieldOf(face.owner, binding.field))
	for (const literal of bindingLiterals(binding.set)) {
		if (literal.kind !== "handle") {
			continue
		}
		if (roster === undefined) {
			throw new AuthoringError({
				message: `schema ${name}: ${face.owner.name}.${binding.field} is not a closed-relation reference — the handle literal ${literal.handle} is legal only on a field carrying a closed relation's roster — ${rendered}`
			})
		}
		if (!roster.handles.includes(literal.handle)) {
			throw new AuthoringError({
				message: `schema ${name}: closed relation ${roster.name} has no handle ${literal.handle} — ${rendered}`
			})
		}
	}
}

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

function closedTargetOf(statements: readonly Statement[], owner: string, field: string): string | undefined {
	for (const statement of statements) {
		const data = statement.data
		if (data.kind !== "containment" && data.kind !== "mirrors") {
			continue
		}
		const pairs: Array<readonly [FaceData, FaceData]> = [[data.source, data.target]]
		if (data.kind === "mirrors") {
			pairs.push([data.target, data.source])
		}
		for (const [source, target] of pairs) {
			if (
				source.owner.name === owner &&
				source.projection.length === 1 &&
				source.projection[0] === field &&
				target.projection.length === 1 &&
				target.projection[0] === "id" &&
				isClosedMember(target.owner)
			) {
				return target.owner.name
			}
		}
	}
	return undefined
}

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
	const roster = rosterOf(sealedFieldOf(face.owner, binding.field))
	if (roster === undefined) {
		return
	}
	if (isClosedMember(face.owner) && binding.field === "id") {
		return
	}
	const resolved = closedTargetOf(statements, face.owner.name, binding.field)
	if (resolved !== roster.name) {
		throw new AuthoringError({
			message: `schema ${name}: ${face.owner.name}.${binding.field} spells a ${roster.name} handle, but no declared containment resolves the closed reference — a closed reference is the plain u64 column plus its declared containment; declare contained(on(${face.owner.name}, "${binding.field}"), on(${roster.name}, "id")) — ${rendered}`
		})
	}
}

/**
 * THE TARGET-KEY WALL, value tier (60-containment-parity — the runtime
 * twin of `law.ts`'s `TargetKeyWall`, the engine's `resolve_target_key` /
 * `resolve_capacity_target` mirrored exactly): every `contained`/
 * `mirrors`/`capacity` statement's target projection must resolve a key
 * of the target relation, judged over the SAME key population the engine
 * materializes — the fresh-implied and closed auto-keys
 * ({@link collectImplied}'s roster) first, then the declared `key`
 * statements in written order (a key may be declared after its probe, so
 * this wall runs over the COMPLETE list, never inside the statement
 * loop). `mirrors` materializes as two containments source-first, so both
 * orientations judge their own target. SOUNDNESS BAR: the set-match +
 * closed-id rule below is the engine's COMPLETE rule for this law
 * (`matching_functionality` compares field SETS — permutations resolve,
 * subsets/supersets refuse), so this wall never rejects what the engine
 * accepts; every other schema judgment stays engine-first.
 */
function verifyTargetKeys(
	name: string,
	statements: readonly Statement[],
	implied: ReadonlyMap<string, ReadonlyArray<readonly string[]>>
): void {
	const declared = new Map<string, Array<readonly string[]>>()
	for (const statement of statements) {
		const data = statement.data
		if (data.kind !== "key") {
			continue
		}
		const keys = declared.get(data.owner.name)
		if (keys === undefined) {
			declared.set(data.owner.name, [data.projection])
		} else {
			keys.push(data.projection)
		}
	}
	for (const statement of statements) {
		const data = statement.data
		if (data.kind === "key") {
			continue
		}
		const rendered = renderStatement(statement)

		const faces = data.kind === "mirrors" ? [data.target, data.source] : [data.target]
		for (const face of faces) {
			verifyTargetKeyFace(name, face, implied, declared, rendered)
		}
	}
}

/**
 * One target face's key resolution (the {@link verifyTargetKeys} leaf).
 * Closed target: the handle id is the ONE probe-able identity of a closed
 * relation, so the projection must be exactly `["id"]` — its own refusal
 * (the engine's `ClosedTargetNotHandle`): the rule is CLOSEDNESS, not key
 * absence. Ordinary target: the projection's field-name SET must equal
 * some roster member's set (the engine's `matching_functionality` —
 * permutations resolve, subsets and supersets do not). The refusal speaks
 * the engine's shape in NAMES, with the engine's pointwise hint verbatim
 * when the projection carries an interval position.
 */
function verifyTargetKeyFace(
	name: string,
	face: FaceData,
	implied: ReadonlyMap<string, ReadonlyArray<readonly string[]>>,
	declared: ReadonlyMap<string, ReadonlyArray<readonly string[]>>,
	rendered: string
): void {
	if (isClosedMember(face.owner)) {
		if (face.projection.length === 1 && face.projection[0] === "id") {
			return
		}
		throw new AuthoringError({
			message: `schema ${name}: ${rendered}: closed target ${face.owner.name} is addressed by its synthetic id only — projection (${face.projection.join(", ")}) must be exactly (id) (rewrite the target side as on(${face.owner.name}, "id"))`
		})
	}
	const roster = [...(implied.get(face.owner.name) ?? []), ...(declared.get(face.owner.name) ?? [])]
	const want = new Set(face.projection)
	const matched = roster.some(function sameFieldSet(key) {
		// engine's FieldSet refuses (duplicates are refused at the key()

		if (key.length !== face.projection.length || want.size !== face.projection.length) {
			return false
		}
		const keySet = new Set(key)
		if (keySet.size !== want.size) {
			return false
		}
		for (const field of keySet) {
			if (!want.has(field)) {
				return false
			}
		}
		return true
	})
	if (matched) {
		return
	}
	const available = roster.length === 0 ? "none" : roster.map((key) => `(${key.join(", ")})`).join("; ")
	const pointwise = face.projection.some(function carriesInterval(fieldName) {
		const descriptor = sealedFieldOf(face.owner, fieldName)
		return descriptor !== undefined && descriptor.kind === "interval"
	})
	const hint = pointwise ? "; hint: declare the exact pointwise key `R(prefix…, interval) -> R`" : ""
	throw new AuthoringError({
		message: `schema ${name}: ${rendered}: target projection (${face.projection.join(", ")}) matches no declared key of ${face.owner.name} — available keys: ${available}${hint}`
	})
}

type SchemaRelation = AnyRelation | AnyClosed

type SchemaRelations = Record<string, SchemaRelation>

interface Schema<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> {
	readonly name: string
	readonly relations: Rels
	readonly statements: readonly Statement[]
	readonly classes: Classes
}

type AnySchema = Schema<SchemaRelations>

type EvaluatedClasses<C extends SchemaClasses> = C extends SchemaClasses
	? { readonly [N in keyof C]: { readonly [F in keyof C[N]]: C[N][F] } }
	: never

function schema<const Rels extends SchemaRelations, const Stmts extends readonly Statement[]>(
	name: string,
	relations: Rels,
	statements: Stmts & LawfulStatements<Rels, Stmts>
): Schema<Rels, EvaluatedClasses<ClassesOf<Rels, Stmts>>> {
	const implied = collectImplied(name, relations)
	const seen = new Set<string>()
	for (const statement of statements) {
		if (!isStatement(statement)) {
			throw new AuthoringError({
				message: `schema ${name}: a statement is minted only by key/contained/mirrors/capacity — a structural literal skips the construction-time arity and roster walls`
			})
		}
		const rendered = renderStatement(statement)
		verifyMembership(name, relations, statement, rendered)
		if (implied.rendered.has(rendered)) {
			throw new AuthoringError({
				message: `schema ${name}: ${rendered} is redundant here (the fresh mark or closedness already implies it) — and rejected as a duplicate`
			})
		}
		if (seen.has(rendered)) {
			throw new AuthoringError({ message: `schema ${name}: duplicate statement — ${rendered}` })
		}
		seen.add(rendered)
		verifyHandles(name, statement, rendered)
	}
	verifyClosedReferences(name, statements)
	verifyTargetKeys(name, statements, implied.roster)
	const classes = computeClasses(name, relations, statements)
	if (!classesComplete<EvaluatedClasses<ClassesOf<Rels, Stmts>>>(classes, relations)) {
		throw new SdkInvariantError({ message: `schema ${name}: class-map construction incomplete` })
	}
	return Object.freeze({ name, relations, statements: Object.freeze([...statements]), classes })
}

export type { AnySchema, Schema, SchemaRelation, SchemaRelations }
export { schema }
