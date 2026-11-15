import type { Duration } from "effect"

/** How long an idle tenant's database stays open. */
export const runtimePolicy: {
	readonly idleTenant: Duration.Input
} = {
	idleTenant: "5 minutes"
}
