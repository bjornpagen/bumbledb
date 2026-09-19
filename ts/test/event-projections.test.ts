import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Option } from "effect"
import {
	alternatives,
	bool,
	ChangeSet,
	closed,
	contained,
	Db,
	Event,
	ExactRational,
	event,
	FiniteFunction,
	type Key,
	key,
	mirrors,
	NativeRuntime,
	on,
	query,
	relation,
	renderStatement,
	Schema,
	schema,
	select,
	u64,
	v
} from "#index.ts"
import { lower } from "#lower.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Roster = relation("Roster", { group: u64 })
const Branch = relation("Branch", { group: u64, choice: u64, when: event })
const fullKey = key(Roster, ["group", true])
const partitions = schema("Partitions", { Roster, Branch }, [
	key(Roster, ["group"]),
	fullKey,
	key(Branch, ["group", "when"]),
	contained(on(Branch, "group"), on(Roster, "group")),
	mirrors(on(Roster, ["group", true]), on(Branch, ["group", "when"]))
])

function typePins() {
	const singleton = key(Roster, [true])
	const emptyInput: Key<typeof singleton> = {}
	void emptyInput
	// @ts-expect-error A full-only key requires an empty record, not any object.
	const fabricated: Key<typeof singleton> = { true: true }
	void fabricated
	const input: Key<typeof fullKey> = { group: 1n }
	void input
	// @ts-expect-error Full is not a stored key field.
	const extra: Key<typeof fullKey> = { group: 1n, true: true }
	void extra
	// @ts-expect-error False is not Event projection syntax.
	on(Roster, ["group", false])
	// @ts-expect-error The full term must be trailing.
	on(Roster, [true, "group"])
	// @ts-expect-error Multiple full terms are invalid.
	key(Roster, [true, true])
	// @ts-expect-error A contextual Event does not pair with a numeric field.
	contained(on(Roster, true), on(Roster, "group"))
	// @ts-expect-error The scalar key cannot substitute for a full key.
	schema("MissingFullKey", { Roster, Branch }, [
		key(Roster, ["group"]),
		contained(on(Branch, ["group", "when"]), on(Roster, ["group", true]))
	])
}
void typePins

