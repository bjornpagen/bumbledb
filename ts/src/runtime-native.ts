/** Exact-version internal bridge, shared by core and log. Not a public API. */
import { native } from "#native.ts"
import type { DbHandle, Violation } from "#native.ts"
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

export type ManagedDbOutcome =
	| { readonly tag: "accepted"; readonly db: DbHandle }
	| { readonly tag: "rejected"; readonly violations: readonly Violation[] }
	| { readonly tag: "refused"; readonly kind: "schemaError" | "newtypeMismatch" | "fingerprintMismatch"; readonly message: string }

export interface FsRequestWire {
	readonly verb: "get" | "poll" | "create" | "swap" | "delete"
	readonly root: string
	readonly key: string
	readonly bytes?: Uint8Array
	readonly etag?: string
	readonly token?: bigint
}
export type FsResultWire =
	| { readonly tag: "absent" }
	| { readonly tag: "fetched"; readonly bytes: Uint8Array; readonly etag: string }
	| { readonly tag: "unchanged" }
	| { readonly tag: "changed"; readonly bytes: Uint8Array; readonly etag: string }
	| { readonly tag: "created" | "swapped"; readonly etag: string }
	| { readonly tag: "exists" | "moved" | "ambiguous" | "deleted" }

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
	runtimeDirectoryAcquire(runtime: RuntimeHandle, policy: PolicyWire, path: string, callback: () => void): OperationHandle
	runtimeDirectoryTake(operation: OperationHandle): DirectoryHandle
	runtimeDirectoryBegin(owner: DirectoryHandle, policy: PolicyWire): OperationHandle
	runtimeDirectoryCheck(operation: OperationHandle): void
	runtimeDirectoryEnd(operation: OperationHandle): void
	runtimeDirectoryClose(owner: DirectoryHandle, remove: boolean, callback: (report: CloseWire) => void): void
	runtimeDirectoryDbOpen(owner: DirectoryHandle, policy: PolicyWire, childName: string, spec: SchemaSpec, create: boolean, callback: () => void): OperationHandle
	runtimeDbTake(operation: OperationHandle): ManagedDbOutcome
	runtimeManagedDbClose(db: DbHandle, callback: (report: CloseWire) => void): void
	runtimeFs(runtime: RuntimeHandle, policy: PolicyWire, request: FsRequestWire, callback: () => void): OperationHandle
	runtimeFsTake(operation: OperationHandle): FsResultWire
}

// The checked source/fresh-addon roster test pins this private declaration.
export const runtimeNative = native as typeof native & RuntimeNative
