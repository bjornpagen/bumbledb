/**
 * A durable database: an immutable log in an object store, read through a local LMDB cache that is
 * only a cache. Writes are submitted commands decided once per request id; reads pin snapshots of
 * the cache at a chosen freshness. Migrations are bundled with the code and run on open (`migrate`)
 * or are required to have run already (`verify`).
 */
import { randomBytes } from "node:crypto"
import type { Scope } from "effect"
import { Context, Duration, Effect, Layer, Option, RcMap, Schema } from "effect"
import type { ChangeSet } from "../changes.ts"
import { ChangeSetLive, ChangeSet as ChangeSets } from "../changes.ts"
import { compiledOf } from "../compile.ts"
import type { QueryReader } from "../db.ts"
import { scopedSnapshot } from "../db.ts"
import { argumentError, DbError, dbError } from "../errors.ts"
import { lower } from "../lower.ts"
import { addon } from "../native/addon.ts"
import type { HeadOut, HostedOpenIn, ReceiptOut, RefusalOut, SettledOut } from "../native/binding.d.ts"
import { call, scoped } from "../native/op.ts"
import type { Bumble } from "../runtime.ts"
import { runtimeHandle } from "../runtime.ts"
import type { AnySchema } from "../schema.ts"
import { schemaDescriptor, schemasAgree } from "../schema.ts"
import { Driver } from "./driver.ts"
import type { HostedRef, HostedRequest } from "./hosted.ts"
import { hostedPort } from "./hosted.ts"
import type { ExecutorOptions, ObjectStore } from "./io.ts"
import { defaultExecutorOptions } from "./io.ts"
import type { Migrations } from "./migration.ts"
import { populateOf, verifyBundle } from "./migration.ts"

/** A submission's idempotency key: 32 lowercase hex digits. */
const RequestId = Schema.String.check(Schema.isPattern(/^[0-9a-f]{32}$/)).pipe(Schema.brand("RequestId"))
type RequestId = typeof RequestId.Type

/** How fresh a read must be: the cache as it is, the log's tip, or at least a submitted `seq`. */
type Consistency = "cached" | "latest" | { readonly atLeast: bigint }

/** A decided submission (its receipt), or a refusal. Every refusal but `Unknown` proves it is not in the log. */
type SubmitOutcome = Extract<SettledOut, { readonly _tag: "Decided" | "Refused" }>

interface SubmitOptions {
	readonly requestId: RequestId
	/** Decide only if the database is exactly at this revision. */
	readonly precondition?: bigint
}

/** A snapshot reader, with the log position and revision the cache had reached when it was pinned. */
interface DatabaseReader<S extends AnySchema> extends QueryReader<S> {
	readonly seq: bigint
	/** The committed revision; a submit can require it with `precondition`. */
	readonly revision: bigint
}

interface Database<S extends AnySchema> {
	readonly schema: S
	/** The last decided state this process has seen. */
	readonly head: Effect.Effect<HeadOut>
	submit(changes: ChangeSet<S>, options: SubmitOptions): Effect.Effect<SubmitOutcome, DbError>
	read(consistency?: Consistency): Effect.Effect<DatabaseReader<S>, DbError, Scope.Scope>
	/** The receipt of `requestId`, after catching up with the log. */
	resolve(requestId: RequestId): Effect.Effect<Option.Option<ReceiptOut>, DbError>
}

interface Tuning {
	/** Parallel tail reads while catching up. */
	readonly probeWindow: number
	/** Log entries between automatic checkpoints. */
	readonly checkpointEvery: bigint
	readonly checkpointKeep: number
	readonly maxEntryBytes: number
	/** Store calls in flight at once. */
	readonly concurrency: number
	readonly executor: ExecutorOptions
	/** Stale migration attempts before writers are frozen for one more attempt. */
	readonly migrationAttempts: number
	readonly freezeLease: Duration.Input
}

const defaultTuning: Tuning = {
	probeWindow: 32,
	checkpointEvery: 256n,
	checkpointKeep: 3,
	maxEntryBytes: 4 * 1024 * 1024,
	concurrency: 32,
	executor: defaultExecutorOptions,
	migrationAttempts: 3,
	freezeLease: "5 minutes"
}

