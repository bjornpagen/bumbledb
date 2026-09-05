/** Isolated tarball runner for D07/D22. Specimens do not self-provide. */
import assert from "node:assert/strict"
import { createRequire } from "node:module"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { Db, DbError, Id128, internalBlake3 } from "@bjornpagen/bumbledb"
import { ProtocolError, protocolErrorCodes } from "@bjornpagen/bumbledb-log"
import { AuthoringError } from "@bjornpagen/bumbledb"
import { Effect, Exit } from "effect"
import {
	Learning,
	attemptsFor,
	collectUnderTinyBudget,
	coreProgram,
	drainPages,
	incrementUnits,
	incrementUnitsAsF64,
	makeConsumerRuntime,
	newAttempt,
	readAttempts,
	tinyDelivery,
	work
} from "./core-ts/consumer.ts"
import { incrementUnitsIntent, knownInvalidMixRefuses, mintIntent } from "./log-ts/consumer.ts"
import { collectPublishedUnderTinyBudget, mintCommand } from "./native-ledger/consumer.ts"

const consumer = createRequire(import.meta.url)
const core = createRequire(consumer.resolve("@bjornpagen/bumbledb"))
const log = createRequire(consumer.resolve("@bjornpagen/bumbledb-log"))
assert.equal(log.resolve("@bjornpagen/bumbledb"), consumer.resolve("@bjornpagen/bumbledb"))
assert.equal(core.resolve("effect"), consumer.resolve("effect"))
assert.equal(log.resolve("effect"), consumer.resolve("effect"))
assert.equal(internalBlake3(new Uint8Array()).length, 32)

assert.equal(incrementUnits.kind, "add")
assert.equal(incrementUnits.result, "unresolved")
assert.equal(incrementUnitsAsF64.kind, "toF64")
const convertUnits = incrementUnitsIntent.entries[0]
assert.equal(convertUnits?.kind, "convert")
assert.equal(convertUnits && "field" in convertUnits ? convertUnits.field : "", "units")
assert.equal(incrementUnitsIntent.schema.name, "Learning")
assert.ok(knownInvalidMixRefuses, "D27: I64/U64 mixing refuses at authoring")

const authoringRecovery = Effect.gen(function* () {
	return yield* new AuthoringError({ message: "packed authoring refusal" })
}).pipe(Effect.catchTag("AuthoringError", (failure) => Effect.succeed(failure.message)))
assert.equal(Effect.runSync(authoringRecovery), "packed authoring refusal")

const resourceError = new DbError({
	operation: "packed-consumer",
	reason: { _tag: "ResourceLimit", dimension: "workingBytes", used: 0n, requested: 10n, limit: 9n }
})
assert.equal(
	Effect.runSync(
		Effect.fail(resourceError).pipe(
			Effect.catchReason("DbError", "ResourceLimit", (reason) => Effect.succeed(reason.limit))
		)
	),
	9n
)
assert.ok(protocolErrorCodes.includes("ForeignIdentity"))
assert.equal(
	Effect.runSync(
		Effect.fail(
			new ProtocolError({ operation: "packed-consumer", reason: { _tag: "Contention", attempts: 3 } })
		).pipe(Effect.catchTag("ProtocolError", (failure) => Effect.succeed(failure.code)))
	),
	"Contention"
)

const dir = fs.mkdtempSync(path.join(os.tmpdir(), "packed-consumer-"))
const runtime = makeConsumerRuntime()

try {
	const created = await runtime.runPromise(coreProgram(path.join(dir, "core")))
	assert.ok(created.outcome.kind === "accepted" || created.outcome.kind === "no-change")
	assert.ok(Array.isArray(created.rows))
	assert.ok(created.closed)

	const d07 = await runtime.runPromise(
		Effect.scoped(
			Effect.gen(function* () {
				const studentId = yield* Id128.random()
				const attemptId = yield* Id128.random()
				const store = path.join(dir, "d07")
				const db = yield* Db.create(store, Learning, work)
				const changes = yield* newAttempt(studentId, attemptId, work)
				const outcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
				assert.ok(outcome.kind === "accepted" || outcome.kind === "no-change")
				const snapshot = yield* db.snapshot(work)
				const rows = yield* readAttempts(snapshot, studentId, work)
				assert.ok(rows.length >= 1)
				const paged = yield* drainPages(snapshot, studentId, work)
				assert.equal(paged, rows.length, "D07: pages and collect must agree on admitted rows")
				const tiny = yield* Effect.exit(collectUnderTinyBudget(snapshot, studentId))
				assert.ok(Exit.isFailure(tiny), "D07: tiny collect must fail, not return a complete page")
				const tinyLedger = yield* Effect.exit(collectPublishedUnderTinyBudget(snapshot, studentId))
				assert.ok(Exit.isFailure(tinyLedger), "D07: native-ledger tiny collect must fail")
				const cursor = yield* snapshot.execute(attemptsFor, { student: studentId }, work)
				const refused = yield* Effect.exit(
					cursor.collect({ maxBytes: tinyDelivery.resultBytes }, tinyDelivery)
				)
				assert.ok(Exit.isFailure(refused), "D07: same-cursor tiny collect must refuse")
				const retried = yield* cursor.collect({ maxBytes: work.resultBytes }, work)
				assert.ok(
					retried.length >= 1,
					"D12: predelivery refusal must return no data and leave the cursor unadvanced"
				)
				return yield* db.close()
			})
		)
	)
	assert.ok(d07)

	const intent = await runtime.runPromise(mintIntent)
	assert.ok(intent.studentId)
	const command = await runtime.runPromise(mintCommand)
	assert.ok(command.requestId)
	assert.ok(command.receiptEpoch)
	assert.ok(intent.commandId.requestId)
} finally {
	await runtime.dispose()
	fs.rmSync(dir, { recursive: true, force: true })
}
