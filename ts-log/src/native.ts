/**
 * The private log wire over the ONE exact-version native binding shared
 * with the core (C09). This file declares the log verbs the internal Rust
 * machine (`crates/bumbledb-log` behind `ts/crate`) exposes and re-types
 * the core's already-loaded binding — there is no second addon, no public
 * low-level handle, and no JS reimplementation of any protocol transition.
 * Every operation follows the C09 executor pattern: registration returns an
 * `OperationHandle` before any completion can run in JS; `runtimeCancel`
 * cancels and joins the native drain; take-functions throw the typed wire
 * error frame decoded by `#errors.ts`.
 *
 * The native implementations are owned by P06 (bridge)/P05 (backends)/P09
 * (migration execution); this declaration is the C10 log-addition roster
 * they implement, pinned by the authored roster test against
 * `logErrorCodes()`.
 */
import type { OperationHandle, RuntimeHandle, PolicyWire, CloseWire, Violation } from "@bjornpagen/bumbledb"
import { runtimeNative } from "@bjornpagen/bumbledb"

// ── Handles ────────────────────────────────────────────────────────────────

/** An owner history or an independently spent cache borrow. */
export interface HistoryCapability {
	readonly __logHistory: unique symbol
}
export interface LogSnapshotHandle {
	readonly __logSnapshot: unique symbol
}
/** The published core read transaction behind a log snapshot. */
export interface CoreSnapshotHandle {
	readonly __coreSnapshot: unique symbol
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
	| { readonly kind: "decided"; readonly receipt: ReceiptWire; readonly localHealth: HealthWire }
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
	readonly diskBytes: bigint
	readonly workingBytes: bigint
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
			readonly snapshot: LogSnapshotHandle
			readonly core: CoreSnapshotHandle
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
	readonly budgetBytes: bigint
	readonly expected: { readonly schemaId: string; readonly appliedPrefixDigest: string } | null
	/** The lowered core `SchemaSpec` shared by every slot of this cache. */
	readonly schema: unknown
}

export interface CacheInspectionWire {
	readonly openCount: number
	readonly opening: number
	readonly budgetBytes: bigint
	readonly maxOpen: number
	readonly evictions: bigint
	readonly slots: readonly {
		readonly binding: string
		readonly state: "opening" | "ready" | "closing" | "faulted"
		readonly borrows: number
		readonly diskBytes: bigint
	}[]
}

export interface ManifestEntryWire {
	readonly sequence: string
	readonly id: string
	readonly fromSchemaId: string
	readonly toSchemaId: string
	readonly planDigest: string
	readonly prefixDigest: string
}

export interface PlansWire {
	readonly manifestVersion: number
	readonly planVersion: number
	readonly baseSchemaId: string
	readonly basePrefixDigest: string
	readonly entries: readonly ManifestEntryWire[]
	/** Canonical inert plan JSON bodies, order-matched with `entries`. */
	readonly plans: readonly string[]
	/**
	 * Canonical schema snapshots (the native `schema_file::render` texts the
	 * generator wrote to `meta/`): the BASE schema first, then each entry's
	 * TARGET schema — exactly `entries.length + 1` rows, order-matched.
	 * Required by the verbs that must compile migration steps
	 * (migration-initialize/migration-migrate: digests alone cannot
	 * reconstruct descriptors); absent ⇒ a typed native not-started refusal.
	 */
	readonly snapshots?: readonly string[]
}

export interface ActivationRefWire {
	readonly operationId: string
	readonly planSetDigest: string
	readonly target: IdentityWire
	readonly targetGenesis: string
}

