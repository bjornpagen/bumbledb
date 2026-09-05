/**
 * Faithful native-ledger acceptance specimen (TS-018 / D22): the durable
 * command vocabulary Edullm's adapter preserves — stable request IDs,
 * retained command/admin refs, terminal-slot conflict rules, witnessed
 * corrections against published StateStamp, QueryReader reuse, generated
 * initialize/migrate/reopen, field-arithmetic backfill, backup/restore,
 * and joined close. Business observation keys stay strings. All-public
 * imports only; no Promise twin, no private protocol bytes.
 *
 * Verification: NotRun until packed-consumer qualification.
 */
import { ChangeSet, type ChangeSet as ChangeSetType, Id128, type ExecutionPolicy, type QueryReader } from "@bjornpagen/bumbledb"
import {
	Command,
	type GeneratedMigrations,
	LocalHistory,
	type CommandRef,
	type HistoryBinding,
	type LocalBinding,
	type OperationId,
	RequestId,
	ReceiptEpoch,
	type RuntimeExpectation,
	type SubmitOutcome,
	type SubmitOptions
} from "@bjornpagen/bumbledb-log"
import { Effect, Option } from "effect"
import {
	Attempt,
	attemptsFor,
	incrementUnits,
	Learning,
	makeConsumerRuntime,
	readAttempts,
	tinyDelivery,
	work
} from "../core-ts/consumer.ts"
import {
	backupAndRestore,
	generateIncrementUnits,
	incrementUnitsIntent,
	initializeLearning,
	migrateLearning,
	resolveAfterInterrupt,
	retrySameId
} from "../log-ts/consumer.ts"

export { incrementUnits, incrementUnitsIntent, makeConsumerRuntime, resolveAfterInterrupt }

/** Application-owned slot identity — never truncated or reminted on retry. */
export interface NativeCommand {
	readonly receiptEpoch: ReceiptEpoch
	readonly requestId: RequestId
	readonly attempt: Id128
}

/** Observation key preserved from native-ledger — a string, not Id128. */
export interface Observation {
	readonly source: "published"
	readonly reportToken: string
}

export const mintCommand = Effect.gen(function* (attempt: Id128) {
	const requestSource = yield* Id128.random()
	const requestId = yield* Effect.fromResult(RequestId.from(requestSource))
	const receiptEpoch = yield* Effect.fromResult(ReceiptEpoch.from(1n))
	return { receiptEpoch, requestId, attempt } satisfies NativeCommand
})

export interface OutboxState {
	readonly rememberRef: (ref: CommandRef) => Effect.Effect<void>
	readonly rememberOutcome: (outcome: SubmitOutcome) => Effect.Effect<void>
	readonly rememberAdminRef: (ref: unknown) => Effect.Effect<void>
}

const asRequestState = (state: OutboxState) => ({
	rememberCommandRef: state.rememberRef,
	rememberSubmitOutcome: state.rememberOutcome,
	rememberAdminRef: state.rememberAdminRef
})

export const submitTerminal = (
	binding: LocalBinding,
	command: NativeCommand,
	changes: ChangeSetType<typeof Learning>,
	state: OutboxState,
	options: SubmitOptions
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const history = yield* LocalHistory.open(binding, Learning, options)
			const sealed = yield* Command.seal(
				{
					scope: history.identity,
					id: { receiptEpoch: command.receiptEpoch, requestId: command.requestId },
					changes,
					precondition: { kind: "blind" },
					result: { attempt: command.attempt }
				},
				work
			)
			yield* state.rememberRef(sealed.ref)
			const outcome = yield* history.submit(sealed, options)
			yield* state.rememberOutcome(outcome)
			const closed = yield* history.close()
			return { outcome, ref: sealed.ref, closed }
		})
	)

export const retrySameCommand = (
	binding: LocalBinding,
	command: NativeCommand,
	studentId: Id128,
	state: OutboxState,
	options: ExecutionPolicy
) =>
	retrySameId(
		binding,
		{
			studentId,
			attemptId: command.attempt,
			commandId: { receiptEpoch: command.receiptEpoch, requestId: command.requestId }
		},
		asRequestState(state),
		options
	)

export const witnessedPin = (
	binding: LocalBinding,
	attemptId: Id128,
	correction: NativeCommand,
	state: OutboxState,
	options: SubmitOptions
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const history = yield* LocalHistory.open(binding, Learning, options)
			const observed = yield* Effect.scoped(
				Effect.gen(function* () {
					const snapshot = yield* history.snapshot({ ...work, consistency: { kind: "latest" } })
					const row = yield* snapshot.get(Attempt, { id: attemptId }, work)
					if (Option.isNone(row)) {
						return yield* Effect.fail({ kind: "missing" as const })
					}
					return { previous: row.value, at: snapshot.stateStamp }
				})
			)
			const draft = yield* ChangeSet.builder(Learning, work)
			yield* draft.delete(Attempt, [observed.previous])
			yield* draft.insert(Attempt, [{ ...observed.previous, score: 0.99 }])
			const changes = yield* draft.finish()
			const sealed = yield* Command.seal(
				{
					scope: history.identity,
					id: { receiptEpoch: correction.receiptEpoch, requestId: correction.requestId },
					changes,
					precondition: { kind: "exact-state", at: observed.at },
					result: { attempt: attemptId }
				},
				work
			)
			yield* state.rememberRef(sealed.ref)
			const outcome = yield* history.submit(sealed, options)
			const closed = yield* history.close()
			return { outcome, closed }
		})
	)

/** Same read helper on published snapshots — no adapter. */
export const readPublishedAttempts = (
	reader: QueryReader<typeof Learning>,
	student: Id128,
	delivery: ExecutionPolicy
) => readAttempts(reader, student, delivery)

export const collectPublishedUnderTinyBudget = (
	reader: QueryReader<typeof Learning>,
	student: Id128
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const result = yield* reader.execute(attemptsFor, { student }, work)
			return yield* result.collect({ maxBytes: tinyDelivery.resultBytes }, tinyDelivery)
		})
	)

export const provisionAndIncrement = (
	binding: HistoryBinding,
	plans: GeneratedMigrations,
	operationId: OperationId,
	state: OutboxState,
	admin: ExecutionPolicy
) =>
	Effect.gen(function* () {
		const initialized = yield* initializeLearning(binding, plans, { ...admin, operationId }, asRequestState(state))
		const generated = yield* generateIncrementUnits({ directory: "bumbledb/migrations" }, admin)
		const migrated = yield* migrateLearning(
			binding,
			generated.generated,
			{ ...admin, operationId },
			asRequestState(state)
		)
		return { initialized, generated, migrated }
	})

export const backupRestoreClose = (
	source: HistoryBinding,
	destination: { readonly kind: "filesystem"; readonly directory: string },
	target: HistoryBinding,
	operationId: OperationId,
	state: OutboxState,
	admin: ExecutionPolicy,
	expected: RuntimeExpectation
) =>
	Effect.gen(function* () {
		const cycle = yield* backupAndRestore(
			source,
			destination,
			target,
			{ ...admin, operationId },
			asRequestState(state)
		)
		return { cycle, expected }
	})
