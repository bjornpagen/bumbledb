import { isDeepStrictEqual } from "node:util"
import { isCompiledSchema, registerCompiled } from "./compile.ts"
import { AuthoringError, internalError } from "./errors.ts"
/**
 * `schema` assembles relations and statements into a schema value and has the engine compile it.
 * The SDK checks only what the engine cannot see: that every statement names this schema's own
 * relation values. Every other judgment is the engine's, reported as an `AuthoringError`.
 */

import type { AnyClosed } from "./closed.ts"
import { memberDescriptor, membersAgree } from "./closed.ts"
import { assertDeclarationOrderKey, assertDeclarationRecord } from "./fields.ts"
import { descriptorCache, isImmutable } from "./immutable.ts"
import { type ClassesOf, classesComplete, computeClasses, type LawfulStatements, type SchemaClasses } from "./law.ts"
import type { AnyRelation } from "./relation.ts"
import { renderStatement, type Statement, statementDescriptor } from "./statements.ts"
import { arrayValue, recordValue } from "./values.ts"

/** Each relation's record key is its declared name. */
function verifyRelationNames(name: string, relations: SchemaRelations): void {
	assertDeclarationRecord(`schema ${name} relations`, relations)
	for (const [recordKey, member] of Object.entries(relations)) {
		assertDeclarationOrderKey(`schema ${name} relation`, recordKey)
		if (member.name !== recordKey) {
			throw new AuthoringError({
				message: `schema ${name}: record key ${recordKey} holds relation ${member.name} — the key must equal the relation's declared name`
			})
		}
	}
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
	return isCompiledSchema(input) && isImmutable(input) ? input : checkedSchema(input)
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
	verifyRelationNames(name, ownedRelations)
	for (const statement of ownedStatements) verifyMembership(name, ownedRelations, statement, renderStatement(statement))
	const classes = computeClasses(name, ownedRelations, ownedStatements)
	if (!classesComplete<EvaluatedClasses<ClassesOf<Rels, Stmts>>>(classes, ownedRelations)) {
		throw internalError(`schema ${name}: class-map construction incomplete`)
	}
	const result = Object.freeze({ name, relations: ownedRelations, statements: Object.freeze(ownedStatements), classes })
	registerCompiled(result)
	return result
}

export type { AnySchema, Schema, SchemaRelation, SchemaRelations }
export { schema, schemaDescriptor, schemasAgree }