interface DatabaseOptions<S extends AnySchema> {
	/** The schema the code reads and writes; the last migration's schema. */
	readonly schema: S
	readonly migrations: Migrations
	readonly store: ObjectStore
	/** Where the disposable local cache lives. */
	readonly cache: { readonly directory: string }
	/** `migrate` creates the database and runs pending migrations; `verify` refuses to open with any pending. */
	readonly onOpen: "verify" | "migrate"
	readonly tuning?: Partial<Tuning>
}

function describeRefusal(refusal: RefusalOut): string {
	switch (refusal._tag) {
		case "Frozen":
			return `writes are frozen until ${refusal.deadline}`
		case "MigrationPending":
			return `migration ${refusal.next} has not run`
		case "MigrationRejected":
			return `migration ${refusal.migration.name} was rejected: ${refusal.evidence.violations.map((violation) => violation.spelling).join("; ")}`
		case "MigrationsDiverged":
			return `bundled migration ${refusal.index} differs from the one the log applied`
		case "Stale":
			return `the log moved to ${refusal.head}`
		case "RequestReused":
			return `request ${refusal.request} was decided for another command`
		case "Cache":
			return refusal.message
		case "Corrupt":
			return `log entry ${refusal.seq} is corrupt`
		default:
			return refusal._tag
	}
}

/** A refusal of the machine, as a `DbError`. */
function refused(operation: string, refusal: RefusalOut): DbError {
	return new DbError({ operation, reason: { _tag: "Engine", kind: refusal._tag, message: describeRefusal(refusal) } })
}

function hex(bytes: number): string {
	return randomBytes(bytes).toString("hex")
}

function settledAs<T extends SettledOut["_tag"]>(
	operation: string,
	settled: SettledOut,
	tag: T
): Effect.Effect<Extract<SettledOut, { readonly _tag: T }>, DbError> {
	if (settled._tag === tag) return Effect.succeed(settled as Extract<SettledOut, { readonly _tag: T }>)
	if (settled._tag === "Refused") return Effect.fail(refused(operation, settled.refusal))
	return Effect.fail(dbError(operation, { _tag: "Internal" }))
}

class DatabaseLive<S extends AnySchema> implements Database<S> {
	readonly schema: S
	readonly #hosted: HostedRef
	readonly #run: (request: HostedRequest) => Effect.Effect<SettledOut, DbError>
	readonly #head: () => HeadOut | undefined

	constructor(
		schema: S,
		hosted: HostedRef,
		run: (request: HostedRequest) => Effect.Effect<SettledOut, DbError>,
		head: () => HeadOut | undefined
	) {
		this.schema = schema
		this.#hosted = hosted
		this.#run = run
		this.#head = head
	}

	get head(): Effect.Effect<HeadOut> {
		return Effect.suspend(() => {
			const head = this.#head()
			return head === undefined ? Effect.die(dbError("Database.head", { _tag: "Internal" })) : Effect.succeed(head)
		})
	}

	submit(changes: ChangeSet<S>, options: SubmitOptions): Effect.Effect<SubmitOutcome, DbError> {
		const schemaId = compiledOf(this.schema).schemaId
		const run = this.#run
		return Effect.gen(function* () {
			const handle = ChangeSetLive.handle(changes, schemaId)
			if (handle === undefined) {
				return yield* Effect.fail(new DbError({ operation: "Database.submit", reason: { _tag: "InvalidArgument" } }))
			}
			const settled = yield* run({
				_tag: "Submit",
				request: options.requestId,
				revision: options.precondition ?? null,
				changes: handle
			})
			if (settled._tag !== "Decided" && settled._tag !== "Refused") {
				return yield* Effect.fail(dbError("Database.submit", { _tag: "Internal" }))
			}
			return settled
		}).pipe(Effect.withSpan("Database.submit"))
	}

	read(consistency: Consistency = "cached"): Effect.Effect<DatabaseReader<S>, DbError, Scope.Scope> {
		const run = this.#run
		const hosted = this.#hosted
		const schema = this.schema
		const seen = this.#head
		return Effect.gen(function* () {
			const known = seen()?.seq ?? 0n
			if (consistency === "latest" || (typeof consistency === "object" && known < consistency.atLeast)) {
				yield* Effect.flatMap(run({ _tag: "Sync" }), (settled) => settledAs("Database.read", settled, "Synced"))
			}
			const head = seen()
			const snapshot = yield* scopedSnapshot("Database.read", schema, (done) => addon.hostedSnapshot(hosted, done))
			return {
				seq: head?.seq ?? 0n,
				revision: head?.revision ?? 0n,
				get: snapshot.get.bind(snapshot),
				execute: snapshot.execute.bind(snapshot),
				prepare: snapshot.prepare.bind(snapshot)
			} satisfies DatabaseReader<S>
		}).pipe(Effect.withSpan("Database.read"))
	}

