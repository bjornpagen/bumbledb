import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import type { Scope } from "effect"
import { Effect, ManagedRuntime, Option } from "effect"
import { ChangeSet } from "../../src/changes.ts"
import { Database } from "../../src/database/database.ts"
import type { ObjectStore } from "../../src/database/io.ts"
import { Migration } from "../../src/database/migration.ts"
import type { DbError } from "../../src/errors.ts"
import { str, u64 } from "../../src/fields.ts"
import { query } from "../../src/query/lower.ts"
import { v } from "../../src/query/scope.ts"
import { relation } from "../../src/relation.ts"
import { Bumble } from "../../src/runtime.ts"
import { schema } from "../../src/schema.ts"
import { key } from "../../src/statements.ts"
import { runtimeOptions } from "./learning.ts"

const Note = relation("Note", { id: u64, text: str })
const NoteById = key(Note, ["id"])
const V1 = schema("Notes", { Note }, [NoteById])
const Tag = relation("Tag", { note: u64, label: str })
const V2 = schema("Notes", { Note, Tag }, [NoteById, key(Tag, ["note", "label"])])

const notesV1 = query(V1).rule((r) => {
	const { id, text } = v(Note)
	return r.match(Note, { id, text }).find({ id, text })
})
const notesV2 = query(V2).rule((r) => {
	const { id, text } = v(Note)
	return r.match(Note, { id, text }).find({ id, text })
})
const tagsV2 = query(V2).rule((r) => {
	const { note, label } = v(Tag)
	return r.match(Tag, { note, label }).find({ note, label })
})

const init = Migration.make({ id: "0001_init", hash: "1".repeat(64), to: V1 })
const tags = Migration.make({
	id: "0002_tags",
	hash: "2".repeat(64),
	from: V1,
	to: V2,
	populate: ({ from, into }) =>
		Effect.gen(function* () {
			const notes = yield* (yield* from.execute(notesV1, {})).collect()
			yield* into.insert(
				Tag,
				notes.map((note) => ({ note: note.id, label: "imported" }))
			)
		})
})

async function run<A>(body: Effect.Effect<A, DbError, Scope.Scope | Bumble>): Promise<A> {
	const runtime = ManagedRuntime.make(Bumble.layer(runtimeOptions))
	try {
		return await runtime.runPromise(Effect.scoped(body))
	} finally {
		await runtime.dispose()
	}
}

/**
 * `store`, recording the checkpoint images a cache asked to upload, the first one that was created,
 * and the ones it served to a cache. Log reads wait for the first checkpoint listing to answer, so a
 * cold open always sees the checkpoint before it could replay past it.
 */
function watched(store: ObjectStore) {
	const listed = Promise.withResolvers<void>()
	const created = Promise.withResolvers<void>()
	const offered = new Set<string>()
	const served: string[] = []
	const watchedStore: ObjectStore = {
		...store,
		putIfAbsent: (bucket, name, body) =>
			Effect.suspend(() => {
				if (name.startsWith("ckpt/")) offered.add(name)
				return Effect.tap(store.putIfAbsent(bucket, name, body), (reply) =>
					Effect.sync(() => {
						if (name.startsWith("ckpt/") && reply.result._tag === "Created") created.resolve()
					})
				)
			}),
		list: (prefix, startAfter, maxKeys) =>
			Effect.ensuring(
				store.list(prefix, startAfter, maxKeys),
				Effect.sync(() => listed.resolve())
			),
		get: (bucket, name, target) =>
			Effect.andThen(
				bucket === "Log" ? Effect.promise(() => listed.promise) : Effect.void,
				Effect.tap(store.get(bucket, name, target), (reply) =>
					Effect.sync(() => {
						if (name.startsWith("ckpt/") && reply.result._tag === "Saved") served.push(name)
					})
				)
			)
	}
	return { store: watchedStore, created: created.promise, offered, served }
}

/**
 * The hosted `Database` contract over `store()`: a second process with an empty cache opens from the
 * checkpoint image the first uploaded, and bundled migrations run once and are read by every later
 * process.
 */
export function hostedConformance(name: string, store: () => ObjectStore): void {
	const caches = () => fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-hosted-"))

	test(`${name}: an empty cache opens from the checkpoint another process uploaded`, async () => {
		const root = caches()
		const log = watched(store())
		try {
			const first = Database.requestId()
			await run(
				Effect.gen(function* () {
					const db = yield* Database.make({
						schema: V1,
						migrations: [init],
						store: log.store,
						cache: { directory: path.join(root, "a") },
						onOpen: "migrate",
						tuning: { checkpointEvery: 2n }
					})
					for (const id of [1n, 2n, 3n]) {
						const draft = yield* ChangeSet.builder(V1)
						yield* draft.insert(Note, [{ id, text: `note ${id}` }])
						const outcome = yield* db.submit(yield* draft.finish(), {
							requestId: id === 1n ? first : Database.requestId()
						})
						assert.equal(outcome._tag, "Decided")
					}
					yield* Effect.promise(() => log.created)
				})
			)
			await run(
				Effect.gen(function* () {
					const db = yield* Database.make({
						schema: V1,
						migrations: [init],
						store: log.store,
						cache: { directory: path.join(root, "b") },
						onOpen: "verify"
					})
					assert.equal(log.served.length, 1, "the cold open installed one image")
					assert.ok(log.offered.has(log.served[0] ?? ""), "the image is one the first process uploaded")
					const reader = yield* db.read("latest")
					const rows = yield* (yield* reader.execute(notesV1, {})).collect()
					assert.deepEqual(rows.map((row) => row.id).sort(), [1n, 2n, 3n], "the image and the log tail hold every note")
					assert.ok(Option.isSome(yield* db.resolve(first)))
				})
			)
		} finally {
			fs.rmSync(root, { recursive: true, force: true })
		}
	})

	test(`${name}: a migration runs once on open and every later process reads its population`, async () => {
		const root = caches()
		const shared = store()
		try {
			await run(
				Effect.gen(function* () {
					const db = yield* Database.make({
						schema: V1,
						migrations: [init],
						store: shared,
						cache: { directory: path.join(root, "a") },
						onOpen: "migrate"
					})
					const draft = yield* ChangeSet.builder(V1)
					yield* draft.insert(Note, [{ id: 7n, text: "seven" }])
					const outcome = yield* db.submit(yield* draft.finish(), { requestId: Database.requestId() })
					assert.equal(outcome._tag, "Decided")
				})
			)
			for (const [cache, onOpen] of [
				["b", "migrate"],
				["c", "verify"]
			] as const) {
				await run(
					Effect.gen(function* () {
						const db = yield* Database.make({
							schema: V2,
							migrations: [init, tags],
							store: shared,
							cache: { directory: path.join(root, cache) },
							onOpen
						})
						const reader = yield* db.read("latest")
						assert.deepEqual(yield* (yield* reader.execute(notesV2, {})).collect(), [{ id: 7n, text: "seven" }])
						assert.deepEqual(yield* (yield* reader.execute(tagsV2, {})).collect(), [{ note: 7n, label: "imported" }])
					})
				)
			}
		} finally {
			fs.rmSync(root, { recursive: true, force: true })
		}
	})
}
