/**
 * H1 pins — the precise roster type. A closed reference descriptor carries
 * its vocabulary name AND handle union in the TYPE (`ClosedIdField<"Kind",
 * "DirectPass" | "JudgedPass" | "Failed">` — the name literal keeps two
 * same-shaped vocabularies distinct, 063), `Infer` yields the union as the
 * column's VALUE TYPE, and
 * every `Infer`-reading surface (`Fact`, `InsertFact`) sees it. A wrong
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
import { type Fact, type InsertFact, relation } from "#relation.ts"
import { contained } from "#statements.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
type Expect<T extends true> = T extends true ? true : never

const Kind = closed("Kind", ["DirectPass", "JudgedPass", "Failed"])

/**
 * A SECOND vocabulary sharing the `DirectPass` handle name — the structural
 * doctrine's honest overlap, pinned in {@link OverlapCases} below.
 */
const Method = closed("Method", ["DirectPass", "Manual"])

const Certificate = relation("Certificate", {
	id: u64.fresh,
	student: u64,
	kind: Kind.id
})

/**
 * `InsertFact` ACCEPTS the handle spelled as its string literal — the ONE
 * spelling (used at runtime below so the claim carries its twin).
 */
const wellTyped: InsertFact<typeof Certificate> = { student: 7n, kind: "DirectPass" }

type Cases = [
	// ——— Infer yields the exact handle union ———
	Expect<Equal<Infer<typeof Kind.id>, "DirectPass" | "JudgedPass" | "Failed">>,
	// ——— the union flows through Fact wherever the field is declared `kind: Kind.id` ———
	Expect<Equal<Fact<typeof Certificate>["kind"], "DirectPass" | "JudgedPass" | "Failed">>,
	Expect<
		Equal<
			InsertFact<typeof Certificate>,
			{
				student: bigint
				kind: "DirectPass" | "JudgedPass" | "Failed"
				id?: bigint | undefined
			}
		>
	>,
	// ——— hover legibility: the descriptor IS the evaluated named generic,
	// not conditional soup — the Equal probe fails on anything weaker ———
	Expect<Equal<typeof Kind.id, ClosedIdField<"Kind", "DirectPass" | "JudgedPass" | "Failed">>>,
	Expect<Equal<(typeof Kind.id)["closed"], ClosedRoster<"Kind", "DirectPass" | "JudgedPass" | "Failed">>>,
	// ——— the unbound generic default remains the wide fallback, so the
	// precise descriptor is admitted everywhere the wide shape was ———
	Expect<Equal<typeof Kind.id extends ClosedIdField ? true : false, true>>
]

/**
 * Two DIFFERENT vocabularies sharing a handle name overlap structurally
 * where the names coincide — `"DirectPass"` is assignable to BOTH unions.
 * This is the structural doctrine (types are encodings; no brands), pinned
 * as an honest fact: it is strictly better than the bigint era, where ANY
 * bigint assigned to EVERY closed column — the overlap is now exactly the
 * shared names and nothing else.
 */
type OverlapCases = [
	Expect<Equal<Extract<Infer<typeof Method.id>, Infer<typeof Kind.id>>, "DirectPass">>,
	// the non-shared names do NOT cross vocabularies
	Expect<Equal<"Manual" extends Infer<typeof Kind.id> ? true : false, false>>,
	Expect<Equal<"Failed" extends Infer<typeof Method.id> ? true : false, false>>
]

/** The shared handle name is assignable to both vocabularies' unions. */
function sharedHandleAssignsAcrossVocabularies(shared: "DirectPass"): [Infer<typeof Kind.id>, Infer<typeof Method.id>] {
	return [shared, shared]
}

/**
 * The compile-FAIL probes: a wrong string is a compile error, and a bigint
 * is no longer assignable to a closed-referencing column (each directive is
 * REAL — removing it breaks compilation).
 */
function insertRefusals(): unknown[] {
	// @ts-expect-error — H1: "DirectPas" is a typo off the roster — a wrong string is a compile error
	const typo: InsertFact<typeof Certificate> = { student: 7n, kind: "DirectPas" }
	// @ts-expect-error — H1: a bigint no longer types a closed-referencing column — the value type is the handle union
	const forgedId: InsertFact<typeof Certificate> = { student: 7n, kind: 0n }
	return [typo, forgedId]
}

test("two same-shaped vocabularies are distinct at BOTH tiers — the roster slot carries the name literal", function probeSameShapedVocabularies() {
	/** A vocabulary sharing Kind's exact handle set — only the name differs. */
	const Answer = closed("Answer", ["DirectPass", "JudgedPass", "Failed"])
	const Cert = relation("Cert", { k: Kind.id })
	assert.throws(function crossVocabularyPairing() {
		// @ts-expect-error — 063: a Kind reference cannot pair with Answer's id — the type-tier roster slot compares [name, handles], matching the runtime's roster-identity judgment
		contained(on(Cert, "k"), on(Answer, "id"))
	}, /is a Kind reference but Answer\.id is a Answer reference/)
})

test("the precise type's runtime twin is the same frozen declaration-order roster", function probeRuntimeTwin() {
	// the descriptor is a frozen plain object with own properties end to end
	assert.ok(Object.isFrozen(Kind.id))
	assert.ok(Object.isFrozen(Kind.id.closed))
	assert.ok(Object.isFrozen(Kind.id.closed.handles))
	assert.ok(Object.hasOwn(Kind.id, "kind"))
	assert.ok(Object.hasOwn(Kind.id, "closed"))
	assert.ok(Object.hasOwn(Kind.id.closed, "name"))
	assert.ok(Object.hasOwn(Kind.id.closed, "handles"))
	// the handles array carries the union's members, in declaration order
	assert.deepStrictEqual(Kind.id, {
		kind: "u64",
		closed: { name: "Kind", handles: ["DirectPass", "JudgedPass", "Failed"] }
	})
	// the well-typed insert row spells the handle as its string literal
	assert.equal(wellTyped.kind, "DirectPass")
	assert.deepStrictEqual(sharedHandleAssignsAcrossVocabularies("DirectPass"), ["DirectPass", "DirectPass"])
	assert.equal(insertRefusals().length, 2)
})

export type { Cases, OverlapCases }
export { insertRefusals, sharedHandleAssignsAcrossVocabularies }
