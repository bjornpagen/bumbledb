import type { DbHandle, Violation } from "./native.ts"
import { native } from "./native.ts"
import type { SchemaSpec } from "./spec.ts"

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
	| {
			readonly tag: "refused"
			readonly kind: "schemaError" | "newtypeMismatch" | "fingerprintMismatch" | "destinationExists"
			readonly message: string
			readonly diagnostic?: Extract<import("./errors.ts").DbError["reason"], { _tag: "Engine" }>["diagnostic"]
	  }

export interface OptionsWire {
	readonly workers?: number | undefined
	readonly queueCapacity?: number | undefined
	readonly cleanupCapacity?: number | undefined
	readonly ownerCapacity?: number | undefined
	readonly nativeHandleCapacity?: number | undefined
	readonly cleanupTimeoutMs?: number | undefined
}

export interface InspectionWire {
	readonly phase: "open" | "closing" | "closed"
	readonly queued: bigint
	readonly active: bigint
	readonly retained: bigint
	readonly owners: bigint
	readonly databases: bigint
	readonly natives: bigint
}

export type CloseWire =
	| { readonly kind: "closed" }
	| { readonly kind: "incomplete"; readonly outstanding: InspectionWire }
	| { readonly kind: "failed" }

interface RuntimeNative {
	runtimeErrorCodes(): readonly string[]
	runtimeOpen(options: OptionsWire): RuntimeHandle
	runtimeReady(runtime: RuntimeHandle, callback: () => void): OperationHandle
	runtimeHash(runtime: RuntimeHandle, bytes: Uint8Array, callback: () => void): OperationHandle
	runtimeTake(operation: OperationHandle): Uint8Array | null
	runtimeCancel(operation: OperationHandle, callback: (report: CloseWire) => void): void
	runtimeClose(runtime: RuntimeHandle, callback: (report: CloseWire) => void): void
	runtimeInspect(runtime: RuntimeHandle): InspectionWire
	/** Test hook: the next payload delivery is cancelled after its work runs and before its output is published. */
	runtimeArmPublicationCancel(runtime: RuntimeHandle): void
	runtimeDirectoryAcquire(runtime: RuntimeHandle, path: string, callback: () => void): OperationHandle
	runtimeDirectoryTake(operation: OperationHandle): DirectoryHandle
	runtimeDirectoryBegin(owner: DirectoryHandle): OperationHandle
	runtimeDirectoryCheck(operation: OperationHandle): void
	runtimeDirectoryEnd(operation: OperationHandle): void
	runtimeDirectoryClose(owner: DirectoryHandle, remove: boolean, callback: (report: CloseWire) => void): void
	runtimeDirectoryDbOpen(
		owner: DirectoryHandle,
		childName: string,
		spec: SchemaSpec,
		create: boolean,
		callback: () => void
	): OperationHandle
	runtimeDbTake(operation: OperationHandle): ManagedDbOutcome
	runtimeManagedDbClose(db: DbHandle, callback: (report: CloseWire) => void): void
}

export const runtimeNative = native as typeof native & RuntimeNative
