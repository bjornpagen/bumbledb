/**
 * Caller-supplied bindings and option records. Bindings are discriminated
 * local/hosted data — neither history constructor accepts the other
 * backend's fields — and are trusted host configuration, not tenant-label
 * authority. Credentials are configuration handed to the one native
 * machine's supported provider path; they never construct a second JS
 * S3 client, cache, or protocol machine. All options extend the core
 * `ExecutionPolicy`; there is no second cancellation channel, TTL, or
 * Effect `Schedule` slot anywhere in this file.
 */
import type { ExecutionPolicy } from "@bjornpagen/bumbledb"
import type { DatabaseIdentity, OperationId, ReadConsistency } from "#identity.ts"

/** Rust-side supported credential resolution; no static default. */
export type HostedCredentials =
	| { readonly kind: "provider-chain" }
	| {
			readonly kind: "static"
			readonly accessKeyId: string
			readonly secretAccessKey: string
			readonly sessionToken?: string
	  }

export interface HostedOrigin {
	readonly bucket: string
	readonly prefix: string
	readonly region?: string
}

/** One durable local LMDB history: the directory IS the database. */
export interface LocalBinding {
	readonly kind: "local"
	readonly directory: string
	readonly identity: DatabaseIdentity
}

/** S3 HEAD authority plus a disposable local materialization directory. */
export interface HostedBinding {
	readonly kind: "hosted"
	readonly origin: HostedOrigin
	readonly directory: string
	readonly identity: DatabaseIdentity
	readonly credentials?: HostedCredentials
}

export type HistoryBinding = LocalBinding | HostedBinding

export interface LocalOpenOptions extends ExecutionPolicy {}

export interface HostedOpenOptions extends ExecutionPolicy {
	/**
	 * Explicit policy for a cache whose verified binding mismatches: close/
	 * quarantine and rebuild in a newly owned location. It never submits the
	 * old cache's pending commands or deletes remote objects.
	 */
	readonly discardMismatchedCache?: boolean
}

/**
 * Creation is explicit and validated: a retry after uncertain creation
 * validates this stable identity and completes genesis instead of adopting
 * an unrelated database. The artifact is the checked canonical
 * initialization data (ordinarily produced by the generated-plan
 * `initialize` operation, C11), never fabricated migration history.
 */
export interface CreationOptions {
	readonly operationId: OperationId
	readonly artifact: Uint8Array
}

export interface LocalCreateOptions extends LocalOpenOptions {
	readonly creation: CreationOptions
}

export interface HostedCreateOptions extends HostedOpenOptions {
	readonly creation: CreationOptions
}

/** Chapter 30's consistency sum over the core policy — nothing else. */
export interface ReadOptions extends ExecutionPolicy {
	readonly consistency: ReadConsistency
}

/**
 * A finite native publication-attempt limit and backoff bounds. The native
 * protocol owns catch-up/CAS retries; retries consume this one operation
 * budget, not a fresh deadline per attempt.
 */
export interface SubmitOptions extends ExecutionPolicy {
	readonly attempts: number
	readonly backoff: {
		readonly baseMillis: number
		readonly capMillis: number
	}
}

/**
 * The generated runtime expectation (chapter 33 `runtime-contract.json`):
 * exact canonical schema and applied migration-prefix digests. Shape is
 * C11-provisional until P09/P10 freeze the generated contract.
 */
export interface RuntimeExpectation {
	readonly schemaId: string
	readonly appliedPrefixDigest: string
}

/**
 * One bounded native tenant registry configuration. There is no wall-clock
 * TTL, renewal, or pre-lock cleanup: pressure is byte/count budgets, and
 * eviction of a borrowed slot refuses.
 */
export interface TenantCacheOptions {
	readonly maxOpen: number
	readonly budgetBytes: bigint
	/** Charged against maintenance work (inspect/evict), not acquires. */
	readonly maintenance: ExecutionPolicy
	readonly expected?: RuntimeExpectation
}
