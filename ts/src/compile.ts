import { Effect } from "effect"
import { isClosedMember } from "#closed.ts"
import { dbNative } from "#db-native.ts"
import { SdkInvariantError } from "#errors.ts"
import { lower } from "#lower.ts"
import type { SealedDescriptor } from "#native.ts"
import type { ExecutionPolicy } from "#runtime.ts"
import { nativeOperationWith, policyWire, runtimeHandle } from "#runtime.ts"
import type { SchemaClasses } from "#law.ts"
import type { AnySchema, Schema as SchemaDeclaration, SchemaRelations } from "#schema.ts"
import type { Statement } from "#statements.ts"

/**
 * `SchemaId` — the canonical schema/theory identity (chapter 30's value
 * vocabulary): the engine's canonical schema fingerprint as lowercase hex.
 * Independent of database identity; a branded string so arbitrary text does
 * not typecheck where a schema identity is required.
 */
declare const schemaIdBrand: unique symbol

type SchemaId = string & { readonly [schemaIdBrand]: "bumbledb.SchemaId" }

/**
 * `CompiledSchema<S>` — bounded detached immutable descriptor data plus the
 * canonical `schemaId` (chapter 35). NOT a tenant handle or a second
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
 * consecutive slots, source-first — the theory's `StatementId` law), and
 * each relation's primary key (a closed relation's synthetic `(id)`,
 * otherwise its FIRST declared `key` statement).
 */
interface PrimaryKey {
	readonly statementId: number
	readonly projection: readonly string[]
}

interface SchemaTables {
	readonly relationIds: ReadonlyMap<string, number>
	readonly primaryKeys: ReadonlyMap<string, PrimaryKey>
}

function declaredWidth(statement: Statement): number {
	return statement.data.kind === "mirrors" ? 2 : 1
}

function tablesOf(theory: AnySchema): SchemaTables {
	const relationIds = new Map<string, number>()
	const primaryKeys = new Map<string, PrimaryKey>()
	let autoKeys = 0
	Object.entries(theory.relations).forEach(function assignRelation([name, member], ordinal) {
		relationIds.set(name, ordinal)
		if (isClosedMember(member)) {
			primaryKeys.set(name, Object.freeze({ statementId: autoKeys, projection: Object.freeze(["id"]) }))
			autoKeys += 1
		}
	})
	let offset = autoKeys
	for (const statement of theory.statements) {
		const data = statement.data
		if (data.kind === "key" && !primaryKeys.has(data.owner.name)) {
			primaryKeys.set(
				data.owner.name,
				Object.freeze({ statementId: offset, projection: data.projection })
			)
		}
		offset += declaredWidth(statement)
	}
	return Object.freeze({ relationIds, primaryKeys })
}

const compiledCache = new WeakMap<AnySchema, SchemaTables>()

/** Memoized pure tables (per schema value identity; no native work). */
function schemaTables(theory: AnySchema): SchemaTables {
	const cached = compiledCache.get(theory)
	if (cached !== undefined) {
		return cached
	}
	const built = tablesOf(theory)
	compiledCache.set(theory, built)
	return built
}

function admitSchemaId(fingerprint: string): SchemaId {
	if (typeof fingerprint !== "string" || fingerprint.length === 0) {
		throw new SdkInvariantError({ message: "Schema.compile: the engine returned no canonical fingerprint" })
	}
	return fingerprint as SchemaId
}

/**
 * Charged, effectful native schema admission/compilation (chapter 35): a
 * schema declaration is pure metadata and never a claim its theory has been
 * admitted. Runs on the one bounded executor under `work`, requires the
 * acquired {@link NativeRuntime}, and yields detached immutable descriptor
 * data plus the canonical schema identity. No database is opened and no
 * native finalizer is created.
 */
const compile = Effect.fn("Schema.compile")(function* <S extends AnySchema>(schema: S, work: ExecutionPolicy) {
	const handle = yield* runtimeHandle()
	const spec = lower(schema)
	const descriptor = yield* nativeOperationWith(
		"Schema.compile",
		(callback) => dbNative.runtimeSchemaCompile(handle, policyWire(work, "Schema.compile"), spec, callback),
		dbNative.runtimeSchemaTake,
		(value) => value
	)
	const compiled: CompiledSchema<S> = Object.freeze({
		schema,
		schemaId: admitSchemaId(descriptor.fingerprint),
		descriptor
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
 * is also in scope — chapter 35). The `Schema<Rels>` TYPE remains the pure
 * declaration from `#schema.ts` (the local alias above); this value owns
 * the effectful half. One exported name, two meanings.
 */
const Schema = Object.freeze({ compile })

export type { CompiledSchema, PrimaryKey, SchemaId, SchemaTables }
export { Schema, schemaTables }
