/**
 * The embedded engine surface. `Db.create`/`Db.open` own a database directory for the current
 * scope; snapshots, prepared queries and results are scoped the same way, so every native
 * resource is released by its scope and a release that does not drain cleanly is a
 * `CloseFailure` defect. Every verb is a lazy Effect that runs on the shared native executor.
 */
import type { Scope } from "effect"
import { Effect, Option } from "effect"
import type { ChangeSet } from "./changes.ts"
import { ChangeSetLive } from "./changes.ts"
import { membersAgree } from "./closed.ts"
import type { SchemaId } from "./compile.ts"
import { compiledOf, declaredKey, schemaTables } from "./compile.ts"
import { argumentError, DbError } from "./errors.ts"
import type { DbRef, DirectoryRef, PreparedRef, SnapshotRef } from "./native/addon.ts"
import { addon } from "./native/addon.ts"
import type {
	ApplyOutcome,
	DbInspection as DbInspectionOut,
	DbOpened,
	JudgeOutcome,
	WitnessOut
} from "./native/binding.d.ts"
import type { Start } from "./native/op.ts"
import { call, drain, scoped } from "./native/op.ts"
import type { AnyQuery } from "./query/lower.ts"
import { queryHandleOf } from "./query/lower.ts"
import { wireParams } from "./query/run.ts"
import type { ParamsRecord } from "./query/scope.ts"
import type { Fact } from "./relation.ts"
import type { CompleteResult } from "./result.ts"
import { CompleteResultLive } from "./result.ts"
import type { CellValue } from "./rows.ts"
import { factOfCells, keyCellsOf } from "./rows.ts"
import type { Bumble } from "./runtime.ts"
import { runtimeHandle } from "./runtime.ts"
import type { AnySchema } from "./schema.ts"
import { schemaDescriptor, schemasAgree } from "./schema.ts"
import type { Key, QueryTemplate, Rel } from "./shape.ts"
import type { KeyStatement } from "./statements.ts"

/** The store's identity plus its generation: the state a write can be conditioned on. */
type Witness = WitnessOut

/** Database diagnostics: measurements, never retained rows. */
interface DbInspection extends DbInspectionOut {
	readonly schemaId: SchemaId
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
	readonly witness: Witness
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
	/** Applies `changes` atomically; with `expected`, only if the database is still at that state. */
	apply(changes: ChangeSet<S>, expected?: Witness): Effect.Effect<ApplyOutcome, DbError>
	/** Judges `changes` against the current state (or `expected`) and stores nothing. */
	judge(changes: ChangeSet<S>, expected?: Witness): Effect.Effect<JudgeOutcome, DbError>
	inspect(): Effect.Effect<DbInspection, DbError>
	/** Clears shared query caches without invalidating live snapshots or results. */
	clearCache(): Effect.Effect<void, DbError>
}

function invalid(operation: string): DbError {
	return new DbError({ operation, reason: { _tag: "InvalidArgument" } })
}

/** A database the engine would not open: its refusal, as an engine error. */
function refused(operation: string, opened: Exclude<DbOpened, { _tag: "Opened" }>): DbError {
	const message =
		opened._tag === "Rejected" ? opened.violations.map((violation) => violation.spelling).join("; ") : opened.message
	return new DbError({ operation, reason: { _tag: "Engine", kind: opened._tag, message } })
}

function changesOf(schemaId: SchemaId, changes: object, operation: string) {
	return Effect.suspend(() => {
		const handle = ChangeSetLive.handle(changes, schemaId)
		return handle === undefined ? Effect.fail(invalid(operation)) : Effect.succeed(handle)
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
			call(operation, start, addon.runtimeResultTake),
			(handle) => (done) => addon.runtimeResultClose(handle, done)
		),
		(handle) => new CompleteResultLive<A>(handle, finds)
	)
}

