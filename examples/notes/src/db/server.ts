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
import * as fs from "node:fs"
import * as path from "node:path"
import { NativeRuntime } from "@bjornpagen/bumbledb"
import type { RuntimeExpectation } from "@bjornpagen/bumbledb-log"
import { TenantCache } from "@bjornpagen/bumbledb-log"
import { Context, Effect, Layer, ManagedRuntime, Result, Schema } from "effect"
import { App } from "./schema.ts"
import { maintenanceWork, runtimePolicy } from "./runtime-policy.ts"

const Contract = Schema.Struct({
	contractVersion: Schema.Number,
	schemaId: Schema.String,
	appliedPrefixDigest: Schema.String
})
const decodeContract = Schema.decodeUnknownEffect(Contract)

/**
 * The generated runtime expectation (`bumbledb-log generate` emits it
 * beside the plans): the exact canonical schema and applied-prefix this
 * build of the app expects deployed. Read once at layer build — bounded
 * file work inside the scoped acquisition, never at module import.
 */
const loadExpectation = Effect.fn("server.loadExpectation")(function* () {
	const file =
		process.env.BUMBLEDB_RUNTIME_CONTRACT ??
		path.join(process.cwd(), "bumbledb", "migrations", "runtime-contract.json")
	const raw = Result.try(() => fs.readFileSync(file, "utf8"))
	if (Result.isFailure(raw)) {
		return yield* Effect.die(
			new Error(`missing generated runtime contract at ${file}; run bumbledb-log generate and commit the artifacts`)
		)
	}
	const parsed = Result.try(() => JSON.parse(raw.success))
	if (Result.isFailure(parsed)) {
		return yield* Effect.die(new Error(`runtime contract is not JSON: ${file}`))
	}
	const contract = yield* decodeContract(parsed.success).pipe(Effect.orDie)
	return {
		schemaId: contract.schemaId,
		appliedPrefixDigest: contract.appliedPrefixDigest
	} satisfies RuntimeExpectation
})

/**
 * One typed tenant cache for the whole process: independent scoped
 * borrows per request, byte/count pressure budgets, NO wall-clock TTL.
 * The concrete schema type is preserved — no generic service tag erases
 * `typeof App`.
 */
export class Databases extends Context.Service<Databases, TenantCache<typeof App>>()("app/Databases") {
	static readonly layer = Layer.effect(
		Databases,
		Effect.gen(function* () {
			const expected = yield* loadExpectation()
			return yield* TenantCache.make(App, {
				maxOpen: runtimePolicy.cache.maxOpen,
				budgetBytes: runtimePolicy.cache.budgetBytes,
				maintenance: maintenanceWork,
				expected
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
