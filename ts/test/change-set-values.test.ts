import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Option, Stream } from "effect"
import { type ChangeRecord, ChangeSet } from "#changes.ts"
import { closed, closedId } from "#closed.ts"
import { Db } from "#db.ts"
import { dbNative } from "#db-native.ts"
import { on } from "#face.ts"
import { bool, bytes, f64, i64, interval, str, u64, uuid } from "#fields.ts"
import { type Fact, relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { schema } from "#schema.ts"
import { contained, key } from "#statements.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Item = relation("Item", { id: u64, label: str })
const ItemById = key(Item, ["id"])
const Other = relation("Other", { active: bool })
const Theory = schema("Changes", { Item, Other }, [ItemById])

const build = Effect.fn(function* (adds: readonly Fact<typeof Item>[], removes: readonly Fact<typeof Item>[] = []) {
	const draft = yield* ChangeSet.builder(Theory)
	yield* draft.insert(Item, adds)
	yield* draft.delete(Item, removes)
	return yield* draft.finish()
})

async function run(body: Effect.Effect<void, unknown, import("effect").Scope.Scope | NativeRuntime>) {
	const rt = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await rt.runPromise(Effect.scoped(body))
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
}

test("typed records, canonical byte ownership and counts round trip without a database", () =>
	run(
		Effect.gen(function* () {
			const draft = yield* ChangeSet.builder(Theory)
			yield* draft.insert(Item, [
				{ id: 2n, label: "two" },
				{ id: 1n, label: "one" },
				{ id: 1n, label: "one" }
			])
			yield* draft.delete(Item, [
				{ id: 1n, label: "one" },
				{ id: 3n, label: "three" }
			])
			yield* draft.insert(Other, [{ active: true }])
			const changes = yield* draft.finish()
			assert.deepEqual(changes.counts, { added: 3n, removed: 1n })
			const records = yield* Stream.runCollect(changes.records())
			assert.deepEqual(records, [
				{ relation: "Item", kind: "add", fact: { id: 1n, label: "one" } },
				{ relation: "Item", kind: "add", fact: { id: 2n, label: "two" } },
				{ relation: "Item", kind: "remove", fact: { id: 3n, label: "three" } },
				{ relation: "Other", kind: "add", fact: { active: true } }
			])
			for (const record of records) {
				// The relation name narrows the fact without a cast.
				if (record.relation === "Item") {
					const id: bigint = record.fact.id
					assert.ok(id > 0n)
				} else {
					const active: boolean = record.fact.active
					assert.equal(active, true)
				}
			}
			const encoded = yield* changes.toBytes()
			assert.equal(BigInt(encoded.byteLength), changes.byteLength)
			const decoded = yield* ChangeSet.fromBytes(Theory, encoded)
			assert.equal(decoded.schemaId, changes.schemaId)
			assert.deepEqual(decoded.counts, changes.counts)
			assert.deepEqual(yield* decoded.toBytes(), encoded)
			encoded.fill(0)
			assert.deepEqual(yield* Stream.runCollect(decoded.records()), records)
			assert.deepEqual(yield* Stream.runCollect(changes.records()), records)
			assert.notEqual((yield* changes.toBytes())[0], 0)
			const duringCompile = yield* changes.toBytes()
			const compile = dbNative.runtimeSchemaCompile
			dbNative.runtimeSchemaCompile = (runtime, spec, callback) => {
				duringCompile.fill(0)
				return compile(runtime, spec, callback)
			}
			try {
				const owned = yield* ChangeSet.fromBytes(Theory, duringCompile)
				assert.deepEqual(yield* Stream.runCollect(owned.records()), records)
			} finally {
				dbNative.runtimeSchemaCompile = compile
			}
		})
	))

test("composition is add-wins, commutative and idempotent; admission still judges distinct key conflicts", () =>
	run(
		Effect.gen(function* () {
			const a = { id: 1n, label: "a" }
			const b = { id: 2n, label: "b" }
			const left = yield* build([a], [b])
			const right = yield* build([b], [a])
			const expected = yield* build([a, b])
			const lr = yield* left.compose(right)
			const rl = yield* right.compose(left)
			assert.deepEqual(yield* lr.toBytes(), yield* expected.toBytes())
			assert.deepEqual(yield* rl.toBytes(), yield* lr.toBytes())
			assert.deepEqual(yield* (yield* lr.compose(lr)).toBytes(), yield* lr.toBytes())
			assert.deepEqual(yield* (yield* lr.compose(yield* build([]))).toBytes(), yield* lr.toBytes())
			const third = yield* build([], [b])
			assert.deepEqual(
				yield* (yield* lr.compose(third)).toBytes(),
				yield* (yield* left.compose(yield* right.compose(third))).toBytes()
			)
			const conflict = yield* build([{ ...a, label: "different" }])
			const conflicting = yield* left.compose(conflict)
			assert.deepEqual(conflicting.counts, { added: 2n, removed: 1n })
			const db = yield* Db.create(storeDir("change-composition"), Theory)
			assert.equal((yield* db.judge(conflicting, { expected: { kind: "any" } })).kind, "invariant-rejected")
			assert.equal((yield* db.apply(lr, { expected: { kind: "any" } })).kind, "accepted")
			const noChange = yield* db.judge(lr, { expected: { kind: "any" } })
			assert.equal(noChange.kind, "admitted")
			if (noChange.kind === "admitted") assert.deepEqual(noChange.changes, { added: 0n, removed: 0n })
			assert.deepEqual(lr.counts, { added: 2n, removed: 0n }, "requested counts are independent of current state")
			// Sequential commands are different: a later removal really does remove.
			yield* db.apply(third, { expected: { kind: "any" } })
			const snapshot = yield* db.snapshot()
			assert.ok(Option.isNone(yield* snapshot.get(ItemById, { id: b.id })))
		})
	))

test("checked bytes reject malformed framing, order, kinds, foreign schemas and unsafe backing", () =>
	run(
		Effect.gen(function* () {
			const changes = yield* build([
				{ id: 1n, label: "a" },
				{ id: 2n, label: "b" }
			])
			const bytes = yield* changes.toBytes()
			const bad = (offset: number, byte: number) => {
				const value = new Uint8Array(bytes)
				value[offset] = byte
				return value
			}
			const duplicate = new Uint8Array(bytes)
			const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
			const firstRecordLength = 13 + Number(view.getBigUint64(55))
			duplicate.copyWithin(50 + firstRecordLength, 50, 50 + firstRecordLength)
			const reordered = new Uint8Array(bytes)
			reordered.set(bytes.subarray(50 + firstRecordLength), 50)
			reordered.set(bytes.subarray(50, 50 + firstRecordLength), 50 + firstRecordLength)
			for (const invalid of [
				bytes.subarray(0, 49),
				bytes.subarray(0, bytes.length - 1),
				bad(0, 0),
				bad(9, 2),
				bad(10, view.getUint8(10) ^ 1),
				bad(50, 2),
				duplicate,
				reordered,
				new Uint8Array([...bytes, 0])
			]) {
				assert.ok(Exit.isFailure(yield* Effect.exit(ChangeSet.fromBytes(Theory, invalid))))
			}
			const foreign = schema("OtherSchema", { Other }, [])
			assert.ok(Exit.isFailure(yield* Effect.exit(ChangeSet.fromBytes(foreign, bytes))))
			const shared = new Uint8Array(new SharedArrayBuffer(bytes.length))
			shared.set(bytes)
			Object.defineProperty(shared, "buffer", { value: new ArrayBuffer(bytes.length) })
			const detached = new Uint8Array(bytes)
			structuredClone(detached.buffer, { transfer: [detached.buffer] })
			for (const invalid of [shared, detached])
				assert.ok(Exit.isFailure(yield* Effect.exit(ChangeSet.fromBytes(Theory, invalid))))
			assert.deepEqual(yield* changes.toBytes(), bytes, "refusal cannot damage the reusable source")
		})
	))

test("inspection is bounded, repeatable and releases early-terminated cursors", () =>
	run(
		Effect.gen(function* () {
			const rows = Array.from({ length: 10_000 }, (_, i) => ({ id: BigInt(i), label: "x" }))
			const changes = yield* build(rows)
			const service = yield* NativeRuntime
			const before = yield* service.inspect()
			let copied = 0
			let pulls = 0
			const take = dbNative.runtimeChangePageTake
			dbNative.runtimeChangePageTake = (operation) => {
				const page = take(operation)
				copied += page?.length ?? 0
				pulls++
				return page
			}
			try {
				assert.deepEqual(yield* Stream.runCollect(changes.records().pipe(Stream.take(1))), [
					{ relation: "Item", kind: "add", fact: rows[0] }
				])
				assert.equal(pulls, 1)
				assert.ok(copied > 0 && copied <= 256)
				const after = yield* service.inspect()
				assert.equal(after.natives, before.natives)
				assert.equal(after.retained, before.retained)
				const all = yield* Stream.runCollect(changes.records())
				assert.equal(all.length, rows.length)
				assert.deepEqual(
					all.map((record) => record.fact),
					rows
				)
			} finally {
				dbNative.runtimeChangePageTake = take
			}
		})
	))

test("inspection uses the shared field codec for all scalars, intervals and closed handles", () =>
	run(
		Effect.gen(function* () {
			const Kind = closed("Kind", ["a", "b"])
			const Values = relation("Values", {
				id: uuid,
				kind: closedId(Kind),
				yes: bool,
				integer: u64,
				signed: i64,
				float: f64,
				text: str,
				bytes: bytes(3),
				discrete: interval(i64),
				fixed: interval(u64, 3n),
				dense: interval(f64)
			})
			const theory = schema("Values", { Kind, Values }, [contained(on(Values, "kind"), on(Kind, "id"))])
			const fact: Fact<typeof Values> = {
				id: crypto.randomUUID(),
				kind: "b",
				yes: true,
				integer: (1n << 64n) - 1n,
				signed: -1n,
				float: Number.NaN,
				text: "🥝",
				bytes: new Uint8Array([0, 1, 255]),
				discrete: { start: -3n, end: 5n },
				fixed: { start: 0n, end: 3n },
				dense: { start: Number.NEGATIVE_INFINITY, end: 0.25 }
			}
			const draft = yield* ChangeSet.builder(theory)
			yield* draft.insert(Values, [fact])
			const changes = yield* draft.finish()
			const decoded = yield* ChangeSet.fromBytes(theory, yield* changes.toBytes())
			const expected: ChangeRecord<typeof theory>[] = [{ relation: "Values", kind: "add", fact }]
			assert.deepEqual(yield* Stream.runCollect(decoded.records()), expected)
		})
	))

test("lazy operations and retained wrappers obey closure, including an open cursor's independent share", () =>
	run(
		Effect.gen(function* () {
			const changes = yield* build([{ id: 1n, label: "one" }])
			const bytes = yield* changes.toBytes()
			const laterBytes = changes.toBytes()
			const laterCompose = changes.compose(changes)
			const laterRecords = changes.records()
			const close = changes.close()
			assert.deepEqual(yield* laterBytes, bytes, "constructing close is inert")
			// Opening a cursor retains the immutable source; later source close does not revoke it.
			const seen = yield* Stream.runCollect(changes.records().pipe(Stream.tap(() => close)))
			assert.equal(seen.length, 1)
			assert.ok(Exit.isFailure(yield* Effect.exit(laterBytes)))
			assert.ok(Exit.isFailure(yield* Effect.exit(laterCompose)))
			assert.ok(Exit.isFailure(yield* Effect.exit(Stream.runCollect(laterRecords))))
			assert.equal((yield* close).kind, "closed")
			const decoded = yield* Effect.scoped(ChangeSet.fromBytes(Theory, bytes))
			assert.ok(Exit.isFailure(yield* Effect.exit(decoded.toBytes())))
		})
	))

test("cancelled composition delivery cannot leak a native handle or consume its inputs", () =>
	run(
		Effect.gen(function* () {
			const changes = yield* build([{ id: 1n, label: "a" }])
			const original = dbNative.runtimeChangesCompose
			const completed = Promise.withResolvers<() => void>()
			dbNative.runtimeChangesCompose = (left, right, callback) =>
				original(left, right, () => completed.resolve(callback))
			try {
				const service = yield* NativeRuntime
				const before = yield* service.inspect()
				const child = yield* Effect.forkChild(Effect.scoped(changes.compose(changes)))
				const lateCallback = yield* Effect.promise(() => completed.promise)
				yield* Fiber.interrupt(child)
				assert.ok(Exit.hasInterrupts(yield* Fiber.await(child)))
				lateCallback()
				const after = yield* service.inspect()
				assert.equal(after.natives, before.natives)
				assert.equal(after.retained, before.retained)
				assert.equal((yield* changes.toBytes()).byteLength, Number(changes.byteLength))
			} finally {
				dbNative.runtimeChangesCompose = original
			}
		})
	))
