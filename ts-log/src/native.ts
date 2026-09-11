/**
 * The private log wire over the ONE exact-version native binding shared
 * with the core. This file declares the log verbs the internal Rust
 * machine (`crates/bumbledb-log` behind `ts/crate`) exposes and re-types
 * the core's already-loaded binding — there is no second addon, no public
 * low-level handle, and no JS reimplementation of any protocol transition.
 * Every operation follows the shared executor protocol: registration returns an
 * `OperationHandle` before any completion can run in JS; `runtimeCancel`
 * cancels and joins the native drain; take-functions throw the typed wire
 * error frame decoded by `#errors.ts`.
 *
 * The error-code roster is checked against the native `logErrorCodes()`.
 */
import type { StorageInspection, Violation } from "@bjornpagen/bumbledb"
import type { CloseWire, OperationHandle, RuntimeHandle, SnapshotHandle } from "@bjornpagen/bumbledb/internal/log"
import { runtimeNative } from "@bjornpagen/bumbledb/internal/log"

// ── Handles ────────────────────────────────────────────────────────────────

/** An owner history or an independently spent cache borrow. */
export interface HistoryCapability {
	readonly __logHistory: unique symbol
}
export interface CommandHandle {
	readonly __logCommand: unique symbol
}
export interface CacheHandle {
	readonly __logCache: unique symbol
}

// ── Plain wire data ────────────────────────────────────────────────────────

export interface IdentityWire {
	readonly databaseId: string
	readonly incarnationId: string
	readonly schemaId: string
}

export interface StampWire {
	readonly seq: bigint
	readonly hash: string
}

export interface StateWire {
	readonly incarnation: string
	readonly dataRevision: bigint
}

export type FreshnessWire =
	| { readonly kind: "cached" }
	| { readonly kind: "latest" }
	| { readonly kind: "at-least"; readonly requested: StampWire }

export interface ProvenanceWire {
	readonly identity: IdentityWire
	readonly decision: StampWire
	readonly state: StateWire
	readonly freshness: FreshnessWire
}

export type CredentialsWire =
	| { readonly kind: "provider-chain" }
	| {
			readonly kind: "static"
			readonly accessKeyId: string
			readonly secretAccessKey: string
			readonly sessionToken: string | null
	  }

export type BindingWire =
	| { readonly kind: "local"; readonly directory: string; readonly identity: IdentityWire }
	| {
			readonly kind: "hosted"
			readonly directory: string
			readonly bucket: string
			readonly prefix: string
			readonly region: string | null
			readonly identity: IdentityWire
			readonly credentials: CredentialsWire
	  }

export type DestinationWire =
	| { readonly kind: "filesystem"; readonly directory: string }
	| {
			readonly kind: "s3"
			readonly bucket: string
			readonly prefix: string
			readonly region: string | null
			readonly credentials: CredentialsWire
	  }

export type ConsistencyWire =
	| { readonly kind: "cached" }
	| { readonly kind: "latest" }
	| { readonly kind: "at-least"; readonly seq: bigint; readonly hash: string }

export interface CommandRefWire {
	readonly identity: IdentityWire
	readonly receiptEpoch: bigint
	readonly requestId: string
	readonly digest: string
}

export type ResultWire = Readonly<Record<string, bigint | number | string | boolean | Uint8Array>>

export type PreconditionWire =
	| { readonly kind: "blind" }
	| { readonly kind: "exact-state"; readonly incarnation: string; readonly dataRevision: bigint }

/** The typed error frame thrown by take-functions; decoded in #errors.ts. */
export interface ErrorWire {
	readonly source: "core" | "protocol"
	readonly reason: unknown
}

export type OutcomeWire =
	| { readonly kind: "committed"; readonly added: bigint; readonly removed: bigint; readonly result: ResultWire }
	| { readonly kind: "no-change"; readonly result: ResultWire }
	| { readonly kind: "precondition-failed"; readonly expected: StateWire; readonly observed: StateWire }
	| { readonly kind: "invariant-rejected"; readonly violations: readonly Violation[] }

export interface ReceiptWire {
	readonly command: CommandRefWire
	readonly decisionAt: StampWire
	readonly stateAt: StateWire
	readonly outcome: OutcomeWire
}

