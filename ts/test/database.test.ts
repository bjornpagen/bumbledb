import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, test } from "node:test"
import type { Scope } from "effect"
import { Effect, Exit, ManagedRuntime, Option } from "effect"
import { ChangeSet } from "../src/changes.ts"
import type { DatabaseOptions } from "../src/database/database.ts"
import { Database } from "../src/database/database.ts"
import { FsStore } from "../src/database/fs.ts"
import type { ObjectStore } from "../src/database/io.ts"
import { MemStore } from "../src/database/mem.ts"
import type { Migrations } from "../src/database/migration.ts"
import { Migration } from "../src/database/migration.ts"
import type { DbError } from "../src/errors.ts"
import { str, u64 } from "../src/fields.ts"
import { query } from "../src/query/lower.ts"
import { v } from "../src/query/scope.ts"
import { relation } from "../src/relation.ts"
import { Bumble } from "../src/runtime.ts"
import type { AnySchema } from "../src/schema.ts"
import { schema } from "../src/schema.ts"
import { key } from "../src/statements.ts"
import { runtimeOptions } from "./fixtures/learning.ts"

const root = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-database-"))
after(() => fs.rmSync(root, { recursive: true, force: true }))
let directories = 0
const cacheDir = () => {
	directories += 1
	return path.join(root, `cache-${directories}`)
}

const Note = relation("Note", { id: u64, text: str })
const NoteById = key(Note, ["id"])
const V1 = schema("Notes", { Note }, [NoteById])
const Tag = relation("Tag", { note: u64, label: str })
const V2 = schema("Notes", { Note, Tag }, [NoteById, key(Tag, ["note", "label"])])

const hash = (digit: string) => digit.repeat(64)
const init = Migration.make({ id: "0001_init", hash: hash("1"), to: V1 })
const tags = Migration.make({
	id: "0002_tags",
	hash: hash("2"),
	from: V1,
	to: V2,
	populate: ({ from, into }) =>
		Effect.gen(function* () {
			const notes = yield* (yield* from.execute(allNotes, {})).collect()
			yield* into.insert(
				Tag,
				notes.map((note) => ({ note: note.id, label: "imported" }))
			)
		})
})
const allNotes = query(V1).rule((r) => {
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

function options<S extends AnySchema>(
	theory: S,
	migrations: Migrations,
	store: ObjectStore,
	onOpen: "migrate" | "verify" = "migrate"
): DatabaseOptions<S> {
	return { schema: theory, migrations, store, cache: { directory: cacheDir() }, onOpen }
}

async function run<A>(body: Effect.Effect<A, DbError, Scope.Scope | Bumble>): Promise<A> {
	const runtime = ManagedRuntime.make(Bumble.layer(runtimeOptions))
	try {
		return await runtime.runPromise(Effect.scoped(body))
	} finally {
		await runtime.dispose()
	}
}

const notes = (rows: readonly { readonly id: bigint; readonly text: string }[]) =>
	Effect.gen(function* () {
		const draft = yield* ChangeSet.builder(V1)
		yield* draft.insert(Note, rows)
		return yield* draft.finish()
	})

test("a submitted command is decided once and read back at the latest position", async () => {
	const store = MemStore.make()
	await run(
		Effect.gen(function* () {
			const db = yield* Database.make(options(V1, [init], store))
			const requestId = Database.requestId()
			const changes = yield* notes([{ id: 1n, text: "one" }])
			const outcome = yield* db.submit(changes, { requestId })
			assert.equal(outcome._tag, "Decided")
			if (outcome._tag !== "Decided") return
			assert.equal(outcome.receipt.outcome._tag, "Committed")
			const again = yield* db.submit(changes, { requestId })
			assert.deepEqual(again, outcome, "a request id is decided at most once")
			const reader = yield* db.read("latest")
			assert.ok(reader.seq >= outcome.receipt.seq)
			assert.deepEqual(yield* reader.get(NoteById, { id: 1n }), Option.some({ id: 1n, text: "one" }))
			assert.deepEqual(yield* db.resolve(requestId), Option.some(outcome.receipt))
		})
	)
})

test("a second process sees another's commit once it reads at least that position", async () => {
	const store = MemStore.make()
	await run(
		Effect.gen(function* () {
			const writer = yield* Database.make(options(V1, [init], store))
			const reader = yield* Database.make(options(V1, [init], store))
			const outcome = yield* writer.submit(yield* notes([{ id: 7n, text: "seven" }]), {
				requestId: Database.requestId()
			})
			assert.ok(outcome._tag === "Decided")
			const cached = yield* reader.read("cached")
			assert.ok(Option.isNone(yield* cached.get(NoteById, { id: 7n })), "a cached read does not reach the log")
			const fresh = yield* reader.read({ atLeast: outcome.receipt.seq })
			assert.deepEqual(yield* fresh.get(NoteById, { id: 7n }), Option.some({ id: 7n, text: "seven" }))
		})
	)
})

test("a precondition on a stale revision is decided as failed, without effect", async () => {
	const store = MemStore.make()
	await run(
		Effect.gen(function* () {
			const db = yield* Database.make(options(V1, [init], store))
			const first = yield* db.submit(yield* notes([{ id: 1n, text: "a" }]), { requestId: Database.requestId() })
			assert.ok(first._tag === "Decided")
			const stale = first.receipt.revision - 1n
			const second = yield* db.submit(yield* notes([{ id: 2n, text: "b" }]), {
				requestId: Database.requestId(),
				precondition: stale
			})
			assert.ok(second._tag === "Decided")
			assert.equal(second.receipt.outcome._tag, "PreconditionFailed")
			const reader = yield* db.read("latest")
			assert.ok(Option.isNone(yield* reader.get(NoteById, { id: 2n })))
		})
	)
})

test("an invariant violation is decided as rejected with the engine's evidence", async () => {
	const store = MemStore.make()
	await run(
		Effect.gen(function* () {
			const db = yield* Database.make(options(V1, [init], store))
			const conflicting = yield* notes([
				{ id: 1n, text: "a" },
				{ id: 1n, text: "b" }
			])
			const outcome = yield* db.submit(conflicting, { requestId: Database.requestId() })
			assert.ok(outcome._tag === "Decided")
			const decided = outcome.receipt.outcome
			assert.equal(decided._tag, "InvariantRejected")
			if (decided._tag === "InvariantRejected") assert.ok(decided.evidence.violations.length > 0)
		})
	)
})

test("bundled migrations run on open with migrate, and verify refuses while one is pending", async () => {
	const store = MemStore.make()
	await run(
		Effect.gen(function* () {
			yield* Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Database.make(options(V1, [init], store))
					const outcome = yield* db.submit(yield* notes([{ id: 3n, text: "three" }]), {
						requestId: Database.requestId()
					})
					assert.ok(outcome._tag === "Decided")
				})
			)
			const verify = yield* Effect.exit(Effect.scoped(Database.make(options(V2, [init, tags], store, "verify"))))
			assert.ok(Exit.isFailure(verify), "verify refuses a pending migration")
			const db = yield* Database.make(options(V2, [init, tags], store))
			const reader = yield* db.read("latest")
			assert.deepEqual(yield* (yield* reader.execute(notesV2, {})).collect(), [{ id: 3n, text: "three" }])
			assert.deepEqual(yield* (yield* reader.execute(tagsV2, {})).collect(), [{ note: 3n, label: "imported" }])
			const verified = yield* Effect.exit(Effect.scoped(Database.make(options(V2, [init, tags], store, "verify"))))
			assert.ok(Exit.isSuccess(verified), "verify opens once nothing is pending")
		})
	)
})

