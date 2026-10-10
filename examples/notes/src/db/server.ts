/**
 * The one server-only database module. The ManagedRuntime is the framework boundary: building it
 * opens nothing, the first request builds the layer, and hot reload never replaces live native
 * owners (a changed policy needs a dev-server restart). Each tenant's database comes from one pool;
 * production opens with `verify`, so a tenant whose migrations have not run is refused.
 */
import "server-only"
import type { DatabasePool } from "@bjornpagen/bumbledb"
import { Bumble, Database } from "@bjornpagen/bumbledb"
import { Context, Layer, ManagedRuntime } from "effect"
import { migrations } from "../../migrations/index.ts"
import { runtimePolicy } from "./runtime-policy.ts"
import { App } from "./schema.ts"
import { cacheFor, storeFor } from "./stores.ts"

export class Databases extends Context.Service<Databases, DatabasePool<typeof App>>()("app/Databases") {
	static readonly layer = Layer.effect(
		Databases,
		Database.pool({
			schema: App,
			migrations,
			onOpen: process.env.NODE_ENV === "production" ? "verify" : "migrate",
			store: storeFor,
			cache: cacheFor,
			idleTimeToLive: runtimePolicy.idleTenant
		})
	)
}

const appLayer = Databases.layer.pipe(Layer.provideMerge(Bumble.layer(runtimePolicy.native)))

const makeRuntime = () => ManagedRuntime.make(appLayer)

const state = globalThis as typeof globalThis & {
	__bdb?: { policy: typeof runtimePolicy; runtime: ReturnType<typeof makeRuntime> }
}
if (state.__bdb && state.__bdb.policy !== runtimePolicy) {
	throw new Error("Database runtime settings changed; restart the development server")
}
state.__bdb ??= { policy: runtimePolicy, runtime: makeRuntime() }

/**
 * The app's one runtime. Handlers call `appRuntime.runPromiseExit(effect, { signal: request.signal })`;
 * the request signal becomes fiber interruption only at this boundary.
 */
export const appRuntime = state.__bdb.runtime