	resolve(requestId: RequestId): Effect.Effect<Option.Option<ReceiptOut>, DbError> {
		return this.#run({ _tag: "Resolve", request: requestId }).pipe(
			Effect.flatMap((settled) => settledAs("Database.resolve", settled, "Resolved")),
			Effect.map((resolved) => Option.fromNullishOr(resolved.receipt)),
			Effect.withSpan("Database.resolve")
		)
	}
}

/** Runs bundled migration `step` to completion, freezing writers if it keeps losing to them. */
const migrateStep = (
	migrations: Migrations,
	step: number,
	hosted: HostedRef,
	run: (request: HostedRequest) => Effect.Effect<SettledOut, DbError>,
	head: () => HeadOut | undefined,
	tuning: Tuning
) =>
	Effect.gen(function* () {
		const migration = migrations[step]
		const previous = migrations[step - 1]
		if (migration === undefined || previous === undefined) {
			return yield* Effect.fail(dbError("Database.migrate", { _tag: "Internal" }))
		}
		const attempt = Effect.scoped(
			Effect.gen(function* () {
				const base = head()?.seq ?? 0n
				const from = yield* scopedSnapshot("Database.migrate", previous.to, (done) =>
					addon.hostedSnapshot(hosted, done)
				)
				const into = yield* ChangeSets.builder(migration.to)
				const populate = populateOf(migration)
				if (populate !== undefined) yield* populate({ from, into })
				const rows = ChangeSetLive.handle(yield* into.finish(), compiledOf(migration.to).schemaId)
				if (rows === undefined) return yield* Effect.fail(dbError("Database.migrate", { _tag: "Internal" }))
				const copy = addon.hostedUnchanged(hosted, step)
				return yield* run({ _tag: "Migrate", step, base, copy, rows })
			})
		)
		let settled: SettledOut = { _tag: "Refused", refusal: { _tag: "NotPending" } }
		for (let tries = 0; tries <= tuning.migrationAttempts; tries++) {
			if (tries === tuning.migrationAttempts) {
				const lease = BigInt(Math.ceil(Duration.toMillis(tuning.freezeLease)))
				yield* Effect.flatMap(run({ _tag: "Freeze", leaseMillis: lease }), (frozen) =>
					settledAs("Database.migrate", frozen, "Frozen")
				)
			}
			settled = yield* attempt
			if (settled._tag !== "Refused" || settled.refusal._tag !== "Stale") break
		}
		yield* settledAs("Database.migrate", settled, "Migrated")
	}).pipe(Effect.withSpan("Database.migrate"))

/**
 * Seeds a database nothing has written to yet with its initial migration's rows, after every
 * migration ran: the seed's relations must still exist unchanged in the current schema. The request
 * id comes from the migration's hash, so racing openers decide the seed once.
 */
const seedInitial = <S extends AnySchema>(
	database: Database<S>,
	hash: string,
	seed: NonNullable<ReturnType<typeof populateOf>>
) =>
	Effect.scoped(
		Effect.gen(function* () {
			const from = yield* database.read("cached")
			const into = yield* ChangeSets.builder(database.schema)
			yield* seed({ from, into } as never)
			const outcome = yield* database.submit(yield* into.finish(), { requestId: RequestId.make(hash.slice(0, 32)) })
			if (outcome._tag === "Refused" && outcome.refusal._tag !== "RequestReused") {
				return yield* Effect.fail(refused("Database.seed", outcome.refusal))
			}
		})
	).pipe(Effect.withSpan("Database.seed"))

