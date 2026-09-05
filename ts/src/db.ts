import { Effect, Option } from "effect"
import type { Scope } from "effect"
import { drainClose, releaseOwner } from "#close.ts"
import type { CompiledSchema, SchemaId } from "#compile.ts"
import { Schema as CoreSchema, schemaTables } from "#compile.ts"
import type { ChangeSet } from "#changes.ts"
import { internalChanges } from "#changes.ts"
import type { ApplyOutcomeWire, DbInspectionWire, ExpectedWire, SessionHandle, SnapshotHandle, WitnessWire } from "#db-native.ts"
import { dbNative } from "#db-native.ts"
import { lower } from "#lower.ts"
import type { DbHandle, Violation } from "#native.ts"
import type { AnyQuery } from "#query/lower.ts"
import { lowerQuery } from "#query/lower.ts"
import { wireParams } from "#query/run.ts"
import type { ParamsRecord } from "#query/scope.ts"
import type { Fact } from "#relation.ts"
import type { CompleteResult } from "#result.ts"
import { internalResult, makeCompleteResult } from "#result.ts"
import type { CellValue } from "#rows.ts"
import { factOfCells, keyCellsOf } from "#rows.ts"
import type { CloseReport } from "#runtime-errors.ts"
import { DbError } from "#runtime-errors.ts"
import type { DirectoryHandle } from "#runtime-native.ts"
import { runtimeNative } from "#runtime-native.ts"
import type { ExecutionPolicy } from "#runtime.ts"
import { nativeOperationWith, policyWire, runtimeHandle } from "#runtime.ts"
import type { AnySchema } from "#schema.ts"
import type { Key, QueryTemplate, Rel } from "#shape.ts"

/**
 * The chapter 35 core surface: `Db.create`/`Db.open`, scoped coherent
 * `Snapshot`s, reusable `ExecutionSession`s, the shared `QueryReader`
 * capability, one immutable final-state `apply`, bounded `inspect` and
 * honest `close`. Effect-only: every method constructs a lazy effect; all
 * native work runs on the ONE bounded runtime executor under an explicit
 * `ExecutionPolicy`; resources are scoped with `CloseFailure`-defect
 * finalizers. There is no Promise/sync/disposal twin, no transaction
 * callback, no per-row fiber and no Proxy row anywhere.
 *
 * Database open is the managed two-step handshake (C09/P06.md): acquire the
 * kernel-held directory owner, then open the managed child database under
 * it. `Db.open` NEVER creates on missing/error; `Db.create` refuses
 * existing authority.
 */

/** The core-local witness: catalog/store identity plus generation — never a log StateStamp. */
interface CoreWitness {
	readonly store: string
	readonly generation: bigint
}

type ApplyExpected = { readonly kind: "any" } | { readonly kind: "exact"; readonly at: CoreWitness }

/** Core work policy plus the expected-state intent (chapter 35). */
type ApplyOptions = ExecutionPolicy & { readonly expected: ApplyExpected }

type ApplyOutcome =
	| { readonly kind: "accepted"; readonly witness: CoreWitness }
	| { readonly kind: "no-change"; readonly witness: CoreWitness }
	| { readonly kind: "invariant-rejected"; readonly violations: readonly Violation[] }
	| { readonly kind: "moved"; readonly witnessed: CoreWitness; readonly current: CoreWitness }

/** Bounded database diagnostics: measurements, never retained rows. */
interface DbInspection {
	readonly schemaId: SchemaId
	readonly generation: bigint
	readonly mapBytes: bigint
	readonly populatedBytes: bigint
	readonly diskBytes: bigint
	readonly residentEstimateBytes: bigint
	readonly retainedOperations: bigint
}

/**
 * The one shared read capability (chapter 30): the same typed `get` and
 * `execute` on a core snapshot and on a log published snapshot, so a
 * cross-package read helper takes this interface with no adapter. Missing
 * key is `Option.none`, never a fake I/O error or nullable row. It carries
 * no writable authority — never a `Db`, an `apply`, or a raw transaction.
 */
