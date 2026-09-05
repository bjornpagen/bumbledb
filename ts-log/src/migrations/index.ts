/**
 * `@bjornpagen/bumbledb-log/migrations` — generation, checking and the
 * explicit admin workflow over generated inert plan data (chapters 33/35,
 * C11). Pure intent constructors live in `@bjornpagen/bumbledb-log/schema`.
 *
 * The runner operations (`migrationStatus`, `initialize`, `migrate`,
 * `activateMigration`, `abortMigration`) are P08's wrappers over the ONE
 * native executor and durable workflow (P09) — re-exported here, never
 * reimplemented; their outcome vocabulary is P08's `#outcome.ts`. The
 * generator (`generateMigrations`, `checkMigrations`) is this packet's, bound
 * once over the native migration codec.
 */
export type { AdminIdentityOptions } from "#machine.ts"
export { abortMigration, activateMigration, initialize, migrate, migrationStatus } from "#migration-ops.ts"
export type {
	AbortReport,
	ActivationReport,
	AdminOutcome,
	InitializeValue,
	MigrateValue,
	MigrationRef,
	MigrationStatus
} from "#outcome.ts"
export type { CompiledChainInput } from "#migrations/codec.ts"
export type { HeldRepositoryLock, RepositoryExclusion } from "#migrations/lock.ts"
export { checkMigrations, generateMigrations } from "#migrations/workflow.ts"
export {
	decodeActivationRef,
	decodeGeneratedMigrations,
	decodeManifestData,
	decodePlanData,
	decodeReadyToSwitchActivation,
	decodeRuntimeContract
} from "#migrations/decode.ts"
export type { DecodeResult } from "#migrations/decode.ts"
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
export { EMPTY_THEORY, parseTheory } from "#migrations/theory.ts"
export type { TheoryResult } from "#migrations/theory.ts"
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
