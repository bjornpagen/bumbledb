/**
 * The application's tenant-binding registry (chapter 33: "the application
 * supplies its existing authenticated tenant-binding registry or one
 * deployment environment value"). Bumbledb provides no router or alias
 * service: this app maps an AUTHENTICATED principal to the tenant's
 * verified history binding, recorded when the tenant was provisioned
 * (`scripts/init-tenant.ts` persists the binding `initialize` returned).
 *
 * A missing tenant is a typed "not provisioned" refusal — never implicit
 * genesis: `LocalHistory.open`/`HostedHistory.open` of a missing database
 * refuses, and nothing here calls create.
 *
 * Registry file shape (JSON, bounded):
 *   { "tenants": { "<tenantId>": {
 *       "kind": "hosted", "identity": "<rendered identity>",
 *       "bucket": "...", "prefix": "...", "region": "..." }
 *     | { "kind": "local", "identity": "<rendered>", "directory": "..." } } }
 *
 * Hosted bindings get a per-process DISPOSABLE materialization directory
 * (S3 is the authority; local files are cache). Local bindings point at
 * genuinely durable owned storage — the dev flow — and are never claimed
 * durable on an ephemeral function filesystem.
 */
import { randomUUID } from "node:crypto"
import * as fs from "node:fs"
import * as path from "node:path"
import type { HistoryBinding } from "@bjornpagen/bumbledb-log"
import { parseDatabaseIdentity } from "@bjornpagen/bumbledb-log"
import { Effect, Result, Schema } from "effect"

export class TenantNotProvisioned extends Schema.TaggedError<TenantNotProvisioned>()("TenantNotProvisioned", {
	tenantId: Schema.String
}) {}

export class BindingRegistryInvalid extends Schema.TaggedError<BindingRegistryInvalid>()("BindingRegistryInvalid", {
	detail: Schema.String
}) {}

const HostedRecord = Schema.Struct({
	kind: Schema.Literal("hosted"),
	identity: Schema.String,
	bucket: Schema.String,
	prefix: Schema.String,
	region: Schema.optional(Schema.String)
})

const LocalRecord = Schema.Struct({
	kind: Schema.Literal("local"),
	identity: Schema.String,
	directory: Schema.String
})

const Registry = Schema.Struct({
	tenants: Schema.Record(Schema.String, Schema.Union([HostedRecord, LocalRecord]))
})

const REGISTRY_LIMIT_BYTES = 1_048_576

function registryPath(): string {
	return process.env.BUMBLEDB_TENANT_BINDINGS_FILE ?? path.join(process.cwd(), ".bumbledb", "tenants.json")
}

/** Where hosted tenants materialize locally — disposable cache, per process. */
function materializationDir(tenantId: string): string {
	const base = process.env.BUMBLEDB_MATERIALIZATION_DIR ?? path.join(process.cwd(), ".bumbledb", "cache")
	return path.join(base, tenantId)
}

const decodeRegistry = Schema.decodeUnknownEffect(Registry)

/**
 * Bounded read + strict decode of the registry file. Read per resolution
 * (small file, bounded); a production app may hold it in its own config
 * service — that cache is app policy, never a second database authority.
 */
const loadRegistry = Effect.fn("bindings.loadRegistry")(function* () {
	const file = registryPath()
	const stat = fs.statSync(file, { throwIfNoEntry: false })
	if (stat === undefined) {
		return yield* new BindingRegistryInvalid({ detail: `no tenant registry at ${file}; run scripts/init-tenant.ts` })
	}
	if (stat.size > REGISTRY_LIMIT_BYTES) {
		return yield* new BindingRegistryInvalid({ detail: `tenant registry exceeds ${REGISTRY_LIMIT_BYTES} bytes` })
	}
	const raw = fs.readFileSync(file, "utf8")
	const parsed = yield* Effect.try({
		try: () => JSON.parse(raw),
		catch: () => new BindingRegistryInvalid({ detail: `tenant registry is not JSON: ${file}` })
	})
	return yield* decodeRegistry(parsed).pipe(
		Effect.mapError((cause) => new BindingRegistryInvalid({ detail: String(cause) }))
	)
})

/** Resolve an authenticated tenant to its verified binding. */
export const bindingFor = Effect.fn("bindings.bindingFor")(function* (tenantId: string) {
	const registry = yield* loadRegistry()
	const record = registry.tenants[tenantId]
	if (record === undefined) {
		return yield* new TenantNotProvisioned({ tenantId })
	}
	const identity = parseDatabaseIdentity(record.identity)
	if (Result.isFailure(identity)) {
		return yield* new BindingRegistryInvalid({ detail: `tenant ${tenantId} carries a malformed identity` })
	}
	if (record.kind === "local") {
		return {
			kind: "local",
			directory: record.directory,
			identity: identity.success
		} satisfies HistoryBinding
	}
	return {
		kind: "hosted",
		origin: {
			bucket: record.bucket,
			prefix: record.prefix,
			...(record.region !== undefined ? { region: record.region } : {})
		},
		directory: materializationDir(tenantId),
		identity: identity.success
		// credentials omitted: the supported provider chain resolves and
		// REFRESHES the deployed role's credentials natively; no static keys.
	} satisfies HistoryBinding
})

const decodeRegistrySync = Schema.decodeUnknownSync(Registry)

/** Used by scripts/init-tenant.ts to persist a verified binding record. */
export function saveTenantBinding(
	tenantId: string,
	record:
		| { kind: "hosted"; identity: string; bucket: string; prefix: string; region?: string }
		| { kind: "local"; identity: string; directory: string }
): void {
	const file = registryPath()
	fs.mkdirSync(path.dirname(file), { recursive: true })
	const stat = fs.statSync(file, { throwIfNoEntry: false })
	const current =
		stat === undefined ? { tenants: {} } : decodeRegistrySync(JSON.parse(fs.readFileSync(file, "utf8")))
	current.tenants = { ...current.tenants, [tenantId]: record }
	const staged = `${file}.tmp-${randomUUID()}`
	fs.writeFileSync(staged, `${JSON.stringify(current, null, "\t")}\n`)
	fs.renameSync(staged, file)
}
