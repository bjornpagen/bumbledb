import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { closed } from "#closed.ts"
import { type BoolField, bool, type Infer, type U64Field, u64 } from "#fields.ts"
import { contained, Db, on, relation, schema } from "#index.ts"
import type { SelectionInput } from "#relation.ts"
import { accepted } from "#test/accepted.ts"
import { put } from "#test/put.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

type Expect<T extends true> = T extends true ? true : never

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-closed-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Kind = closed("Kind", ["Checking", "Savings"])
const Grade = closed(
	"Grade",
	{ mastered: bool, score: u64 },
	{
		DirectPass: { mastered: true, score: 100n },
		Retried: { mastered: true, score: 60n },
		Failed: { mastered: false, score: 0n }
	}
)

const Weird = closed("Weird", ["match", "where", "id"])

const bareSurface: readonly (keyof typeof Kind)[] = ["name", "id", "data", "axioms", "columns"]
const payloadSurface: readonly (keyof typeof Grade)[] = ["name", "id", "data", "axioms", "columns", "where"]

type Cases = [
	Expect<Equal<keyof typeof Kind, "name" | "id" | "data" | "axioms" | "columns">>,
	Expect<Equal<keyof typeof Grade, "name" | "id" | "data" | "axioms" | "columns" | "where">>,
	Expect<Equal<typeof Grade.columns.mastered, BoolField>>,
	Expect<Equal<typeof Grade.columns.score, U64Field>>,
	Expect<Equal<keyof typeof Grade.columns, "mastered" | "score">>,
	Expect<Equal<Infer<typeof Grade.id>, "DirectPass" | "Retried" | "Failed">>,
	Expect<Equal<Parameters<typeof Grade.where>[0], SelectionInput<typeof Grade.columns>>>,
	Expect<Equal<Infer<typeof Weird.id>, "match" | "where" | "id">>,
	Expect<Equal<keyof typeof Weird.axioms, "match" | "where" | "id">>
]

function handleConstantsDied(): unknown[] {
	return [
		// @ts-expect-error — H5: the handle constants died entirely; the literal "Checking" is the ONE spelling
		Kind.Checking,
		// @ts-expect-error — H5: not on the payload tier either; the literal "DirectPass" is the ONE spelling
		Grade.DirectPass
	]
}

function matchDied(): unknown {
	// @ts-expect-error — H5: Kind.match is hard-removed; dispatch is native switch narrowing over the handle union
	return Kind.match(0n, {})
}

function fromIdDied(): unknown {
	// @ts-expect-error — H5: fromId died with the bigint era; facts and rows already speak handle names
	return Kind.fromId(0n)
}

describe("the slimmed closed surface", function describeSurface() {
	test("the type claims EXACTLY the runtime properties, per tier", function probeTypeLieSweep() {
		assert.deepStrictEqual(Object.keys(Kind).toSorted(), [...bareSurface].toSorted())
		assert.deepStrictEqual(Object.keys(Grade).toSorted(), [...payloadSurface].toSorted())
		assert.ok(Object.isFrozen(Kind), "the bare tier's value is sealed")
		assert.ok(Object.isFrozen(Grade), "the payload tier's value is sealed")
	})

	test("where is an OWN function exactly when payload columns exist", function probeWhereMint() {
		assert.ok(Object.hasOwn(Grade, "where"), "the payload tier mints where")
		assert.equal(typeof Grade.where, "function")
		assert.equal(Object.hasOwn(Kind, "where"), false, "the bare tier mints no where")
	})

	test("where() still mints, seals, and rides the ONE selection machine", function probeWhereSelection() {
		const selected = Grade.where({ mastered: true })
		assert.strictEqual(selected.relation, Grade, "the ψ selection points back at the one minted value")
		assert.ok(Object.isFrozen(selected))
		assert.deepStrictEqual(selected.selection, [
			{ field: "mastered", set: { kind: "one", literal: { kind: "value", value: { kind: "bool", value: true } } } }
		])
	})
})

