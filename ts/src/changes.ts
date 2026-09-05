import { Effect, Exit } from "effect"
import type { Scope } from "effect"
import { drainClose, releaseOwner } from "#close.ts"
import type { SchemaId } from "#compile.ts"
import { Schema as CoreSchema, schemaTables } from "#compile.ts"
import type { ChangesHandle, DraftHandle } from "#db-native.ts"
import { dbNative } from "#db-native.ts"
import { lower } from "#lower.ts"
import type { AnyRelation, Fact } from "#relation.ts"
import type { CellValue } from "#rows.ts"
import { assertHostCellFits, cellOf, recordOf } from "#rows.ts"
import type { CloseReport } from "#runtime-errors.ts"
import { DbError } from "#runtime-errors.ts"
import type { ExecutionPolicy } from "#runtime.ts"
import { nativeOperationWith, policyWire, runtimeHandle } from "#runtime.ts"
import type { AnySchema } from "#schema.ts"
import type { Rel } from "#shape.ts"

/**
 * `ChangeSet` — the public name of the engine's checked immutable delta
 * (chapter 30/34): schema-fingerprint-bound canonical native bytes with
 * one-command `(add, remove ∖ add)` normalization and exact same-fact
 * add-wins. Immutable and reusable while open; sealing/submitting it later
 * retains the SAME native value — no second JS row walk ever happens.
 */
interface ChangeSet<S extends AnySchema> {
	readonly schemaId: SchemaId
	/** Phantom schema brand: a ChangeSet is only ever applied to its own S. */
	readonly schema?: S
	close(): Effect.Effect<CloseReport>
}

/**
 * `ChangeDraft` — the scoped, database-free construction capability.
 * Every method constructs a LAZY effect; execution reads the then-current
 * iterable and charges its work again on every sequential rerun (no hidden
 * memoization, no automatic retry, no iterator replay). Input must stay
 * stable from an ingestion effect's execution start through its Exit;
 * after successful ingestion the accepted native bytes are independent.
 * Failure/interruption SPENDS the draft and initiates tracked drain.
 * Concurrent/reentrant construction refuses and spends/drains the draft.
 * `finish` consumes the draft; later ingestion or a second finish refuses
 * through the spent capability state.
 */
interface ChangeDraft<S extends AnySchema> {
	insert<R extends Rel<S>>(relation: R, rows: Iterable<Fact<R>>): Effect.Effect<void, DbError>
	delete<R extends Rel<S>>(relation: R, rows: Iterable<Fact<R>>): Effect.Effect<void, DbError>
	finish(): Effect.Effect<ChangeSet<S>, DbError, Scope.Scope>
	close(): Effect.Effect<CloseReport>
}

/**
 * Host-copy granularity: one bounded host-to-native message per chunk, so
 * real event-loop turns happen between chunks (each chunk completes through
 * a native callback, not a microtask chain). These are converter
 * granularity bounds, not database-size policy — the draft's aggregate
 * input/working/spill budget is charged natively and is CUMULATIVE across
 * calls and chunks (chunks never reset it).
 */
const CHUNK_BYTES = 65536n
const CHUNK_ROWS = 4096

interface DraftState {
	readonly handle: DraftHandle
	readonly policy: ExecutionPolicy
	readonly theory: AnySchema
	spent: boolean
	inFlight: boolean
}

interface ChangesInternal {
	readonly handle: ChangesHandle
	readonly schemaId: SchemaId
	closed: boolean
}

const changesInternals = new WeakMap<object, ChangesInternal>()

/** Private cross-module accessor (db.apply retains the native ChangeSet). */
function internalChanges(value: object): ChangesInternal | undefined {
	return changesInternals.get(value)
}

function refusal(operation: string, reason: "SpentHandle" | "ClosedHandle" | "InvalidArgument"): DbError {
	return new DbError({ operation, reason: { _tag: reason } })
}

interface Chunk {
	readonly rows: bigint
	readonly cells: readonly CellValue[]
	readonly done: boolean
	readonly leftover: object | undefined
}

function eventLoopTurn(): Effect.Effect<void> {
	return Effect.callback<void>((resume) => {
		const id = setImmediate(() => resume(Effect.void))
		return Effect.sync(() => clearImmediate(id))
	})
}

function hostFactCharge(relation: AnyRelation, record: Readonly<Record<string, unknown>>): bigint {
	const data = relation.data
	let bytes = 0n
	for (const declared of data.fields) {
		const value = record[declared.name]
		if (value === undefined) {
			throw refusal("ChangeDraft.ingest", "InvalidArgument")
		}
		bytes += assertHostCellFits(
			`relation ${data.name} field ${declared.name}`,
			value,
			CHUNK_BYTES
		)
	}
	return bytes
}

