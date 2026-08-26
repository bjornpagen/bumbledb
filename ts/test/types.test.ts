import assert from "node:assert/strict"
import { test } from "node:test"

import { type Axioms, closed } from "#closed.ts"
import { type BoolField, bool, bytes, type Infer, type IntervalValue, i64, interval, str, u64 } from "#fields.ts"
import { type AnyRelation, type Fact, type FreshKeys, relation } from "#relation.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

type Expect<T extends true> = T extends true ? true : never

const Kind = closed("Kind", ["Checking", "Savings"])
const Grade = closed(
	"Grade",
	["DirectPass", "Failed"],
	{ mastered: bool },
	{
		DirectPass: { mastered: true },
		Failed: { mastered: false }
	}
)
const Tag = bytes(32)
const ActiveDuring = interval(i64)

const Stay = interval(u64, 7n)

const RawBytes = bytes(4)
const RawInterval = interval(u64)

const Holder = relation("Holder", { id: u64.fresh, name: str })
const Account = relation("Account", {
	id: u64.fresh,
	holder: u64,
	kind: Kind.id,
	active: ActiveDuring
})

const Everything = relation("Everything", {
	id: u64.fresh,
	flag: bool,
	note: str,
	tag: Tag,
	raw: u64,
	score: i64,
	kind: Kind.id,
	at: RawInterval,
	stay: Stay
})

const Pair = relation("Pair", { a: u64, b: u64 })

test("the minimal kernel loads and the roster is pure data at runtime", function probeCompiled() {
	assert.deepEqual(Kind.data.handles, ["Checking", "Savings"])
	assert.deepEqual(Grade.data.handles, ["DirectPass", "Failed"])
	assert.equal(u64.fresh.fresh, true)
	assert.equal(u64.kind, "u64")
})

type Cases = [
	Expect<
		Equal<
			Fact<typeof Account>,
			{
				id: bigint
				holder: bigint
				kind: "Checking" | "Savings"
				active: IntervalValue
			}
		>
	>,
	Expect<
		Equal<
			Fact<typeof Everything>,
			{
				id: bigint
				flag: boolean
				note: string
				tag: Uint8Array
				raw: bigint
				score: bigint
				kind: "Checking" | "Savings"
				at: IntervalValue
				stay: IntervalValue
			}
		>
	>,
	Expect<Equal<FreshKeys<typeof Account>, "id">>,
	Expect<Equal<FreshKeys<typeof Pair>, never>>,
	Expect<Equal<Fact<typeof Holder>["id"], Fact<typeof Account>["holder"]>>,
	Expect<Equal<typeof i64, { readonly kind: "i64" }>>,
	Expect<Equal<typeof bool, { readonly kind: "bool" }>>,
	Expect<Equal<typeof str, { readonly kind: "str" }>>,
	Expect<Equal<(typeof u64)["fresh"], { readonly kind: "u64"; readonly fresh: true }>>,
	Expect<Equal<typeof Tag, { readonly kind: "bytes"; readonly width: 32 }>>,
	Expect<Equal<typeof Stay, { readonly kind: "interval"; readonly element: "u64"; readonly width: 7n }>>,
	Expect<Equal<Infer<typeof bool>, boolean>>,
	Expect<Equal<Infer<typeof str>, string>>,
	Expect<Equal<Infer<typeof u64>, bigint>>,
	Expect<Equal<Infer<typeof i64>, bigint>>,
	Expect<Equal<Infer<typeof u64.fresh>, bigint>>,
	Expect<Equal<Infer<typeof Tag>, Uint8Array>>,
	Expect<Equal<Infer<typeof RawBytes>, Uint8Array>>,
	Expect<Equal<Infer<typeof ActiveDuring>, IntervalValue>>,
	Expect<Equal<Infer<typeof Stay>, IntervalValue>>,
	Expect<Equal<Infer<typeof Kind.id>, "Checking" | "Savings">>,
	Expect<Equal<Infer<typeof Grade.id>, "DirectPass" | "Failed">>,
	Expect<Equal<(typeof u64.fresh)["fresh"], true>>,
	Expect<Equal<typeof u64.fresh extends { fresh: true } ? true : false, true>>,
	Expect<Equal<typeof u64 extends { fresh: true } ? true : false, false>>,
	Expect<Equal<(typeof Tag)["width"], 32>>,
	Expect<Equal<(typeof RawBytes)["width"], 4>>,
	Expect<Equal<(typeof Stay)["width"], 7n>>,
	Expect<Equal<(typeof Stay)["element"], "u64">>,
	Expect<Equal<(typeof ActiveDuring)["width"], undefined>>,
	Expect<Equal<(typeof ActiveDuring)["element"], "i64">>,
	Expect<Equal<"domain" extends keyof typeof u64 ? true : false, false>>,
	Expect<Equal<"domain" extends keyof typeof u64.fresh ? true : false, false>>,
	Expect<Equal<"domain" extends keyof typeof i64 ? true : false, false>>,
	Expect<Equal<"domain" extends keyof typeof bool ? true : false, false>>,
	Expect<Equal<"domain" extends keyof typeof str ? true : false, false>>,
	Expect<Equal<"domain" extends keyof typeof RawBytes ? true : false, false>>,
	Expect<Equal<"domain" extends keyof typeof RawInterval ? true : false, false>>,
	Expect<Equal<"domain" extends keyof typeof Kind.id ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof u64 ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof i64 ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof RawBytes ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof RawInterval ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof bool ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof str ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof Kind.id ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof u64 ? true : false, true>>,
	Expect<Equal<"fresh" extends keyof typeof i64 ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof bool ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof str ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof RawBytes ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof RawInterval ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof Kind.id ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof u64 ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof i64 ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof RawBytes ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof RawInterval ? true : false, false>>,
	Expect<Equal<Infer<typeof Kind.id>, "Checking" | "Savings">>,
	Expect<Equal<(typeof Kind.data.handles)[number], string>>,
	Expect<
		Equal<
			Axioms<"DirectPass" | "Failed", { mastered: BoolField }>,
			{
				readonly DirectPass: { readonly mastered: boolean }
				readonly Failed: { readonly mastered: boolean }
			}
		>
	>,
	Expect<Equal<typeof Grade.axioms.DirectPass.mastered, boolean>>,
	Expect<Equal<typeof Kind extends AnyRelation ? true : false, false>>,
	Expect<Equal<typeof Grade extends AnyRelation ? true : false, false>>,
	Expect<Equal<typeof Account extends AnyRelation ? true : false, true>>,
	Expect<Equal<"where" extends keyof typeof Kind ? true : false, false>>,
	Expect<Equal<"fields" extends keyof typeof Grade ? true : false, false>>
]

