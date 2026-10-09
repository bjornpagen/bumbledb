/**
 * The embedded engine surface. `Db.create`/`Db.open` own a database directory for the current
 * scope; snapshots, prepared queries and results are scoped the same way, so every native
 * resource is released by its scope and a release that does not drain cleanly is a
 * `CloseFailure` defect. Every verb is a lazy Effect that runs on the shared native executor.
 */
import type { Scope } from "effect"
import { Effect, Option } from "effect"
import type { ChangeCounts, ChangeSet } from "./changes.ts"
import { ChangeSetLive } from "./changes.ts"
import { membersAgree } from "./closed.ts"
import type { SchemaId } from "./compile.ts"
import { Schema as CoreSchema, declaredKey, schemaTables } from "./compile.ts"
import type {
	ApplyOutcomeWire,
	DbInspectionWire,
	ExpectedWire,
	JudgeOutcomeWire,
	PreparedHandle,
	SnapshotHandle,
	SnapshotWire,
	WitnessWire
} from "./db-native.ts"
import { dbNative } from "./db-native.ts"
import { argumentError, DbError } from "./errors.ts"
import { lower } from "./lower.ts"
import type { Start } from "./native/op.ts"
import { call, drain, scoped } from "./native/op.ts"
import type { DbHandle, Violation } from "./native.ts"
import type { AnyQuery } from "./query/lower.ts"
import { lowerQuery } from "./query/lower.ts"
import { wireParams } from "./query/run.ts"
import type { ParamsRecord } from "./query/scope.ts"
import type { Fact } from "./relation.ts"
import type { CompleteResult } from "./result.ts"
import { CompleteResultLive } from "./result.ts"
import type { CellValue } from "./rows.ts"
import { factOfCells, keyCellsOf } from "./rows.ts"
import type { Bumble } from "./runtime.ts"
import { runtimeHandle } from "./runtime.ts"
import type { DirectoryHandle } from "./runtime-native.ts"
import { runtimeNative } from "./runtime-native.ts"
import type { AnySchema } from "./schema.ts"
import { schemasAgree } from "./schema.ts"
import type { Key, QueryTemplate, Rel } from "./shape.ts"
import type { KeyStatement } from "./statements.ts"
import { integerValue, recordValue } from "./values.ts"

/** The store's identity plus its generation: the point a write can be conditioned on. */
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

/** A judgment against the current base; nothing is stored. A moved witness is not judged. */
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
 * Typed reads over one coherent snapshot. A missing key is `Option.none`. It carries no write
 * authority.
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
}

/** One compiled plan and its reusable buffers, pinned to the snapshot that prepared it. */
interface PreparedQuery<P extends ParamsRecord, A> {
	execute(params: P): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope>
	/** Drops the reusable execution buffers, keeping the compiled plan. */
	releaseMemory(): Effect.Effect<void, DbError>
}

interface Db<S extends AnySchema> {
	readonly schemaId: SchemaId
	snapshot(): Effect.Effect<Snapshot<S>, DbError, Scope.Scope>
	apply(changes: ChangeSet<S>, options: WriteOptions): Effect.Effect<ApplyOutcome, DbError>
	/** Judges `changes` against the current base and aborts; stores nothing. */
	judge(changes: ChangeSet<S>, options: WriteOptions): Effect.Effect<JudgeOutcome, DbError>
	inspect(): Effect.Effect<DbInspection, DbError>
	/** Clears shared query caches without invalidating live snapshots or results. */
	clearCache(): Effect.Effect<void, DbError>
}