export type HealthWire =
	| { readonly kind: "ready"; readonly at: StampWire }
	| { readonly kind: "unavailable"; readonly error: ErrorWire }

export type SubmitWire =
	| {
			readonly kind: "decided"
			readonly receipt: ReceiptWire
			readonly localHealth: HealthWire
	  }
	| { readonly kind: "not-submitted"; readonly error: ErrorWire }
	| { readonly kind: "outcome-unknown"; readonly error: ErrorWire }

export type ResolveWire =
	| { readonly kind: "found"; readonly receipt: ReceiptWire }
	| { readonly kind: "not-recorded-at"; readonly decisionAt: StampWire }
	| { readonly kind: "command-epoch-closed" }
	| { readonly kind: "receipt-expired-unknown" }

export interface HistoryInspectionWire {
	readonly identity: IdentityWire
	readonly accessMode: "active" | "frozen" | "deleted"
	readonly headRevision: bigint
	readonly decision: StampWire
	readonly state: StateWire
	readonly openEpoch: bigint
	readonly retiredThrough: bigint
	readonly tailCount: bigint
	readonly tailBytes: bigint
	readonly unknownCount: bigint
	readonly unknownOldestMillis: number | null
	readonly rootCount: number
	readonly rootCapacity: number
	readonly gc: "idle" | "marking" | "sweeping"
	readonly lastMaintenanceError: string | null
	readonly storage: StorageInspection
	readonly queued: bigint
	readonly active: bigint
}

export interface HistoryOpenWire {
	readonly mode: "open" | "create"
	readonly binding: BindingWire
	/** The lowered core `SchemaSpec` — the same value `Db.open` admits. */
	readonly schema: unknown
	readonly discardMismatchedCache: boolean
	readonly creation: { readonly operationId: string; readonly artifact: Uint8Array } | null
}

export interface HistoryMetaWire {
	readonly identity: IdentityWire
	readonly receiptEpoch: bigint
}

export interface HistoryHandleWire {
	readonly history: HistoryCapability
	readonly meta: HistoryMetaWire
}

export type HistoryRequestWire =
	| {
			readonly verb: "submit"
			readonly command: CommandHandle
			readonly attempts: number
			readonly backoffBaseMillis: number
			readonly backoffCapMillis: number
	  }
	| { readonly verb: "resolve"; readonly ref: CommandRefWire }
	| { readonly verb: "inspect" }
	| { readonly verb: "snapshot"; readonly consistency: ConsistencyWire }

export type HistoryResultWire =
	| { readonly verb: "submit"; readonly outcome: SubmitWire }
	| { readonly verb: "resolve"; readonly outcome: ResolveWire }
	| { readonly verb: "inspect"; readonly inspection: HistoryInspectionWire }
	| {
			readonly verb: "snapshot"
			readonly snapshot: SnapshotHandle
			readonly provenance: ProvenanceWire
	  }

export interface SealRequestWire {
	readonly scope: IdentityWire
	readonly receiptEpoch: bigint
	readonly requestId: string
	readonly precondition: PreconditionWire
	readonly result: ResultWire
}

export interface CommandWire {
	readonly command: CommandHandle
	readonly ref: CommandRefWire
}

export interface CacheMakeWire {
	readonly maxOpen: number
	/** The lowered core `SchemaSpec` shared by every slot of this cache. */
	readonly schema: unknown
}

export interface CacheInspectionWire {
	readonly openCount: number
	readonly opening: number
	readonly maxOpen: number
	readonly evictions: bigint
	readonly slots: readonly {
		readonly binding: string
		readonly state: "opening" | "ready" | "closing" | "faulted"
		readonly borrows: number
	}[]
}

/**
 * Every binding-carrying arm additionally accepts `schema?: unknown` — the
 * lowered core `SchemaSpec` (the same value history open sends). The native
 * side needs it whenever the verb must open the local materialization and
 * the tenant is not already open in the runtime's registry; absent ⇒ a typed
 * native not-started refusal (`Misuse`), never a fabricated open. `backup`
 * on restore/verify-backup is the BACKUP operation id (manifests are
 * operation-scoped at `<dest>/backup/<op>/manifest`); absent ⇒ the same
 * typed refusal. All of these are checked caller data, not authority.
 */
