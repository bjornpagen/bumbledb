import type { Scope } from "effect"
import { Effect, Exit, Option, Stream } from "effect"
import { isClosedMember, membersAgree } from "./closed.ts"
import type { SchemaId } from "./compile.ts"
import { compiledOf, schemaTables } from "./compile.ts"
import { argumentError, DbError, dbError, internalError } from "./errors.ts"
import type { ChangesRef, DraftRef } from "./native/addon.ts"
import { addon } from "./native/addon.ts"
import type { ChangeCounts, ChangeKindOut, ChangeRecordOut, ChangesOut } from "./native/binding.d.ts"
import { call, drain, release, scoped } from "./native/op.ts"
import { type AnyRelation, type Fact, relationFields } from "./relation.ts"
import type { CellValue } from "./rows.ts"
import { factCellsOf, factOfCells, hostCellCharge } from "./rows.ts"
import { runtimeHandle } from "./runtime.ts"
import type { AnySchema, SchemaRelation } from "./schema.ts"
import { schemaDescriptor } from "./schema.ts"
import type { Rel } from "./shape.ts"
import { bytesValue, recordValue } from "./values.ts"

/**
 * `ChangeSet` — the engine's checked immutable delta:
 * schema-fingerprint-bound canonical native bytes with
 * one-command `(add, remove ∖ add)` normalization and exact same-fact
 * add-wins. Immutable and reusable while open; sealing/submitting it later
 * retains the SAME native value — no second JS row walk ever happens.
 */
/** Counts of distinct fact additions and removals, never input events. */

/** A relation-name-discriminated union of plain, fully typed facts. */
type ChangeRecord<S extends AnySchema> = {
	[N in keyof S["relations"]]: S["relations"][N] extends AnyRelation
		? { readonly relation: N; readonly kind: ChangeKindOut; readonly fact: Fact<S["relations"][N]> }
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
}

/**
 * A scoped, database-free change draft. Each method returns a lazy effect that reads its iterable
 * when it runs (every rerun reads it again); accepted rows are copied, so later mutation of the
 * input changes nothing. A failure, an interruption or concurrent use spends the draft; `finish`
 * consumes it.
 */
interface ChangeDraft<S extends AnySchema> {
	insert<R extends Rel<S>>(relation: R, rows: Iterable<Fact<R>>): Effect.Effect<void, DbError>
	delete<R extends Rel<S>>(relation: R, rows: Iterable<Fact<R>>): Effect.Effect<void, DbError>
	finish(): Effect.Effect<ChangeSet<S>, DbError, Scope.Scope>
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
	readonly handle: DraftRef
	readonly theory: AnySchema
	spent: boolean
	inFlight: boolean
}

function refusal(operation: string, reason: "SpentHandle" | "ClosedHandle" | "InvalidArgument"): DbError {
	return new DbError({ operation, reason: { _tag: reason } })
}

interface Chunk {
	readonly rows: bigint
	readonly cells: CellValue[]
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

function spendAndDrain(state: DraftState): Effect.Effect<void> {
	return Effect.suspend(() => {
		if (state.spent) {
			return Effect.void
		}
		state.spent = true
		return drain((done) => addon.runtimeDraftClose(state.handle, done)).pipe(Effect.asVoid)
	})
}

function ingest(
	state: DraftState,
	operation: "ChangeDraft.insert" | "ChangeDraft.delete",
	relation: AnyRelation,
	rows: Iterable<object>
): Effect.Effect<void, DbError> {
	const verb = operation === "ChangeDraft.insert" ? addon.runtimeDraftInsert : addon.runtimeDraftDelete
	return Effect.gen(function* () {
		if (state.spent) {
			return yield* Effect.fail(refusal(operation, "SpentHandle"))
		}
		if (state.inFlight) {
			// Reentrant construction refuses AND spends/drains — there is
			// no implicit queue.
			yield* spendAndDrain(state)
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
					}).pipe(Effect.catch((error) => spendAndDrain(state).pipe(Effect.andThen(Effect.fail(error)))))
					leftover = chunk.leftover
					done = chunk.done && leftover === undefined
					if (chunk.rows === 0n) {
						continue
					}
					yield* eventLoopTurn()
					yield* call(
						operation,
						(callback) => verb(state.handle, relationId, chunk.rows, chunk.cells, callback),
						(lease) => {
							addon.runtimeStagedTake(lease)
						}
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
			return spendAndDrain(state)
		})
	})
}

