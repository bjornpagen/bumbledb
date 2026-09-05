/**
 * One native-backed typed tenant registry. `TenantCache.make(schema,
 * options)` acquires the scoped cache; `acquire(binding, work)` returns a
 * DISTINCT scoped `HistoryBorrow<S>` whose release frees only that borrow —
 * never the shared owner. The native registry owns slots, kernel directory
 * locks, opening-work accounting, byte/count pressure and joined close.
 * There is deliberately no renewable wall-clock TTL, pre-lock cleanup,
 * `_shared` magic tenant, JS LRU, timer, or second cache authority here:
 * the successor deleted them. Different tenant schemas use separately
 * constructed typed caches, not casts through one untyped cache.
 */
import type { Effect, Scope } from "effect"
import type { AnySchema, NativeRuntime } from "@bjornpagen/bumbledb"
import type { LogError } from "#errors.ts"
import type { TenantCacheOptions } from "#options.ts"
import { log } from "#production.ts"
import type { TenantCache as TenantCacheInterface } from "#surface.ts"

/** The scoped typed cache value an app service can own with Layer.effect. */
export type TenantCache<S extends AnySchema> = TenantCacheInterface<S>

export const TenantCache: {
	make<S extends AnySchema>(
		schema: S,
		options: TenantCacheOptions
	): Effect.Effect<TenantCache<S>, LogError, NativeRuntime | Scope.Scope>
} = log.TenantCache

export type { HistoryBorrow } from "#surface.ts"
export type { TenantCacheOptions } from "#options.ts"
