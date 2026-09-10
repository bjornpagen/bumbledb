import type { Scope } from "effect"
import { Effect, Option } from "effect"
import type { ChangeCounts, ChangeSet } from "#changes.ts"
import { internalChanges } from "#changes.ts"
import { drainClose, releaseOwner } from "#close.ts"
import { membersAgree } from "#closed.ts"
import type { CompiledSchema, SchemaId } from "#compile.ts"
import { Schema as CoreSchema, declaredKey, schemaTables } from "#compile.ts"
import type {
	ApplyOutcomeWire,
	DbInspectionWire,
	ExpectedWire,
	JudgeOutcomeWire,
	PreparedHandle,
	SnapshotHandle,
	WitnessWire
} from "#db-native.ts"
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
import type { NativeRuntime } from "#runtime.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import type { CloseReport } from "#runtime-errors.ts"
import { argumentError, DbError } from "#runtime-errors.ts"
import type { DirectoryHandle } from "#runtime-native.ts"
import { runtimeNative } from "#runtime-native.ts"
import type { AnySchema } from "#schema.ts"
import { schemaDescriptor, schemasAgree } from "#schema.ts"
import type { Key, QueryTemplate, Rel } from "#shape.ts"
import type { KeyStatement } from "#statements.ts"
import { integerValue, recordValue } from "#values.ts"

/**
 * The core surface: `Db.create`/`Db.open`, scoped coherent
 * `Snapshot`s, reusable `PreparedQuery`s, the shared `QueryReader`
 * capability, one immutable final-state `apply`, bounded `inspect` and
 * honest `close`. Effect-only: every method constructs a lazy effect; all
 * native work runs on the shared executor with cooperative cancellation.
 * Resources are scoped with `CloseFailure`-defect
 * finalizers. There is no Promise/sync/disposal twin, no transaction
 * callback, no per-row fiber and no Proxy row anywhere.
 *
 * Database open is the managed two-step handshake: acquire the kernel-held
 * directory owner and register its finalizer before the interruptible
 * child-open step. `Db.open` NEVER creates on missing/error; `Db.create`
 * refuses existing authority. Published log snapshots use this same
 * `QueryReader` literally — no second reader or adapter.
 */

/** The core-local witness: catalog/store identity plus generation — never a log StateStamp. */
interface CoreWitness {
	readonly store: string
	readonly generation: bigint
}

type WriteExpected = { readonly kind: "any" } | { readonly kind: "exact"; readonly at: CoreWitness }

/** Expected-state intent shared by judgment and application. */
type WriteOptions = { readonly expected: WriteExpected }

type ApplyOutcome =
	| { readonly kind: "accepted"; readonly witness: CoreWitness }
	| { readonly kind: "no-change"; readonly witness: CoreWitness }
	| { readonly kind: "invariant-rejected"; readonly violations: readonly Violation[] }
	| { readonly kind: "moved"; readonly witnessed: CoreWitness; readonly current: CoreWitness }

/** A judgment against the actual current base, not a promise about a later apply.
 * A moved witness is unjudged and therefore carries no made-up change counts.
 */
type JudgeOutcome =
	| { readonly kind: "admitted"; readonly base: CoreWitness; readonly changes: ChangeCounts }
	| {
			readonly kind: "invariant-rejected"
			readonly base: CoreWitness
			readonly changes: ChangeCounts
			readonly violations: readonly Violation[]
	  }
	| { readonly kind: "moved"; readonly witnessed: CoreWitness; readonly current: CoreWitness }

/** Storage measurements, not heap usage or mapped-page residency. */
interface StorageInspection {
	/** Reserved virtual address range for the LMDB mapping. */
	readonly virtualMapBytes: bigint
	/** File length; may include sparse regions and free pages. */
	readonly populatedFileBytes: bigint
	/** LMDB's non-free branch, leaf and overflow pages. Not resident RAM. */
	readonly nonFreePageBytes: bigint
	/** Allocated filesystem blocks, or null when the OS cannot report them. */
	readonly allocatedDiskBytes: bigint | null
}

