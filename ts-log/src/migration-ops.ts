/**
 * The migration workflow operations (chapter 33/35): read-only
 * `migrationStatus` with typed E, and mutating `initialize` / `migrate` /
 * `activateMigration` / `abortMigration` returning `AdminOutcome` certainty
 * with a stable operation reference supplied before dispatch. P10's
 * `@bjornpagen/bumbledb-log/migrations` module re-exports these wrappers
 * next to its generator; P09's native executor owns freeze, one-final-
 * target plan execution, validation, genesis publication, activation and
 * the abort fence. `completed` means the reported transition is KNOWN —
 * `migrate` may complete as `paused` with the source still frozen; that is
 * not permission to cut over, and no timer ever thaws a frozen source.
 */
import type { MigrationOperations } from "#machine.ts"
import { log } from "#production.ts"

export const migrationStatus: MigrationOperations["migrationStatus"] = log.migrations.migrationStatus
export const initialize: MigrationOperations["initialize"] = log.migrations.initialize
export const migrate: MigrationOperations["migrate"] = log.migrations.migrate
export const activateMigration: MigrationOperations["activateMigration"] = log.migrations.activateMigration
export const abortMigration: MigrationOperations["abortMigration"] = log.migrations.abortMigration

export type { MigrationOperations, MigrationPlansInput, MigrationTargetOptions } from "#machine.ts"
