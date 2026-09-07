/**
 * `@bjornpagen/bumbledb-log/migrations` — generation, checking and the
 * explicit admin workflow over generated inert plan data. Pure intent
 * constructors live in `@bjornpagen/bumbledb-log/schema`.
 *
 * The runner operations (`migrationStatus`, `initialize`, `migrate`,
 * `activateMigration`, `abortMigration`) are re-exported from
 * `#migration-ops.ts`; their outcomes come from `#outcome.ts`. Generation
 * and checking use the same native migration codec as execution.
 */
export type { AdminIdentityOptions } from "#machine.ts"
export { abortMigration, activateMigration, initialize, migrate, migrationStatus } from "#migration-ops.ts"
export type { DecodeResult } from "#migrations/decode.ts"
export {
	decodeActivationRef,
	decodeGeneratedMigrations,
	decodeManifestData,
	decodePlanData,
	decodeReadyToSwitchActivation,
	decodeRuntimeContract
} from "#migrations/decode.ts"
export type { MigrationIntent, MigrationIntentEntry } from "#migrations/intent.ts"
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
export type { HeldRepositoryLock, RepositoryExclusion } from "#migrations/lock.ts"
export type { TheoryResult } from "#migrations/theory.ts"
export { EMPTY_THEORY, parseTheory } from "#migrations/theory.ts"
export type {
	ActivationRef,
	CheckOptions,
	CheckReport,
	GeneratedMigrations,
	GenerateOptions,
	GenerationReport,
	ManifestEntry,
	MigrationManifest,
	MigrationPlan,
	MigrationRepository,
	PlanExpression,
	PlanFieldMap,
	PlanLoss,
	PlanOperation,
	PlanValue,
	RuntimeContract,
	TheoryField,
	TheoryRelation,
	TheorySnapshot
} from "#migrations/types.ts"
export { checkMigrations, generateMigrations } from "#migrations/workflow.ts"
export type {
	AbortReport,
	ActivationReport,
	AdminOutcome,
	InitializeValue,
	MigrateValue,
	MigrationRef,
	MigrationStatus
} from "#outcome.ts"