/** Database diagnostics: measurements, never retained rows. */
interface DbInspection {
	readonly schemaId: SchemaId
	readonly generation: bigint
	readonly storage: StorageInspection
	readonly retainedOperations: bigint
}

/**
 * The shared read capability: the same typed `get` and
 * `execute` on a core snapshot and on a log published snapshot, so a
 * cross-package read helper takes this interface with no adapter. Missing
 * key is `Option.none`, never a fake I/O error or nullable row. It carries
 * no writable authority — never a `Db`, an `apply`, or a raw transaction.
 */
interface QueryReader<S extends AnySchema> {
	get<K extends KeyStatement<Rel<S>, readonly string[]>>(
		key: K,
		value: NoInfer<Key<K>>
	): Effect.Effect<Option.Option<Fact<K["owner"]>>, DbError>
	execute<P extends ParamsRecord, A>(
		query: QueryTemplate<S, P, A>,
		params: P
	): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope>
	prepare<P extends ParamsRecord, A>(
		query: QueryTemplate<S, P, A>
	): Effect.Effect<PreparedQuery<P, A>, DbError, Scope.Scope>
}

interface Snapshot<S extends AnySchema> extends QueryReader<S> {
	readonly witness: CoreWitness
	close(): Effect.Effect<CloseReport>
}

/** One compiled plan and its reusable buffers, pinned to the preparing snapshot.
 * Closing it releases its own state, not the snapshot or completed results.
 */
interface PreparedQuery<P extends ParamsRecord, A> {
	execute(params: P): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope>
	/** Drop reusable execution buffers, keeping the compiled query and snapshot. */
	releaseMemory(): Effect.Effect<void, DbError>
	close(): Effect.Effect<CloseReport>
}

interface Db<S extends AnySchema> {
	readonly schemaId: SchemaId
	snapshot(): Effect.Effect<Snapshot<S>, DbError, Scope.Scope>
	apply(changes: ChangeSet<S>, options: WriteOptions): Effect.Effect<ApplyOutcome, DbError>
	/** Briefly acquires the writer, judges and aborts. Stores no facts or generation. */
	judge(changes: ChangeSet<S>, options: WriteOptions): Effect.Effect<JudgeOutcome, DbError>
	inspect(): Effect.Effect<DbInspection, DbError>
	/** Clear shared query caches without invalidating live snapshots or results. */
	clearCache(): Effect.Effect<void, DbError>
	close(): Effect.Effect<CloseReport>
}

