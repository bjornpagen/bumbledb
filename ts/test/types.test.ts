import assert from "node:assert/strict"
import { test } from "node:test"

import { type Axioms, closed, closedId } from "../src/closed.ts"
import {
	type BoolField,
	bool,
	bytes,
	type FloatIntervalValue,
	f64,
	type Infer,
	type IntervalValue,
	i64,
	interval,
	str,
	u64,
	uuid
} from "../src/fields.ts"
import { type AnyRelation, type Fact, relation } from "../src/relation.ts"
import type { Uuid } from "../src/uuid.ts"

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
const Confidence = interval(f64)

const Stay = interval(u64, 7n)

const RawBytes = bytes(4)
const RawInterval = interval(u64)

const Holder = relation("Holder", { id: uuid, name: str })
const Account = relation("Account", {
	id: uuid,
	holder: uuid,
	kind: closedId(Kind),
	active: ActiveDuring
})

const Everything = relation("Everything", {
	id: uuid,
	flag: bool,
	note: str,
	tag: Tag,
	raw: u64,
	score: i64,
	weight: f64,
	kind: closedId(Kind),
	at: RawInterval,
	stay: Stay,
	sureness: Confidence
})

test("the minimal kernel loads and the roster is pure data at runtime", function probeCompiled() {
	assert.deepEqual(Kind.handles, ["Checking", "Savings"])
	assert.deepEqual(Grade.handles, ["DirectPass", "Failed"])
	assert.equal(u64.kind, "u64")
	assert.equal(uuid.kind, "uuid")
	assert.equal(f64.kind, "f64")
	assert.equal(Confidence.element, "f64")
})

type Cases = [
	Expect<
		Equal<
			Fact<typeof Account>,
			{
				id: Uuid
				holder: Uuid
				kind: "Checking" | "Savings"
				active: IntervalValue
			}
		>
	>,
	Expect<
		Equal<
			Fact<typeof Everything>,
			{
				id: Uuid
				flag: boolean
				note: string
				tag: Uint8Array
				raw: bigint
				score: bigint
				weight: number
				kind: "Checking" | "Savings"
				at: IntervalValue
				stay: IntervalValue
				sureness: FloatIntervalValue
			}
		>
	>,
	// The same uuid field infers the SAME host value everywhere: the
	// canonical Uuid, never a per-relation generated entity class.
	Expect<Equal<Fact<typeof Holder>["id"], Fact<typeof Account>["holder"]>>,
	Expect<Equal<typeof i64, { readonly kind: "i64" }>>,
	Expect<Equal<typeof bool, { readonly kind: "bool" }>>,
	Expect<Equal<typeof str, { readonly kind: "str" }>>,
	Expect<Equal<typeof f64, { readonly kind: "f64" }>>,
	Expect<Equal<typeof uuid, { readonly kind: "uuid" }>>,
	Expect<Equal<typeof Tag, { readonly kind: "bytes"; readonly width: 32 }>>,
	Expect<Equal<typeof Stay, { readonly kind: "interval"; readonly element: "u64"; readonly width: 7n }>>,
	Expect<Equal<typeof Confidence, { readonly kind: "interval"; readonly element: "f64"; readonly width: undefined }>>,
	Expect<Equal<Infer<typeof bool>, boolean>>,
	Expect<Equal<Infer<typeof str>, string>>,
	Expect<Equal<Infer<typeof u64>, bigint>>,
	Expect<Equal<Infer<typeof i64>, bigint>>,
	// No implicit bigint/number coercion: integers are bigint, f64 is number.
	Expect<Equal<Infer<typeof f64>, number>>,
	Expect<Equal<Infer<typeof uuid>, Uuid>>,
	Expect<Equal<Infer<typeof Tag>, Uint8Array>>,
	Expect<Equal<Infer<typeof RawBytes>, Uint8Array>>,
	Expect<Equal<Infer<typeof ActiveDuring>, IntervalValue>>,
	Expect<Equal<Infer<typeof Stay>, IntervalValue>>,
	// The dense float interval infers the number-endpoint value, never the
	// bigint one — the two interval shapes cannot cross-assign.
	Expect<Equal<Infer<typeof Confidence>, FloatIntervalValue>>,
	Expect<Equal<Infer<typeof Confidence> extends IntervalValue ? true : false, false>>,
	Expect<Equal<Infer<typeof ActiveDuring> extends FloatIntervalValue ? true : false, false>>,
	Expect<Equal<Infer<ReturnType<typeof closedId<typeof Kind>>>, "Checking" | "Savings">>,
	Expect<Equal<Infer<ReturnType<typeof closedId<typeof Grade>>>, "DirectPass" | "Failed">>,
	Expect<Equal<(typeof Tag)["width"], 32>>,
	Expect<Equal<(typeof RawBytes)["width"], 4>>,
	Expect<Equal<(typeof Stay)["width"], 7n>>,
	Expect<Equal<(typeof Stay)["element"], "u64">>,
	Expect<Equal<(typeof ActiveDuring)["width"], undefined>>,
	Expect<Equal<(typeof ActiveDuring)["element"], "i64">>,
	Expect<Equal<(typeof Kind.handles)[number], "Checking" | "Savings">>,
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
	Expect<Equal<"fields" extends keyof typeof Grade ? true : false, false>>,
	// Uuid is structural: host UUID types and UUID-shaped literals need no cast.
	// Arbitrary strings still require parsing; exact spelling is checked at runtime.
	Expect<Equal<"00112233-4455-6677-8899-aabbccddeeff" extends Uuid ? true : false, true>>,
	Expect<Equal<ReturnType<typeof crypto.randomUUID> extends Uuid ? true : false, true>>,
	Expect<Equal<string extends Uuid ? true : false, false>>,
	Expect<Equal<Uuid extends string ? true : false, true>>
]

/**
 * Order stays refused where the engine refuses it — REPRESENTATIONALLY: no
 * comparator exists anywhere on a `bytes`/interval value (the exact-keyof
 * pin in {@link OrderCases} holds the interval value to `start`/`end` and
 * nothing else, and the method probes below are type-level absences).
 */
type OrderCases = [
	Expect<Equal<keyof Infer<typeof ActiveDuring>, "start" | "end">>,
	Expect<Equal<keyof Infer<typeof Stay>, "start" | "end">>,
	Expect<Equal<keyof Infer<typeof Confidence>, "start" | "end">>
]

declare const someId: Uuid

function insertTakesCompleteFacts(): unknown {
	// @ts-expect-error — Fact requires every field, including the application-owned id
	const omitted: Fact<typeof Account> = {
		holder: someId,
		kind: "Checking",
		active: { start: 0n, end: 1n }
	}
	return omitted
}

function idsAreCheckedValuesNotStrings(): unknown {
	// @ts-expect-error — this literal does not have UUID structure
	const forged: Fact<typeof Holder> = { id: "not-an-id", name: "x" }
	return forged
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

export type { Cases, OrderCases }
export { idsAreCheckedValuesNotStrings, insertTakesCompleteFacts, orderStaysRefused }
