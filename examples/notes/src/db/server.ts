/**
 * The one server-only database module (chapter 33 "Next.js: a small
 * server-only module"). Effect's ManagedRuntime is the framework
 * boundary: constructing it opens nothing; the first request builds the
 * layer once; the app owns this process-lifetime runtime. There is no
 * per-request runtime, no runtime per tenant, and hot reload NEVER
 * silently replaces live native owners — an identical immutable policy
 * reuses the slot, a changed policy demands a dev-server restart.
 */
import "server-only"
import { NativeRuntime } from "@bjornpagen/bumbledb"
import { TenantCache } from "@bjornpagen/bumbledb-log"
import { Context, Effect, Layer, ManagedRuntime } from "effect"
import { App } from "./schema.ts"
import { runtimePolicy } from "./runtime-policy.ts"

/**
 * One typed tenant cache for the whole process: independent scoped
 * borrows per request, an open-tenant admission limit, and no wall-clock TTL.
 * The concrete schema type is preserved — no generic service tag erases
 * `typeof App`.
 */
export class Databases extends Context.Service<Databases, TenantCache<typeof App>>()("app/Databases") {
	static readonly layer = Layer.effect(
		Databases,
		Effect.gen(function* () {
			return yield* TenantCache.make(App, {
				maxOpen: runtimePolicy.cache.maxOpen
			})
		})
	)
}

const appLayer = Databases.layer.pipe(Layer.provideMerge(NativeRuntime.layer(runtimePolicy.native)))

const makeRuntime = () => ManagedRuntime.make(appLayer)

const state = globalThis as typeof globalThis & {
	__bumbledb?: {
		policy: typeof runtimePolicy
		runtime: ReturnType<typeof makeRuntime>
	}
}
if (state.__bumbledb && state.__bumbledb.policy !== runtimePolicy) {
	throw new Error("Database runtime settings changed; restart the development server")
}
state.__bumbledb ??= { policy: runtimePolicy, runtime: makeRuntime() }

/**
 * The app's one runtime. Request handlers call
 * `appRuntime.runPromise(effect, { signal: request.signal })` — the
 * request signal enters ONLY at this outer boundary and becomes fiber
 * interruption. Supported process shutdown disposes it; hard process
 * death uses ordinary database recovery, not a claimed finalizer
 * guarantee.
 */
export const appRuntime = state.__bumbledb.runtime