interface QueryReader<S extends AnySchema> {
	get<R extends Rel<S>>(relation: R, key: Key<R>, work: ExecutionPolicy): Effect.Effect<Option.Option<Fact<R>>, DbError>
	execute<P extends ParamsRecord, A>(
		query: QueryTemplate<S, P, A>,
		params: P,
		work: ExecutionPolicy
	): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope>
}

interface Snapshot<S extends AnySchema> extends QueryReader<S> {
	readonly witness: CoreWitness
	session(work: ExecutionPolicy): Effect.Effect<ExecutionSession<S>, DbError, Scope.Scope>
	close(): Effect.Effect<CloseReport>
}

interface ExecutionSession<S extends AnySchema> {
	execute<P extends ParamsRecord, A>(
		query: QueryTemplate<S, P, A>,
		params: P,
		work: ExecutionPolicy
	): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope>
	close(): Effect.Effect<CloseReport>
}

interface Db<S extends AnySchema> {
	readonly schemaId: SchemaId
	snapshot(work: ExecutionPolicy): Effect.Effect<Snapshot<S>, DbError, Scope.Scope>
	apply(changes: ChangeSet<S>, options: ApplyOptions): Effect.Effect<ApplyOutcome, DbError>
	inspect(work: ExecutionPolicy): Effect.Effect<DbInspection, DbError>
	close(): Effect.Effect<CloseReport>
}

function refusal(operation: string, reason: "InvalidArgument" | "Incompatible" | "ClosedHandle"): DbError {
	return new DbError({ operation, reason: { _tag: reason } })
}

function witnessOf(wire: WitnessWire): CoreWitness {
	return Object.freeze({ store: wire.store, generation: wire.generation })
}

function outcomeOf(wire: ApplyOutcomeWire): ApplyOutcome {
	switch (wire.tag) {
		case "accepted":
			return Object.freeze({ kind: "accepted", witness: witnessOf(wire.witness) })
		case "no-change":
			return Object.freeze({ kind: "no-change", witness: witnessOf(wire.witness) })
		case "invariant-rejected":
			return Object.freeze({ kind: "invariant-rejected", violations: wire.violations })
		case "moved":
			return Object.freeze({ kind: "moved", witnessed: witnessOf(wire.witnessed), current: witnessOf(wire.current) })
	}
}

/**
 * Validates the query template's schema binding (identity, the membership
 * rule) and lowers it to IR plus wire params. Pure host preparation; the
 * engine's IR validation under budget remains the authority.
 */
function preparedOf<S extends AnySchema>(
	theory: S,
	query: AnyQuery,
	params: Readonly<Record<string, unknown>>
): { readonly ir: ReturnType<typeof lowerQuery>; readonly wire: ReturnType<typeof wireParams>; readonly finds: AnyQuery["data"]["finds"] } {
	if (query.schema !== theory) {
		throw refusal("QueryReader.execute", "Incompatible")
	}
	const ir = lowerQuery(query)
	const wire = wireParams(query.data.params, params)
	return { ir, wire, finds: query.data.finds }
}

interface SnapshotState<S extends AnySchema> {
	readonly theory: S
	readonly handle: SnapshotHandle
}

function executeOn<S extends AnySchema, A>(
	state: SnapshotState<S>,
	session: SessionHandle | undefined,
	query: AnyQuery,
	params: Readonly<Record<string, unknown>>,
	work: ExecutionPolicy
): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope> {
	return Effect.acquireRelease(
		Effect.gen(function* () {
			const prepared = yield* Effect.try({
				try: () => preparedOf(state.theory, query, params),
				catch: (cause) => (cause instanceof DbError ? cause : refusal("QueryReader.execute", "InvalidArgument"))
			})
			const wire = policyWire(work, "QueryReader.execute")
			const handle = yield* nativeOperationWith(
				"QueryReader.execute",
				(callback) =>
					session === undefined
						? dbNative.runtimeSnapshotExecute(state.handle, wire, prepared.ir, prepared.wire, callback)
						: dbNative.runtimeSessionExecute(session, wire, prepared.ir, prepared.wire, callback),
				dbNative.runtimeResultTake,
				(value) => value
			)
			return makeCompleteResult<A>(handle, prepared.finds, work)
		}),
		(result) =>
			Effect.suspend(() => {
				const internal = internalResultHandle(result)
				if (internal === undefined) {
					return Effect.void
				}
				return releaseOwner("CompleteResult.close", (callback) => dbNative.runtimeResultClose(internal, callback))
			}),
		{ interruptible: true }
	)
}

