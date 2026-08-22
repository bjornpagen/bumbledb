/**
 * H1 pins — the precise roster type. A closed reference descriptor carries
 * its vocabulary name AND handle union in the TYPE (`ClosedIdField<"Kind",
 * "DirectPass" | "JudgedPass" | "Failed">` — the name literal keeps two
 * same-shaped vocabularies distinct, 063), `Infer` yields the union as the
 * column's VALUE TYPE, and
 * every `Infer`-reading surface (`Fact`) sees it. A wrong
 * string is a COMPILE error (real `@ts-expect-error` fail-probes), and a
 * bigint is no longer assignable to a closed-referencing column. The
 * type-lie law's runtime twin: the precise type's carrier is the SAME
 * frozen declaration-order handles array that was always there — pinned
 * own-property by own-property at the bottom.
 */

import assert from "node:assert/strict"
import { test } from "node:test"

import { closed } from "#closed.ts"
import { on } from "#face.ts"
import { type ClosedIdField, type ClosedRoster, type Infer, u64 } from "#fields.ts"
import { type Fact, relation } from "#relation.ts"
import { contained } from "#statements.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

type Expect<T extends true> = T extends true ? true : never

const Kind = closed("Kind", ["DirectPass", "JudgedPass", "Failed"])

const Method = closed("Method", ["DirectPass", "Manual"])

const Certificate = relation("Certificate", {
	id: u64.fresh,
	student: u64,
	kind: Kind.id
})

const wellTyped: Fact<typeof Certificate> = { id: 1n, student: 7n, kind: "DirectPass" }

type Cases = [
	Expect<Equal<Infer<typeof Kind.id>, "DirectPass" | "JudgedPass" | "Failed">>,
	Expect<Equal<Fact<typeof Certificate>["kind"], "DirectPass" | "JudgedPass" | "Failed">>,
	Expect<
		Equal<
			Fact<typeof Certificate>,
			{
				id: bigint
				student: bigint
				kind: "DirectPass" | "JudgedPass" | "Failed"
			}
		>
	>,
	Expect<Equal<typeof Kind.id, ClosedIdField<"Kind", "DirectPass" | "JudgedPass" | "Failed">>>,
	Expect<Equal<(typeof Kind.id)["closed"], ClosedRoster<"Kind", "DirectPass" | "JudgedPass" | "Failed">>>,
	Expect<Equal<typeof Kind.id extends ClosedIdField ? true : false, true>>
]

type OverlapCases = [
	Expect<Equal<Extract<Infer<typeof Method.id>, Infer<typeof Kind.id>>, "DirectPass">>,
	// the non-shared names do NOT cross vocabularies
	Expect<Equal<"Manual" extends Infer<typeof Kind.id> ? true : false, false>>,
	Expect<Equal<"Failed" extends Infer<typeof Method.id> ? true : false, false>>
]

function sharedHandleAssignsAcrossVocabularies(shared: "DirectPass"): [Infer<typeof Kind.id>, Infer<typeof Method.id>] {
	return [shared, shared]
}

function insertRefusals(): unknown[] {
	// @ts-expect-error — H1: "DirectPas" is a typo off the roster — a wrong string is a compile error
	const typo: Fact<typeof Certificate> = { id: 1n, student: 7n, kind: "DirectPas" }

	// @ts-expect-error — H1: a bigint no longer types a closed-referencing column — the value type is the handle union
	const forgedId: Fact<typeof Certificate> = { id: 1n, student: 7n, kind: 0n }
	return [typo, forgedId]
}

test("two same-shaped vocabularies are distinct at BOTH tiers — the roster slot carries the name literal", function probeSameShapedVocabularies() {
	const Answer = closed("Answer", ["DirectPass", "JudgedPass", "Failed"])
	const Cert = relation("Cert", { k: Kind.id })
	assert.throws(function crossVocabularyPairing() {
		// @ts-expect-error — 063: a Kind reference cannot pair with Answer's id — the type-tier roster slot compares [name, handles], matching the runtime's roster-identity judgment
		contained(on(Cert, "k"), on(Answer, "id"))
	}, /is a Kind reference but Answer\.id is a Answer reference/)
})

test("the precise type's runtime twin is the same frozen declaration-order roster", function probeRuntimeTwin() {
	assert.ok(Object.isFrozen(Kind.id))
	assert.ok(Object.isFrozen(Kind.id.closed))
	assert.ok(Object.isFrozen(Kind.id.closed.handles))
	assert.ok(Object.hasOwn(Kind.id, "kind"))
	assert.ok(Object.hasOwn(Kind.id, "closed"))
	assert.ok(Object.hasOwn(Kind.id.closed, "name"))
	assert.ok(Object.hasOwn(Kind.id.closed, "handles"))

	assert.deepStrictEqual(Kind.id, {
		kind: "u64",
		closed: { name: "Kind", handles: ["DirectPass", "JudgedPass", "Failed"] }
	})

	assert.equal(wellTyped.kind, "DirectPass")
	assert.deepStrictEqual(sharedHandleAssignsAcrossVocabularies("DirectPass"), ["DirectPass", "DirectPass"])
	assert.equal(insertRefusals().length, 2)
})

export type { Cases, OverlapCases }
export { insertRefusals, sharedHandleAssignsAcrossVocabularies }