export interface MigrationRefWire {
	readonly identity: IdentityWire
	readonly operationId: string
	readonly planSetDigest: string
	readonly target: IdentityWire
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
	| {
			readonly verb: "migration-status"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly plans: PlansWire
	  }
	| {
			readonly verb: "migration-initialize"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly operationId: string
			readonly plans: PlansWire
	  }
	| {
			readonly verb: "migration-migrate"
			readonly binding: BindingWire
			readonly schema?: unknown
			readonly operationId: string
			readonly plans: PlansWire
			readonly to: string | null
	  }
	| {
			readonly verb: "migration-activate"
			readonly ref: ActivationRefWire
			/** The SOURCE binding: locates the stable `<dir>/targets` namespace. */
			readonly binding?: BindingWire
			/** The lowered core SchemaSpec of the TARGET. */
			readonly schema?: unknown
	  }
	| {
			readonly verb: "migration-abort"
			readonly ref: MigrationRefWire
			/** The SOURCE binding: locates the stable `<dir>/targets` namespace. */
			readonly binding?: BindingWire
			/** The lowered core SchemaSpec of the TARGET. */
			readonly schema?: unknown
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
	| { readonly verb: "restore"; readonly identity: IdentityWire; readonly genesis: string; readonly binding: BindingWire }
	| {
			readonly verb: "erase"
			readonly tombstoned: boolean
			readonly retainedRoots: readonly string[]
			readonly residual: readonly { readonly kind: string; readonly location: string }[]
	  }
	| { readonly verb: "migration-status"; readonly status: MigrationStatusWire }
	| { readonly verb: "migration-initialize"; readonly binding: BindingWire; readonly genesis: string }
	| { readonly verb: "migration-migrate"; readonly value: MigrateValueWire }
	| {
			readonly verb: "migration-activate"
			readonly target: IdentityWire
			readonly accessMode: "active" | "frozen" | "deleted"
			readonly operationId: string
			readonly activatedNow: boolean
	  }
	| {
			readonly verb: "migration-abort"
			readonly target: IdentityWire
			readonly targetFenced: boolean
			readonly sourceAccess: "active" | "frozen" | "deleted"
	  }

export type SourceAccessWire = {
	readonly access: "active" | "frozen" | "deleted"
	readonly operationId: string | null
}

export type MigrateValueWire =
	| { readonly kind: "up-to-date"; readonly binding: BindingWire }
	| {
			readonly kind: "ready-to-switch"
			readonly deploymentBinding: BindingWire
			readonly activation: ActivationRefWire
	  }
	| { readonly kind: "paused"; readonly error: ErrorWire; readonly sourceState: SourceAccessWire }

export type MigrationStatusWire =
	| { readonly kind: "up-to-date"; readonly appliedPrefixDigest: string }
	| { readonly kind: "pending"; readonly pending: readonly string[] }
	| { readonly kind: "in-progress"; readonly operationRef: MigrationRefWire }
	| {
			readonly kind: "paused"
			readonly operationRef: MigrationRefWire
			readonly error: ErrorWire
			readonly sourceState: SourceAccessWire
	  }
	| { readonly kind: "ready-to-switch"; readonly operationRef: MigrationRefWire; readonly activation: ActivationRefWire }
	| { readonly kind: "activated"; readonly operationRef: MigrationRefWire; readonly target: IdentityWire }
	| { readonly kind: "aborted"; readonly operationRef: MigrationRefWire }
	| { readonly kind: "outcome-unknown"; readonly operationRef: MigrationRefWire; readonly error: ErrorWire }
	| { readonly kind: "drift"; readonly detail: string }
	| { readonly kind: "database-ahead"; readonly detail: string }

export type AdminResultWire =
	| { readonly certainty: "completed"; readonly value: AdminValueWire }
	| { readonly certainty: "not-started"; readonly error: ErrorWire }
	| { readonly certainty: "outcome-unknown"; readonly error: ErrorWire }
	| { readonly certainty: "report"; readonly value: AdminValueWire }

// ── The verb roster ────────────────────────────────────────────────────────

export interface LogNative {
	/** Shared with the runtime surface; here so a wire double can supply it. */
	runtimeCancel(operation: OperationHandle, callback: (report: CloseWire) => void): void

	logErrorCodes(): readonly string[]

	logHistoryOpen(
		runtime: RuntimeHandle,
		policy: PolicyWire,
		request: HistoryOpenWire,
		callback: () => void
	): OperationHandle
	logHistoryTake(operation: OperationHandle): HistoryHandleWire
	logHistoryCall(
		history: HistoryCapability,
		policy: PolicyWire,
		request: HistoryRequestWire,
		callback: () => void
	): OperationHandle
	logHistoryResult(operation: OperationHandle): HistoryResultWire
	logHistoryClose(history: HistoryCapability, callback: (report: CloseWire) => void): void
	logSnapshotClose(snapshot: LogSnapshotHandle, callback: (report: CloseWire) => void): void

	/**
	 * Seals over the ALREADY-REGISTERED native change: the change handle is a
	 * registered resource of the one runtime registry, so the native side
	 * derives its runtime from the handle (chapter 35: seal "retains the
	 * change's captured runtime, never loads a second one"; R has no
	 * NativeRuntime).
	 */
	logCommandSeal(change: unknown, policy: PolicyWire, request: SealRequestWire, callback: () => void): OperationHandle
	logCommandDecode(
		runtime: RuntimeHandle,
		policy: PolicyWire,
		bytes: Uint8Array,
		schema: unknown,
		callback: () => void
	): OperationHandle
	logCommandTake(operation: OperationHandle): CommandWire
	logCommandEncode(command: CommandHandle, policy: PolicyWire, callback: () => void): OperationHandle
	logBytesTake(operation: OperationHandle): Uint8Array
	logCommandClose(command: CommandHandle, callback: (report: CloseWire) => void): void

	logCacheMake(
		runtime: RuntimeHandle,
		policy: PolicyWire,
		request: CacheMakeWire,
		callback: () => void
	): OperationHandle
	logCacheTake(operation: OperationHandle): CacheHandle
	logCacheAcquire(
		cache: CacheHandle,
		policy: PolicyWire,
		request: { readonly binding: BindingWire },
		callback: () => void
	): OperationHandle
	logBorrowTake(operation: OperationHandle): HistoryHandleWire
	logCacheInspect(cache: CacheHandle, policy: PolicyWire, callback: () => void): OperationHandle
	logCacheInspectTake(operation: OperationHandle): CacheInspectionWire
	logCacheEvict(
		cache: CacheHandle,
		policy: PolicyWire,
		request: { readonly binding: BindingWire },
		callback: () => void
	): OperationHandle
	logCacheEvictTake(operation: OperationHandle): CloseWire
	logBorrowRelease(borrow: HistoryCapability, callback: (report: CloseWire) => void): void
	logCacheClose(cache: CacheHandle, callback: (report: CloseWire) => void): void

	logAdmin(runtime: RuntimeHandle, policy: PolicyWire, request: AdminRequestWire, callback: () => void): OperationHandle
	logAdminTake(operation: OperationHandle): AdminResultWire
}

/**
 * The one binding, re-typed with the log roster exactly as the core's
 * `#runtime-native.ts` re-types it with the runtime roster. The authored
 * roster test pins this declaration against the addon's actual exports.
 */
export const logNative = runtimeNative as typeof runtimeNative & LogNative
