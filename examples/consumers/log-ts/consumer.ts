/**
 * Packed log-TypeScript consumer (D07/D22/D27): the SAME core schema,
 * changes, QueryReader helper, and field-arithmetic intent as
 * `core-ts/consumer.ts`, submitted through the durable envelope — sealed
 * commands with retained refs, same-ID retry/resolve, generated
 * initialize/migrate/reopen, backup/restore, and joined close.
 *
 * Verification: NotRun until packed-consumer qualification.
 */
import { ChangeSet, Uuid, Scalar } from "@bjornpagen/bumbledb"
import * as fs from "node:fs"
import * as path from "node:path"
import { randomUUID } from "node:crypto"
import {
	backup,
	Command,
	type CommandRef,
	HostedHistory,
	type HostedBinding,
	type HistoryBinding,
	LocalHistory,
	type LocalBinding,
	type OperationId,
	parseDatabaseIdentity,
	ReceiptEpoch,
	RequestId,
	restore,
	type RuntimeExpectation,
	type SubmitOptions,
	type SubmitOutcome,
	TenantCache,
	verifyBackup
} from "@bjornpagen/bumbledb-log"
import {
	decodeGeneratedMigrations,
	decodeManifestData,
	generateMigrations,
	type GeneratedMigrations,
	initialize,
	migrate
} from "@bjornpagen/bumbledb-log/migrations"
import { convert, migrationIntent } from "@bjornpagen/bumbledb-log/schema"
import { Effect, Option, Result, Schema } from "effect"
import {
	Attempt,
	AttemptById,
	incrementUnits,
	Learning,
	makeConsumerRuntime,
	newAttempt,
	readAttempts,
} from "../core-ts/consumer.ts"

export { incrementUnits, makeConsumerRuntime }

export interface Intent {
	readonly studentId: Uuid
	readonly attemptId: Uuid
	readonly commandId: { readonly receiptEpoch: ReceiptEpoch; readonly requestId: RequestId }
}

export const mintIntent = Effect.gen(function* () {
	const studentId = yield* Effect.sync(() => randomUUID())
	const attemptId = yield* Effect.sync(() => randomUUID())
	const requestSource = yield* Effect.sync(() => randomUUID())
	const requestId = yield* Effect.fromResult(RequestId.from(requestSource))
	const receiptEpoch = yield* Effect.fromResult(ReceiptEpoch.from(1n))
	return { studentId, attemptId, commandId: { receiptEpoch, requestId } } satisfies Intent
})

export interface RequestState {
	readonly rememberCommandRef: (ref: CommandRef) => Effect.Effect<void>
	readonly rememberSubmitOutcome: (outcome: SubmitOutcome) => Effect.Effect<void>
	readonly rememberAdminRef: (ref: unknown) => Effect.Effect<void>
}

export const submitOptions: SubmitOptions = {
	attempts: 4,
	backoff: { baseMillis: 50, capMillis: 2_000 }
}

/** Same sealed bytes, same command identity — never a reminted request id. */
export const submitAttempt = (
	binding: HostedBinding | LocalBinding,
	intent: Intent,
	state: RequestState
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const history =
				binding.kind === "hosted"
					? yield* HostedHistory.open(binding, Learning)
					: yield* LocalHistory.open(binding, Learning)
			const changes = yield* newAttempt(intent.studentId, intent.attemptId)
			const command = yield* Command.seal(
				{
					scope: history.identity,
					id: intent.commandId,
					changes,
					precondition: { kind: "blind" },
					result: { attempt: intent.attemptId }
				},
			)
			yield* state.rememberCommandRef(command.ref)
			const outcome = yield* history.submit(command, submitOptions)
			yield* state.rememberSubmitOutcome(outcome)
			const closed = yield* history.close()
			return { outcome, ref: command.ref, closed }
		})
	)

export const retrySameId = submitAttempt

export const resolveAfterInterrupt = (
	binding: LocalBinding,
	ref: CommandRef
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const history = yield* LocalHistory.open(binding, Learning)
			const resolved = yield* history.resolve(ref)
			const closed = yield* history.close()
			return { resolved, closed }
		})
	)

export class AttemptMissing extends Schema.TaggedError<AttemptMissing>()("AttemptMissing", {}) {}