/** Opens a database for the current scope. */
const make = Effect.fn("Database.make")(function* <S extends AnySchema>(options: DatabaseOptions<S>) {
	const tuning: Tuning = { ...defaultTuning, ...options.tuning }
	const { migrations } = options
	const last = migrations[migrations.length - 1]
	const theory = yield* Effect.try({
		try: () => {
			verifyBundle(migrations)
			const theory = schemaDescriptor(options.schema)
			if (last === undefined || !schemasAgree(theory, last.to)) {
				throw new DbError({ operation: "Database.make", reason: { _tag: "InvalidArgument" } })
			}
			return theory as S
		},
		catch: (cause) => argumentError("Database.make", cause)
	})
	const opening: HostedOpenIn = {
		bundle: migrations.map((migration) => ({ name: migration.id, hash: migration.hash, schema: lower(migration.to) })),
		seed: hex(16),
		...(options.onOpen === "migrate" ? { create: hex(16) } : {}),
		probeWindow: tuning.probeWindow,
		checkpointEvery: tuning.checkpointEvery.toString(),
		checkpointKeep: tuning.checkpointKeep,
		maxEntryBytes: tuning.maxEntryBytes
	}
	const runtime = yield* runtimeHandle
	const directory = yield* scoped(
		"Database.cacheRelease",
		call(
			"Database.make",
			(done) => addon.runtimeDirectoryAcquire(runtime, options.cache.directory, done),
			addon.runtimeDirectoryTake
		),
		(owned) => (done) => addon.runtimeDirectoryClose(owned, false, done)
	)
	const hosted = yield* scoped(
		"Database.release",
		call(
			"Database.make",
			(done) => addon.hostedOpen(directory, "cache", JSON.stringify(opening), done),
			addon.hostedTake
		),
		(owned) => (done) => addon.hostedClose(owned, done)
	)
	let head: HeadOut | undefined
	const seen = () => head
	const driver = yield* Driver.make(
		hostedPort(hosted, (next) => {
			head = next
		}),
		options.store,
		{ concurrency: tuning.concurrency, executor: tuning.executor }
	)
	const opened = yield* Effect.flatMap(driver.run({ _tag: "Open" }), (settled) =>
		settledAs("Database.open", settled, "Opened")
	)
	if (opened.pending > 0 && options.onOpen === "verify") {
		return yield* Effect.fail(
			refused("Database.open", { _tag: "MigrationPending", next: migrations.length - opened.pending })
		)
	}
	const fresh = seen()?.revision === 0n
	for (let step = migrations.length - opened.pending; step < migrations.length; step++) {
		yield* migrateStep(migrations, step, hosted, driver.run, seen, tuning)
	}
	const database = new DatabaseLive(theory, hosted, driver.run, seen) as Database<S>
	const initial = migrations[0]
	const seed = populateOf(initial)
	if (options.onOpen === "migrate" && seed !== undefined && fresh) {
		yield* seedInitial(database, initial.hash, seed)
	}
	return database
})

/** A service key for one application's database. */
function tag<S extends AnySchema>(key: string): Context.Key<Database<S>, Database<S>> {
	return Context.Service<Database<S>>(key)
}

/** Provides `key` with a database opened from `options` for the layer's lifetime. */
function layer<S extends AnySchema>(
	key: Context.Key<Database<S>, Database<S>>,
	options: DatabaseOptions<S>
): Layer.Layer<Database<S>, DbError, Bumble> {
	return Layer.effect(key, make(options))
}

interface PoolOptions<S extends AnySchema> extends Omit<DatabaseOptions<S>, "store" | "cache"> {
	readonly store: (tenant: string) => ObjectStore
	readonly cache: (tenant: string) => { readonly directory: string }
	/** How long an unused tenant's database stays open. */
	readonly idleTimeToLive?: Duration.Input
}

/** Databases opened on demand per tenant and closed once idle. */
interface DatabasePool<S extends AnySchema> {
	get(tenant: string): Effect.Effect<Database<S>, DbError, Scope.Scope>
}

/** A pool of per-tenant databases owned by the current scope. */
const pool = Effect.fn("Database.pool")(function* <S extends AnySchema>(options: PoolOptions<S>) {
	const map = yield* RcMap.make({
		lookup: (tenant: string) => make({ ...options, store: options.store(tenant), cache: options.cache(tenant) }),
		idleTimeToLive: options.idleTimeToLive ?? "5 minutes"
	})
	return { get: (tenant: string) => RcMap.get(map, tenant) } satisfies DatabasePool<S>
})

/** A fresh random request id. */
function requestId(): RequestId {
	return RequestId.make(hex(16))
}

const Database = Object.freeze({ make, layer, pool, tag, requestId })

export type {
	Consistency,
	DatabaseOptions,
	DatabasePool,
	DatabaseReader,
	PoolOptions,
	SubmitOptions,
	SubmitOutcome,
	Tuning
}
export { Database, RequestId }
