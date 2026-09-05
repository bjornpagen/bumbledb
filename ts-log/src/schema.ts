/**
 * `@bjornpagen/bumbledb-log/schema` — ONLY pure schema-evolution intent
 * constructors (chapter 35). These build inert typed metadata over the core's
 * own schema/`ScalarExpr` values; generation, filesystem work, hashing and
 * execution live in `@bjornpagen/bumbledb-log/migrations` and the native
 * codec. Importing this module performs no native work.
 */
export {
	backfill,
	convert,
	dropField,
	dropRelation,
	migrationIntent,
	renameField,
	renameRelation,
	seed
} from "#migrations/intent.ts"
export type { MigrationIntent, MigrationIntentEntry } from "#migrations/intent.ts"
