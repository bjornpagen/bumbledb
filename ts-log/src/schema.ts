/** Pure schema-evolution intent. Execution and filesystem work are in /migrations. */
export { backfill, convert, dropField, dropRelation, migrationIntent, renameField, renameRelation, seed } from "./migrations/intent.ts"
export type { MigrationIntent } from "./migrations/intent.ts"