function internalResultHandle(result: object) {
	return internalResult(result)?.handle
}

function getOn<S extends AnySchema, R extends Rel<S>>(
	state: SnapshotState<S>,
	relation: R,
	key: Key<R>,
	work: ExecutionPolicy
): Effect.Effect<Option.Option<Fact<R>>, DbError> {
	return Effect.gen(function* () {
		if (state.theory.relations[relation.name] !== relation) {
			return yield* Effect.fail(refusal("QueryReader.get", "InvalidArgument"))
		}
		const tables = schemaTables(state.theory)
		const relationId = tables.relationIds.get(relation.name)
		const primary = tables.primaryKeys.get(relation.name)
		if (relationId === undefined || primary === undefined) {
			return yield* Effect.fail(refusal("QueryReader.get", "InvalidArgument"))
		}
		const cells = yield* Effect.try({
			try: () => keyCellsOf(relation.data, primary.projection, key as Readonly<Record<string, unknown>>),
			catch: (cause) => (cause instanceof DbError ? cause : refusal("QueryReader.get", "InvalidArgument"))
		})
		const row = yield* nativeOperationWith(
			"QueryReader.get",
			(callback) =>
				dbNative.runtimeSnapshotGet(
					state.handle,
					policyWire(work, "QueryReader.get"),
					relationId,
					primary.statementId,
					cells,
					callback
				),
			dbNative.runtimeRowTake,
			(value) => value
		)
		if (row === null) {
			return Option.none<Fact<R>>()
		}
		return Option.some(factOfCells(relation, row as readonly CellValue[]))
	})
}

function makeSession<S extends AnySchema>(state: SnapshotState<S>, handle: SessionHandle): ExecutionSession<S> {
	const session: ExecutionSession<S> = {
		execute(query, params, work) {
			return executeOn(state, handle, query, params, work)
		},
		close() {
			return drainClose("ExecutionSession.close", (callback) => dbNative.runtimeSessionClose(handle, callback))
		}
	}
	Object.freeze(session)
	sessionHandles.set(session, handle)
	return session
}

function makeSnapshot<S extends AnySchema>(theory: S, handle: SnapshotHandle, witness: CoreWitness): Snapshot<S> {
	const state: SnapshotState<S> = { theory, handle }
	const snapshot: Snapshot<S> = {
		witness,
		get(relation, key, work) {
			return getOn(state, relation, key, work)
		},
		execute(query, params, work) {
			return executeOn(state, undefined, query, params, work)
		},
		session(work) {
			return Effect.acquireRelease(
				nativeOperationWith(
					"Snapshot.session",
					(callback) => dbNative.runtimeSnapshotSession(handle, policyWire(work, "Snapshot.session"), callback),
					dbNative.runtimeSessionTake,
					(value) => makeSession(state, value)
				),
				(session) =>
					Effect.suspend(() =>
						releaseOwner("ExecutionSession.close", (callback) =>
							dbNative.runtimeSessionClose(sessionHandles.get(session) ?? missingSession(), callback)
						)
					),
				{ interruptible: true }
			)
		},
		close() {
			return drainClose("Snapshot.close", (callback) => dbNative.runtimeSnapshotClose(handle, callback))
		}
	}
	Object.freeze(snapshot)
	snapshotHandles.set(snapshot, handle)
	return snapshot
}

const sessionHandles = new WeakMap<object, SessionHandle>()

function missingSession(): never {
	throw new DbError({ operation: "ExecutionSession.close", reason: { _tag: "Internal" } })
}

interface DbState {
	readonly theory: AnySchema
	readonly db: DbHandle
	readonly directory: DirectoryHandle
	readonly schemaId: SchemaId
}

