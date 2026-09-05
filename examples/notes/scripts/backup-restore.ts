/**
 * Explicit admin backup / verify / restore. The operation id is supplied
 * and persisted BEFORE dispatch. A lost response is resolved by reusing
 * that id — never by assuming failure or minting a second identity.
 *
 *   node --experimental-strip-types scripts/backup-restore.ts \
 *     backup <tenantId> <operationIdHex> <destinationDir>
 *   node --experimental-strip-types scripts/backup-restore.ts \
 *     verify <destinationDir>
 *   node --experimental-strip-types scripts/backup-restore.ts \
 *     restore <operationIdHex> <destinationDir> <targetTenantId>
 *
 * Verification: NotRun until F3.
 */
import * as fs from "node:fs"
import * as path from "node:path"
import { Id128, NativeRuntime } from "@bjornpagen/bumbledb"
import { backup, OperationId, restore, verifyBackup } from "@bjornpagen/bumbledb-log"
import { Effect, Result } from "effect"
import { bindingFor } from "../src/db/bindings.ts"
import { adminWork, runtimePolicy } from "../src/db/runtime-policy.ts"

const OUTCOME_DIR = path.join(process.cwd(), ".bumbledb", "admin")

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
	const [command, ...rest] = process.argv.slice(2)
	const layer = NativeRuntime.layer(runtimePolicy.native)

	if (command === "backup") {
		const [tenantId, operationHex, destinationDir] = rest
		if (tenantId === undefined || operationHex === undefined || destinationDir === undefined) {
			console.error("usage: backup-restore.ts backup <tenantId> <operationIdHex> <destinationDir>")
			process.exitCode = 2
			return
		}
		const operationId = operationIdOf(operationHex)
		const destination = { kind: "filesystem" as const, directory: destinationDir }
		const outcome = await Effect.runPromise(
			Effect.gen(function* () {
				const binding = yield* bindingFor(tenantId)
				return yield* backup(binding, { ...adminWork, operationId, destination })
			}).pipe(Effect.provide(layer))
		)
		saveOutcome(`backup-${tenantId}`, outcome)
		console.log(`backup: ${outcome.kind}`)
		return
	}

	if (command === "verify") {
		const [destinationDir] = rest
		if (destinationDir === undefined) {
			console.error("usage: backup-restore.ts verify <destinationDir>")
			process.exitCode = 2
			return
		}
		const verified = await Effect.runPromise(
			verifyBackup({ kind: "filesystem", directory: destinationDir }, adminWork).pipe(Effect.provide(layer))
		)
		saveOutcome("verify", verified)
		console.log(`verify: identity ${verified.identity === undefined ? "refused" : "present"}`)
		return
	}

	if (command === "restore") {
		const [operationHex, destinationDir, targetTenantId] = rest
		if (operationHex === undefined || destinationDir === undefined || targetTenantId === undefined) {
			console.error("usage: backup-restore.ts restore <operationIdHex> <destinationDir> <targetTenantId>")
			process.exitCode = 2
			return
		}
		const operationId = operationIdOf(operationHex)
		const outcome = await Effect.runPromise(
			Effect.gen(function* () {
				const target = yield* bindingFor(targetTenantId)
				return yield* restore({ kind: "filesystem", directory: destinationDir }, target, {
					...adminWork,
					operationId
				})
			}).pipe(Effect.provide(layer))
		)
		saveOutcome(`restore-${targetTenantId}`, outcome)
		console.log(`restore: ${outcome.kind}`)
		return
	}

	console.error("usage: backup-restore.ts <backup|verify|restore> ...")
	process.exitCode = 2
}

await main()