export type AdminRequestWire =
	| {
			readonly verb: "checkpoint"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly operationId: string
	  }
	| {
			readonly verb: "pin-root"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly operationId: string
			readonly label: string
	  }
	| {
			readonly verb: "release-root"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly operationId: string
			readonly root: string
	  }
	| {
			readonly verb: "rotate-receipt-epoch"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly operationId: string
	  }
	| {
			readonly verb: "retire-receipts"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly operationId: string
			readonly through: bigint
	  }
	| {
			readonly verb: "collect-garbage"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly operationId: string
	  }
	| {
			readonly verb: "backup"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly operationId: string
			readonly destination: DestinationWire
	  }
	| { readonly verb: "verify-backup"; readonly destination: DestinationWire; readonly backup?: string }
	| {
			readonly verb: "restore"
			readonly source: DestinationWire
			readonly target: BindingWire
			/** The lowered core SchemaSpec of the restore TARGET. */
			readonly schema?: unknown
			readonly operationId: string
			readonly backup?: string
	  }
	| {
			readonly verb: "erase"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly operationId: string
			readonly retainRoots: readonly string[]
	  }

export type AdminValueWire =
	| { readonly verb: "checkpoint"; readonly at: StampWire; readonly state: StateWire; readonly root: string }
	| { readonly verb: "pin-root"; readonly root: string; readonly at: StampWire; readonly state: StateWire }
	| { readonly verb: "release-root"; readonly root: string; readonly wasCurrentRecoveryBase: boolean }
	| { readonly verb: "rotate-receipt-epoch"; readonly openEpoch: bigint }
	| { readonly verb: "retire-receipts"; readonly retiredThrough: bigint }
	| {
			readonly verb: "collect-garbage"
			readonly objectEpoch: bigint
			readonly swept: bigint
			readonly orphansObserved: bigint
	  }
	| {
			readonly verb: "backup"
			readonly manifestDigest: string
			readonly objects: bigint
			readonly bytes: bigint
			readonly at: StampWire
	  }
	| {
			readonly verb: "verify-backup"
			readonly identity: IdentityWire
			readonly at: StampWire
			readonly state: StateWire
			readonly objects: bigint
			readonly bytes: bigint
			readonly manifestDigest: string
	  }
	| {
			readonly verb: "restore"
			readonly identity: IdentityWire
			readonly genesis: string
			readonly binding: BindingWire
	  }
	| {
			readonly verb: "erase"
			readonly tombstoned: boolean
			readonly retainedRoots: readonly string[]
			readonly residual: readonly { readonly kind: string; readonly location: string }[]
	  }

export type AdminResultWire =
	| { readonly certainty: "completed"; readonly value: AdminValueWire }
	| { readonly certainty: "not-started"; readonly error: ErrorWire }
	| {
			readonly certainty: "outcome-unknown"
			readonly error: ErrorWire
	  }
	| { readonly certainty: "report"; readonly value: AdminValueWire }

export interface PopulationHandle {
	readonly __population: unique symbol
}
export interface TransitionContractWire {
	readonly operationId: string
	readonly source: IdentityWire
	readonly target: IdentityWire
	readonly commitment: string
}
export interface TransitionCaptureWire {
	readonly contract: TransitionContractWire
	readonly decision: StampWire
	readonly state: StateWire
}
export interface InstalledTransitionWire {
	readonly captured: TransitionCaptureWire
	readonly applicationDigest: string
	readonly bytes: Uint8Array
}
export interface TransitionSnapshotWire {
	readonly snapshot: SnapshotHandle
	readonly provenance: ProvenanceWire
}
export type TransitionRequestWire =
	| {
			readonly verb: "begin" | "resolve" | "abort"
			readonly contract: TransitionContractWire
			readonly schema: unknown
	  }
	| { readonly verb: "activate"; readonly evidence: Uint8Array; readonly schema: unknown }
	| { readonly verb: "inspect"; readonly evidence: Uint8Array }