function projectFact(relation: AnyRelation, record: Readonly<Record<string, unknown>>): readonly CellValue[] {
	const data = relation.data
	const cells: CellValue[] = []
	for (const declared of data.fields) {
		const value = record[declared.name]
		if (value === undefined) {
			throw refusal("ChangeDraft.ingest", "InvalidArgument")
		}
		cells.push(cellOf(`relation ${data.name} field ${declared.name}`, declared.field, value))
	}
	return cells
}

/**
 * Pulls one bounded chunk off the caller's iterator. Host length is judged
 * before any string scan or byte copy. A leftover fact that does not fit
 * the current chunk is returned unconverted for the next turn.
 */
function pullChunk(
	relation: AnyRelation,
	iterator: Iterator<object>,
	pending: object | undefined
): Chunk {
	const cells: CellValue[] = []
	let rows = 0n
	let bytes = 0n
	let leftover: object | undefined
	let current: object | undefined = pending
	while (rows < BigInt(CHUNK_ROWS) && leftover === undefined) {
		if (current === undefined) {
			const next = iterator.next()
			if (next.done === true) {
				return { rows, cells, done: true, leftover: undefined }
			}
			current = next.value
		}
		const record = recordOf(current)
		const charge = hostFactCharge(relation, record)
		if (rows > 0n && bytes + charge > CHUNK_BYTES) {
			leftover = current
			break
		}
		cells.push(...projectFact(relation, record))
		bytes += charge
		rows += 1n
		current = undefined
	}
	return { rows, cells, done: false, leftover }
}

function spendAndDrain(state: DraftState, operation: string): Effect.Effect<void> {
	return Effect.suspend(() => {
		if (state.spent) {
			return Effect.void
		}
		state.spent = true
		// Tracked drain: join the native close transition; the report is
		// diagnostic here (the ingestion failure itself is the caller's
		// error), but native Closing accounting is never dropped.
		return drainClose(operation, (callback) => dbNative.runtimeDraftClose(state.handle, callback)).pipe(Effect.asVoid)
	})
}

function ingest(
	state: DraftState,
	operation: "ChangeDraft.insert" | "ChangeDraft.delete",
	relation: AnyRelation,
	rows: Iterable<object>
): Effect.Effect<void, DbError> {
	const verb = operation === "ChangeDraft.insert" ? dbNative.runtimeDraftInsert : dbNative.runtimeDraftDelete
	return Effect.gen(function* () {
		if (state.spent) {
			return yield* Effect.fail(refusal(operation, "SpentHandle"))
		}
		if (state.inFlight) {
			// Reentrant construction refuses AND spends/drains — there is
			// no implicit queue (chapter 35).
			yield* spendAndDrain(state, operation)
			return yield* Effect.fail(refusal(operation, "SpentHandle"))
		}
		if (state.theory.relations[relation.name] !== relation) {
			return yield* Effect.fail(refusal(operation, "InvalidArgument"))
		}
		const tables = schemaTables(state.theory)
		const relationId = tables.relationIds.get(relation.name)
		if (relationId === undefined) {
			return yield* Effect.fail(refusal(operation, "InvalidArgument"))
		}
		state.inFlight = true
		const wire = yield* Effect.try({
			try: () => policyWire(state.policy, operation),
			catch: () => refusal(operation, "InvalidArgument")
		})
		const body = Effect.gen(function* () {
			const iterator = rows[Symbol.iterator]()
			let leftover: object | undefined
			let done = false
			while (!done) {
				const chunk = yield* Effect.try({
					try: () => pullChunk(relation, iterator, leftover),
					catch: (cause) => (cause instanceof DbError ? cause : refusal(operation, "InvalidArgument"))
				}).pipe(
					Effect.catch((error) =>
						spendAndDrain(state, operation).pipe(Effect.andThen(Effect.fail(error)))
					)
				)
				leftover = chunk.leftover
				done = chunk.done && leftover === undefined
				if (chunk.rows === 0n) {
					continue
				}
				yield* eventLoopTurn()
				yield* nativeOperationWith(
					operation,
					(callback) => verb(state.handle, wire, relationId, chunk.rows, chunk.cells, callback),
					dbNative.runtimeReportTake,
					() => undefined
				).pipe(
					Effect.catch((error) =>
						Effect.sync(() => {
							state.spent = true
						}).pipe(Effect.andThen(Effect.fail(error)))
					)
				)
			}
		})
		return yield* Effect.onExit(body, (exit) => {
			state.inFlight = false
			if (state.spent || Exit.isSuccess(exit)) {
				return Effect.void
			}
			return spendAndDrain(state, operation)
		})
	})
}

