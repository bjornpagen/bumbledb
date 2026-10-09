import * as fs from "node:fs/promises"
import { Effect } from "effect"
import { DbError } from "../errors.ts"
import type { Body, Bucket, Created, Deleted, Fetched, Listed, Millis, ObjectStore, Reply, Target } from "./io.ts"

type Verb = "get" | "putIfAbsent" | "list" | "delete"

/** `Fail`: the call fails before it has any effect. `Lose`: the call takes effect, then fails, like a lost response. */
type Fault = "Fail" | "Lose"

interface MemStoreOptions {
	/** The store clock behind `Date` and `Last-Modified`. Defaults to a counter that ticks once per call. */
	readonly clock?: () => Millis
	/** Picks the fault, if any, for each call. */
	readonly fault?: (call: { readonly verb: Verb; readonly bucket: Bucket; readonly key: string }) => Fault | undefined
}

interface Stored {
	readonly bytes: Uint8Array
	readonly lastModified: Millis
}

/** An in-memory store with the create-only semantics of S3, and deterministic fault injection for tests. */
interface MemStore extends ObjectStore {
	/** A copy of the objects currently in `bucket`, by key. */
	readonly objects: (bucket: Bucket) => ReadonlyMap<string, Uint8Array>
}

function failure(operation: string): DbError {
	return new DbError({ operation, reason: { _tag: "Io", kind: "Injected" } })
}

function make(options: MemStoreOptions = {}): MemStore {
	let ticks = 0n
	const clock =
		options.clock ??
		(() => {
			ticks += 1n
			return ticks
		})
	const buckets: Record<Bucket, Map<string, Stored>> = { Log: new Map(), Checkpoints: new Map() }

	function run<R>(
		verb: Verb,
		bucket: Bucket,
		key: string,
		body: (now: Millis) => Effect.Effect<R, DbError>
	): Effect.Effect<Reply<R>, DbError> {
		return Effect.suspend(() => {
			const fault = options.fault?.({ verb, bucket, key })
			const operation = `MemStore.${verb}`
			if (fault === "Fail") return Effect.fail(failure(operation))
			const now = clock()
			return Effect.flatMap(body(now), (result) =>
				fault === "Lose" ? Effect.fail(failure(operation)) : Effect.succeed({ date: now, result })
			)
		})
	}

	const io = <A>(operation: string, f: () => Promise<A>) =>
		Effect.tryPromise({
			try: f,
			catch: () => new DbError({ operation, reason: { _tag: "Io", kind: "File" } })
		})

	return {
		get: (bucket: Bucket, key: string, target: Target) =>
			run("get", bucket, key, (): Effect.Effect<Fetched, DbError> => {
				const stored = buckets[bucket].get(key)
				if (stored === undefined) return Effect.succeed({ _tag: "Missing" })
				if (target._tag === "Memory") {
					return Effect.succeed({ _tag: "Body", bytes: stored.bytes.slice(), lastModified: stored.lastModified })
				}
				return io("MemStore.get", () => fs.writeFile(target.path, stored.bytes)).pipe(
					Effect.as<Fetched>({ _tag: "Saved", lastModified: stored.lastModified })
				)
			}),
		putIfAbsent: (bucket: Bucket, key: string, body: Body) =>
			run("putIfAbsent", bucket, key, (now): Effect.Effect<Created, DbError> => {
				const bytes =
					body._tag === "Bytes"
						? Effect.succeed(body.bytes.slice())
						: io("MemStore.putIfAbsent", () => fs.readFile(body.path)).pipe(
								Effect.map((buffer) => new Uint8Array(buffer))
							)
				return Effect.map(bytes, (owned): Created => {
					const objects = buckets[bucket]
					if (objects.has(key)) return { _tag: "Occupied" }
					objects.set(key, { bytes: owned, lastModified: now })
					return { _tag: "Created" }
				})
			}),
		list: (prefix: string, startAfter: string | null, maxKeys: number) =>
			run("list", "Checkpoints", prefix, (): Effect.Effect<Listed> => {
				const keys = [...buckets.Checkpoints.keys()]
					.filter((key) => key.startsWith(prefix) && (startAfter === null || key > startAfter))
					.sort()
					.slice(0, maxKeys)
				return Effect.succeed({ _tag: "Keys", keys })
			}),
		delete: (key: string) =>
			run("delete", "Checkpoints", key, (): Effect.Effect<Deleted> => {
				buckets.Checkpoints.delete(key)
				return Effect.succeed({ _tag: "Deleted" })
			}),
		objects: (bucket: Bucket) => new Map([...buckets[bucket]].map(([key, stored]) => [key, stored.bytes.slice()]))
	}
}

const MemStore = Object.freeze({ make })

export type { Fault, MemStoreOptions }
export { MemStore }