export type TransitionResultWire =
	| {
			readonly kind: "populating"
			readonly population: PopulationHandle
			readonly captured: TransitionCaptureWire
			readonly source: TransitionSnapshotWire
			readonly directory: string
	  }
	| {
			readonly kind: "ready"
			readonly installed: InstalledTransitionWire
			readonly source?: TransitionSnapshotWire
			readonly directory: string
	  }
	| {
			readonly kind: "activated"
			readonly contract: TransitionContractWire
			readonly genesis: string
			readonly directory: string
	  }
	| { readonly kind: "uninstalled" | "aborted" | "applied" }

// ── The verb roster ────────────────────────────────────────────────────────

export interface LogNative {
	logTransitionCall(history: HistoryCapability, request: TransitionRequestWire, callback: () => void): OperationHandle
	logTransitionResult(operation: OperationHandle): TransitionResultWire
	logPopulationApply(population: PopulationHandle, changes: unknown, callback: () => void): OperationHandle
	logPopulationFinish(population: PopulationHandle, callback: () => void): OperationHandle
	logPopulationClose(population: PopulationHandle, callback: (report: CloseWire) => void): void

	/** Shared with the runtime surface; here so a wire double can supply it. */
	runtimeCancel(operation: OperationHandle, callback: (report: CloseWire) => void): void

	logErrorCodes(): readonly string[]

	logHistoryOpen(runtime: RuntimeHandle, request: HistoryOpenWire, callback: () => void): OperationHandle
	logHistoryTake(operation: OperationHandle): HistoryHandleWire
	logHistoryCall(history: HistoryCapability, request: HistoryRequestWire, callback: () => void): OperationHandle
	logHistoryResult(operation: OperationHandle): HistoryResultWire
	logHistoryClose(history: HistoryCapability, callback: (report: CloseWire) => void): void
	runtimeSnapshotClose(snapshot: SnapshotHandle, callback: (report: CloseWire) => void): void

	/**
	 * Seals over the ALREADY-REGISTERED native change: the change handle is a
	 * registered resource of the one runtime registry, so the native side
	 * derives its runtime from the handle (seal "retains the
	 * change's captured runtime, never loads a second one"; R has no
	 * NativeRuntime).
	 */
	logCommandSeal(change: unknown, request: SealRequestWire, callback: () => void): OperationHandle
	logCommandDecode(runtime: RuntimeHandle, bytes: Uint8Array, schema: unknown, callback: () => void): OperationHandle
	logCommandTake(operation: OperationHandle): CommandWire
	logCommandEncode(command: CommandHandle, callback: () => void): OperationHandle
	logBytesTake(operation: OperationHandle): Uint8Array
	logCommandClose(command: CommandHandle, callback: (report: CloseWire) => void): void

	logCacheMake(runtime: RuntimeHandle, request: CacheMakeWire, callback: () => void): OperationHandle
	logCacheTake(operation: OperationHandle): CacheHandle
	logCacheAcquire(cache: CacheHandle, request: { readonly binding: BindingWire }, callback: () => void): OperationHandle
	logBorrowTake(operation: OperationHandle): HistoryHandleWire
	logCacheInspect(cache: CacheHandle, callback: () => void): OperationHandle
	logCacheInspectTake(operation: OperationHandle): CacheInspectionWire
	logCacheEvict(cache: CacheHandle, request: { readonly binding: BindingWire }, callback: () => void): OperationHandle
	logCacheEvictTake(operation: OperationHandle): CloseWire
	logBorrowRelease(borrow: HistoryCapability, callback: (report: CloseWire) => void): void
	logCacheClose(cache: CacheHandle, callback: (report: CloseWire) => void): void

	logAdmin(runtime: RuntimeHandle, request: AdminRequestWire, callback: () => void): OperationHandle
	logAdminTake(operation: OperationHandle): AdminResultWire
}

/**
 * The one binding, re-typed with the log roster exactly as the core's
 * `#runtime-native.ts` re-types it with the runtime roster. The authored
 * roster test pins this declaration against the addon's actual exports.
 */
export const logNative = runtimeNative as typeof runtimeNative & LogNative
