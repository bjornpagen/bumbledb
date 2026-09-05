/** Exact-version internal bridge, shared by core and log. Not a public API. */
import { native } from "#native.ts"
import type {
	DbHandle,
	LogChainKind,
	LogCommandKind,
	LogCommandMetadata,
	LogCommandRef,
	LogCondition,
	LogDecisionStamp,
	LogIdentity,
	LogLimits,
	LogOutcomeWire,
	LogSchemaHandle,
	LogStateStamp,
	OwnedHandle,
	Violation
} from "#native.ts"
import type { SchemaSpec } from "#spec.ts"

export interface RuntimeHandle {
	readonly __runtime: unique symbol
}
export interface OperationHandle {
	readonly __operation: unique symbol
}
export interface DirectoryHandle {
	readonly __directory: unique symbol
}

/** Stamped `NativeKind::RepositoryLock` mint — not a directory-owner twin. */
export interface RepositoryLockHandle {
	readonly __repositoryLock: unique symbol
}

/** F0 C7 kind roster for worker-table resources. */
export type NativeKind = "snapshot" | "result" | "cursor" | "draft" | "changes" | "repository-lock"

/** Worker-routed capability header (C7). Never holds payload bytes. */
export interface ResourceHeader {
	readonly runtime: bigint
	readonly worker: number
	readonly kind: NativeKind
	readonly id: bigint
	readonly generation: bigint
}

/**
 * Checked capability into the native registry (C7). JS tokens validate
 * these five fields; native re-judges kind/generation/owner on every verb.
 */
export interface Capability {
	readonly runtime: bigint
	readonly worker: number
	readonly kind: NativeKind
	readonly id: bigint
	readonly generation: bigint
}

/** Coalesced close obligation — already-owned drain, not rejectable work. */
export interface CloseDrain {
	readonly header: ResourceHeader
}

export type ManagedDbOutcome =
	| { readonly tag: "accepted"; readonly db: DbHandle }
	| { readonly tag: "rejected"; readonly violations: readonly Violation[] }
	| {
			readonly tag: "refused"
			readonly kind: "schemaError" | "newtypeMismatch" | "fingerprintMismatch" | "destinationExists"
			readonly message: string
	  }

// The 0.x five-verb JS filesystem transport (`runtimeFs`/`runtimeFsTake`,
// FsRequestWire/FsResultWire) is DELETED with the TS CAS authority: all
// object-store work is native (C07, P05's store rewrite); the log machine
// drives FsStore/S3Store inside `ts/crate` and no JS layer holds a
// conditional-store verb anymore.

export type LogTakeWire =
	| { readonly ok: false; readonly kind: LogCommandKind | LogChainKind; readonly message: string }
	| {
			/** Command sealed: envelope bytes + reference. */
			readonly ok: true
			readonly bytes: Uint8Array
			readonly ref: LogCommandRef
	  }
	| {
			/** Command parsed. */
			readonly ok: true
			readonly identity: LogIdentity
			readonly receiptEpoch: bigint
			readonly requestId: string
			readonly condition: LogCondition
			readonly changes: Uint8Array
			readonly result: Uint8Array
			readonly ref: LogCommandRef
	  }
	| {
			/** Decision framed. */
			readonly ok: true
			readonly bytes: Uint8Array
			readonly digest: Uint8Array
	  }
	| {
			/** Decision decoded (and chain-verified when a parent was given). */
			readonly ok: true
			readonly identity: LogIdentity
			readonly seq: bigint
			readonly parent: LogDecisionStamp
			readonly beforeState: LogStateStamp
			readonly afterState: LogStateStamp
			readonly commandBytes: Uint8Array
			readonly command: LogCommandRef
			readonly outcome: LogOutcomeWire
			readonly digest: Uint8Array
	  }

export interface LogDecisionParts {
	readonly identity: LogIdentity
	readonly seq: bigint
	readonly parent: LogDecisionStamp
	readonly beforeState: LogStateStamp
	readonly afterState: LogStateStamp
	readonly outcome: LogOutcomeWire
}

export interface PolicyWire {
	readonly inputBytes: bigint
	readonly workingBytes: bigint
	readonly scratchBytes: bigint
	readonly resultBytes: bigint
	readonly rows: bigint
	readonly workUnits: bigint
	readonly timeoutMs: number
}

export interface OptionsWire {
	readonly workers: number
	readonly queueCapacity: number
	readonly cleanupCapacity: number
	readonly ownerCapacity: number
	readonly nativeHandleCapacity: number
	readonly inputBytes: bigint
	readonly workingBytes: bigint
	readonly scratchBytes: bigint
	readonly resultBytes: bigint
	readonly chunkBytes: bigint
	readonly cleanupTimeoutMs: number
}

