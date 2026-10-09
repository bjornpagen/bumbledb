/**
 * A schema is compiled by the engine when it is defined. The compiled handle, descriptor and
 * fingerprint ride with the schema value; the engine's diagnostics come back as authoring errors
 * that name the schema's own relations and statements.
 */
import { Schema as EffectSchema } from "effect"
import { isClosedMember, membersAgree } from "./closed.ts"
import { AuthoringError, internalError } from "./errors.ts"
import { isImmutable } from "./immutable.ts"
import { lower } from "./lower.ts"
import type { SchemaRef } from "./native/addon.ts"
import { addon } from "./native/addon.ts"
import type { SchemaDescriptorOut, SchemaDiagnostic } from "./native/binding.d.ts"
import type { AnyRelation } from "./relation.ts"
import type { AnySchema } from "./schema.ts"
import { type KeyStatement, renderStatement, type Statement, statementDescriptor } from "./statements.ts"

/** The engine's schema fingerprint: 64 lowercase hex digits, independent of any database. */
const SchemaId = EffectSchema.String.check(EffectSchema.isPattern(/^[0-9a-f]{64}$/)).pipe(
	EffectSchema.brand("SchemaId")
)
type SchemaId = typeof SchemaId.Type

interface Compiled {
	readonly handle: SchemaRef
	readonly descriptor: SchemaDescriptorOut
	readonly schemaId: SchemaId
}

/**
 * Declaration-order tables: relation ids, and each declared statement's first materialized
 * statement id (closed relations' id keys come first; `mirrors` takes two consecutive ids).
 */
interface SchemaTables {
	readonly relationIds: ReadonlyMap<string, number>
	readonly statementIds: ReadonlyMap<Statement, number>
}

function tablesOf(theory: AnySchema): SchemaTables {
	const relationIds = new Map<string, number>()
	const statementIds = new Map<Statement, number>()
	let offset = 0
	Object.entries(theory.relations).forEach(([name, member], ordinal) => {
		relationIds.set(name, ordinal)
		if (isClosedMember(member)) offset += 1
	})
	for (const statement of theory.statements) {
		statementIds.set(statement, offset)
		offset += statement.kind === "mirrors" ? 2 : 1
	}
	return Object.freeze({ relationIds, statementIds })
}

const tablesCache = new WeakMap<AnySchema, SchemaTables>()

function schemaTables(theory: AnySchema): SchemaTables {
	const cached = tablesCache.get(theory)
	if (cached !== undefined) return cached
	const built = tablesOf(theory)
	if (isImmutable(theory)) tablesCache.set(theory, built)
	return built
}

/** Resolves a key declaration to the schema's own statement and its statement id. */
function declaredKey<R extends AnyRelation>(
	theory: AnySchema,
	input: KeyStatement<R>
): { readonly key: KeyStatement<R>; readonly statementId: number } | undefined
function declaredKey(
	theory: AnySchema,
	input: KeyStatement
): { readonly key: KeyStatement; readonly statementId: number } | undefined {
	const key = statementDescriptor(input)
	for (const [candidate, statementId] of schemaTables(theory).statementIds) {
		if (
			candidate.kind === "key" &&
			membersAgree(candidate.owner, key.owner) &&
			candidate.projection.length === key.projection.length &&
			candidate.projection.every((field) => key.projection.includes(field))
		)
			return { key: candidate, statementId }
	}
	return undefined
}

function explain(theory: AnySchema, diagnostic: SchemaDiagnostic): string {
	if (diagnostic._tag === "Schema") {
		const cited = [diagnostic.statement, diagnostic.conflict].flatMap((cite) =>
			cite === undefined ? [] : [cite.spelling]
		)
		return cited.length === 0 ? diagnostic.message : `${diagnostic.message} — ${cited.join("; ")}`
	}
	const relations = Object.keys(theory.relations)
	return diagnostic.issues
		.map((issue) => {
			const at = [
				issue.relation === undefined ? undefined : `relation ${relations[issue.relation] ?? issue.relation}`,
				issue.row === undefined ? undefined : `row ${issue.row}`,
				issue.statement === undefined
					? undefined
					: (() => {
							const statement = theory.statements[issue.statement]
							return statement === undefined ? `statement ${issue.statement}` : renderStatement(statement)
						})()
			].filter((part) => part !== undefined)
			return at.length === 0 ? issue.message : `${issue.message} — ${at.join(", ")}`
		})
		.join("; ")
}

const decodeSchemaId = EffectSchema.decodeUnknownOption(SchemaId)

/** Compiles `theory` with the engine; an engine refusal throws `AuthoringError`. */
function compileSchema(theory: AnySchema): Compiled {
	const compiled = addon.compileSchema(JSON.stringify(lower(theory)))
	switch (compiled._tag) {
		case "Compiled": {
			const schemaId = decodeSchemaId(compiled.descriptor.fingerprint)
			if (schemaId._tag === "None") throw internalError("compileSchema: the engine returned no fingerprint")
			return Object.freeze({ handle: compiled.schema, descriptor: compiled.descriptor, schemaId: schemaId.value })
		}
		case "Invalid":
			throw new AuthoringError({ message: `schema ${theory.name}: ${explain(theory, compiled.diagnostic)}` })
		case "Malformed":
			throw internalError(`compileSchema: ${compiled.path}: ${compiled.message}`)
	}
}

const compiledSchemas = new WeakMap<AnySchema, Compiled>()

/** Records the compiled form of a schema value `schema()` produced. */
function registerCompiled(theory: AnySchema): void {
	compiledSchemas.set(theory, compileSchema(theory))
}

/** Whether `value` is a schema value `schema()` produced (and so has a compiled form). */
function isCompiledSchema(value: unknown): value is AnySchema {
	return typeof value === "object" && value !== null && compiledSchemas.has(value as AnySchema)
}

/** The engine's compiled form of `theory`; every schema value `schema()` produced has one. */
function compiledOf(theory: AnySchema): Compiled {
	const compiled = compiledSchemas.get(theory)
	if (compiled === undefined) throw internalError("compiledOf: the schema was not produced by schema()")
	return compiled
}

export type { Compiled, SchemaTables }
export { compiledOf, declaredKey, isCompiledSchema, registerCompiled, SchemaId, schemaTables }
