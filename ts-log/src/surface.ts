/**
 * The chapter 35 log signature roster: the same core, with durable
 * identity. `PublishedSnapshot<S>` extends the exact core `QueryReader<S>`
 * — same `get`/`execute`, parameters, policies, errors and result owners;
 * the log adds only identity/stamps/freshness. A `HistoryBorrow<S>` has the
 * same members as `History<S>` with `release` instead of owner `close`.
 * Everything effectful is lazy, scoped and bounded; there is no Promise,
 * sync, or disposal twin anywhere on this surface.
 */
import type {
	AnySchema,
	ChangeSet,
	CloseReport,
	DbError,
	ExecutionPolicy,
	ExecutionSession,
	QueryReader
} from "@bjornpagen/bumbledb"
import type { Effect, Scope } from "effect"
import type { LogError } from "#errors.ts"
import type {
	CommandRef,
	DatabaseIdentity,
	DecisionStamp,
	Freshness,
	ReceiptEpoch,
	RequestId,
	StateStamp
} from "#identity.ts"
import type { HistoryBinding, ReadOptions, SubmitOptions } from "#options.ts"
import type { CacheInspection, CommandResult, HistoryInspection, ResolveOutcome, SubmitOutcome } from "#outcome.ts"

export interface PublishedSnapshot<S extends AnySchema> extends QueryReader<S> {
	readonly identity: DatabaseIdentity
	readonly decisionStamp: DecisionStamp
	readonly stateStamp: StateStamp
	readonly freshness: Freshness
	session(work: ExecutionPolicy): Effect.Effect<ExecutionSession<S>, DbError, Scope.Scope>
	close(): Effect.Effect<CloseReport>
}

export interface Command<S extends AnySchema> {
	readonly ref: CommandRef
	close(): Effect.Effect<CloseReport>
}

export type Precondition = { readonly kind: "blind" } | { readonly kind: "exact-state"; readonly at: StateStamp }

/** Exactly chapter 30's `{ scope, id, changes, precondition, result }`. */
export interface CommandInput<S extends AnySchema> {
	readonly scope: DatabaseIdentity
	readonly id: { readonly receiptEpoch: ReceiptEpoch; readonly requestId: RequestId }
	readonly changes: ChangeSet<S>
	readonly precondition: Precondition
	readonly result: CommandResult
}

export interface History<S extends AnySchema> {
	readonly identity: DatabaseIdentity
	readonly receiptEpoch: ReceiptEpoch
	snapshot(options: ReadOptions): Effect.Effect<PublishedSnapshot<S>, LogError, Scope.Scope>
	submit(command: Command<S>, options: SubmitOptions): Effect.Effect<SubmitOutcome>
	resolve(ref: CommandRef, work: ExecutionPolicy): Effect.Effect<ResolveOutcome, LogError>
	inspect(work: ExecutionPolicy): Effect.Effect<HistoryInspection, LogError>
	close(): Effect.Effect<CloseReport>
}

/** Releasing frees only this borrow; it cannot close the shared owner. */
export interface HistoryBorrow<S extends AnySchema> {
	readonly identity: DatabaseIdentity
	readonly receiptEpoch: ReceiptEpoch
	snapshot(options: ReadOptions): Effect.Effect<PublishedSnapshot<S>, LogError, Scope.Scope>
	submit(command: Command<S>, options: SubmitOptions): Effect.Effect<SubmitOutcome>
	resolve(ref: CommandRef, work: ExecutionPolicy): Effect.Effect<ResolveOutcome, LogError>
	inspect(work: ExecutionPolicy): Effect.Effect<HistoryInspection, LogError>
	release(): Effect.Effect<CloseReport>
}

export interface TenantCache<S extends AnySchema> {
	acquire(binding: HistoryBinding, options: ExecutionPolicy): Effect.Effect<HistoryBorrow<S>, LogError, Scope.Scope>
	inspect(work: ExecutionPolicy): Effect.Effect<CacheInspection, LogError>
	/** Refuses a borrowed/active slot instead of revoking another request. */
	evict(binding: HistoryBinding): Effect.Effect<CloseReport, LogError>
	close(): Effect.Effect<CloseReport>
}
