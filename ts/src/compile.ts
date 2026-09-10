import { Effect } from "effect"
import { isClosedMember, membersAgree } from "#closed.ts"
import { dbNative } from "#db-native.ts"
import { SdkInvariantError } from "#errors.ts"
import { isImmutable, snapshotData } from "#immutable.ts"
import type { SchemaClasses } from "#law.ts"
import { lower } from "#lower.ts"
import type { SealedDescriptor } from "#native.ts"
import type { AnyRelation } from "#relation.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError } from "#runtime-errors.ts"
import type { AnySchema, Schema as SchemaDeclaration, SchemaRelations } from "#schema.ts"
import { schemaDescriptor } from "#schema.ts"
import { type KeyStatement, type Statement, statementDescriptor } from "#statements.ts"

/**
 * `SchemaId` — the engine's canonical schema fingerprint as lowercase hex.
 * Independent of database identity; ordinary structural text. Native
 * boundaries verify the fingerprint wherever it grants admission.
 */
type SchemaId = string

/**
 * `CompiledSchema<S>` — bounded detached immutable descriptor data plus the
 * canonical `schemaId`. NOT a tenant handle or a second
 * user-authored schema: it needs no native finalizer, holds no native
 * resource, and open/create/build compile through the same implementation —
 * prior compilation is optional, never a mandatory prepare ceremony.
 */
interface CompiledSchema<S extends AnySchema> {
	readonly schema: S
	readonly schemaId: SchemaId
	readonly descriptor: SealedDescriptor
}

/**
 * Pure declaration-order tables shared by the whole core surface: relation
 * ids (declaration order = ids), materialized statement ids (closed
 * relations' auto-handle keys FIRST in relation-declaration order, then
 * declared statements in declaration order with `mirrors` occupying two
 * consecutive slots, source-first — the theory's `StatementId` law).
 * Every declared statement retains its own native identity; no key is
 * privileged by its position in the declaration.
 */
interface SchemaTables {
	readonly relationIds: ReadonlyMap<string, number>
	readonly statementIds: ReadonlyMap<Statement, number>
}

function declaredWidth(statement: Statement): number {
	return statement.kind === "mirrors" ? 2 : 1
}

function tablesOf(theory: AnySchema): SchemaTables {
	const relationIds = new Map<string, number>()
	const statementIds = new Map<Statement, number>()
	let autoKeys = 0
	Object.entries(theory.relations).forEach(function assignRelation([name, member], ordinal) {
		relationIds.set(name, ordinal)
		if (isClosedMember(member)) {
			autoKeys += 1
		}
	})
	let offset = autoKeys
	for (const statement of theory.statements) {
		statementIds.set(statement, offset)
		offset += declaredWidth(statement)
	}
	return Object.freeze({ relationIds, statementIds })
}

const compiledCache = new WeakMap<AnySchema, SchemaTables>()

/** Memoized pure tables (per schema value identity; no native work). */
function schemaTables(theory: AnySchema): SchemaTables {
	const cached = compiledCache.get(theory)
	if (cached !== undefined) {
		return cached
	}
	const built = tablesOf(theory)
	if (isImmutable(theory)) compiledCache.set(theory, built)
	return built
}

/** Resolve a logical key declaration, never an opaque constructor token. */
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

function admitSchemaId(fingerprint: string): SchemaId {
	if (typeof fingerprint !== "string" || !/^[0-9a-f]{64}$/.test(fingerprint)) {
		throw new SdkInvariantError({ message: "Schema.compile: the engine returned no canonical fingerprint" })
	}
	return fingerprint
}

/**
 * Effectful native schema admission/compilation: a
 * schema declaration is pure metadata and never a claim its theory has been
 * admitted. Runs on the shared executor, requires the
 * acquired {@link NativeRuntime}, and yields detached immutable descriptor
 * data plus the canonical schema identity. No database is opened and no
 * native finalizer is created.
 */
const compile = Effect.fn("Schema.compile")(function* <S extends AnySchema>(schema: S) {
	const owned = yield* Effect.try({
		try: () => schemaDescriptor(schema),
		catch: (cause) => argumentError("Schema.compile", cause)
	})
	const spec = yield* Effect.try({ try: () => lower(owned), catch: (cause) => argumentError("Schema.compile", cause) })
	const handle = yield* runtimeHandle()
	const descriptor = yield* nativeOperationWith(
		"Schema.compile",
		(callback) => dbNative.runtimeSchemaCompile(handle, spec, callback),
		dbNative.runtimeSchemaTake,
		(value) => value
	)
	const ownedDescriptor = snapshotData(descriptor)
	const compiled: CompiledSchema<S> = Object.freeze({
		get schema() {
			return snapshotData(owned)
		},
		schemaId: admitSchemaId(descriptor.fingerprint),
		get descriptor() {
			return snapshotData(ownedDescriptor)
		}
	})
	return compiled
})

/**
 * The declaration-tier `Schema<Rels, Classes>` TYPE (from `#schema.ts`),
 * re-aliased LOCALLY so the one name carries both meanings through a
 * SINGLE export specifier below — a local `type` + `const` merge is the
 * plain TypeScript type/value merge, with no same-name pair of export
 * declarations for any checker to refuse (the earlier
 * `export type { Schema } from "#schema.ts"` beside `export { Schema }`
 * spelled the type meaning twice at the barrel; this spelling cannot).
 */
type Schema<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> = SchemaDeclaration<
	Rels,
	Classes
>

/**
 * The core `Schema` namespace (import as `BumbleSchema` when Effect Schema
 * is also in scope). The `Schema<Rels>` type remains the pure
 * declaration from `#schema.ts` (the local alias above); this value owns
 * the effectful half. One exported name, two meanings.
 */
const Schema = Object.freeze({ compile })

export type { CompiledSchema, SchemaId, SchemaTables }
export { declaredKey, Schema, schemaTables }