export interface InspectionWire {
	readonly phase: "open" | "closing" | "closed"
	readonly queued: bigint
	readonly active: bigint
	readonly retained: bigint
	readonly owners: bigint
	readonly databases: bigint
	readonly natives: bigint
	readonly inputBytes: bigint
	readonly workingBytes: bigint
	readonly scratchBytes: bigint
	readonly resultBytes: bigint
}

export type CloseWire =
	| { readonly kind: "closed" }
	| { readonly kind: "incomplete"; readonly outstanding: InspectionWire }
	| { readonly kind: "failed" }

interface RuntimeNative {
	runtimeErrorCodes(): readonly string[]
	runtimeOpen(options: OptionsWire): RuntimeHandle
	runtimeReady(runtime: RuntimeHandle, policy: PolicyWire, callback: () => void): OperationHandle
	runtimeHash(runtime: RuntimeHandle, policy: PolicyWire, bytes: Uint8Array, callback: () => void): OperationHandle
	runtimeTake(operation: OperationHandle): Uint8Array | null
	runtimeCancel(operation: OperationHandle, callback: (report: CloseWire) => void): void
	runtimeClose(runtime: RuntimeHandle, callback: (report: CloseWire) => void): void
	runtimeInspect(runtime: RuntimeHandle): InspectionWire
	/**
	 * L12 request (D12/D25): one-shot arm. Next `dispatch_payload_message`
	 * must cancel after `work()` returns and the post-work `checkpoint()`
	 * (`runtime.rs` ~860–862), before `complete_operation` writes
	 * `operation.output`. A local `QueuedOutput` is not registration.
	 * Predelivery: no JS take; cursor stays on row1 so retry does not skip.
	 */
	runtimeArmPublicationCancel(runtime: RuntimeHandle): void
	runtimeDirectoryAcquire(runtime: RuntimeHandle, policy: PolicyWire, path: string, callback: () => void): OperationHandle
	runtimeDirectoryTake(operation: OperationHandle): DirectoryHandle
	runtimeDirectoryBegin(owner: DirectoryHandle, policy: PolicyWire): OperationHandle
	runtimeDirectoryCheck(operation: OperationHandle): void
	runtimeDirectoryEnd(operation: OperationHandle): void
	runtimeDirectoryClose(owner: DirectoryHandle, remove: boolean, callback: (report: CloseWire) => void): void
	runtimeDirectoryDbOpen(owner: DirectoryHandle, policy: PolicyWire, childName: string, spec: SchemaSpec, create: boolean, callback: () => void): OperationHandle
	runtimeDbTake(operation: OperationHandle): ManagedDbOutcome
	runtimeManagedDbClose(db: DbHandle, callback: (report: CloseWire) => void): void
	/** Managed publish: attach an admitted OwnedInstance to the directory owner. Take with `runtimeDbTake`. */
	runtimeDirectoryPublish(owner: DirectoryHandle, policy: PolicyWire, childName: string, instance: OwnedHandle, callback: () => void): OperationHandle

	// --- successor log grammar on the executor (charged hashing over
	// whole canonical change payloads; C06 through C09) ---
	runtimeLogCommandSeal(runtime: RuntimeHandle, schema: LogSchemaHandle, policy: PolicyWire, metadata: LogCommandMetadata, changes: Uint8Array, result: Uint8Array | null, limits: LogLimits, callback: () => void): OperationHandle
	runtimeLogCommandParse(runtime: RuntimeHandle, schema: LogSchemaHandle, policy: PolicyWire, bytes: Uint8Array, limits: LogLimits, callback: () => void): OperationHandle
	runtimeLogDecisionEncode(runtime: RuntimeHandle, policy: PolicyWire, parts: LogDecisionParts, commandBytes: Uint8Array, limits: LogLimits, callback: () => void): OperationHandle
	runtimeLogDecisionDecode(runtime: RuntimeHandle, policy: PolicyWire, bytes: Uint8Array, parent: LogDecisionStamp | null, limits: LogLimits, callback: () => void): OperationHandle
	runtimeLogTake(operation: OperationHandle): LogTakeWire
	/** L14 mint: `Runtime::mint_repository_lock` stamps `NativeKind::RepositoryLock` at take. */
	logRepositoryLockAcquire(runtime: RuntimeHandle, policy: PolicyWire, directory: string, callback: () => void): OperationHandle
	logRepositoryLockTake(operation: OperationHandle): RepositoryLockHandle
	logRepositoryLockRelease(owner: RepositoryLockHandle, callback: (report: CloseWire) => void): void
}

// The checked source/fresh-addon roster test pins this private declaration.
export const runtimeNative = native as typeof native & RuntimeNative