const getOn = Effect.fn("QueryReader.get")(function* <
	S extends AnySchema,
	K extends KeyStatement<Rel<S>, readonly string[]>
>(theory: S, handle: SnapshotRef, key: K, value: Key<K>) {
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
		(done) => addon.runtimeSnapshotGet(handle, relationId, resolved.statementId, cells, done),
		addon.runtimeRowTake
	)
	return row === null
		? Option.none<Fact<K["owner"]>>()
		: Option.some(factOfCells(relation, row as readonly CellValue[]) as Fact<K["owner"]>)
})

function executeOn<S extends AnySchema, A>(
	theory: S,
	handle: SnapshotRef,
	query: AnyQuery,
	params: Readonly<Record<string, unknown>>
): Effect.Effect<CompleteResult<A>, DbError, Scope.Scope> {
	return Effect.gen(function* () {
		const prepared = yield* Effect.try({
			try: () => {
				if (!schemasAgree(query.schema, theory)) throw invalid("QueryReader.execute")
				return { query: queryHandleOf(query), wire: wireParams(query.data.params, params) }
			},
			catch: (cause) => argumentError("QueryReader.execute", cause)
		})
		return yield* scopedResult<A>(
			"QueryReader.execute",
			(done) => addon.runtimeSnapshotExecute(handle, prepared.query, prepared.wire, done),
			query.data.finds
		)
	}).pipe(Effect.withSpan("QueryReader.execute"))
}

class PreparedQueryLive<P extends ParamsRecord, A> implements PreparedQuery<P, A> {
	readonly #handle: PreparedRef
	readonly #query: AnyQuery

	constructor(handle: PreparedRef, query: AnyQuery) {
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
				(done) => addon.runtimePreparedExecute(handle, args, done),
				query.data.finds
			)
		}).pipe(Effect.withSpan("PreparedQuery.execute"))
	}

	releaseMemory(): Effect.Effect<void, DbError> {
		const handle = this.#handle
		return call(
			"PreparedQuery.releaseMemory",
			(done) => addon.runtimePreparedReleaseMemory(handle, done),
			addon.runtimeTake
		).pipe(Effect.withSpan("PreparedQuery.releaseMemory"))
	}
}

function prepareOn<S extends AnySchema, P extends ParamsRecord, A>(
	theory: S,
	handle: SnapshotRef,
	query: QueryTemplate<S, P, A>
): Effect.Effect<PreparedQuery<P, A>, DbError, Scope.Scope> {
	return Effect.gen(function* () {
		const ir = yield* Effect.try({
			try: () => {
				if (!schemasAgree(query.schema, theory)) throw invalid("QueryReader.prepare")
				return queryHandleOf(query as AnyQuery)
			},
			catch: (cause) => argumentError("QueryReader.prepare", cause)
		})
		const prepared = yield* scoped(
			"PreparedQuery.release",
			call("QueryReader.prepare", (done) => addon.runtimeSnapshotPrepare(handle, ir, done), addon.runtimePreparedTake),
			(owned) => (done) => addon.runtimePreparedClose(owned, done)
		)
		return new PreparedQueryLive<P, A>(prepared, query)
	}).pipe(Effect.withSpan("QueryReader.prepare"))
}

class SnapshotLive<S extends AnySchema> implements Snapshot<S> {
	readonly #theory: S
	readonly #handle: SnapshotRef
	readonly witness: Witness