function decodeChangeRecord<S extends AnySchema>(
	relations: readonly SchemaRelation[],
	record: ChangeRecordOut
): ChangeRecord<S> {
	const relation = relations[record.relation]
	if (relation === undefined || isClosedMember(relation)) {
		throw internalError("ChangeSet.records: invalid native record descriptor")
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
	handle: ChangesRef,
	relations: readonly SchemaRelation[]
): Stream.Stream<ChangeRecord<S>, DbError> {
	return Stream.unwrap(
		Effect.gen(function* () {
			const cursor = yield* Effect.acquireRelease(
				call(
					"ChangeSet.records",
					(callback) => addon.runtimeChangesCursor(handle, callback),
					addon.runtimeChangesCursorTake
				),
				(value) => release("ChangeCursor.close", (callback) => addon.runtimeChangesCursorClose(value, callback)),
				{ interruptible: true }
			)
			return Stream.paginate(undefined, () =>
				call(
					"ChangeSet.records",
					(callback) => addon.runtimeChangesCursorNext(cursor, callback),
					addon.runtimeChangePageTake
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
	acquire: Effect.Effect<ChangesOut, DbError>
): Effect.Effect<ChangeSet<S>, DbError, Scope.Scope> {
	return Effect.map(
		scoped("ChangeSet.release", acquire, (wire) => (done) => addon.runtimeChangesClose(wire.changes, done)),
		(wire): ChangeSet<S> => new ChangeSetLive(theory, wire, schemaId)
	)
}

class ChangeSetLive<S extends AnySchema> implements ChangeSet<S> {
	readonly #theory: S
	readonly #handle: ChangesRef
	readonly schemaId: SchemaId
	readonly counts: ChangeCounts
	readonly byteLength: bigint

	constructor(theory: S, wire: ChangesOut, schemaId: SchemaId) {
		this.#theory = theory
		this.#handle = wire.changes
		this.schemaId = schemaId
		this.counts = Object.freeze({ ...wire.counts })
		this.byteLength = wire.byteLength
	}

	records(): Stream.Stream<ChangeRecord<S>, DbError> {
		return changeRecords<S>(this.#handle, Object.values(this.#theory.relations)).pipe(
			Stream.withSpan("ChangeSet.records")
		)
	}

	toBytes(): Effect.Effect<Uint8Array, DbError> {
		const handle = this.#handle
		return call("ChangeSet.toBytes", (done) => addon.runtimeChangesBytes(handle, done), addon.runtimeBytesTake).pipe(
			Effect.withSpan("ChangeSet.toBytes")
		)
	}

	compose(other: ChangeSet<S>): Effect.Effect<ChangeSet<S>, DbError, Scope.Scope> {
		const left = this.#handle
		const right = ChangeSetLive.handle(other, this.schemaId)
		if (right === undefined) return Effect.fail(refusal("ChangeSet.compose", "InvalidArgument"))
		return acquireChanges(
			this.#theory,
			this.schemaId,
			call("ChangeSet.compose", (done) => addon.runtimeChangesCompose(left, right, done), addon.runtimeChangesTake)
		).pipe(Effect.withSpan("ChangeSet.compose"))
	}

	/** The native change set behind `value`, when `value` is one this SDK made for `schemaId`. */
	static handle(value: object, schemaId: SchemaId): ChangesRef | undefined {
		if (!(#handle in value)) return undefined
		const changes = value as unknown as ChangeSetLive<AnySchema>
		return changes.schemaId === schemaId ? changes.#handle : undefined
	}
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
						yield* spendAndDrain(state)
						return yield* Effect.fail(refusal("ChangeDraft.finish", "SpentHandle"))
					}
					// Finish CONSUMES the draft, success or failure.
					state.spent = true
					return yield* call(
						"ChangeDraft.finish",
						(callback) => addon.runtimeDraftFinish(state.handle, callback),
						addon.runtimeChangesTake
					)
				})
			)
		}
	}
	return Object.freeze(draft)
}

/**
 * `ChangeSet.builder(schema)` — lazy scoped acquisition of a
 * database-free draft. Requires the acquired
 * `Bumble`; the draft's native resources release with its scope, and
 * the scope finalizer surfaces incomplete/failed teardown as a
 * `CloseFailure` defect.
 */
const builder = Effect.fn("ChangeSet.builder")(function* <S extends AnySchema>(schema: S) {
	const handle = yield* runtimeHandle
	const theory = yield* Effect.try({
		try: () => schemaDescriptor(schema),
		catch: (cause) => argumentError("ChangeSet.builder", cause)
	})
	const compiled = compiledOf(theory)
	const state = yield* scoped(
		"ChangeDraft.release",
		Effect.try({
			try: (): DraftState => ({
				handle: addon.runtimeDraftOpen(handle, compiled.handle),
				theory,
				spent: false,
				inFlight: false
			}),
			catch: (cause) => dbError("ChangeSet.builder", cause)
		}),
		(owned) => (done) => {
			owned.spent = true
			addon.runtimeDraftClose(owned.handle, done)
		}
	)
	return makeDraft(theory as S, state, compiled.schemaId)
})

/** Parse canonical native bytes, owning the input when the Effect starts. */
const fromBytes = Effect.fn("ChangeSet.fromBytes")(function* <S extends AnySchema>(schema: S, bytes: Uint8Array) {
	const ownedBytes = yield* Effect.try({
		try: () => bytesValue("ChangeSet.fromBytes", bytes),
		catch: (cause) => argumentError("ChangeSet.fromBytes", cause)
	})
	const handle = yield* runtimeHandle
	const theory = yield* Effect.try({
		try: () => schemaDescriptor(schema),
		catch: (cause) => argumentError("ChangeSet.fromBytes", cause)
	})
	const compiled = compiledOf(theory)
	return yield* acquireChanges(
		theory as S,
		compiled.schemaId,
		call(
			"ChangeSet.fromBytes",
			(done) => addon.runtimeChangesParse(handle, compiled.handle, ownedBytes, done),
			addon.runtimeChangesTake
		)
	)
})

const ChangeSet = Object.freeze({ builder, fromBytes })

export type { ChangeCounts } from "./native/binding.d.ts"
export type { ChangeDraft, ChangeRecord }
export { ChangeSet, ChangeSetLive }