function asIsDeleted(): unknown[] {
	return [
		// @ts-expect-error — `.as` died with declared domains: schema() computes u64 domains from the statements
		u64.as("HolderId"),
		// @ts-expect-error — `.as` died with declared domains: schema() computes i64 domains from the statements
		i64.as("Cents"),
		// @ts-expect-error — `.as` died with declared domains: schema() computes bytes domains from the statements
		bytes(4).as("Tag"),
		// @ts-expect-error — `.as` died with declared domains: schema() computes interval domains from the statements
		interval(i64).as("ActiveDuring"),
		// @ts-expect-error — bool never carried `.as` (macro parity), and the property does not exist anywhere now
		bool.as("Flag"),
		// @ts-expect-error — str never carried `.as` (macro parity), and the property does not exist anywhere now
		str.as("Note"),
		// @ts-expect-error — a closed reference descriptor never carried `.as`
		Kind.id.as("KindId")
	]
}

function freshStaysU64Only(): unknown[] {
	return [
		// @ts-expect-error — fresh is legal on u64 only, never i64
		i64.fresh,
		// @ts-expect-error — fresh is legal on u64 only, never bool
		bool.fresh,
		// @ts-expect-error — fresh is legal on u64 only, never str
		str.fresh,
		// @ts-expect-error — fresh is legal on u64 only, never bytes
		RawBytes.fresh,
		// @ts-expect-error — fresh is legal on u64 only, never an interval
		RawInterval.fresh,
		// @ts-expect-error — a closed reference field is never minted
		Kind.id.fresh
	]
}

function newtypeIsGone(): unknown[] {
	return [
		// @ts-expect-error — `.newtype` died with the brand era, and no declared-domain spelling replaced it
		u64.newtype("AccountId"),
		// @ts-expect-error — `.newtype` died with the brand era
		i64.newtype("Cents"),
		// @ts-expect-error — `.newtype` died with the brand era
		bytes(4).newtype("Tag"),
		// @ts-expect-error — `.newtype` died with the brand era
		interval(i64).newtype("ActiveDuring")
	]
}

// @ts-expect-error — the brand module is deleted with the nominal era: no brand type exists to reference
type BrandIsGone = typeof import("#brand.ts")

/**
 * Order stays refused where the engine refuses it — REPRESENTATIONALLY: no
 * comparator exists anywhere on a `bytes`/interval value (the exact-keyof
 * pin in {@link OrderCases} holds the interval value to `start`/`end` and
 * nothing else, and the method probes below are type-level absences).
 * JavaScript's bare `<` on two objects is not refusable by TypeScript (the
 * language types relational operators on any mutually-assignable pair), so
 * the wall is the absence of any order VOCABULARY here plus the query
 * surface's own operator typing — the engine refuses order on
 * bytes/intervals as the final authority.
 */
type OrderCases = [
	Expect<Equal<keyof Infer<typeof ActiveDuring>, "start" | "end">>,
	Expect<Equal<keyof Infer<typeof Stay>, "start" | "end">>
]

function insertTakesCompleteFacts(): unknown {
	// @ts-expect-error — Fact requires every field, including the fresh cell; mint with reserve first
	const omitted: Fact<typeof Account> = {
		holder: 1n,
		kind: "Checking",
		active: { start: 0n, end: 1n }
	}
	return omitted
}

function orderStaysRefused(
	tag: Infer<typeof Tag>,
	otherTag: Infer<typeof Tag>,
	active: Infer<typeof ActiveDuring>,
	otherActive: Infer<typeof ActiveDuring>
): unknown[] {
	return [
		// @ts-expect-error — bytes values derive no order: no compare() exists on the value
		tag.compare(otherTag),
		// @ts-expect-error — interval values derive no order: no compare() exists on the value
		active.compare(otherActive)
	]
}

export type { BrandIsGone, Cases, OrderCases }
export { asIsDeleted, freshStaysU64Only, insertTakesCompleteFacts, newtypeIsGone, orderStaysRefused }
