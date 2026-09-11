/** Explicit tenant creation. Retain all three UUIDs and reuse them on retry.
 * The current schema is compiled directly; application initialization writes
 * its own ordinary seed command before adopting the tenant binding.
 */
import * as path from "node:path"
import { NativeRuntime, Schema, Uuid } from "@bjornpagen/bumbledb"
import type { DatabaseIdentity, HistoryBinding } from "@bjornpagen/bumbledb-log"
import { DatabaseId, IncarnationId, OperationId, renderDatabaseIdentity } from "@bjornpagen/bumbledb-log"
import { Effect, Result } from "effect"
import { saveTenantBinding } from "../src/db/bindings.ts"
import { initializeTenant } from "../src/db/initialize.ts"
import { runtimePolicy } from "../src/db/runtime-policy.ts"
import { App } from "../src/db/schema.ts"

function uuidOf(name: string, hex: string | undefined): Uuid {
	if (hex === undefined) {
		throw new Error(`${name} is required (canonical UUID text)`)
	}
	const parsed = Uuid.parse(hex)
	if (Result.isFailure(parsed)) {
		throw new Error(`${name} must be canonical UUID text, got ${hex}`)
	}
	return parsed.success
}

function unwrap<A>(name: string, value: Result.Result<A, unknown>): A {
	if (Result.isFailure(value)) {
		throw new Error(`${name} refused`)
	}
	return value.success
}

async function main(): Promise<void> {
	const [mode, tenantId, operationHex, databaseHex, incarnationHex] = process.argv.slice(2)
	if ((mode !== "local" && mode !== "hosted") || tenantId === undefined) {
		console.error("usage: init-tenant.ts <local|hosted> <tenantId> <operationIdHex> <databaseIdHex> <incarnationIdHex>")
		process.exitCode = 2
		return
	}
	const operationId = unwrap("operation id", OperationId.from(uuidOf("operation id", operationHex)))
	const databaseId = unwrap("database id", DatabaseId.from(uuidOf("database id", databaseHex)))
	const incarnationId = unwrap("incarnation id", IncarnationId.from(uuidOf("incarnation id", incarnationHex)))

	const verified = await Effect.runPromise(
		Effect.gen(function* () {
			const schemaId = (yield* Schema.compile(App)).schemaId
			const identity: DatabaseIdentity = { databaseId, incarnationId, schemaId }
			const binding: HistoryBinding =
				mode === "local"
					? {
							kind: "local",
							directory: path.join(process.cwd(), ".bumbledb", "tenants", tenantId),
							identity
						}
					: {
							kind: "hosted",
							origin: {
								bucket: requireEnv("BUMBLEDB_LOG_BUCKET"),
								prefix: `${process.env.BUMBLEDB_LOG_PREFIX ?? "log"}/${tenantId}`
							},
							directory: path.join(process.cwd(), ".bumbledb", "cache", tenantId),
							identity
						}

			return yield* initializeTenant(binding, operationId)
		}).pipe(Effect.provide(NativeRuntime.layer(runtimePolicy.native)))
	)

	if (verified.kind === "local") {
		saveTenantBinding(tenantId, {
			kind: "local",
			identity: renderDatabaseIdentity(verified.identity),
			directory: verified.directory
		})
	} else {
		saveTenantBinding(tenantId, {
			kind: "hosted",
			identity: renderDatabaseIdentity(verified.identity),
			bucket: verified.origin.bucket,
			prefix: verified.origin.prefix,
			...(verified.origin.region !== undefined ? { region: verified.origin.region } : {})
		})
	}
	console.log(`tenant ${tenantId} initialized: ${renderDatabaseIdentity(verified.identity)}`)
}

function requireEnv(name: string): string {
	const value = process.env[name]
	if (value === undefined || value === "") {
		throw new Error(`${name} is required`)
	}
	return value
}

await main()