function invalid(operation: string): DbError {
	return new DbError({ operation, reason: { _tag: "InvalidArgument" } })
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

function expectedOf(options: WriteOptions): ExpectedWire {
	const record = recordValue("write options", options, ["expected"])
	const intent = recordValue(
		"expected state",
		record.expected,
		options.expected.kind === "any" ? ["kind"] : ["kind", "at"]
	)
	if (intent.kind === "any") return { kind: "any" }
	if (intent.kind !== "exact") throw invalid("write options")
	const at = recordValue("expected witness", intent.at, ["store", "generation"])
	if (typeof at.store !== "string" || !/^[0-9a-f]{32}$/.test(at.store)) throw invalid("write options")
	return { kind: "exact", store: at.store, generation: integerValue("expected generation", "u64", at.generation) }
}

function writeInputs(schemaId: SchemaId, changes: object, options: WriteOptions, operation: string) {
	return Effect.try({
		try: () => {
			const handle = ChangeSetLive.handle(changes, schemaId)
			if (handle === undefined) throw invalid(operation)
			return { handle, expected: expectedOf(options) }
		},
		catch: (cause) => argumentError(operation, cause)
	})
}

/** A completed result owned by the current scope. */
function scopedResult<A>(
	operation: string,
	start: Start,
	finds: AnyQuery["data"]["finds"]
): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope> {
	return Effect.map(
		scoped(
			"CompleteResult.release",
			call(operation, start, dbNative.runtimeResultTake),
			(handle) => (done) => dbNative.runtimeResultClose(handle, done)
		),
		(handle) => new CompleteResultLive<A>(handle, finds)
	)
}

const getOn = Effect.fn("QueryReader.get")(function* <
	S extends AnySchema,
	K extends KeyStatement<Rel<S>, readonly string[]>
>(theory: S, handle: SnapshotHandle, key: K, value: Key<K>) {
	const resolved = yield* Effect.try({
		try: () => declaredKey(theory, key),
		catch: (cause) => argumentError("QueryReader.get", cause)
	})
	if (resolved === undefined || key.kind !== "key") return yield* Effect.fail(invalid("QueryReader.get"))
	const relation = resolved.key.owner
	const relationId = schemaTables(theory).relationIds.get(relation.name)
	if (!membersAgree(theory.relations[relation.name], relation) || relationId === undefined) {
		return yield* Effect.fail(invalid("QueryReader.get"))
	}
	const cells = yield* Effect.try({
		try: () => keyCellsOf(relation, resolved.key.projection, value),
		catch: (cause) => argumentError("QueryReader.get", cause)
	})
	const row = yield* call(
		"QueryReader.get",
		(done) => dbNative.runtimeSnapshotGet(handle, relationId, resolved.statementId, cells, done),
		dbNative.runtimeRowTake
	)
	return row === null
		? Option.none<Fact<K["owner"]>>()
		: Option.some(factOfCells(relation, row as readonly CellValue[]) as Fact<K["owner"]>)
})

function executeOn<S extends AnySchema, A>(
	theory: S,
	handle: SnapshotHandle,
	query: AnyQuery,
	params: Readonly<Record<string, unknown>>
): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope> {
	return Effect.gen(function* () {
		const prepared = yield* Effect.try({
			try: () => {
				if (!schemasAgree(query.schema, theory)) throw invalid("QueryReader.execute")
				return { ir: lowerQuery(query), wire: wireParams(query.data.params, params) }
			},
			catch: (cause) => argumentError("QueryReader.execute", cause)
		})
		return yield* scopedResult<A>(
			"QueryReader.execute",
			(done) => dbNative.runtimeSnapshotExecute(handle, prepared.ir, prepared.wire, done),
			query.data.finds
		)
	}).pipe(Effect.withSpan("QueryReader.execute"))
}

class PreparedQueryLive<P extends ParamsRecord, A> implements PreparedQuery<P, A> {
	readonly #handle: PreparedHandle
	readonly #query: AnyQuery

	constructor(handle: PreparedHandle, query: AnyQuery) {
		this.#handle = handle
		this.#query = query
	}

	execute(params: P): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope> {
		const handle = this.#handle
		const query = this.#query
		return Effect.gen(function* () {
			const args = yield* Effect.try({
				try: () => wireParams(query.data.params, params),
				catch: (cause) => argumentError("PreparedQuery.execute", cause)
			})
			return yield* scopedResult<A>(
				"PreparedQuery.execute",
				(done) => dbNative.runtimePreparedExecute(handle, args, done),
				query.data.finds
			)
		}).pipe(Effect.withSpan("PreparedQuery.execute"))
	}

	releaseMemory(): Effect.Effect<void, DbError> {
		const handle = this.#handle
		return call(
			"PreparedQuery.releaseMemory",
			(done) => dbNative.runtimePreparedReleaseMemory(handle, done),
			runtimeNative.runtimeTake
		).pipe(Effect.withSpan("PreparedQuery.releaseMemory"))
	}
}

function prepareOn<S extends AnySchema, P extends ParamsRecord, A>(
	theory: S,
	handle: SnapshotHandle,
	query: QueryTemplate<S, P, A>
): Effect.Effect<PreparedQuery<P, A>, DbError, Scope.Scope> {
	return Effect.gen(function* () {
		const ir = yield* Effect.try({
			try: () => {
				if (!schemasAgree(query.schema, theory)) throw invalid("QueryReader.prepare")
				return lowerQuery(query)
			},
			catch: (cause) => argumentError("QueryReader.prepare", cause)
		})
		const prepared = yield* scoped(
			"PreparedQuery.release",
			call(
				"QueryReader.prepare",
				(done) => dbNative.runtimeSnapshotPrepare(handle, ir, done),
				dbNative.runtimePreparedTake
			),
			(owned) => (done) => dbNative.runtimePreparedClose(owned, done)
		)
		return new PreparedQueryLive<P, A>(prepared, query)
	}).pipe(Effect.withSpan("QueryReader.prepare"))
}

class SnapshotLive<S extends AnySchema> implements Snapshot<S> {
	readonly #theory: S
	readonly #handle: SnapshotHandle
	readonly witness: CoreWitness

	constructor(theory: S, handle: SnapshotHandle, witness: CoreWitness) {
		this.#theory = theory
		this.#handle = handle
		this.witness = witness
	}

	get<K extends KeyStatement<Rel<S>, readonly string[]>>(
		key: K,
		value: NoInfer<Key<K>>
	): Effect.Effect<Option.Option<Fact<K["owner"]>>, DbError> {
		return getOn(this.#theory, this.#handle, key, value)
	}

	execute<P extends ParamsRecord, A>(
		query: QueryTemplate<S, P, A>,
		params: P
	): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope> {
		return executeOn<S, A>(this.#theory, this.#handle, query as AnyQuery, params)
	}

	prepare<P extends ParamsRecord, A>(
		query: QueryTemplate<S, P, A>
	): Effect.Effect<PreparedQuery<P, A>, DbError, Scope.Scope> {
		return prepareOn(this.#theory, this.#handle, query)
	}
}

class DbLive<S extends AnySchema> implements Db<S> {
	readonly #theory: S
	readonly #handle: DbHandle
	readonly schemaId: SchemaId

	constructor(theory: S, handle: DbHandle, schemaId: SchemaId) {
		this.#theory = theory
		this.#handle = handle
		this.schemaId = schemaId
	}

	snapshot(): Effect.Effect<Snapshot<S>, DbError, Scope.Scope> {
		const theory = this.#theory
		const handle = this.#handle
		return Effect.map(
			scoped(
				"Snapshot.release",
				call("Db.snapshot", (done) => dbNative.runtimeDbSnapshot(handle, done), dbNative.runtimeSnapshotTake),
				(wire: SnapshotWire) => (done) => dbNative.runtimeSnapshotClose(wire.snapshot, done)
			),
			(wire): Snapshot<S> => new SnapshotLive(theory, wire.snapshot, witnessOf(wire.witness))
		).pipe(Effect.withSpan("Db.snapshot"))
	}

	apply(changes: ChangeSet<S>, options: WriteOptions): Effect.Effect<ApplyOutcome, DbError> {
		const handle = this.#handle
		const schemaId = this.schemaId
		return Effect.gen(function* () {
			const input = yield* writeInputs(schemaId, changes, options, "Db.apply")
			return yield* call(
				"Db.apply",
				(done) => dbNative.runtimeDbApply(handle, input.handle, input.expected, done),
				(lease) => outcomeOf(dbNative.runtimeApplyTake(lease))
			)
		}).pipe(Effect.withSpan("Db.apply"))
	}

	judge(changes: ChangeSet<S>, options: WriteOptions): Effect.Effect<JudgeOutcome, DbError> {
		const handle = this.#handle
		const schemaId = this.schemaId
		return Effect.gen(function* () {
			const input = yield* writeInputs(schemaId, changes, options, "Db.judge")
			return yield* call(
				"Db.judge",
				(done) => dbNative.runtimeDbJudge(handle, input.handle, input.expected, done),
				(lease) => judgmentOf(dbNative.runtimeJudgeTake(lease))
			)
		}).pipe(Effect.withSpan("Db.judge"))
	}

	inspect(): Effect.Effect<DbInspection, DbError> {
		const handle = this.#handle
		const schemaId = this.schemaId
		return call(
			"Db.inspect",
			(done) => dbNative.runtimeDbInspect(handle, done),
			(lease): DbInspection => {
				const wire: DbInspectionWire = dbNative.runtimeDbInspectTake(lease)
				return Object.freeze({
					schemaId,
					generation: wire.generation,
					storage: Object.freeze(wire.storage),
					retainedOperations: wire.retainedOperations
				})
			}
		).pipe(Effect.withSpan("Db.inspect"))
	}

	clearCache(): Effect.Effect<void, DbError> {
		const handle = this.#handle
		return call("Db.clearCache", (done) => dbNative.runtimeDbClearCache(handle, done), runtimeNative.runtimeTake).pipe(
			Effect.withSpan("Db.clearCache")
		)
	}
}

/** The database's child directory inside the owned directory. */
const CHILD = "store"

const closeDirectory =
	(directory: DirectoryHandle) => (done: Parameters<typeof runtimeNative.runtimeDirectoryClose>[2]) =>
		runtimeNative.runtimeDirectoryClose(directory, false, done)

/**
 * Takes the directory lock, then opens the database inside it. Both are owned by the current
 * scope, which releases the database before the directory. A failed open releases the
 * directory at once rather than when the scope ends.
 */
const openDatabase = <S extends AnySchema>(
	operation: "Db.create" | "Db.open",
	path: string,
	schema: S,
	create: boolean
): Effect.Effect<Db<S>, DbError, Bumble | Scope.Scope> =>
	Effect.gen(function* () {
		const runtime = yield* runtimeHandle
		const compiled = yield* CoreSchema.compile(schema)
		const spec = lower(compiled.schema)
		const directory = yield* scoped(
			"Db.directoryRelease",
			call(
				operation,
				(done) => runtimeNative.runtimeDirectoryAcquire(runtime, path, done),
				runtimeNative.runtimeDirectoryTake
			),
			closeDirectory
		)
		const opened = call(
			operation,
			(done) => runtimeNative.runtimeDirectoryDbOpen(directory, CHILD, spec, create, done),
			runtimeNative.runtimeDbTake
		).pipe(
			Effect.flatMap((outcome) =>
				outcome.tag === "accepted" ? Effect.succeed(outcome.db) : Effect.fail(invalid(operation))
			),
			Effect.tapError(() => drain(`${operation}.directoryRelease`, closeDirectory(directory)))
		)
		const handle = yield* scoped("Db.release", opened, (db) => (done) => runtimeNative.runtimeManagedDbClose(db, done))
		return new DbLive(compiled.schema, handle, compiled.schemaId) as Db<S>
	}).pipe(Effect.withSpan(operation))

/**
 * `Db.create` makes a new database and refuses an existing one; `Db.open` opens an existing one
 * and never creates. Both compile the schema first.
 */
const Db = Object.freeze({
	create<S extends AnySchema>(path: string, schema: S) {
		return openDatabase("Db.create", path, schema, true)
	},
	open<S extends AnySchema>(path: string, schema: S) {
		return openDatabase("Db.open", path, schema, false)
	}
})

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
export { Db }