test("contextual full projections remain pure syntax with exact keys and no invented classes", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	assert.deepEqual(lower(partitions).statements[1], {
		kind: "fd",
		relation: "Roster",
		projection: ["group", { event: "full" }]
	})
	assert.equal(renderStatement(fullKey), "Roster(group, true) -> Roster")
	assert.equal(partitions.classes.Branch.when, undefined)
	assert.equal(partitions.classes.Roster.group, partitions.classes.Branch.group)
	assert.deepEqual(Object.keys(partitions.classes.Roster), ["group"])
	const Config = relation("Config", { true: bool })
	const both = schema("Both", { Config }, [key(Config, [true]), key(Config, ["true"])])
	assert.ok(both.statements[0] && both.statements[1])
	assert.notEqual(renderStatement(both.statements[0]), renderStatement(both.statements[1]))
	assert.equal(renderStatement(key(Config, ["true"])), 'Config("true") -> Config')
	const Quoted = relation("Quoted", { true: bool, '"true"': bool })
	assert.notEqual(renderStatement(key(Quoted, ["true"])), renderStatement(key(Quoted, ['"true"'])))
	const Token = closed("Token", ["Only"])
	const closedFull = key(Token, [true])
	assert.throws(
		() => alternatives(closedFull as never, "id" as never, Token, { Only: closedFull as never }),
		/ordinary relations/
	)
	for (const bad of [[true, "group"], [true, true], [false], ["group", { event: "full" }], new Array(1)])
		assert.throws(() => on(Roster, bad as never))
	assert.throws(() => on(Branch, ["when", true]), /scalar field prefix/)
	assert.throws(
		() =>
			schema("Missing", { Roster, Branch }, [
				key(Roster, ["group"]),
				contained(on(Branch, ["group", "when"]), on(Roster, ["group", true]))
			] as never),
		/matches no declared key/
	)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("full coverage partitions measured worlds, including zero-mass branches, through commit and reopen", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const path = storeDir("full-projections")
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const raw = yield* Event.space(new Uint8Array(32).fill(173), 2n)
					const measured = yield* FiniteFunction.designate(
						yield* FiniteFunction.new(raw, [
							{
								region: yield* Event.coordinate(raw, 0n),
								value: yield* ExactRational.fraction(1n, 2n)
							}
						])
					)
					const a = yield* Event.coordinate(measured, 0n)
					const b = yield* Event.complement(a)
					assert.equal(yield* ExactRational.toString(yield* Event.mass(b)), "0")
					assert.equal(yield* Event.count(b), 2n)
					const db = yield* Db.create(path, partitions)
					const missing = yield* ChangeSet.builder(partitions)
					yield* missing.insert(Roster, [{ group: 1n }])
					assert.equal(
						(yield* db.apply(yield* missing.finish(), { expected: { kind: "any" } })).kind,
						"invariant-rejected"
					)
					const left = { group: 1n, choice: 1n, when: a }
					const right = { group: 1n, choice: 2n, when: b }
					const draft = yield* ChangeSet.builder(partitions)
					yield* draft.insert(Roster, [{ group: 1n }])
					yield* draft.insert(Branch, [left, right])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const snapshot = yield* db.snapshot()
					assert.deepEqual(Option.getOrThrow(yield* snapshot.get(fullKey, { group: 1n })), { group: 1n })
					assert.ok(Option.isNone(yield* snapshot.get(fullKey, { group: 2n })))
					for (const mode of ["delete", "overlap", "orphan"] as const) {
						const change = yield* ChangeSet.builder(partitions)
						if (mode === "delete") yield* change.delete(Branch, [right])
						else if (mode === "overlap") yield* change.insert(Branch, [{ ...left, choice: 3n }])
						else yield* change.insert(Branch, [{ group: 2n, choice: 1n, when: yield* Event.empty(measured) }])
						assert.equal(
							(yield* db.apply(yield* change.finish(), { expected: { kind: "any" } })).kind,
							"invariant-rejected",
							mode
						)
					}
				})
			)
		)
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.open(path, partitions)
					const q = query(partitions).rule((r) => {
						const row = v(Branch)
						return r.match(Branch, row).find({ when: r.pack(row.when) })
					})
					const rows = yield* (yield* (yield* db.snapshot()).execute(q, {})).collect()
					assert.equal(rows.length, 1)
					assert.ok(rows[0])
					assert.equal(yield* Event.isFull(rows[0].when), true)
					assert.equal(yield* Event.count(rows[0].when), 4n)
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
})

test("full singleton lookup supplies no fabricated cell and remains distinct from a field named true", async () => {
	const Config = relation("Config", { true: bool, value: u64 })
	const singleton = key(Config, [true])
	const flagKey = key(Config, ["true"])
	const Theory = schema("Config", { Config }, [singleton, flagKey])
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("singleton-full"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Config, [{ true: false, value: 7n }])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const reader = yield* db.snapshot()
					assert.deepEqual(Option.getOrThrow(yield* reader.get(singleton, {})), { true: false, value: 7n })
					assert.ok(Option.isNone(yield* reader.get(flagKey, { true: true })))
					const bad = yield* Effect.flip(reader.get(singleton, { true: true } as never))
					assert.equal(bad.reason._tag, "InvalidArgument")
					const duplicate = yield* ChangeSet.builder(Theory)
					yield* duplicate.insert(Config, [{ true: true, value: 8n }])
					assert.equal(
						(yield* db.apply(yield* duplicate.finish(), { expected: { kind: "any" } })).kind,
						"invariant-rejected"
					)
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
})

test("closed scalar rosters admit explicit full keys and native ground coverage", async () => {
	const A = closed("A", ["One"], { code: u64 }, { One: { code: 1n } })
	const B = closed(
		"B",
		["One", "Two"],
		{ code: u64, active: bool },
		{ One: { code: 1n, active: true }, Two: { code: 2n, active: false } }
	)
	const a = key(A, ["code", true])
	const b = key(B, ["code", true])
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const good = schema("ClosedFull", { A, B }, [
					a,
					b,
					mirrors(on(A, ["code", true]), on(select(B, { active: true }), ["code", true]))
				])
				assert.ok((yield* Schema.compile(good)).schemaId)
				const bad = schema("MissingGround", { A, B }, [a, b, contained(on(B, ["code", true]), on(A, ["code", true]))])
				assert.equal((yield* Effect.flip(Schema.compile(bad))).reason._tag, "Engine")
			})
		)
	} finally {
		await runtime.dispose()
	}
})