	constructor(theory: S, handle: SnapshotRef, witness: Witness) {
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

/** A snapshot `start` pins, read through `theory` and released by the current scope. */
function scopedSnapshot<S extends AnySchema>(
	operation: string,
	theory: S,
	start: Start
): Effect.Effect<Snapshot<S>, DbError, Scope.Scope> {
	return Effect.map(
		scoped(
			"Snapshot.release",
			call(operation, start, addon.runtimeSnapshotTake),
			(opened) => (done) => addon.runtimeSnapshotClose(opened.snapshot, done)
		),
		(opened): Snapshot<S> => new SnapshotLive(theory, opened.snapshot, Object.freeze(opened.witness))
	)
}

class DbLive<S extends AnySchema> implements Db<S> {
	readonly #theory: S
	readonly #handle: DbRef
	readonly schemaId: SchemaId

	constructor(theory: S, handle: DbRef, schemaId: SchemaId) {
		this.#theory = theory
		this.#handle = handle
		this.schemaId = schemaId
	}

	snapshot(): Effect.Effect<Snapshot<S>, DbError, Scope.Scope> {
		const handle = this.#handle
		return scopedSnapshot("Db.snapshot", this.#theory, (done) => addon.runtimeDbSnapshot(handle, done)).pipe(
			Effect.withSpan("Db.snapshot")
		)
	}

	apply(changes: ChangeSet<S>, expected?: Witness): Effect.Effect<ApplyOutcome, DbError> {
		const handle = this.#handle
		return changesOf(this.schemaId, changes, "Db.apply").pipe(
			Effect.flatMap((owned) =>
				call("Db.apply", (done) => addon.runtimeDbApply(handle, owned, expected ?? null, done), addon.runtimeApplyTake)
			),
			Effect.withSpan("Db.apply")
		)
	}

	judge(changes: ChangeSet<S>, expected?: Witness): Effect.Effect<JudgeOutcome, DbError> {
		const handle = this.#handle
		return changesOf(this.schemaId, changes, "Db.judge").pipe(
			Effect.flatMap((owned) =>
				call("Db.judge", (done) => addon.runtimeDbJudge(handle, owned, expected ?? null, done), addon.runtimeJudgeTake)
			),
			Effect.withSpan("Db.judge")
		)
	}

	inspect(): Effect.Effect<DbInspection, DbError> {
		const handle = this.#handle
		const schemaId = this.schemaId
		return call(
			"Db.inspect",
			(done) => addon.runtimeDbInspect(handle, done),
			(lease): DbInspection => Object.freeze({ ...addon.runtimeDbInspectTake(lease), schemaId })
		).pipe(Effect.withSpan("Db.inspect"))
	}

	clearCache(): Effect.Effect<void, DbError> {
		const handle = this.#handle
		return call("Db.clearCache", (done) => addon.runtimeDbClearCache(handle, done), addon.runtimeTake).pipe(
			Effect.withSpan("Db.clearCache")
		)
	}
}

/** The database's child directory inside the owned directory. */
const CHILD = "store"

const closeDirectory = (directory: DirectoryRef) => (done: Parameters<typeof addon.runtimeDirectoryClose>[2]) =>
	addon.runtimeDirectoryClose(directory, false, done)

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
		const theory = yield* Effect.try({
			try: () => schemaDescriptor(schema),
			catch: (cause) => argumentError(operation, cause)
		})
		const compiled = compiledOf(theory)
		const directory = yield* scoped(
			"Db.directoryRelease",
			call(operation, (done) => addon.runtimeDirectoryAcquire(runtime, path, done), addon.runtimeDirectoryTake),
			closeDirectory
		)
		const opened = call(
			operation,
			(done) => addon.runtimeDirectoryDbOpen(directory, CHILD, compiled.handle, create, done),
			addon.runtimeDbTake
		).pipe(
			Effect.flatMap((opened) =>
				opened._tag === "Opened" ? Effect.succeed(opened.db) : Effect.fail(refused(operation, opened))
			),
			Effect.tapError(() => drain(closeDirectory(directory)))
		)
		const handle = yield* scoped("Db.release", opened, (db) => (done) => addon.runtimeManagedDbClose(db, done))
		return new DbLive(theory as S, handle, compiled.schemaId) as Db<S>
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

export type { ApplyOutcome, JudgeOutcome } from "./native/binding.d.ts"
export type { DbInspection, PreparedQuery, QueryReader, Snapshot, Witness }
export { Db, scopedSnapshot }
