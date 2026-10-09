import type { Effect, Scope } from "effect"
import type { ChangeDraft } from "../changes.ts"
import type { QueryReader } from "../db.ts"
import type { DbError } from "../errors.ts"
import { AuthoringError } from "../errors.ts"
import type { AnySchema } from "../schema.ts"
import { schemaDescriptor, schemasAgree } from "../schema.ts"

/** What `populate` reads from (the database at the old schema) and writes into (rows at the new one). */
interface PopulateContext<From extends AnySchema, To extends AnySchema> {
	readonly from: QueryReader<From>
	readonly into: ChangeDraft<To>
}

/**
 * Computes the rows a migration adds at its new schema, reading the database at the old one.
 * Relations that keep their name and fields are copied unchanged without code.
 */
type Populate<From extends AnySchema, To extends AnySchema> = (
	context: PopulateContext<From, To>
) => Effect.Effect<void, DbError, Scope.Scope>

/**
 * One bundled migration: its stable `id` (the directory name), the content `hash` that
 * `bumbledb generate` wrote (64 hex digits), and the schema it migrates to. The first migration of a
 * bundle has no `from`: it is the initial schema, and its `populate` seeds a newly created database
 * once.
 */
interface Migration<From extends AnySchema | undefined = AnySchema | undefined, To extends AnySchema = AnySchema> {
	readonly id: string
	readonly hash: string
	readonly from: From
	readonly to: To
	readonly populate?: Populate<From extends AnySchema ? From : To, To>
}

/** Any migration of a bundle; its `populate` is read back through {@link populateOf}. */
interface AnyMigration {
	readonly id: string
	readonly hash: string
	readonly from: AnySchema | undefined
	readonly to: AnySchema
	readonly populate?: unknown
}

/** A bundle: the initial migration first, then each migration from its predecessor's schema. */
type Migrations = readonly [AnyMigration, ...AnyMigration[]]

const HASH = /^[0-9a-f]{64}$/
const ID = /^[0-9]{4}_[A-Za-z0-9_]+$/

function make<To extends AnySchema, From extends AnySchema = never>(input: {
	readonly id: string
	readonly hash: string
	readonly from?: From
	readonly to: To
	readonly populate?: Populate<[From] extends [never] ? To : From, To>
}): Migration<[From] extends [never] ? undefined : From, To>
function make(input: {
	readonly id: string
	readonly hash: string
	readonly from?: AnySchema
	readonly to: AnySchema
	readonly populate?: Populate<AnySchema, AnySchema>
}): AnyMigration {
	if (!ID.test(input.id)) throw new AuthoringError({ message: `migration ${input.id}: an id is NNNN_name` })
	if (!HASH.test(input.hash)) throw new AuthoringError({ message: `migration ${input.id}: a hash is 64 hex digits` })
	const migration = {
		id: input.id,
		hash: input.hash,
		from: input.from === undefined ? undefined : schemaDescriptor(input.from),
		to: schemaDescriptor(input.to),
		...(input.populate === undefined ? {} : { populate: input.populate })
	}
	return Object.freeze(migration)
}

/** The populate step of a migration `make` built. */
function populateOf(migration: AnyMigration): Populate<AnySchema, AnySchema> | undefined {
	return migration.populate as Populate<AnySchema, AnySchema> | undefined
}

/** Checks that each migration starts from its predecessor's schema and that ids are unique. */
function verifyBundle(migrations: Migrations): void {
	const ids = new Set<string>()
	migrations.forEach((migration, index) => {
		if (ids.has(migration.id)) throw new AuthoringError({ message: `migrations: ${migration.id} appears twice` })
		ids.add(migration.id)
		const previous = migrations[index - 1]
		if (previous === undefined) {
			if (migration.from !== undefined) {
				throw new AuthoringError({ message: `migrations: the first migration ${migration.id} has no from schema` })
			}
			return
		}
		if (migration.from === undefined || !schemasAgree(migration.from, previous.to)) {
			throw new AuthoringError({
				message: `migrations: ${migration.id} must start from the schema ${previous.id} migrates to`
			})
		}
	})
}

const Migration = Object.freeze({ make })

export type { AnyMigration, Migrations, Populate, PopulateContext }
export { Migration, populateOf, verifyBundle }
