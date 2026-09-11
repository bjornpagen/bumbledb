import type { Scope } from "effect"
import { Effect, Exit, Option, Stream } from "effect"
import { drainClose, releaseOwner } from "#close.ts"
import { isClosedMember, membersAgree } from "#closed.ts"
import type { SchemaId } from "#compile.ts"
import { Schema as CoreSchema, schemaTables } from "#compile.ts"
import type { ChangeRecordWire, ChangesHandle, ChangesWire, DraftHandle } from "#db-native.ts"
import { dbNative } from "#db-native.ts"
import { SdkInvariantError } from "#errors.ts"
import { lower } from "#lower.ts"
import { type AnyRelation, type Fact, relationFields } from "#relation.ts"
import type { CellValue } from "#rows.ts"
import { factCellsOf, factOfCells, hostCellCharge } from "#rows.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import type { CloseReport } from "#runtime-errors.ts"
import { argumentError, DbError } from "#runtime-errors.ts"
import type { AnySchema, SchemaRelation } from "#schema.ts"
import type { Rel } from "#shape.ts"
import { bytesValue, recordValue } from "#values.ts"

/**
 * `ChangeSet` — the engine's checked immutable delta:
 * schema-fingerprint-bound canonical native bytes with
 * one-command `(add, remove ∖ add)` normalization and exact same-fact
 * add-wins. Immutable and reusable while open; sealing/submitting it later
 * retains the SAME native value — no second JS row walk ever happens.
 */
/** Counts of distinct fact additions and removals, never input events. */
interface ChangeCounts {
	readonly added: bigint
	readonly removed: bigint
}

/** A relation-name-discriminated union of plain, fully typed facts. */
type ChangeRecord<S extends AnySchema> = {
	[N in keyof S["relations"]]: S["relations"][N] extends AnyRelation
		? { readonly relation: N; readonly kind: "add" | "remove"; readonly fact: Fact<S["relations"][N]> }
		: never
}[keyof S["relations"]]

interface ChangeSet<S extends AnySchema> {
	readonly schemaId: SchemaId
	/** Distinct requested actions; use judge for effective counts against a store. */
	readonly counts: ChangeCounts
	readonly byteLength: bigint
	/** Phantom schema brand: a ChangeSet is only ever applied to its own S. */
	readonly schema?: S
	/** Independent scoped traversal, delivered in bounded native batches. Reusable. */
	records(): Stream.Stream<ChangeRecord<S>, DbError>
	/** Explicit full byte materialization. Mutating returned bytes cannot alter this value. */
	toBytes(): Effect.Effect<Uint8Array, DbError>
	/** Commutative add-wins composition into one command, not sequential replay. */
	compose(other: ChangeSet<S>): Effect.Effect<ChangeSet<S>, DbError, Scope.Scope>
	close(): Effect.Effect<CloseReport>
}

/**
 * `ChangeDraft` — the scoped, database-free construction capability.
 * Every method constructs a LAZY effect; execution reads the then-current
 * iterable again on every sequential rerun (no hidden
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
 * granularity targets, not database-size limits. One larger row travels
 * alone; otherwise native callbacks provide backpressure between batches.
 */
const CHUNK_BYTES = 65536n
const CHUNK_ROWS = 4096

interface DraftState {
	readonly handle: DraftHandle
	readonly theory: AnySchema
	spent: boolean
	inFlight: boolean
}

interface ChangesInternal {
	readonly handle: ChangesHandle
	readonly schemaId: SchemaId
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
	const fields = relationFields(relation)
	let bytes = 0n
	for (const declared of fields) {
		const value = record[declared.name]
		bytes += hostCellCharge(value)
	}
	return bytes
}

/**
 * Pulls one bounded chunk off the caller's iterator. Host length is judged
 * before any string scan or byte copy. A leftover fact that does not fit
 * the current chunk is returned unconverted for the next turn.
 */
