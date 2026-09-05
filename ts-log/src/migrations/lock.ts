/**
 * Repository exclusion for generation (C8): the named internal seam
 * `internalAcquireRepositoryLock` / `RepositoryLock`. The native stamped
 * lock is the exclusion authority — this module does not keep a second
 * in-memory ownership table. It never opens a core `Db`, never unlinks
 * the lock inode, and never composes `runtimeDirectoryAcquire` / `Close`.
 * Callers provide `Scope`. On cancel, L17 joins host I/O
 * (`joinPendingIo`) and then the L16 hook `RepositoryLock.release`
 * (`logRepositoryLockRelease`) runs from Scope. No public lock API.
 */
import type { ExecutionPolicy, NativeRuntime } from "@bjornpagen/bumbledb"
import { internalAcquireRepositoryLock } from "@bjornpagen/bumbledb/internal/log"
import type { RepositoryLock } from "@bjornpagen/bumbledb/internal/log"
import { Effect } from "effect"
import type { Scope } from "effect"
import type { LogError } from "#errors.ts"
import { logFailure } from "#errors.ts"

/** The held fence is the core `RepositoryLock` — directory + joined release. */
export type HeldRepositoryLock = RepositoryLock

export interface RepositoryExclusion {
	readonly acquire: (
		operation: string,
		directory: string,
		work: ExecutionPolicy
	) => Effect.Effect<RepositoryLock, LogError, NativeRuntime | Scope.Scope>
}

/**
 * Production exclusion: `internalAcquireRepositoryLock` only. Same-process
 * and cross-process refuse live on the native stamped lock. Scope registers
 * the joined close inside that acquire; do not open a `Db`.
 */
export const productionExclusion: RepositoryExclusion = {
	acquire(operation, directory, work) {
		return internalAcquireRepositoryLock(operation, directory, work).pipe(
			Effect.mapError((error) => logFailure(operation, error))
		)
	}
}
