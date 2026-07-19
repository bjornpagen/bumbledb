/**
 * Probes for the K6 closed ergonomics. `match()`: exhaustive dispatch over
 * the handle union on BOTH tiers, without literal types or brands — the
 * mapped arms type is the exhaustiveness proof (a missing arm is a REAL
 * missing-property compile failure, an extra arm a REAL excess-property
 * one), the payload tier's arm receives the typed axiom row
 * (identity-strength `Equal` pin), the bare tier's arm takes nothing, and
 * the runtime roster refuses an out-of-roster id with a throw, never a
 * misdispatch. The 3-arg `closed(name, columns, axioms)`: `Cols` infers
 * from the column block, the handle set from the axioms record's keys
 * (reverse mapped-type inference), a wrong-typed axiom value errors ON its
 * property (locality), and the deleted curried tier-2 spelling
 * `closed(name, cols)(axioms)` is uncompilable (REAL directive) with a
 * pointed runtime refusal as its twin.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"

import { closed } from "#closed.ts"
import { type BoolField, bool, type U64Field, u64 } from "#fields.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
type Expect<T extends true> = T extends true ? true : never

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

/** The payload tier's arms record and one arm's row, read off the value's own method type. */
type GradeArms = Parameters<typeof Grade.match<"x">>[1]
type GradeRow = Parameters<GradeArms["DirectPass"]>[0]
/** The bare tier's arms record: arms take NOTHING (no payload exists). */
type KindArms = Parameters<typeof Kind.match<"x">>[1]

/** The pinned cases, exported so the compiler counts every probe as used. */
type Cases = [
	// ——— 3-arg inference: Cols from arg 2 (the columns carrier reads back the declared descriptors) ———
	Expect<Equal<typeof Grade.columns.mastered, BoolField>>,
	Expect<Equal<typeof Grade.columns.score, U64Field>>,
	Expect<Equal<keyof typeof Grade.columns, "mastered" | "score">>,
	// ——— 3-arg inference: the handle set from arg 3's keys (the weld is exact) ———
	Expect<Equal<ReturnType<typeof Grade.fromId>, "DirectPass" | "Retried" | "Failed" | undefined>>,
	Expect<Equal<typeof Grade.DirectPass, bigint>>,
	// ——— the payload arm receives the typed axiom row ———
	Expect<Equal<GradeRow, { readonly mastered: boolean; readonly score: bigint }>>,
	Expect<
		Equal<
			GradeArms,
			{
				readonly DirectPass: (row: GradeRow) => "x"
				readonly Retried: (row: GradeRow) => "x"
				readonly Failed: (row: GradeRow) => "x"
			}
		>
	>,
	// ——— the bare arm takes no row ———
	Expect<Equal<KindArms, { readonly Checking: () => "x"; readonly Savings: () => "x" }>>,
	// ——— the ψ surface stays payload-only; match exists on BOTH tiers ———
	Expect<Equal<"where" extends keyof typeof Kind ? true : false, false>>,
	Expect<Equal<"where" extends keyof typeof Grade ? true : false, true>>,
	Expect<Equal<"match" extends keyof typeof Kind ? true : false, true>>,
	Expect<Equal<"match" extends keyof typeof Grade ? true : false, true>>
]

/**
 * Exhaustiveness fail-probes — REAL directives (removing any one breaks
 * compilation). Never called: the arms are compile subjects, not runtime
 * paths.
 */
function matchRefusesAMissingArm(): string {
	// @ts-expect-error — the Savings arm is missing: exhaustiveness is a missing-property compile error
	return Kind.match(Kind.Checking, {
		Checking: function armChecking() {
			return "c"
		}
	})
}

function matchRefusesAnExtraArm(): string {
	return Kind.match(Kind.Checking, {
		Checking: function armChecking() {
			return "c"
		},
		Savings: function armSavings() {
			return "s"
		},
		// @ts-expect-error — Frozen is outside the roster: an extra arm is an excess-property compile error
		Frozen: function armFrozen() {
			return "f"
		}
	})
}

function bareArmsTakeNoRow(): string {
	return Kind.match(Kind.Checking, {
		// @ts-expect-error — the bare tier's arm takes NO row: there is no payload to receive
		Checking: function armChecking(row: object) {
			return `c${String(row)}`
		},
		Savings: function armSavings() {
			return "s"
		}
	})
}

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

