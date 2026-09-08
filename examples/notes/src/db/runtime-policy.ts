import type { NativeRuntimeOptions } from "@bjornpagen/bumbledb"

/** Process-wide scheduling overrides; omitted settings use native defaults. */
export const runtimePolicy: {
	readonly native: NativeRuntimeOptions
	readonly cache: { readonly maxOpen: number }
} = {
	native: {
		workers: 2,
		queueCapacity: 64,
		cleanupCapacity: 16,
		ownerCapacity: 32,
		nativeHandleCapacity: 256
	},
	cache: { maxOpen: 16 }
}