function makeDb<S extends AnySchema>(theory: S, state: DbState): Db<S> {
	const value: Db<S> = {
		schemaId: state.schemaId,
		snapshot(work) {
			return Effect.acquireRelease(
				nativeOperationWith(
					"Db.snapshot",
					(callback) => dbNative.runtimeDbSnapshot(state.db, policyWire(work, "Db.snapshot"), callback),
					dbNative.runtimeSnapshotTake,
					(wire) => makeSnapshot(theory, wire.snapshot, witnessOf(wire.witness))
				),
				(snapshot) =>
					Effect.suspend(() =>
						releaseOwner("Snapshot.close", (callback) =>
							dbNative.runtimeSnapshotClose(snapshotHandles.get(snapshot) ?? missingSnapshot(), callback)
						)
					),
				{ interruptible: true }
			)
		},
		apply(changes, options) {
			return Effect.gen(function* () {
				const internal = internalChanges(changes)
				if (internal === undefined) {
					// A foreign object supplied dynamically refuses before
					// any native dispatch.
					return yield* Effect.fail(refusal("Db.apply", "InvalidArgument"))
				}
				if (internal.schemaId !== state.schemaId) {
					return yield* Effect.fail(refusal("Db.apply", "Incompatible"))
				}
				const expected: ExpectedWire =
					options.expected.kind === "any"
						? { kind: "any" }
						: { kind: "exact", store: options.expected.at.store, generation: options.expected.at.generation }
				return yield* nativeOperationWith(
					"Db.apply",
					(callback) =>
						dbNative.runtimeDbApply(state.db, policyWire(options, "Db.apply"), internal.handle, expected, callback),
					dbNative.runtimeApplyTake,
					outcomeOf
				)
			})
		},
		inspect(work) {
			return nativeOperationWith(
				"Db.inspect",
				(callback) => dbNative.runtimeDbInspect(state.db, policyWire(work, "Db.inspect"), callback),
				dbNative.runtimeDbInspectTake,
				(wire: DbInspectionWire): DbInspection =>
					Object.freeze({
						schemaId: state.schemaId,
						generation: wire.generation,
						mapBytes: wire.mapBytes,
						populatedBytes: wire.populatedBytes,
						diskBytes: wire.diskBytes,
						residentEstimateBytes: wire.residentEstimateBytes,
						retainedOperations: wire.retainedOperations
					})
			)
		},
		close() {
			// The one close authority: database child first, then the
			// directory owner releases its kernel lock LAST (C09). Both
			// joins are idempotent natively.
			return drainClose("Db.close", (callback) => dbNative.runtimeManagedDbClose(state.db, callback)).pipe(
				Effect.flatMap((report) =>
					drainClose("Db.directoryClose", (callback) => runtimeNative.runtimeDirectoryClose(state.directory, false, callback)).pipe(
						Effect.map((directoryReport) => (report.kind === "closed" ? directoryReport : report))
					)
				)
			)
		}
	}
	Object.freeze(value)
	return value
}

const snapshotHandles = new WeakMap<object, SnapshotHandle>()

function missingSnapshot(): never {
	throw new DbError({ operation: "Snapshot.close", reason: { _tag: "Internal" } })
}

/** The managed child name under the database directory owner. */
const CHILD = "store"

