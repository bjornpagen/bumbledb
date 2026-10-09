import type { BumbleOptions } from "@bjornpagen/bumbledb"
import type { Duration } from "effect"

/** Process-wide runtime sizing and how long an idle tenant's database stays open. */
export const runtimePolicy: {
	readonly native: BumbleOptions
	readonly idleTenant: Duration.Input
} = {
	native: {
		workers: 2,
		queueCapacity: 64,
		cleanupCapacity: 16,
		ownerCapacity: 32,
		nativeHandleCapacity: 256
	},
	idleTenant: "5 minutes"
}
