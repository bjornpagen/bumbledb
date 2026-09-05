/**
 * Provision one tenant database by EXECUTING the generated plan chain from
 * the declared empty base (chapter 33 initialization: seeds and closed
 * data actually run; no latest-schema shortcut marks skipped plans
 * applied), then persist the VERIFIED binding `initialize` returned into
 * the app's tenant registry. Ordinary open never initializes — a missing
 * tenant stays a typed refusal until this explicit admin job runs.
 *
 *   # local development (LocalHistory, durable app-owned directory):
 *   node --experimental-strip-types scripts/init-tenant.ts \
 *     local <tenantId> <operationIdHex> <databaseIdHex> <incarnationIdHex>
 *
 *   # hosted (S3 authority; bucket/prefix from env):
 *   BUMBLEDB_LOG_BUCKET=... BUMBLEDB_LOG_PREFIX=log \
 *   node --experimental-strip-types scripts/init-tenant.ts \
 *     hosted <tenantId> <operationIdHex> <databaseIdHex> <incarnationIdHex>
 *
 * The three Id128s are the STABLE CREATION IDENTITY (chapter 30): the
 * operator mints them once, records them with the tenant, and reuses them
 * on every retry — a lost response re-runs with the same identity and
 * resolves to the same initialization, never a second database. The
 * target schema identity comes from the generated runtime contract. A
 * failed HEAD read never recreates an empty database.
 */
import * as fs from "node:fs"
import * as path from "node:path"
import { Id128, NativeRuntime } from "@bjornpagen/bumbledb"
import type { DatabaseIdentity, HistoryBinding } from "@bjornpagen/bumbledb-log"
import { DatabaseId, IncarnationId, OperationId, parseSchemaId, renderDatabaseIdentity } from "@bjornpagen/bumbledb-log"
import type { GeneratedMigrations } from "@bjornpagen/bumbledb-log/migrations"
import { decodeGeneratedMigrations, initialize } from "@bjornpagen/bumbledb-log/migrations"
import { Effect, Result } from "effect"
import { saveTenantBinding } from "../src/db/bindings.ts"
import { adminWork, runtimePolicy } from "../src/db/runtime-policy.ts"

const MIGRATIONS_DIR = path.join(process.cwd(), "bumbledb", "migrations")

function loadPlans(): GeneratedMigrations {
	const manifest = JSON.parse(fs.readFileSync(path.join(MIGRATIONS_DIR, "manifest.json"), "utf8")) as {
		entries?: ReadonlyArray<{ sequence: string; id: string }>
	}
	const plans = (manifest.entries ?? []).map((entry) =>
		JSON.parse(fs.readFileSync(path.join(MIGRATIONS_DIR, `${entry.id}.plan.json`), "utf8"))
	)
	const decoded = decodeGeneratedMigrations({ manifest, plans })
	if (!decoded.ok) {
		throw new Error(`generated migrations refuse decoding: ${decoded.detail}`)
	}
	return decoded.value
}

function id128Of(name: string, hex: string | undefined): Id128 {
	if (hex === undefined) {
		throw new Error(`${name} is required (32 lowercase hex characters)`)
	}
	const parsed = Id128.fromHex(hex)
	if (Result.isFailure(parsed)) {
		throw new Error(`${name} must be 32 lowercase hex characters, got ${hex}`)
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
	const operationId = unwrap("operation id", OperationId.from(id128Of("operation id", operationHex)))
	const databaseId = unwrap("database id", DatabaseId.from(id128Of("database id", databaseHex)))
	const incarnationId = unwrap("incarnation id", IncarnationId.from(id128Of("incarnation id", incarnationHex)))

	const plans = loadPlans()

	// The generated contract names the target canonical schema identity.
	const contract = JSON.parse(fs.readFileSync(path.join(MIGRATIONS_DIR, "runtime-contract.json"), "utf8")) as {
		schemaId: string
	}
	const schemaId = unwrap("contract schemaId", parseSchemaId(contract.schemaId))

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

	const outcome = await Effect.runPromise(
		initialize(binding, plans, { ...adminWork, operationId }).pipe(
			Effect.provide(NativeRuntime.layer(runtimePolicy.native))
		)
	)

	if (outcome.kind !== "completed") {
		console.error(`initialize: ${outcome.kind} — resolve with scripts/migrate.ts status before retrying`)
		process.exitCode = 1
		return
	}
	const verified = outcome.value.binding
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
	console.log(
		`tenant ${tenantId} initialized: ${renderDatabaseIdentity(verified.identity)} (genesis ${outcome.value.genesis})`
	)
}

function requireEnv(name: string): string {
	const value = process.env[name]
	if (value === undefined || value === "") {
		throw new Error(`${name} is required`)
	}
	return value
}

await main()