export const correctAttempt = (
	binding: LocalBinding,
	intent: Intent & { readonly correctionCommandId: Intent["commandId"] },
	state: RequestState
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const history = yield* LocalHistory.open(binding, Learning)
			const observed = yield* Effect.scoped(
				Effect.gen(function* () {
					const snapshot = yield* history.snapshot({ consistency: { kind: "latest" } })
					const previous = yield* snapshot.get(AttemptById, { id: intent.attemptId })
					if (Option.isNone(previous)) {
						return yield* new AttemptMissing({})
					}
					return { previous: previous.value, at: snapshot.stateStamp }
				})
			)
			const draft = yield* ChangeSet.builder(Learning)
			yield* draft.delete(Attempt, [observed.previous])
			yield* draft.insert(Attempt, [{ ...observed.previous, score: 0.95 }])
			const changes = yield* draft.finish()
			const command = yield* Command.seal(
				{
					scope: history.identity,
					id: intent.correctionCommandId,
					changes,
					precondition: { kind: "exact-state", at: observed.at },
					result: { attempt: intent.attemptId }
				},
			)
			yield* state.rememberCommandRef(command.ref)
			const outcome = yield* history.submit(command, submitOptions)
			const closed = yield* history.close()
			return { outcome, closed }
		})
	)

export const readPublished = (
	binding: HistoryBinding,
	student: Uuid,
	expected: RuntimeExpectation
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const cache = yield* TenantCache.make(Learning, {
				maxOpen: 8,
				expected
			})
			const borrow = yield* cache.acquire(binding)
			const snapshot = yield* borrow.snapshot({ consistency: { kind: "cached" } })
			const rows = yield* readAttempts(snapshot, student)
			const released = yield* borrow.release()
			const closed = yield* cache.close()
			return { rows, released, closed }
		})
	)

/** D27: convert existing u64 units by the unresolved field expression. */
export const incrementUnitsIntent = migrationIntent(Learning, [convert(Attempt, "units", incrementUnits)])

/** Runner input is the generated `{ manifest, plans, snapshots }` triple. */
export function loadGeneratedMigrations(directory: string): GeneratedMigrations {
	const manifest = JSON.parse(fs.readFileSync(path.join(directory, "manifest.json"), "utf8"))
	const manifestDecoded = decodeManifestData(manifest)
	if (!manifestDecoded.ok) {
		throw new Error(`generated migrations refuse decoding: ${manifestDecoded.detail}`)
	}
	const plans = manifestDecoded.value.entries.map((entry) =>
		JSON.parse(fs.readFileSync(path.join(directory, `${entry.id}.plan.json`), "utf8"))
	)
	const snapshots = JSON.parse(fs.readFileSync(path.join(directory, "snapshots.json"), "utf8"))
	const decoded = decodeGeneratedMigrations({ manifest, plans, snapshots })
	if (!decoded.ok) {
		throw new Error(`generated migrations refuse decoding: ${decoded.detail}`)
	}
	return decoded.value
}

export const generateIncrementUnits = (repository: { readonly directory: string }) =>
	Effect.gen(function* () {
		const report = yield* generateMigrations({
			schema: Learning,
			intent: incrementUnitsIntent,
			label: "increment-units",
			repository,
		})
		return { report, generated: loadGeneratedMigrations(repository.directory) }
	})

export const initializeLearning = (
	binding: HistoryBinding,
	plans: GeneratedMigrations,
	options: { readonly operationId: OperationId },
	state: RequestState
) =>
	Effect.gen(function* () {
		const outcome = yield* initialize(binding, plans, options)
		yield* state.rememberAdminRef(outcome)
		return outcome
	})

export const migrateLearning = (
	binding: HistoryBinding,
	plans: GeneratedMigrations,
	options: { readonly operationId: OperationId },
	state: RequestState
) =>
	Effect.gen(function* () {
		const outcome = yield* migrate(binding, plans, options)
		yield* state.rememberAdminRef(outcome)
		return outcome
	})

export const backupAndRestore = (
	source: HistoryBinding,
	destination: { readonly kind: "filesystem"; readonly directory: string },
	target: HistoryBinding,
	options: { readonly operationId: OperationId },
	state: RequestState
) =>
	Effect.gen(function* () {
		const backed = yield* backup(source, { ...options, destination })
		yield* state.rememberAdminRef(backed)
		if (backed.kind !== "completed") {
			return { backed, verified: null, restored: null }
		}
		const verified = yield* verifyBackup(destination, {})
		const restored = yield* restore(destination, target, options)
		yield* state.rememberAdminRef(restored)
		return { backed, verified, restored }
	})

export const parsedIdentityIsBounded: boolean = Result.isFailure(parseDatabaseIdentity("not-an-identity"))

/** Known-invalid literals refuse at authoring — not after native load. */
export const knownInvalidMixRefuses: boolean = (() => {
	try {
		// @ts-expect-error Deliberately exercise the runtime refusal for untyped callers.
		Scalar.add(Scalar.i64(1n), Scalar.u64(1n))
		return false
	} catch {
		return true
	}
})()

export const consumerRuntime = makeConsumerRuntime