function refusal(operation: string, reason: "InvalidArgument" | "ClosedHandle"): DbError {
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

function judgmentOf(wire: JudgeOutcomeWire): JudgeOutcome {
	switch (wire.tag) {
		case "admitted":
			return Object.freeze({ kind: "admitted", base: witnessOf(wire.base), changes: Object.freeze(wire.changes) })
		case "invariant-rejected":
			return Object.freeze({
				kind: "invariant-rejected",
				base: witnessOf(wire.base),
				changes: Object.freeze(wire.changes),
				violations: wire.violations
			})
		case "moved":
			return Object.freeze({ kind: "moved", witnessed: witnessOf(wire.witnessed), current: witnessOf(wire.current) })
	}
}

function writeInputs(state: DbState, changes: object, options: WriteOptions, operation: string) {
	return Effect.try({
		try: () => {
			const internal = internalChanges(changes)
			if (internal === undefined || internal.schemaId !== state.schemaId) {
				throw refusal(operation, "InvalidArgument")
			}
			const record = recordValue("write options", options, ["expected"])
			const intent = recordValue(
				"expected state",
				record.expected,
				options.expected.kind === "any" ? ["kind"] : ["kind", "at"]
			)
			let expected: ExpectedWire
			if (intent.kind === "any") {
				expected = { kind: "any" }
			} else if (intent.kind === "exact") {
				const at = recordValue("expected witness", intent.at, ["store", "generation"])
				if (typeof at.store !== "string" || !/^[0-9a-f]{32}$/.test(at.store)) {
					throw refusal(operation, "InvalidArgument")
				}
				expected = {
					kind: "exact",
					store: at.store,
					generation: integerValue("expected generation", "u64", at.generation)
				}
			} else {
				throw refusal(operation, "InvalidArgument")
			}
			return { handle: internal.handle, expected }
		},
		catch: (cause) => argumentError(operation, cause)
	})
}

/**
 * Validates the query template's ordered logical schema and lowers it to
 * IR plus wire params. Pure host preparation; the
 * engine's IR validation remains the authority.
 */
function preparedOf<S extends AnySchema>(
	theory: S,
	query: AnyQuery,
	params: Readonly<Record<string, unknown>>
): {
	readonly ir: ReturnType<typeof lowerQuery>
	readonly wire: ReturnType<typeof wireParams>
	readonly finds: AnyQuery["data"]["finds"]
} {
	if (!schemasAgree(query.schema, theory)) {
		throw refusal("QueryReader.execute", "InvalidArgument")
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
	query: AnyQuery,
	params: Readonly<Record<string, unknown>>
): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope> {
	return scopedResult(
		Effect.gen(function* () {
			const prepared = yield* Effect.try({
				try: () => preparedOf(state.theory, query, params),
				catch: (cause) => argumentError("QueryReader.execute", cause)
			})
			const handle = yield* nativeOperationWith(
				"QueryReader.execute",
				(callback) => dbNative.runtimeSnapshotExecute(state.handle, prepared.ir, prepared.wire, callback),
				dbNative.runtimeResultTake,
				(value) => value
			)
			return makeCompleteResult<A>(handle, prepared.finds)
		})
	)
}

function scopedResult<A>(acquire: Effect.Effect<CompleteResult<A>, DbError>) {
	return Effect.acquireRelease(
		acquire,
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

function getOn<S extends AnySchema, K extends KeyStatement<Rel<S>, readonly string[]>>(
	state: SnapshotState<S>,
	key: K,
	value: Key<K>
): Effect.Effect<Option.Option<Fact<K["owner"]>>, DbError> {
	return Effect.gen(function* () {
		const tables = schemaTables(state.theory)
		const resolved = yield* Effect.try({
			try: () => declaredKey(state.theory, key),
			catch: (cause) => argumentError("QueryReader.get", cause)
		})
		if (resolved === undefined || key.kind !== "key") {
			return yield* Effect.fail(refusal("QueryReader.get", "InvalidArgument"))
		}
		const { statementId } = resolved
		const relation = resolved.key.owner
		if (!membersAgree(state.theory.relations[relation.name], relation)) {
			return yield* Effect.fail(refusal("QueryReader.get", "InvalidArgument"))
		}
		const relationId = tables.relationIds.get(relation.name)
		if (relationId === undefined) {
			return yield* Effect.fail(refusal("QueryReader.get", "InvalidArgument"))
		}
		const cells = yield* Effect.try({
			try: () => keyCellsOf(relation, resolved.key.projection, value),
			catch: (cause) => argumentError("QueryReader.get", cause)
		})
		const row = yield* nativeOperationWith(
			"QueryReader.get",
			(callback) => dbNative.runtimeSnapshotGet(state.handle, relationId, statementId, cells, callback),
			dbNative.runtimeRowTake,
			(value) => value
		)
		if (row === null) {
			return Option.none<Fact<K["owner"]>>()
		}
		return Option.some(factOfCells(relation, row as readonly CellValue[]))
	})
}

function makePrepared<P extends ParamsRecord, A>(
	handle: PreparedHandle,
	definitions: AnyQuery["data"]["params"],
	finds: AnyQuery["data"]["finds"]
): PreparedQuery<P, A> {
	return Object.freeze({
		releaseMemory() {
			return nativeOperationWith(
				"PreparedQuery.releaseMemory",
				(callback) => dbNative.runtimePreparedReleaseMemory(handle, callback),
				runtimeNative.runtimeTake,
				() => undefined
			)
		},
		execute(params: P) {
			return scopedResult(
				Effect.gen(function* () {
					const args = yield* Effect.try({
						try: () => wireParams(definitions, params),
						catch: (cause) => argumentError("PreparedQuery.execute", cause)
					})
					const result = yield* nativeOperationWith(
						"PreparedQuery.execute",
						(callback) => dbNative.runtimePreparedExecute(handle, args, callback),
						dbNative.runtimeResultTake,
						(value) => makeCompleteResult<A>(value, finds)
					)
					return result
				})
			)
		},
		close() {
			return drainClose("PreparedQuery.close", (callback) => dbNative.runtimePreparedClose(handle, callback))
		}
	})
}

function prepareOn<S extends AnySchema, P extends ParamsRecord, A>(
	state: SnapshotState<S>,
	query: QueryTemplate<S, P, A>
): Effect.Effect<PreparedQuery<P, A>, DbError, Scope.Scope> {
	return Effect.gen(function* () {
		const ir = yield* Effect.try({
			try: () => {
				if (!schemasAgree(query.schema, state.theory)) {
					throw refusal("QueryReader.prepare", "InvalidArgument")
				}
				return lowerQuery(query)
			},
			catch: (cause) => argumentError("QueryReader.prepare", cause)
		})
		const handle = yield* Effect.acquireRelease(
			nativeOperationWith(
				"QueryReader.prepare",
				(callback) => dbNative.runtimeSnapshotPrepare(state.handle, ir, callback),
				dbNative.runtimePreparedTake,
				(value) => value
			),
			(value) => releaseOwner("PreparedQuery.close", (callback) => dbNative.runtimePreparedClose(value, callback)),
			{ interruptible: true }
		)
		return makePrepared<P, A>(handle, query.data.params, query.data.finds)
	})
}

function makeSnapshot<S extends AnySchema>(theory: S, handle: SnapshotHandle, witness: CoreWitness): Snapshot<S> {
	const state: SnapshotState<S> = { theory, handle }
	const snapshot: Snapshot<S> = {
		witness,
		get(key, value) {
			return getOn(state, key, value)
		},
		execute(query, params) {
			return executeOn(state, query, params)
		},
		prepare(query) {
			return prepareOn(state, query)
		},
		close() {
			return drainClose("Snapshot.close", (callback) => dbNative.runtimeSnapshotClose(handle, callback))
		}
	}
	Object.freeze(snapshot)
	snapshotHandles.set(snapshot, handle)
	return snapshot
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
		clearCache() {
			return nativeOperationWith(
				"Db.clearCache",
				(callback) => dbNative.runtimeDbClearCache(state.db, callback),
				runtimeNative.runtimeTake,
				() => undefined
			)
		},
		snapshot() {
			return Effect.acquireRelease(
				nativeOperationWith(
					"Db.snapshot",
					(callback) => dbNative.runtimeDbSnapshot(state.db, callback),
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
				const input = yield* writeInputs(state, changes, options, "Db.apply")
				return yield* nativeOperationWith(
					"Db.apply",
					(callback) => dbNative.runtimeDbApply(state.db, input.handle, input.expected, callback),
					dbNative.runtimeApplyTake,
					outcomeOf
				)
			})
		},
		judge(changes, options) {
			return Effect.gen(function* () {
				const input = yield* writeInputs(state, changes, options, "Db.judge")
				return yield* nativeOperationWith(
					"Db.judge",
					(callback) => dbNative.runtimeDbJudge(state.db, input.handle, input.expected, callback),
					dbNative.runtimeJudgeTake,
					judgmentOf
				)
			})
		},
		inspect() {
			return nativeOperationWith(
				"Db.inspect",
				(callback) => dbNative.runtimeDbInspect(state.db, callback),
				dbNative.runtimeDbInspectTake,
				(wire: DbInspectionWire): DbInspection =>
					Object.freeze({
						schemaId: state.schemaId,
						generation: wire.generation,
						storage: Object.freeze(wire.storage),
						retainedOperations: wire.retainedOperations
					})
			)
		},
		close() {
			// The one close authority: database child first, then the
			// directory owner releases its kernel lock LAST. Both
			// joins are idempotent natively.
			return drainClose("Db.close", (callback) => runtimeNative.runtimeManagedDbClose(state.db, callback)).pipe(
				Effect.flatMap((report) =>
					drainClose("Db.directoryClose", (callback) =>
						runtimeNative.runtimeDirectoryClose(state.directory, false, callback)
					).pipe(Effect.map((directoryReport) => (report.kind === "closed" ? directoryReport : report)))
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
	create: boolean
): Effect.Effect<Db<S>, DbError, NativeRuntime | Scope.Scope> {
	return Effect.gen(function* () {
		const runtime = yield* runtimeHandle()
		const compiled: CompiledSchema<S> = yield* CoreSchema.compile(schema)
		const spec = lower(compiled.schema)
		// Compound acquisition: register the directory owner and
		// its finalizer BEFORE any interruptible child-open step.
		const directory = yield* Effect.acquireRelease(
			nativeOperationWith(
				operation,
				(callback) => runtimeNative.runtimeDirectoryAcquire(runtime, path, callback),
				runtimeNative.runtimeDirectoryTake,
				(value) => value
			),
			(dir) =>
				releaseOwner(`${operation}.directoryClose`, (callback) =>
					runtimeNative.runtimeDirectoryClose(dir, false, callback)
				),
			{ interruptible: true }
		)
		return yield* Effect.acquireRelease(
			Effect.gen(function* () {
				const outcome = yield* nativeOperationWith(
					operation,
					(callback) => runtimeNative.runtimeDirectoryDbOpen(directory, CHILD, spec, create, callback),
					runtimeNative.runtimeDbTake,
					(value) => value
				).pipe(
					Effect.catch((error) =>
						drainClose(`${operation}.directoryClose`, (callback) =>
							runtimeNative.runtimeDirectoryClose(directory, false, callback)
						).pipe(Effect.andThen(Effect.fail(error)))
					)
				)
				if (outcome.tag !== "accepted") {
					yield* drainClose(`${operation}.directoryClose`, (callback) =>
						runtimeNative.runtimeDirectoryClose(directory, false, callback)
					)
					return yield* Effect.fail(refusal(operation, "InvalidArgument"))
				}
				const state: DbState = { theory: compiled.schema, db: outcome.db, directory, schemaId: compiled.schemaId }
				const db = makeDb(compiled.schema, state)
				dbStates.set(db, state)
				return db
			}),
			(db) =>
				Effect.suspend(() => {
					const state = dbStates.get(db)
					if (state === undefined) {
						return Effect.void
					}
					return releaseOwner("Db.close", (callback) => runtimeNative.runtimeManagedDbClose(state.db, callback)).pipe(
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
 * replacement. Both compile the schema through the same
 * implementation as `Schema.compile` — prior compilation is optional.
 */
const Db = Object.freeze({
	create<S extends AnySchema>(path: string, schema: S) {
		return openDatabase("Db.create", path, schema, true)
	},
	open<S extends AnySchema>(path: string, schema: S) {
		return openDatabase("Db.open", path, schema, false)
	}
})

/**
 * Private log integration: wraps a published core snapshot
 * handle — minted by the internal log machine's native open/snapshot verbs
 * — in the exact core `QueryReader` plus the scoped session acquisition
 * (the log adds identity, stamps and freshness around this capability,
 * never a second reader).
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
	readonly prepare: QueryReader<S>["prepare"]
} {
	const state: SnapshotState<S> = { theory: schemaDescriptor(theory), handle: core as SnapshotHandle }
	return Object.freeze({
		get<K extends KeyStatement<Rel<S>, readonly string[]>>(key: K, value: NoInfer<Key<K>>) {
			return getOn(state, key, value)
		},
		execute<P extends ParamsRecord, A>(queryValue: QueryTemplate<S, P, A>, params: P) {
			return executeOn<S, A>(state, queryValue as AnyQuery, params)
		},
		prepare<P extends ParamsRecord, A>(queryValue: QueryTemplate<S, P, A>) {
			return prepareOn(state, queryValue)
		}
	})
}

export type {
	ApplyOutcome,
	CoreWitness,
	DbInspection,
	JudgeOutcome,
	PreparedQuery,
	QueryReader,
	Snapshot,
	StorageInspection,
	WriteExpected,
	WriteOptions
}
export { Db, internalPublishedReader }
