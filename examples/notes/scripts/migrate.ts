/**
 * The explicit migration admin runner (chapter 33 "Explicit migrate and
 * cutover") — a provisioned Node admin job, never a request hook, build
 * import or worker-startup duty. It consumes the CHECKED-IN generated
 * plan data (bounded decode of the committed files; no schema.ts load, no
 * authoring code, no callbacks) and calls the native executor through the
 * typed admin API.
 *
 *   node --experimental-strip-types scripts/migrate.ts status <tenantId>
 *   node --experimental-strip-types scripts/migrate.ts migrate <tenantId> <operationIdHex>
 *   node --experimental-strip-types scripts/migrate.ts activate <tenantId> <operationIdHex>
 *
 * The operation ID is SUPPLIED and persisted by the operator BEFORE
 * dispatch (a stable Id128 hex; mint one with scripts/mint-session.ts
 * --id). Rerunning with the same ID resumes/resolves the same operation;
 * a lost response is resolved by `status`, never by assuming failure.
 * `migrate` returning ready-to-switch does NOT activate: the target stays
 * frozen until the explicit `activate` step, and its saved activation ref
 * is the one-time cutover coordinate.
 */
import * as fs from "node:fs"
import * as path from "node:path"
import { Id128, NativeRuntime } from "@bjornpagen/bumbledb"
import type { GeneratedMigrations } from "@bjornpagen/bumbledb-log/migrations"
import {
	activateMigration,
	decodeGeneratedMigrations,
	migrate,
	migrationStatus
} from "@bjornpagen/bumbledb-log/migrations"
import { OperationId } from "@bjornpagen/bumbledb-log"
import { Effect, Result } from "effect"
import { bindingFor } from "../src/db/bindings.ts"
import { adminWork, runtimePolicy } from "../src/db/runtime-policy.ts"

const MIGRATIONS_DIR = path.join(process.cwd(), "bumbledb", "migrations")
const OUTCOME_DIR = path.join(process.cwd(), ".bumbledb", "admin")

/** Bounded read + strict decode of the committed generated artifacts. */
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

/** Persist admin evidence — refs and outcomes survive this process. */
function saveOutcome(name: string, body: unknown): void {
	fs.mkdirSync(OUTCOME_DIR, { recursive: true })
	const file = path.join(OUTCOME_DIR, `${name}-${Date.now()}.json`)
	fs.writeFileSync(file, `${JSON.stringify(body, null, "\t")}\n`)
	console.log(`saved: ${file}`)
}

function operationIdOf(hex: string): OperationId {
	const id = Id128.fromHex(hex)
	if (Result.isFailure(id)) {
		throw new Error(`operation id must be 32 lowercase hex characters, got ${hex}`)
	}
	const operation = OperationId.from(id.success)
	if (Result.isFailure(operation)) {
		throw new Error("operation id refused")
	}
	return operation.success
}

async function main(): Promise<void> {
	const [command, tenantId, operationHex] = process.argv.slice(2)
	if (command === undefined || tenantId === undefined) {
		console.error("usage: migrate.ts <status|migrate|activate> <tenantId> [operationIdHex]")
		process.exitCode = 2
		return
	}
	const plans = loadPlans()
	const layer = NativeRuntime.layer(runtimePolicy.native)

	if (command === "status") {
		const status = await Effect.runPromise(
			Effect.gen(function* () {
				const binding = yield* bindingFor(tenantId)
				return yield* migrationStatus(binding, plans, adminWork)
			}).pipe(Effect.provide(layer))
		)
		saveOutcome(`status-${tenantId}`, status)
		console.log(`status: ${status.kind}`)
		return
	}

	if (operationHex === undefined) {
		console.error(`${command} requires a stable operation id (persist it BEFORE the first attempt)`)
		process.exitCode = 2
		return
	}
	const operationId = operationIdOf(operationHex)

	if (command === "migrate") {
		const outcome = await Effect.runPromise(
			Effect.gen(function* () {
				const binding = yield* bindingFor(tenantId)
				return yield* migrate(binding, plans, { ...adminWork, operationId })
			}).pipe(Effect.provide(layer))
		)
		saveOutcome(`migrate-${tenantId}`, outcome)
		if (outcome.kind === "completed" && outcome.value.kind === "ready-to-switch") {
			console.log("ready-to-switch: target frozen; deploy the new binding, verify, then run activate")
		} else {
			console.log(`migrate: ${outcome.kind}${outcome.kind === "completed" ? ` (${outcome.value.kind})` : ""}`)
		}
		return
	}

	if (command === "activate") {
		// The activation ref comes from the saved ready-to-switch outcome.
		const saved = latestSaved(`migrate-${tenantId}`)
		if (saved === undefined) {
			console.error("no saved migrate outcome; run migrate first")
			process.exitCode = 2
			return
		}
		const record = JSON.parse(fs.readFileSync(saved, "utf8")) as {
			kind?: string
			value?: { kind?: string; activation?: unknown }
		}
		if (record.kind !== "completed" || record.value?.kind !== "ready-to-switch") {
			console.error(`latest migrate outcome is not ready-to-switch: ${saved}`)
			process.exitCode = 2
			return
		}
		const outcome = await Effect.runPromise(
			// The saved ref is inert data; activateMigration verifies it natively.
			activateMigration(record.value.activation as Parameters<typeof activateMigration>[0], adminWork).pipe(
				Effect.provide(layer)
			)
		)
		saveOutcome(`activate-${tenantId}`, outcome)
		console.log(`activate: ${outcome.kind}`)
		return
	}

	console.error(`unknown command: ${command}`)
	process.exitCode = 2
}

function latestSaved(prefix: string): string | undefined {
	if (!fs.existsSync(OUTCOME_DIR)) {
		return undefined
	}
	const names = fs
		.readdirSync(OUTCOME_DIR)
		.filter((name) => name.startsWith(prefix))
		.toSorted()
	const last = names.at(-1)
	return last === undefined ? undefined : path.join(OUTCOME_DIR, last)
}

await main()
