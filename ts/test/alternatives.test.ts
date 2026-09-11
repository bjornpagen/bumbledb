import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime } from "effect"
import { alternatives } from "#alternatives.ts"
import { ChangeSet } from "#changes.ts"
import { closed, closedId } from "#closed.ts"
import { Schema } from "#compile.ts"
import { Db } from "#db.ts"
import { on } from "#face.ts"
import { i64, interval, str, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { schema } from "#schema.ts"
import { select } from "#selection.ts"
import { contained, key, mirrors } from "#statements.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

function declarations() {
	const Kind = closed("Kind", ["Imported", "Electronic", "Postal"])
	const Parent = relation("Parent", { id: u64, kind: closedId(Kind) })
	const Imported = relation("Imported", { parent: u64, evidence: str })
	const Electronic = relation("Electronic", { parent: u64, reference: str })
	const Postal = relation("Postal", { parent: u64, window: interval(i64) })
	const parent = key(Parent, ["id"])
	const arms = {
		Imported: key(Imported, ["parent"]),
		Electronic: key(Electronic, ["parent"]),
		Postal: key(Postal, ["parent"])
	}
	const keys = [parent, arms.Imported, arms.Electronic, arms.Postal]
	const members = { Kind, Parent, Imported, Electronic, Postal }
	const expansion = alternatives(parent, "kind", Kind, arms)
	const manual = [
		contained(on(Parent, "kind"), on(Kind, "id")),
		mirrors(on(select(Parent, { kind: "Imported" }), "id"), on(Imported, "parent")),
		mirrors(on(select(Parent, { kind: "Electronic" }), "id"), on(Electronic, "parent")),
		mirrors(on(select(Parent, { kind: "Postal" }), "id"), on(Postal, "parent"))
	]
	const Theory = schema("Alternatives", members, [...keys, ...expansion])
	return { ...members, members, parent, arms, keys, expansion, manual, Theory }
}

test("alternatives are the ordinary law expansion in roster order, independent of arm object order", () => {
	const { parent, Kind, arms, expansion, manual, members, keys, Theory } = declarations()
	assert.deepEqual(expansion, manual)
	assert.ok(Object.isFrozen(expansion))
	assert.deepEqual(
		alternatives(parent, "kind", Kind, { Postal: arms.Postal, Electronic: arms.Electronic, Imported: arms.Imported }),
		manual
	)
	assert.deepEqual(lower(Theory), lower(schema("Alternatives", members, [...keys, ...manual])))
	const independent = declarations()
	assert.deepEqual(alternatives(parent, "kind", independent.Kind, independent.arms), expansion)
})

test("alternatives check dynamic arm coverage and scalar identity shapes", () => {
	const { parent, Kind, arms } = declarations()
	const Bad = relation("Bad", { id: i64 })
	const Span = relation("Span", { id: interval(i64), kind: closedId(Kind) })
	const cases: readonly unknown[] = [
		{ Imported: arms.Imported, Electronic: arms.Electronic },
		{ ...arms, Unknown: arms.Postal },
		{ ...arms, Postal: key(Bad, ["id"]) },
		{ ...arms, Postal: arms.Imported },
		{ ...arms, Postal: parent },
		{ ...arms, Postal: { ...arms.Postal, projection: ["missing"] } },
		{ ...arms, Postal: key(Span, ["id"]) },
		Object.create(arms)
	]
	for (const candidate of cases) assert.throws(() => alternatives(parent, "kind", Kind, candidate as never))
	assert.throws(() => alternatives(key(Span, ["id"]) as never, "kind" as never, Kind, arms as never))
	assert.throws(() => alternatives(parent, "id" as never, Kind, arms))
	assert.throws(() => alternatives(parent, "kind" as never, closed("Other", Kind.handles) as never, arms as never))
	const conflicting = relation("Parent", { id: i64, kind: closedId(Kind) })
	const original = declarations()
	assert.throws(() =>
		schema("Conflict", { ...original.members, Parent: conflicting }, [...original.keys, ...original.expansion])
	)
})

test("alternatives preserve composite keys and payload types", () => {
	const Kind = closed("Kind", ["A"])
	const Parent = relation("Parent", { tenant: str, id: u64, kind: closedId(Kind) })
	const Child = relation("Child", { tenant: str, parent: u64, payload: str })
	const parent = key(Parent, ["tenant", "id"])
	const child = key(Child, ["tenant", "parent"])
	assert.equal(alternatives(parent, "kind", Kind, { A: child }).length, 2)
	assert.deepEqual(child.owner.fields.payload, str)
})

// Uncalled type pins: dynamic callers are covered separately above.
function typePins() {
	const { parent, Kind, arms, Postal } = declarations()
	// @ts-expect-error Every closed arm is required.
	alternatives(parent, "kind", Kind, { Imported: arms.Imported, Electronic: arms.Electronic })
	// @ts-expect-error Unknown arms are not silently ignored.
	alternatives(parent, "kind", Kind, { ...arms, Extra: arms.Postal })
	// @ts-expect-error Only a matching closed discriminator is admitted.
	alternatives(parent, "id", Kind, arms)
	// @ts-expect-error A different roster does not match the discriminator.
	alternatives(parent, "kind", closed("Other", Kind.handles), arms)
	// @ts-expect-error Interval identity keys express point coverage.
	alternatives(parent, "kind", Kind, { ...arms, Postal: key(Postal, ["window"]) })
	const Bad = relation("Bad", { id: i64 })
	// @ts-expect-error Identity shapes must match positionally.
	alternatives(parent, "kind", Kind, { ...arms, Postal: key(Bad, ["id"]) })
}
void typePins

test("ordinary native admission enforces payloads and accepts an atomic arm switch", async () => {
	const { Parent, Imported, Electronic, Postal, Theory, members, keys, manual, expansion } = declarations()
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const compiled = yield* Schema.compile(Theory)
					const equivalent = yield* Schema.compile(schema("Alternatives", members, [...keys, ...manual]))
					assert.deepEqual(compiled.descriptor, equivalent.descriptor)
					assert.equal(compiled.schemaId, equivalent.schemaId)
					for (const missing of keys) {
						assert.throws(() => schema("Alternatives", members, [...keys.filter((k) => k !== missing), ...expansion]))
					}
					const db = yield* Db.create(storeDir("alternatives"), Theory)
					const options = { expected: { kind: "any" } } as const
					for (const invalid of ["missing", "wrong", "orphan", "conflict", "mismatched"] as const) {
						const draft = yield* ChangeSet.builder(Theory)
						if (invalid !== "orphan") yield* draft.insert(Parent, [{ id: 1n, kind: "Imported" }])
						if (invalid !== "missing")
							yield* draft.insert(Imported, [{ parent: invalid === "mismatched" ? 2n : 1n, evidence: "one" }])
						if (invalid === "wrong") yield* draft.insert(Electronic, [{ parent: 1n, reference: "wrong arm" }])
						if (invalid === "conflict") yield* draft.insert(Imported, [{ parent: 1n, evidence: "conflicting payload" }])
						const changes = yield* draft.finish()
						assert.equal((yield* db.judge(changes, options)).kind, "invariant-rejected", invalid)
						assert.equal((yield* db.apply(changes, options)).kind, "invariant-rejected", invalid)
					}
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Parent, [
						{ id: 1n, kind: "Imported" },
						{ id: 2n, kind: "Electronic" },
						{ id: 3n, kind: "Postal" }
					])
					yield* draft.insert(Imported, [{ parent: 1n, evidence: "one" }])
					yield* draft.insert(Electronic, [{ parent: 2n, reference: "two" }])
					yield* draft.insert(Postal, [{ parent: 3n, window: { start: -1n, end: 1n } }])
					assert.equal((yield* db.apply(yield* draft.finish(), options)).kind, "accepted")
					for (const reverse of [false, true]) {
						const change = yield* ChangeSet.builder(Theory)
						const remove = Effect.gen(function* () {
							yield* change.delete(Parent, [{ id: 1n, kind: "Imported" }])
							yield* change.delete(Imported, [{ parent: 1n, evidence: "one" }])
						})
						const add = Effect.gen(function* () {
							yield* change.insert(Parent, [{ id: 1n, kind: "Electronic" }])
							yield* change.insert(Electronic, [{ parent: 1n, reference: "replacement" }])
						})
						yield* reverse ? add : remove
						yield* reverse ? remove : add
						const changes = yield* change.finish()
						assert.equal((yield* db.judge(changes, options)).kind, "admitted")
						if (reverse) assert.equal((yield* db.apply(changes, options)).kind, "accepted")
					}
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})
