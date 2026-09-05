/** Exact-version internal bridge, shared by core and log. Not a public API. */
import { native } from "#native.ts"

export interface RuntimeHandle {
	readonly __runtime: unique symbol
}
export interface OperationHandle {
	readonly __operation: unique symbol
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
}

// The checked source/fresh-addon roster test pins this private declaration.
export const runtimeNative = native as typeof native & RuntimeNative