describe("handles named like methods — pure data, no reserved names", function describeWeird() {
	test("the vocabulary constructs and its axioms record keys correctly", function probeWeirdMint() {
		assert.deepStrictEqual(Weird.data.handles, ["match", "where", "id"])
		assert.deepStrictEqual(Object.keys(Weird.axioms), ["match", "where", "id"])
		assert.equal(
			Object.getPrototypeOf(Weird.axioms),
			Object.prototype,
			"method-named handles are own enumerable rows, never protocol accidents"
		)
		assert.deepStrictEqual(Weird.id, {
			kind: "u64",
			closed: { name: "Weird", handles: ["match", "where", "id"] }
		})

		assert.equal(Object.hasOwn(Weird, "where"), false)
		assert.equal(Object.hasOwn(Weird, "match"), false)
	})

	test("the roster round-trips through a real store", async function probeWeirdRoundTrip() {
		const Uses = relation("Uses", { id: u64.fresh, kind: Weird.id })
		const WeirdTheory = schema("WeirdTheory", { Weird, Uses }, [contained(on(Uses, "kind"), on(Weird, "id"))])
		const db = accepted(await Db.create(path.join(tmpRoot, "weird"), WeirdTheory))
		const result = db.write(function seed(tx) {
			const written = put(tx, Uses, { kind: "match" })
			assert.equal(typeof written.id, "bigint")
		})
		assert.equal(result.tag, "accepted", "the commit lands")
		const rows = db.read((i) => i.scan(Uses))
		assert.equal(rows.length, 1)
		assert.equal(rows[0]?.kind, "match", "the decoded fact speaks the handle name")
	})
})

describe("the 3-arg closed — the payload tier in one call", function describeThreeArg() {
	test("the 3-arg spelling mints the whole payload surface", function probeThreeArg() {
		assert.equal(Grade.name, "Grade")
		assert.deepStrictEqual(Grade.data.handles, ["DirectPass", "Retried", "Failed"])
		assert.equal(Grade.axioms.Retried.score, 60n)
		assert.equal(Grade.axioms.Failed.mastered, false)
		assert.ok(Object.isFrozen(Grade.axioms.DirectPass))
		assert.deepStrictEqual(Object.keys(Grade.columns), ["mastered", "score"])
	})

	test("the curried spelling is deleted: uncompilable, with a pointed runtime refusal", function probeCurriedRefusal() {
		assert.throws(function curriedAtRuntime() {
			// @ts-expect-error — closed(name, columns)(axioms) died with 0.3.0: the payload tier is closed(name, columns, axioms)
			closed("Grade", { mastered: bool })({ DirectPass: { mastered: true } })
		}, /closed relation Grade: payload columns declared without ground axioms/)
	})

	test("the bare tier takes no axioms — the inadmissible third argument is refused", function probeBareAxioms() {
		assert.throws(function bareWithAxioms() {
			// @ts-expect-error — a handle tuple declares no columns, so ground axioms are inadmissible
			closed("Bad", ["Solo"], { Solo: {} })
		}, /closed relation Bad: the bare tier declares no columns, so ground axioms are inadmissible/)
	})

	test("dishonest axiom values still face the ONE literal machine at construction", function probeLiteralMachine() {
		assert.throws(function wrongShapedAxiom() {
			// @ts-expect-error — a bool column refuses a string at the type; the literal machine is the runtime twin
			closed("Bad", { pages: bool }, { Loud: { pages: "yes" }, Quiet: { pages: false } })
		}, /expected boolean/)
	})

	test("a payload column named id is a construction-time error — the sealed shape mints the synthetic id itself", function probeIdColumn() {
		assert.throws(function idColumn() {
			// @ts-expect-error — "id" is unspellable in a column block (PayloadColumns); the runtime wall is the twin
			closed("Bad", { id: bool }, { A: { id: true }, B: { id: false } })
		}, /closed relation Bad: the payload column id collides with the sealed shape's synthetic id/)
	})
})

/**
 * Per-property failure locality: a wrong-typed axiom value errors ON its
 * property, not on the whole call — the directive sits on the property
 * line and is REAL. Never called (the same dishonest row would be refused
 * by the literal machine at construction).
 */
function wrongValueErrsOnItsProperty(): unknown {
	return closed(
		"Grade",
		{ mastered: bool },
		{
			Pass: {
				// @ts-expect-error — a bool column refuses a bigint, and the error lands on this property
				mastered: 1n
			},
			Fail: { mastered: false }
		}
	)
}

export type { Cases }
export { fromIdDied, handleConstantsDied, matchDied, wrongValueErrsOnItsProperty }