describe("match — exhaustive dispatch over the handle union", function describeMatch() {
	test("bare tier: dispatch is exact per handle", function probeBareDispatch() {
		const arms = {
			Checking: function armChecking() {
				return "Checking"
			},
			Savings: function armSavings() {
				return "Savings"
			}
		}
		assert.equal(Kind.match(Kind.Checking, arms), "Checking")
		assert.equal(Kind.match(Kind.Savings, arms), "Savings")
	})

	test("payload tier: dispatch is exact per handle and the arm receives the axiom row", function probePayloadDispatch() {
		function spell(id: bigint): string {
			return Grade.match(id, {
				DirectPass: function armDirectPass(row) {
					return `DirectPass:${row.mastered}:${row.score}`
				},
				Retried: function armRetried(row) {
					return `Retried:${row.mastered}:${row.score}`
				},
				Failed: function armFailed(row) {
					return `Failed:${row.mastered}:${row.score}`
				}
			})
		}
		assert.equal(spell(Grade.DirectPass), "DirectPass:true:100")
		assert.equal(spell(Grade.Retried), "Retried:true:60")
		assert.equal(spell(Grade.Failed), "Failed:false:0")
	})

	test("the arm's row IS the frozen readback row — same identity as .axioms", function probeRowIdentity() {
		const row = Grade.match(Grade.Retried, {
			DirectPass: function armDirectPass(armRow) {
				return armRow
			},
			Retried: function armRetried(armRow) {
				return armRow
			},
			Failed: function armFailed(armRow) {
				return armRow
			}
		})
		assert.equal(row, Grade.axioms.Retried)
		assert.ok(Object.isFrozen(row))
	})

	test("an id outside the roster THROWS — a refusal, never a misdispatch", function probeRosterRefusal() {
		assert.throws(function bareOutOfRoster() {
			Kind.match(7n, {
				Checking: function armChecking() {
					return "c"
				},
				Savings: function armSavings() {
					return "s"
				}
			})
		}, /closed relation Kind: match on id 7 misses the roster \(Checking, Savings\)/)
		assert.throws(function payloadOutOfRoster() {
			Grade.match(-1n, {
				DirectPass: function armDirectPass() {
					return "d"
				},
				Retried: function armRetried() {
					return "r"
				},
				Failed: function armFailed() {
					return "f"
				}
			})
		}, /closed relation Grade: match on id -1 misses the roster/)
	})

	test("match is minted as an OWN function on both tiers (the __proto__ law's discipline)", function probeOwnMint() {
		assert.ok(Object.hasOwn(Kind, "match"), "the bare tier's match is an own property")
		assert.ok(Object.hasOwn(Grade, "match"), "the payload tier's match is an own property")
		assert.equal(typeof Kind.match, "function")
		assert.equal(typeof Grade.match, "function")
		assert.equal(Object.hasOwn(Kind, "where"), false, "the bare tier still mints no where")
		assert.ok(Object.hasOwn(Grade, "where"), "the payload tier still mints where")
	})

	test("a handle named match or where is a construction-time error in both tiers", function probeReservedNames() {
		assert.throws(function bareMatchHandle() {
			closed("Bad", ["Fine", "match"])
		}, /collides with the closed value's own surface/)
		assert.throws(function payloadMatchHandle() {
			closed("Bad", { pages: bool }, { match: { pages: true } })
		}, /collides with the closed value's own surface/)
		assert.throws(function bareWhereHandle() {
			closed("Bad", ["Fine", "where"])
		}, /collides with the closed value's own surface/)
		assert.throws(function payloadWhereHandle() {
			closed("Bad", { pages: bool }, { where: { pages: true } })
		}, /collides with the closed value's own surface/)
	})

	test("a payload column named id is a construction-time error — the sealed shape mints the synthetic id itself", function probeIdColumn() {
		assert.throws(function idColumn() {
			// @ts-expect-error — "id" is unspellable in a column block (PayloadColumns); the runtime wall is the twin
			closed("Bad", { id: bool }, { A: { id: true }, B: { id: false } })
		}, /closed relation Bad: the payload column id collides with the sealed shape's synthetic id/)
	})
})

describe("the 3-arg closed — the payload tier in one call", function describeThreeArg() {
	test("the 3-arg spelling mints the whole payload surface", function probeThreeArg() {
		assert.equal(Grade.name, "Grade")
		assert.deepStrictEqual(Grade.data.handles, ["DirectPass", "Retried", "Failed"])
		assert.equal(Grade.DirectPass, 0n)
		assert.equal(Grade.Retried, 1n)
		assert.equal(Grade.Failed, 2n)
		assert.equal(Grade.fromId(2n), "Failed")
		assert.equal(Grade.axioms.Retried.score, 60n)
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
})

export type { Cases }
export { bareArmsTakeNoRow, matchRefusesAMissingArm, matchRefusesAnExtraArm, wrongValueErrsOnItsProperty }
