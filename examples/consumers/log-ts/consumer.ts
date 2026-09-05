/**
 * Chapter 34 log-TypeScript consumer fixture (API-12 / PKG-03): the SAME
 * core schema/changes/read helper as `core-ts/consumer.ts`, submitted
 * through the durable log envelope — sealed commands with retained refs,
 * typed submit certainty, a witnessed correction against a published
 * StateStamp, and a scoped TenantCache borrow. Compiled by a strict
 * downstream tsc against BOTH staged tarballs; the log package imports
 * the core's actual values (one ChangeSet, one QueryReader, one native
 * runtime) — no adapter, no duplicate type hierarchy, no Promise twin.
 */
import { ChangeSet, type ExecutionPolicy, Id128, NativeRuntime } from "@bjornpagen/bumbledb"
import {
	Command,
	type CommandRef,
	HostedHistory,
	type HostedBinding,
	type HistoryBinding,
	LocalHistory,
	type LocalBinding,
	parseDatabaseIdentity,
	ReceiptEpoch,
	RequestId,
	type SubmitOptions,
	type SubmitOutcome,
	TenantCache
} from "@bjornpagen/bumbledb-log"
import { Effect, Option, Result, Schema } from "effect"
import { Attempt, Learning, newAttempt, readAttempts, runtimePolicy, work } from "../core-ts/consumer.ts"

// ── App-owned intent state: stable IDs generated once, retained refs ─────────

export interface Intent {
	readonly studentId: Id128
	readonly attemptId: Id128
	readonly commandId: { readonly receiptEpoch: ReceiptEpoch; readonly requestId: RequestId }
}

/** Mint one original intent — outside any retry, persisted by the app. */
export const mintIntent = Effect.gen(function* () {
	const studentId = yield* Id128.random()
	const attemptId = yield* Id128.random()
	const requestSource = yield* Id128.random()
	const requestId = yield* Effect.fromResult(RequestId.from(requestSource))
	const receiptEpoch = yield* Effect.fromResult(ReceiptEpoch.from(1n))
	return { studentId, attemptId, commandId: { receiptEpoch, requestId } } satisfies Intent
})

/** The app's request/job persistence, represented as parameters here. */
export interface RequestState {
	readonly rememberCommandRef: (ref: CommandRef) => Effect.Effect<void>
	readonly rememberSubmitOutcome: (outcome: SubmitOutcome) => Effect.Effect<void>
}

export const submitOptions: SubmitOptions = {
	...work,
	attempts: 4,
	backoff: { baseMillis: 50, capMillis: 2_000 }
}

// ── Log: only the durable envelope changes ───────────────────────────────────

export const submitAttempt = (
	binding: HostedBinding,
	intent: Intent,
	state: RequestState,
	hostedOptions: ExecutionPolicy
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const history = yield* HostedHistory.open(binding, Learning, hostedOptions)
			// Local alternative: LocalHistory.open(localBinding, Learning, options).
			const changes = yield* newAttempt(intent.studentId, intent.attemptId, work)
			const command = yield* Command.seal(
				{
					scope: history.identity,
					id: intent.commandId,
					changes,
					precondition: { kind: "blind" },
					result: { attempt: intent.attemptId }
				},
				work
			)
			yield* state.rememberCommandRef(command.ref)
			const outcome = yield* history.submit(command, submitOptions)
			yield* state.rememberSubmitOutcome(outcome)
			return outcome
		})
	).pipe(Effect.provide(NativeRuntime.layer(runtimePolicy)))

// ── Witnessed correction: keep the read scope short ──────────────────────────

export class AttemptMissing extends Schema.TaggedError<AttemptMissing>()("AttemptMissing", {}) {}

export const correctAttempt = (
	binding: LocalBinding,
	intent: Intent & { readonly correctionCommandId: Intent["commandId"] },
	state: RequestState,
	options: ExecutionPolicy
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const history = yield* LocalHistory.open(binding, Learning, options)
			const observed = yield* Effect.scoped(
				Effect.gen(function* () {
					const snapshot = yield* history.snapshot({ ...work, consistency: { kind: "latest" } })
					const previous = yield* snapshot.get(Attempt, { id: intent.attemptId }, work)
					if (Option.isNone(previous)) {
						return yield* new AttemptMissing({})
					}
					return { previous: previous.value, at: snapshot.stateStamp }
				})
			)
			const draft = yield* ChangeSet.builder(Learning, work)
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
				work
			)
			yield* state.rememberCommandRef(command.ref)
			return yield* history.submit(command, submitOptions)
		})
	).pipe(Effect.provide(NativeRuntime.layer(runtimePolicy)))

// ── The same read helper works on published log snapshots ────────────────────

export const readPublished = (binding: HistoryBinding, student: Id128, options: ExecutionPolicy) =>
	Effect.scoped(
		Effect.gen(function* () {
			const cache = yield* TenantCache.make(Learning, {
				maxOpen: 8,
				budgetBytes: 256_000_000n,
				maintenance: work
			})
			const borrow = yield* cache.acquire(binding, options)
			const snapshot = yield* borrow.snapshot({ ...work, consistency: { kind: "cached" } })
			return yield* readAttempts(snapshot, student, work)
		})
	).pipe(Effect.provide(NativeRuntime.layer(runtimePolicy)))

// ── Bounded identity parsing at the app boundary ─────────────────────────────

export const parsedIdentityIsBounded: boolean = Result.isFailure(parseDatabaseIdentity("not-an-identity"))
