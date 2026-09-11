/**
 * Faithful native-ledger acceptance specimen (TS-018 / D22): the durable
 * command vocabulary Edullm's adapter preserves — stable request IDs,
 * retained command/admin refs, terminal-slot conflict rules, witnessed
 * corrections against published StateStamp, QueryReader reuse, explicit
 * create/reopen, backup/restore,
 * and joined close. Business observation keys stay strings. All-public
 * imports only; no Promise twin, no private protocol bytes.
 *
 */
import { ChangeSet, type ChangeSet as ChangeSetType, Uuid, type QueryReader } from "@bjornpagen/bumbledb"
import {
	Command,
	LocalHistory,
	type CommandRef,
	type HistoryBinding,
	type LocalBinding,
	type OperationId,
	RequestId,
	ReceiptEpoch,
	type SubmitOutcome,
	type SubmitOptions
} from "@bjornpagen/bumbledb-log"
import { Effect, Option } from "effect"
import {
	Attempt,
	AttemptById,
	attemptsFor,
	Learning,
	makeConsumerRuntime,
	readAttempts,
} from "../core-ts/consumer.ts"
import {
	backupAndRestore,
	initializeLearning,
	resolveAfterInterrupt,
	retrySameId
} from "../log-ts/consumer.ts"

export { makeConsumerRuntime, resolveAfterInterrupt }

/** Application-owned slot identity — never truncated or reminted on retry. */
export interface NativeCommand {
	readonly receiptEpoch: ReceiptEpoch
	readonly requestId: RequestId
	readonly attempt: Uuid
}

/** Observation key preserved from native-ledger — a string, not Uuid. */
export interface Observation {
	readonly source: "published"
	readonly reportToken: string
}

export const mintCommand = (attempt: Uuid) => Effect.gen(function* () {
	const requestSource = yield* Effect.sync(() => crypto.randomUUID())
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
			const history = yield* LocalHistory.open(binding, Learning)
			const sealed = yield* Command.seal(
				{
					scope: history.identity,
					id: { receiptEpoch: command.receiptEpoch, requestId: command.requestId },
					changes,
					precondition: { kind: "blind" },
					result: { attempt: command.attempt }
				},
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
	studentId: Uuid,
	state: OutboxState
) =>
	retrySameId(
		binding,
		{
			studentId,
			attemptId: command.attempt,
			commandId: { receiptEpoch: command.receiptEpoch, requestId: command.requestId }
		},
		asRequestState(state)
	)

export const witnessedPin = (
	binding: LocalBinding,
	attemptId: Uuid,
	correction: NativeCommand,
	state: OutboxState,
	options: SubmitOptions
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const history = yield* LocalHistory.open(binding, Learning)
			const observed = yield* Effect.scoped(
				Effect.gen(function* () {
					const snapshot = yield* history.snapshot({ consistency: { kind: "latest" } })
					const row = yield* snapshot.get(AttemptById, { id: attemptId })
					if (Option.isNone(row)) {
						return yield* Effect.fail({ kind: "missing" as const })
					}
					return { previous: row.value, at: snapshot.stateStamp }
				})
			)
			const draft = yield* ChangeSet.builder(Learning)
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
	student: Uuid
) => readAttempts(reader, student)

export const provision = initializeLearning

export const backupRestoreClose = (
	source: HistoryBinding,
	destination: { readonly kind: "filesystem"; readonly directory: string },
	target: HistoryBinding,
	operationId: OperationId,
	state: OutboxState
) =>
	Effect.gen(function* () {
		const cycle = yield* backupAndRestore(
			source,
			destination,
			target,
			{ operationId },
			asRequestState(state)
		)
		return cycle
	})
