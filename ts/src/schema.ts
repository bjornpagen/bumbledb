import { isDeepStrictEqual } from "node:util"
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
 * legality, …) stays the engine's `SchemaError` at
 * `Db.create` — the engine is the final authority for every boundary,
 * this wall just makes the SDK agree with it first.
 */

import type { AnyClosed } from "#closed.ts"
import { isClosedMember, memberDescriptor, membersAgree, sealedFieldOf } from "#closed.ts"
import type { AnyFace } from "#face.ts"
import { assertDeclarationOrderKey, assertDeclarationRecord, rosterOf } from "#fields.ts"
import { descriptorCache, isImmutable } from "#immutable.ts"
import { type ClassesOf, classesComplete, computeClasses, type LawfulStatements, type SchemaClasses } from "#law.ts"
import type { AnyRelation } from "#relation.ts"
import type { LiteralSetSpec, LiteralSpec } from "#spec.ts"
import { renderStatement, type Statement, statementDescriptor } from "#statements.ts"
import { arrayValue, recordValue } from "#values.ts"

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
		}
		for (const projection of projections) {
			rendered.add(`${member.name}(${projection.join(", ")}) -> ${member.name}`)
		}
		roster.set(member.name, Object.freeze(projections))
	}
	return { rendered, roster }
}

function statementOwners(statement: Statement): readonly SchemaRelation[] {
	const data = statement
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
		if (!membersAgree(member, owner)) {
			throw new AuthoringError({
				message: `schema ${name}: statement references a different relation declaration named ${owner.name} than the one this schema declares — ${rendered}`
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
	face: AnyFace,
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
	const data = statement
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
		const data = statement
		if (data.kind !== "containment" && data.kind !== "mirrors") {
			continue
		}
		const pairs: Array<readonly [AnyFace, AnyFace]> = [[data.source, data.target]]
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
		const data = statement
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
	face: AnyFace,
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
 * materializes — the closed auto-keys
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
		const data = statement
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
		const data = statement
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
	face: AnyFace,
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

/**
 * Logical schema equivalence, including the ordered native ordinals and
 * host decoding domains. Schema display names do not change the theory.
 * A database/snapshot still has its own resource identity and lifetime.
 */
function schemasAgree(leftInput: AnySchema, rightInput: AnySchema): boolean {
	const left = schemaDescriptor(leftInput)
	const right = schemaDescriptor(rightInput)
	if (left === right) return true
	const cacheable = isImmutable(left) && isImmutable(right)
	const cached = schemaComparisons.get(left)?.get(right)
	if (cacheable && cached !== undefined) return cached
	const names = Object.keys(left.relations)
	const otherNames = Object.keys(right.relations)
	const equal =
		names.length === otherNames.length &&
		names.every((name, index) => {
			const member = right.relations[name]
			return name === otherNames[index] && member !== undefined && membersAgree(left.relations[name], member)
		}) &&
		isDeepStrictEqual(left.statements, right.statements) &&
		isDeepStrictEqual(left.classes, right.classes)
	if (cacheable) {
		let comparisons = schemaComparisons.get(left)
		if (comparisons === undefined) {
			comparisons = new WeakMap()
			schemaComparisons.set(left, comparisons)
		}
		comparisons.set(right, equal)
	}
	return equal
}

const schemaComparisons = new WeakMap<AnySchema, WeakMap<AnySchema, boolean>>()

function schemaDescriptor<S extends AnySchema>(input: S): S
function schemaDescriptor(input: unknown): AnySchema
function schemaDescriptor(input: unknown): AnySchema {
	return checkedSchema(input)
}

const checkedSchema = descriptorCache((input): AnySchema => {
	const raw = recordValue("schema", input, ["name", "relations", "statements", "classes"])
	if (typeof raw.name !== "string") throw new AuthoringError({ message: "schema: expected a name" })
	const relations = ownRelations(raw.name, raw.relations)
	const statements = arrayValue("schema statements", raw.statements, (_, statement) => statementDescriptor(statement))
	const result = schema(raw.name, relations, statements)
	const classes = recordValue("schema classes", raw.classes, Object.keys(result.classes))
	for (const [name, fields] of Object.entries(result.classes)) {
		const supplied = recordValue(`schema classes.${name}`, classes[name], Object.keys(fields))
		for (const [field, value] of Object.entries(fields)) {
			if (supplied[field] !== value)
				throw new AuthoringError({ message: `schema classes.${name}.${field}: incorrect class` })
		}
	}
	return result
})

type EvaluatedClasses<C extends SchemaClasses> = C extends SchemaClasses
	? { readonly [N in keyof C]: { readonly [F in keyof C[N]]: C[N][F] } }
	: never

function ownRelations<R extends SchemaRelations>(name: string, input: R): R
function ownRelations(name: string, input: unknown): SchemaRelations
function ownRelations(name: string, input: unknown): SchemaRelations {
	if (typeof input !== "object" || input === null)
		throw new AuthoringError({ message: `schema ${name}: expected relations` })
	assertDeclarationRecord(`schema ${name} relations`, input)
	return Object.freeze(
		Object.fromEntries(Object.entries(input).map(([name, member]) => [name, memberDescriptor(member)]))
	)
}

function schema<const Rels extends SchemaRelations, const Stmts extends readonly Statement[]>(
	name: string,
	relations: Rels,
	statements: Stmts & LawfulStatements<Rels, Stmts>
): Schema<Rels, EvaluatedClasses<ClassesOf<Rels, Stmts>>> {
	assertDeclarationOrderKey("schema", name)
	const ownedRelations = ownRelations(name, relations)
	const ownedStatements = arrayValue("schema statements", statements, (_, statement) => statementDescriptor(statement))
	const implied = collectImplied(name, ownedRelations)
	const seen = new Set<string>()
	for (const statement of ownedStatements) {
		const rendered = renderStatement(statement)
		verifyMembership(name, ownedRelations, statement, rendered)
		if (implied.rendered.has(rendered)) {
			throw new AuthoringError({
				message: `schema ${name}: ${rendered} is redundant here (closedness already implies it) — and rejected as a duplicate`
			})
		}
		if (seen.has(rendered)) {
			throw new AuthoringError({ message: `schema ${name}: duplicate statement — ${rendered}` })
		}
		seen.add(rendered)
		verifyHandles(name, statement, rendered)
	}
	verifyClosedReferences(name, ownedStatements)
	verifyTargetKeys(name, ownedStatements, implied.roster)
	const classes = computeClasses(name, ownedRelations, ownedStatements)
	if (!classesComplete<EvaluatedClasses<ClassesOf<Rels, Stmts>>>(classes, ownedRelations)) {
		throw new SdkInvariantError({ message: `schema ${name}: class-map construction incomplete` })
	}
	return Object.freeze({ name, relations: ownedRelations, statements: Object.freeze(ownedStatements), classes })
}

export type { AnySchema, Schema, SchemaRelation, SchemaRelations }
export { schema, schemaDescriptor, schemasAgree }
