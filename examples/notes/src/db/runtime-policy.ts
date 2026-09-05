/**
 * Measured execution policies — DEPLOYMENT INPUTS, not engine constants.
 * The numbers below are the local development envelope; production values
 * come from the host qualification runs (APP-06) and must be re-derived
 * when the function memory/disk/deadline configuration changes. Zero is
 * zero (no work allowed), never unlimited.
 */
import type { ExecutionPolicy, NativeRuntimeOptions } from "@bjornpagen/bumbledb"

/**
 * One process, one bounded native runtime. Workers/queues are per-process
 * capacity shared by every tenant this instance serves; the cleanup
 * envelope is reserved so exhausted work budgets can never prevent
 * release.
 */
export const runtimePolicy: {
	readonly native: NativeRuntimeOptions
	readonly cache: { readonly maxOpen: number; readonly budgetBytes: bigint }
} = {
	native: {
		workers: 2,
		queueCapacity: 64,
		cleanupCapacity: 16,
		ownerCapacity: 32,
		nativeHandleCapacity: 256,
		inputBytes: 32_000_000n,
		workingBytes: 128_000_000n,
		scratchBytes: 128_000_000n,
		resultBytes: 32_000_000n,
		chunkBytes: 1_000_000n,
		cleanupTimeout: "5 seconds"
	},
	cache: {
		maxOpen: 16,
		budgetBytes: 512_000_000n
	}
}

/** The maintenance/admin budget (cache inspect/evict, status reads). */
export const maintenanceWork: ExecutionPolicy = {
	inputBytes: 1_000_000n,
	workingBytes: 16_000_000n,
	scratchBytes: 16_000_000n,
	resultBytes: 4_000_000n,
	rows: 10_000n,
	workUnits: 1_000_000n,
	timeout: "5 seconds"
}

/**
 * Per-request work budget. Finite timeout with cleanup margin below the
 * platform's function deadline; request abort arrives as fiber
 * interruption at the ManagedRuntime boundary, never a second
 * cancellation API.
 */
export function requestPolicy(_request: Request): ExecutionPolicy {
	return {
		inputBytes: 2_000_000n,
		workingBytes: 32_000_000n,
		scratchBytes: 32_000_000n,
		resultBytes: 2_000_000n,
		rows: 20_000n,
		workUnits: 5_000_000n,
		timeout: "10 seconds"
	}
}

/** The admin job budget (migrations can be expensive; still bounded). */
export const adminWork: ExecutionPolicy = {
	inputBytes: 64_000_000n,
	workingBytes: 512_000_000n,
	scratchBytes: 1_024_000_000n,
	resultBytes: 16_000_000n,
	rows: 5_000_000n,
	workUnits: 500_000_000n,
	timeout: "15 minutes"
}