test("a directory store keeps the log across processes: a fresh cache catches up from it", async () => {
	const store = FsStore.make(path.join(root, "log"))
	await run(
		Effect.gen(function* () {
			yield* Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Database.make(options(V1, [init], store))
					for (const id of [1n, 2n, 3n]) {
						const outcome = yield* db.submit(yield* notes([{ id, text: `note ${id}` }]), {
							requestId: Database.requestId()
						})
						assert.ok(outcome._tag === "Decided")
					}
				})
			)
			const reopened = yield* Database.make(options(V1, [init], store, "verify"))
			const reader = yield* reopened.read("latest")
			assert.equal((yield* (yield* reader.execute(allNotes, {})).collect()).length, 3)
		})
	)
})

test("a create whose response is lost is still decided exactly once", async () => {
	let puts = 0
	const store = MemStore.make({
		fault: ({ verb, key }) => {
			if (verb !== "putIfAbsent" || !key.startsWith("log/")) return undefined
			puts += 1
			return puts === 2 ? "Lose" : undefined
		}
	})
	await run(
		Effect.gen(function* () {
			const db = yield* Database.make(options(V1, [init], store))
			const outcome = yield* db.submit(yield* notes([{ id: 9n, text: "nine" }]), { requestId: Database.requestId() })
			assert.ok(outcome._tag === "Decided")
			assert.equal(outcome.receipt.outcome._tag, "Committed")
			const reader = yield* db.read("latest")
			assert.deepEqual(yield* reader.get(NoteById, { id: 9n }), Option.some({ id: 9n, text: "nine" }))
		})
	)
})

test("a pool opens one database per tenant", async () => {
	const stores = new Map<string, ObjectStore>()
	const storeOf = (tenant: string) => {
		const existing = stores.get(tenant)
		if (existing !== undefined) return existing
		const created = MemStore.make()
		stores.set(tenant, created)
		return created
	}
	await run(
		Effect.gen(function* () {
			const pool = yield* Database.pool({
				schema: V1,
				migrations: [init],
				onOpen: "migrate",
				store: storeOf,
				cache: (tenant) => ({ directory: path.join(root, `pool-${tenant}`) })
			})
			const a = yield* pool.get("a")
			const outcome = yield* a.submit(yield* notes([{ id: 1n, text: "a" }]), { requestId: Database.requestId() })
			assert.ok(outcome._tag === "Decided")
			const b = yield* pool.get("b")
			const reader = yield* b.read("latest")
			assert.ok(Option.isNone(yield* reader.get(NoteById, { id: 1n })), "tenants do not share a log")
			assert.equal(yield* pool.get("a"), a, "an open tenant is reused")
		})
	)
})

test("the initial migration seeds a created database once, after every migration ran", async () => {
	const seeded = Migration.make({
		id: "0001_init",
		hash: hash("1"),
		to: V1,
		populate: ({ into }) => into.insert(Note, [{ id: 100n, text: "welcome" }])
	})
	const store = MemStore.make()
	await run(
		Effect.gen(function* () {
			const revisions: bigint[] = []
			for (let open = 0; open < 2; open++) {
				const db = yield* Effect.scoped(
					Effect.gen(function* () {
						const db = yield* Database.make(options(V2, [seeded, tags], store))
						const reader = yield* db.read("latest")
						return {
							notes: yield* (yield* reader.execute(notesV2, {})).collect(),
							revision: reader.revision
						}
					})
				)
				assert.deepEqual(db.notes, [{ id: 100n, text: "welcome" }])
				revisions.push(db.revision)
			}
			assert.equal(revisions[1], revisions[0], "reopening does not seed again")
		})
	)
})