function openDatabase<S extends AnySchema>(
	operation: "Db.create" | "Db.open",
	path: string,
	schema: S,
	work: ExecutionPolicy,
	create: boolean
): Effect.Effect<Db<S>, DbError, Scope.Scope> {
	return Effect.gen(function* () {
		const runtime = yield* runtimeHandle()
		const compiled: CompiledSchema<S> = yield* CoreSchema.compile(schema, work)
		const spec = lower(schema)
		const wire = policyWire(work, operation)
		return yield* Effect.acquireRelease(
			Effect.gen(function* () {
				const directory = yield* nativeOperationWith(
					operation,
					(callback) => runtimeNative.runtimeDirectoryAcquire(runtime, wire, path, callback),
					runtimeNative.runtimeDirectoryTake,
					(value) => value
				)
				const outcome = yield* nativeOperationWith(
					operation,
					(callback) => runtimeNative.runtimeDirectoryDbOpen(directory, wire, CHILD, spec, create, callback),
					runtimeNative.runtimeDbTake,
					(value) => value
				).pipe(
					Effect.catch((error) =>
						// The directory owner never leaks past a failed child
						// open: release the kernel lock, then fail.
						drainClose(operation, (callback) => runtimeNative.runtimeDirectoryClose(directory, false, callback)).pipe(
							Effect.andThen(Effect.fail(error))
						)
					)
				)
				if (outcome.tag !== "accepted") {
					yield* drainClose(operation, (callback) => runtimeNative.runtimeDirectoryClose(directory, false, callback))
					// Schema rejection/refusal is an operational open failure
					// here (a store admission refusal), reported typed.
					return yield* Effect.fail(refusal(operation, "Incompatible"))
				}
				const state: DbState = { theory: schema, db: outcome.db, directory, schemaId: compiled.schemaId }
				const db = makeDb(schema, state)
				dbStates.set(db, state)
				return db
			}),
			(db) =>
				Effect.suspend(() => {
					const state = dbStates.get(db)
					if (state === undefined) {
						return Effect.void
					}
					return releaseOwner("Db.close", (callback) => dbNative.runtimeManagedDbClose(state.db, callback)).pipe(
						Effect.ensuring(
							releaseOwner("Db.directoryClose", (callback) =>
								runtimeNative.runtimeDirectoryClose(state.directory, false, callback)
							)
						)
					)
				}),
			{ interruptible: true }
		)
	})
}

const dbStates = new WeakMap<object, DbState>()

/**
 * `Db.create` is the explicit constructor and refuses existing authority;
 * `Db.open` of a missing or unreadable database never creates an empty
 * replacement (chapter 30). Both compile the schema through the same
 * implementation as `Schema.compile` — prior compilation is optional.
 */
const Db = Object.freeze({
	create<S extends AnySchema>(path: string, schema: S, work: ExecutionPolicy) {
		return openDatabase("Db.create", path, schema, work, true)
	},
	open<S extends AnySchema>(path: string, schema: S, work: ExecutionPolicy) {
		return openDatabase("Db.open", path, schema, work, false)
	}
})

/**
 * Private log-integration seam (C10): wraps a PUBLISHED core snapshot
 * handle — minted by the internal log machine's native open/snapshot verbs
 * — in the exact core `QueryReader` plus the scoped session acquisition
 * (`PublishedSnapshot extends QueryReader` in chapter 35; the log adds
 * identity/stamps/freshness AROUND this capability, never a second reader).
 * The argument is the log package's branded handle for the same native
 * registry entry, so the one cast below is a cross-package respelling of
 * one native capability — the native side re-judges kind/generation/owner
 * on every verb, so a forged object refuses there, typed.
 */
function internalPublishedReader<S extends AnySchema>(
	core: object,
	theory: S
): {
	readonly get: QueryReader<S>["get"]
	readonly execute: QueryReader<S>["execute"]
	readonly session: (work: ExecutionPolicy) => Effect.Effect<ExecutionSession<S>, DbError, Scope.Scope>
} {
	const state: SnapshotState<S> = { theory, handle: core as SnapshotHandle }
	return Object.freeze({
		get<R extends Rel<S>>(relation: R, key: Key<R>, work: ExecutionPolicy) {
			return getOn(state, relation, key, work)
		},
		execute<P extends ParamsRecord, A>(queryValue: QueryTemplate<S, P, A>, params: P, work: ExecutionPolicy) {
			return executeOn<S, A>(state, undefined, queryValue as AnyQuery, params, work)
		},
		session(work: ExecutionPolicy) {
			return Effect.acquireRelease(
				nativeOperationWith(
					"Snapshot.session",
					(callback) =>
						dbNative.runtimeSnapshotSession(state.handle, policyWire(work, "Snapshot.session"), callback),
					dbNative.runtimeSessionTake,
					(value) => makeSession(state, value)
				),
				(session) =>
					Effect.suspend(() =>
						releaseOwner("ExecutionSession.close", (callback) =>
							dbNative.runtimeSessionClose(sessionHandles.get(session) ?? missingSession(), callback)
						)
					),
				{ interruptible: true }
			)
		}
	})
}

export type {
	ApplyExpected,
	ApplyOptions,
	ApplyOutcome,
	CoreWitness,
	DbInspection,
	ExecutionSession,
	QueryReader,
	Snapshot
}
export { Db, internalPublishedReader }