function makeChangeSet<S extends AnySchema>(handle: ChangesHandle, schemaId: SchemaId): ChangeSet<S> {
	const value: ChangeSet<S> = {
		schemaId,
		close() {
			return drainClose("ChangeSet.close", (callback) => dbNative.runtimeChangesClose(handle, callback))
		}
	}
	Object.freeze(value)
	changesInternals.set(value, { handle, schemaId, closed: false })
	return value
}

function makeDraft<S extends AnySchema>(state: DraftState, schemaId: SchemaId): ChangeDraft<S> {
	const draft: ChangeDraft<S> = {
		insert(relation, rows) {
			return ingest(state, "ChangeDraft.insert", relation, rows)
		},
		delete(relation, rows) {
			return ingest(state, "ChangeDraft.delete", relation, rows)
		},
		finish() {
			return Effect.acquireRelease(
				Effect.gen(function* () {
					if (state.spent) {
						return yield* Effect.fail(refusal("ChangeDraft.finish", "SpentHandle"))
					}
					if (state.inFlight) {
						yield* spendAndDrain(state, "ChangeDraft.finish")
						return yield* Effect.fail(refusal("ChangeDraft.finish", "SpentHandle"))
					}
					// Finish CONSUMES the draft, success or failure.
					state.spent = true
					const wire = yield* nativeOperationWith(
						"ChangeDraft.finish",
						(callback) => dbNative.runtimeDraftFinish(state.handle, policyWire(state.policy, "ChangeDraft.finish"), callback),
						dbNative.runtimeChangesTake,
						(value) => value
					)
					return makeChangeSet<S>(wire.changes, schemaId)
				}),
				(changes) =>
					Effect.suspend(() => {
						const internal = changesInternals.get(changes)
						if (internal === undefined || internal.closed) {
							return Effect.void
						}
						internal.closed = true
						return releaseOwner("ChangeSet.close", (callback) => dbNative.runtimeChangesClose(internal.handle, callback))
					}),
				{ interruptible: true }
			)
		},
		close() {
			return Effect.suspend(() => {
				state.spent = true
				return drainClose("ChangeDraft.close", (callback) => dbNative.runtimeDraftClose(state.handle, callback))
			})
		}
	}
	return Object.freeze(draft)
}

/**
 * `ChangeSet.builder(schema, work)` — lazy scoped acquisition of a
 * database-free draft (chapter 35 roster). Requires the acquired
 * `NativeRuntime`; the draft's native resources release with its scope, and
 * the scope finalizer surfaces incomplete/failed teardown as a
 * `CloseFailure` defect.
 */
const builder = Effect.fn("ChangeSet.builder")(function* <S extends AnySchema>(schema: S, work: ExecutionPolicy) {
	const handle = yield* runtimeHandle()
	const compiled = yield* CoreSchema.compile(schema, work)
	const spec = lower(schema)
	return yield* Effect.acquireRelease(
		Effect.gen(function* () {
			const draftHandle = yield* nativeOperationWith(
				"ChangeSet.builder",
				(callback) => dbNative.runtimeDraftOpen(handle, policyWire(work, "ChangeSet.builder"), spec, callback),
				dbNative.runtimeDraftTake,
				(value) => value
			)
			const state: DraftState = {
				handle: draftHandle,
				policy: work,
				theory: schema,
				spent: false,
				inFlight: false
			}
			const draft = makeDraft<S>(state, compiled.schemaId)
			draftStates.set(draft, state)
			return draft
		}),
		(draft) =>
			Effect.suspend(() => {
				const state = draftStates.get(draft)
				if (state === undefined) {
					return Effect.void
				}
				// Idempotent: repeated close joins the same native
				// transition; the finalizer runs it unconditionally so an
				// abandoned draft is always drained.
				state.spent = true
				return releaseOwner("ChangeDraft.close", (callback) => dbNative.runtimeDraftClose(state.handle, callback))
			}),
		{ interruptible: true }
	)
})

// The draft value → state registry lets the scope finalizer reach the
// native handle without exposing it on the public capability.
const draftStates = new WeakMap<object, DraftState>()

const ChangeSet = Object.freeze({ builder })

export type { ChangeDraft, ChangeSet }
export { ChangeSet, internalChanges }
