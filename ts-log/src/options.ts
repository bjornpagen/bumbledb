/**
 * Caller-supplied bindings and option records. Bindings are discriminated
 * local/hosted data — neither history constructor accepts the other
 * backend's fields — and are trusted host configuration, not tenant-label
 * authority. Credentials are configuration handed to the one native
 * machine's supported provider path; they never construct a second JS
 * S3 client, cache, or protocol machine. Cancellation uses the caller's
 * Effect scope, not an execution-policy record.
 */
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

export interface HostedOpenOptions {
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
 * schema snapshot emitted by `schemaSnapshot`; initial facts use ordinary
 * commands after creation.
 */
export interface CreationOptions {
	readonly operationId: OperationId
	readonly artifact: Uint8Array
}

export interface LocalCreateOptions {
	readonly creation: CreationOptions
}

export interface HostedCreateOptions extends HostedOpenOptions {
	readonly creation: CreationOptions
}

/** The requested read consistency. */
export interface ReadOptions {
	readonly consistency: ReadConsistency
}

/**
 * A finite native publication-attempt limit and backoff bounds. The native
 * protocol owns catch-up/CAS retries within this attempt count; interruption
 * stops them through the shared cancellation context.
 */
export interface SubmitOptions {
	readonly attempts: number
	readonly backoff: {
		readonly baseMillis: number
		readonly capMillis: number
	}
}

/**
 * One bounded native tenant registry configuration. There is no wall-clock
 * TTL, renewal, or pre-lock cleanup. The open-tenant count provides
 * admission backpressure; eviction of a borrowed slot refuses.
 */
export interface TenantCacheOptions {
	readonly maxOpen: number
}
