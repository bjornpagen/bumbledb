/**
 * The object-store side of the hosted log. The log machine emits `IoRequest`s as data; this module
 * runs them against an `ObjectStore` and turns every outcome into an `IoResponse`. Reads are
 * idempotent and retried here; a create-only PUT is never retried here, because the machine
 * resolves an ambiguous PUT by reading the key back.
 */
import type { Duration } from "effect"
import { Effect, Schedule } from "effect"
import { DbError } from "../runtime-errors.ts"

/** `Log` is the commit log (`log/`, an S3 Express directory bucket); `Checkpoints` holds `ckpt/` and `mig/`. */
type Bucket = "Log" | "Checkpoints"

/** Unix milliseconds on the store's clock. */
type Millis = bigint

/** Where a GET delivers the object: returned in memory, or written to a local file. */
type Target = { readonly _tag: "Memory" } | { readonly _tag: "File"; readonly path: string }

/** What a create-only PUT uploads: bytes in memory, or a local file. */
type Body = { readonly _tag: "Bytes"; readonly bytes: Uint8Array } | { readonly _tag: "File"; readonly path: string }

type Op =
	| { readonly _tag: "Get"; readonly target: Target }
	| { readonly _tag: "PutIfAbsent"; readonly body: Body }
	| { readonly _tag: "List"; readonly startAfter: string | null; readonly maxKeys: number }
	| { readonly _tag: "Delete" }

/** One request from the machine. `key` is relative to the database prefix. */
interface IoRequest {
	readonly id: bigint
	readonly bucket: Bucket
	readonly key: string
	readonly op: Op
}

type Fetched =
	| { readonly _tag: "Body"; readonly bytes: Uint8Array; readonly lastModified: Millis }
	| { readonly _tag: "Saved"; readonly lastModified: Millis }
	| { readonly _tag: "Missing" }

type Created = { readonly _tag: "Created" } | { readonly _tag: "Occupied" }

type Listed = { readonly _tag: "Keys"; readonly keys: readonly string[] }

type Deleted = { readonly _tag: "Deleted" }

/** `Failed` covers everything a store could not answer definitively: 409, 5xx, timeouts, transport. */
type IoResult = Fetched | Created | Listed | Deleted | { readonly _tag: "Failed" }

interface IoResponse {
	readonly id: bigint
	/** The store's `Date` header, when it sent one. */
	readonly date: Millis | null
	readonly result: IoResult
}

/** A store answer plus the store's clock reading. */
interface Reply<R> {
	readonly date: Millis | null
	readonly result: R
}

/**
 * The four verbs the log needs. Keys arrive relative; a store owns its own prefix. LIST and DELETE
 * exist only for `Checkpoints`: LIST order on an Express bucket is not lexicographic, and a `log/`
 * key must never disappear.
 */
interface ObjectStore {
	readonly get: (bucket: Bucket, key: string, target: Target) => Effect.Effect<Reply<Fetched>, DbError>
	readonly putIfAbsent: (bucket: Bucket, key: string, body: Body) => Effect.Effect<Reply<Created>, DbError>
	/** Checkpoint keys under `prefix` after `startAfter`, in lexicographic order. */
	readonly list: (prefix: string, startAfter: string | null, maxKeys: number) => Effect.Effect<Reply<Listed>, DbError>
	/** Deletes one checkpoint key; deleting a missing key succeeds. */
	readonly delete: (key: string) => Effect.Effect<Reply<Deleted>, DbError>
}

interface ExecutorOptions {
	/** Bound on one store call; a call that exceeds it counts as `Failed` (or is retried, for a read). */
	readonly timeout: Duration.Input
	/** Retry policy for GET, LIST and DELETE. */
	readonly readRetry: Schedule.Schedule<unknown, unknown>
}

const defaultExecutorOptions: ExecutorOptions = {
	timeout: "10 seconds",
	readRetry: Schedule.exponential("20 millis").pipe(Schedule.jittered, Schedule.upTo({ times: 4 }))
}

function refused(request: IoRequest): Effect.Effect<never, DbError> {
	return Effect.fail(new DbError({ operation: `ObjectStore.${request.op._tag}`, reason: { _tag: "InvalidArgument" } }))
}

function dispatch(store: ObjectStore, request: IoRequest): Effect.Effect<Reply<IoResult>, DbError> {
	const { bucket, key, op } = request
	switch (op._tag) {
		case "Get":
			return store.get(bucket, key, op.target)
		case "PutIfAbsent":
			return store.putIfAbsent(bucket, key, op.body)
		case "List":
			return bucket === "Checkpoints" ? store.list(key, op.startAfter, op.maxKeys) : refused(request)
		case "Delete":
			return bucket === "Checkpoints" ? store.delete(key) : refused(request)
	}
}

/** Runs one request to an `IoResponse`; it never fails. */
function execute(
	store: ObjectStore,
	request: IoRequest,
	options: ExecutorOptions = defaultExecutorOptions
): Effect.Effect<IoResponse> {
	const once = Effect.timeout(dispatch(store, request), options.timeout)
	const attempts = request.op._tag === "PutIfAbsent" ? once : Effect.retry(once, options.readRetry)
	return attempts.pipe(
		Effect.map((reply): IoResponse => ({ id: request.id, date: reply.date, result: reply.result })),
		Effect.catch(() => Effect.succeed<IoResponse>({ id: request.id, date: null, result: { _tag: "Failed" } }))
	)
}

export type {
	Body,
	Bucket,
	Created,
	Deleted,
	ExecutorOptions,
	Fetched,
	IoRequest,
	IoResponse,
	IoResult,
	Listed,
	Millis,
	ObjectStore,
	Op,
	Reply,
	Target
}
export { defaultExecutorOptions, execute }