function pullChunk(relation: AnyRelation, iterator: Iterator<object>, pending: object | undefined): Chunk {
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
		const record = recordValue(
			`relation ${relation.name}`,
			current,
			relationFields(relation).map((field) => field.name)
		)
		const charge = hostFactCharge(relation, record)
		if (rows > 0n && bytes + charge > CHUNK_BYTES) {
			leftover = current
			break
		}
		for (const cell of factCellsOf(relation, record)) cells.push(cell)
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
			// no implicit queue.
			yield* spendAndDrain(state, operation)
			return yield* Effect.fail(refusal(operation, "SpentHandle"))
		}
		if (!membersAgree(state.theory.relations[relation.name], relation)) {
			return yield* Effect.fail(refusal(operation, "InvalidArgument"))
		}
		const tables = schemaTables(state.theory)
		const relationId = tables.relationIds.get(relation.name)
		if (relationId === undefined) {
			return yield* Effect.fail(refusal(operation, "InvalidArgument"))
		}
		state.inFlight = true
		const body = Effect.gen(function* () {
			const iterator = yield* Effect.try({
				try: () => rows[Symbol.iterator](),
				catch: (cause) => argumentError(operation, cause)
			})
			let leftover: object | undefined
			let done = false
			return yield* Effect.gen(function* () {
				while (!done) {
					const chunk = yield* Effect.try({
						try: () => pullChunk(relation, iterator, leftover),
						catch: (cause) => argumentError(operation, cause)
					}).pipe(Effect.catch((error) => spendAndDrain(state, operation).pipe(Effect.andThen(Effect.fail(error)))))
					leftover = chunk.leftover
					done = chunk.done && leftover === undefined
					if (chunk.rows === 0n) {
						continue
					}
					yield* eventLoopTurn()
					yield* nativeOperationWith(
						operation,
						(callback) => verb(state.handle, relationId, chunk.rows, chunk.cells, callback),
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
			}).pipe(
				Effect.ensuring(
					Effect.sync(() => {
						if (!done) iterator.return?.()
					})
				)
			)
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

function decodeChangeRecord<S extends AnySchema>(
	relations: readonly SchemaRelation[],
	record: ChangeRecordWire
): ChangeRecord<S> {
	const relation = relations[record.relation]
	if (relation === undefined || isClosedMember(relation) || (record.kind !== "add" && record.kind !== "remove")) {
		throw new SdkInvariantError({ message: "ChangeSet.records: invalid native record descriptor" })
	}
	// Relation id resolves through this schema's own ordered roster. The
	// shared projector validates all field values before this union seam.
	return Object.freeze({
		relation: relation.name,
		kind: record.kind,
		fact: factOfCells(relation, record.values)
	}) as ChangeRecord<S>
}

function changeRecords<S extends AnySchema>(
	handle: ChangesHandle,
	relations: readonly SchemaRelation[]
): Stream.Stream<ChangeRecord<S>, DbError> {
	return Stream.unwrap(
		Effect.gen(function* () {
			const cursor = yield* Effect.acquireRelease(
				nativeOperationWith(
					"ChangeSet.records",
					(callback) => dbNative.runtimeChangesCursor(handle, callback),
					dbNative.runtimeChangesCursorTake,
					(value) => value
				),
				(value) =>
					releaseOwner("ChangeCursor.close", (callback) => dbNative.runtimeChangesCursorClose(value, callback)),
				{ interruptible: true }
			)
			return Stream.paginate(undefined, () =>
				nativeOperationWith(
					"ChangeSet.records",
					(callback) => dbNative.runtimeChangesCursorNext(cursor, callback),
					dbNative.runtimeChangePageTake,
					(page) => page
				).pipe(
					Effect.map((page) =>
						page === null
							? ([[], Option.none<undefined>()] as const)
							: ([page.map((record) => decodeChangeRecord<S>(relations, record)), Option.some(undefined)] as const)
					)
				)
			)
		})
	)
}

function acquireChanges<S extends AnySchema>(
	theory: S,
	schemaId: SchemaId,
	acquire: Effect.Effect<ChangesWire, DbError>
): Effect.Effect<ChangeSet<S>, DbError, Scope.Scope> {
	return Effect.acquireRelease(
		acquire.pipe(Effect.map((wire) => makeChangeSet(theory, wire, schemaId))),
		(changes) =>
			Effect.suspend(() => {
				const internal = changesInternals.get(changes)
				if (internal === undefined) return Effect.void
				return releaseOwner("ChangeSet.close", (callback) => dbNative.runtimeChangesClose(internal.handle, callback))
			}),
		{ interruptible: true }
	)
}

function makeChangeSet<S extends AnySchema>(theory: S, wire: ChangesWire, schemaId: SchemaId): ChangeSet<S> {
	const handle = wire.changes
	const relations = Object.values(theory.relations)
	const internal: ChangesInternal = { handle, schemaId }
	const value: ChangeSet<S> = {
		schemaId,
		counts: Object.freeze({ ...wire.counts }),
		byteLength: wire.byteLength,
		records() {
			return changeRecords<S>(handle, relations)
		},
		toBytes() {
			return nativeOperationWith(
				"ChangeSet.toBytes",
				(callback) => dbNative.runtimeChangesBytes(handle, callback),
				dbNative.runtimeBytesTake,
				(bytes) => bytes
			)
		},
		compose(other) {
			return Effect.suspend(() => {
				const right = internalChanges(other)
				if (right === undefined || right.schemaId !== schemaId)
					return Effect.fail(refusal("ChangeSet.compose", "InvalidArgument"))
				return acquireChanges(
					theory,
					schemaId,
					nativeOperationWith(
						"ChangeSet.compose",
						(callback) => dbNative.runtimeChangesCompose(handle, right.handle, callback),
						dbNative.runtimeChangesTake,
						(result) => result
					)
				)
			})
		},
		close() {
			return drainClose("ChangeSet.close", (callback) => dbNative.runtimeChangesClose(handle, callback))
		}
	}
	Object.freeze(value)
	changesInternals.set(value, internal)
	return value
}

function makeDraft<S extends AnySchema>(theory: S, state: DraftState, schemaId: SchemaId): ChangeDraft<S> {
	const draft: ChangeDraft<S> = {
		insert(relation, rows) {
			return ingest(state, "ChangeDraft.insert", relation, rows)
		},
		delete(relation, rows) {
			return ingest(state, "ChangeDraft.delete", relation, rows)
		},
		finish() {
			return acquireChanges(
				theory,
				schemaId,
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
					return yield* nativeOperationWith(
						"ChangeDraft.finish",
						(callback) => dbNative.runtimeDraftFinish(state.handle, callback),
						dbNative.runtimeChangesTake,
						(value) => value
					)
				})
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
 * `ChangeSet.builder(schema)` — lazy scoped acquisition of a
 * database-free draft. Requires the acquired
 * `NativeRuntime`; the draft's native resources release with its scope, and
 * the scope finalizer surfaces incomplete/failed teardown as a
 * `CloseFailure` defect.
 */
const builder = Effect.fn("ChangeSet.builder")(function* <S extends AnySchema>(schema: S) {
	const handle = yield* runtimeHandle()
	const compiled = yield* CoreSchema.compile(schema)
	const spec = lower(compiled.schema)
	return yield* Effect.acquireRelease(
		Effect.gen(function* () {
			const draftHandle = yield* nativeOperationWith(
				"ChangeSet.builder",
				(callback) => dbNative.runtimeDraftOpen(handle, spec, callback),
				dbNative.runtimeDraftTake,
				(value) => value
			)
			const state: DraftState = {
				handle: draftHandle,
				theory: compiled.schema,
				spent: false,
				inFlight: false
			}
			const draft = makeDraft(compiled.schema, state, compiled.schemaId)
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

/** Parse canonical native bytes, owning the input when the Effect starts. */
const fromBytes = Effect.fn("ChangeSet.fromBytes")(function* <S extends AnySchema>(schema: S, bytes: Uint8Array) {
	const ownedBytes = yield* Effect.try({
		try: () => bytesValue("ChangeSet.fromBytes", bytes),
		catch: (cause) => argumentError("ChangeSet.fromBytes", cause)
	})
	const handle = yield* runtimeHandle()
	const compiled = yield* CoreSchema.compile(schema)
	return yield* acquireChanges(
		compiled.schema,
		compiled.schemaId,
		nativeOperationWith(
			"ChangeSet.fromBytes",
			(callback) => dbNative.runtimeChangesParse(handle, lower(compiled.schema), ownedBytes, callback),
			dbNative.runtimeChangesTake,
			(wire) => wire
		)
	)
})

const ChangeSet = Object.freeze({ builder, fromBytes })

export type { ChangeCounts, ChangeDraft, ChangeRecord }
export { ChangeSet, internalChanges }
