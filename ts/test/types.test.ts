/**
 * Type-level pins for the STRUCTURAL field & domain kernel (PRD-S1),
 * compiled by the package typecheck. Values are bare and structural
 * (`bigint`/`string`/`boolean`/`Uint8Array`/`{ start, end }` — no brands,
 * no phantoms); domains are string labels in the field DESCRIPTOR type,
 * attached by `.as("Domain")`. Positive space is asserted through
 * identity-strength `Equal` probes; the deleted brand-era surface and the
 * macro's refusals are asserted through REAL `@ts-expect-error` fail-probes
 * (removing any directive breaks compilation). One runtime test at the
 * bottom proves the module loads and the closed weld holds.
 */

import assert from "node:assert/strict"
import { test } from "node:test"

import { type Axioms, closed } from "#closed.ts"
import { type BoolField, bool, bytes, type Infer, type IntervalValue, i64, interval, str, u64 } from "#fields.ts"
import { type AnyRelation, type Fact, type FreshKeys, type InsertFact, relation } from "#relation.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
type Expect<T extends true> = T extends true ? true : never

const Kind = closed("Kind", ["Checking", "Savings"])
const Grade = closed("Grade", { mastered: bool })({
	DirectPass: { mastered: true },
	Failed: { mastered: false }
})
const HolderId = u64.as("HolderId")
const AccountId = u64.as("AccountId")
const EverythingId = u64.as("EverythingId")
const Cents = i64.as("Cents")
const Tag = bytes(32).as("Tag")
const ActiveDuring = interval(i64).as("ActiveDuring")
/** The fixed-width interval family: the width is a descriptor-type label. */
const Stay = interval(u64, 7n)
/** Bare (undomained) constructor values used as fields directly. */
const RawBytes = bytes(4)
const RawInterval = interval(u64)

const Holder = relation("Holder", { id: HolderId.fresh, name: str })
const Account = relation("Account", {
	id: AccountId.fresh,
	holder: HolderId,
	kind: Kind.id,
	active: ActiveDuring
})

/** Every field kind in one relation — the Infer-totality target. */
const Everything = relation("Everything", {
	id: EverythingId.fresh,
	flag: bool,
	note: str,
	tag: Tag,
	raw: u64,
	score: Cents,
	kind: Kind.id,
	at: RawInterval,
	stay: Stay
})

/** A keyless-by-type relation: no fresh field, so `FreshKeys` is `never`. */
const Pair = relation("Pair", { a: u64, b: u64 })

test("the structural kernel loads and the closed weld holds at runtime", function probeCompiled() {
	assert.equal(Kind.fromId(Kind.Checking), "Checking")
	assert.equal(Grade.fromId(Grade.Failed), "Failed")
	assert.equal(HolderId.domain, "HolderId")
	assert.equal(AccountId.fresh.fresh, true)
})

/**
 * The pinned cases, exported so the compiler counts every probe as used.
 * Hover-quality pins lead: `Fact` and `InsertFact` must BE plain object
 * types with bare structural values — the `Equal` probe fails on any
 * conditional tangle that is not identical to the spelled-out object.
 */
type Cases = [
	// ——— values are bare and structural (no brand appears anywhere) ———
	Expect<
		Equal<
			Fact<typeof Account>,
			{
				id: bigint
				holder: bigint
				kind: bigint
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
				kind: bigint
				at: IntervalValue
				stay: IntervalValue
			}
		>
	>,
	Expect<
		Equal<
			InsertFact<typeof Account>,
			{
				holder: bigint
				kind: bigint
				active: IntervalValue
				id?: bigint | undefined
			}
		>
	>,
	Expect<Equal<FreshKeys<typeof Account>, "id">>,
	Expect<Equal<FreshKeys<typeof Pair>, never>>,
	// two fields of DIFFERENT domains are mutually assignable at the value
	// level — that is the point of structural: the domain wall lives in the
	// builders (S2/S3) and the engine, never on the value.
	Expect<Equal<Fact<typeof Holder>["id"], Fact<typeof Account>["holder"]>>,
	Expect<Equal<Infer<typeof HolderId>, Infer<typeof AccountId>>>,
	// ——— Infer is total and precise over every field kind ———
	Expect<Equal<Infer<typeof bool>, boolean>>,
	Expect<Equal<Infer<typeof str>, string>>,
	Expect<Equal<Infer<typeof u64>, bigint>>,
	Expect<Equal<Infer<typeof i64>, bigint>>,
	Expect<Equal<Infer<typeof HolderId>, bigint>>,
	Expect<Equal<Infer<typeof AccountId.fresh>, bigint>>,
	Expect<Equal<Infer<typeof Cents>, bigint>>,
	Expect<Equal<Infer<typeof Tag>, Uint8Array>>,
	Expect<Equal<Infer<typeof RawBytes>, Uint8Array>>,
	Expect<Equal<Infer<typeof ActiveDuring>, IntervalValue>>,
	Expect<Equal<Infer<typeof Stay>, IntervalValue>>,
	Expect<Equal<Infer<typeof Kind.id>, bigint>>,
	// ——— the domain is a string-literal label in the DESCRIPTOR type ———
	Expect<Equal<(typeof HolderId)["domain"], "HolderId">>,
	Expect<Equal<(typeof AccountId.fresh)["domain"], "AccountId">>,
	Expect<Equal<(typeof u64)["domain"], undefined>>,
	Expect<Equal<(typeof Tag)["domain"], "Tag">>,
	Expect<Equal<(typeof Kind.id)["domain"], "KindId">>,
	Expect<Equal<(typeof Grade.id)["domain"], "GradeId">>,
	// ——— the fresh mark is a structural `fresh: true` label ———
	Expect<Equal<(typeof AccountId.fresh)["fresh"], true>>,
	Expect<Equal<typeof AccountId.fresh extends { fresh: true } ? true : false, true>>,
	Expect<Equal<typeof AccountId extends { fresh: true } ? true : false, false>>,
	// ——— width labels live in the descriptor type, not the value ———
	Expect<Equal<(typeof Tag)["width"], 32>>,
	Expect<Equal<(typeof RawBytes)["width"], 4>>,
	Expect<Equal<(typeof Stay)["width"], 7n>>,
	Expect<Equal<(typeof Stay)["element"], "u64">>,
	Expect<Equal<(typeof ActiveDuring)["width"], undefined>>,
	Expect<Equal<(typeof ActiveDuring)["element"], "i64">>,
	// ——— `.as` exists on the four Rust-`as`-legal constructors only, once ———
	Expect<Equal<"as" extends keyof typeof u64 ? true : false, true>>,
	Expect<Equal<"as" extends keyof typeof i64 ? true : false, true>>,
	Expect<Equal<"as" extends keyof typeof RawBytes ? true : false, true>>,
	Expect<Equal<"as" extends keyof typeof RawInterval ? true : false, true>>,
	Expect<Equal<"as" extends keyof typeof bool ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof str ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof HolderId ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof AccountId.fresh ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof Kind.id ? true : false, false>>,
	// ——— `.fresh` exists only on u64 (bare or after `.as`) ———
	Expect<Equal<"fresh" extends keyof typeof u64 ? true : false, true>>,
	Expect<Equal<"fresh" extends keyof typeof HolderId ? true : false, true>>,
	Expect<Equal<"fresh" extends keyof typeof i64 ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof Cents ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof bool ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof str ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof RawBytes ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof RawInterval ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof Kind.id ? true : false, false>>,
	// ——— the brand-era `.newtype` spelling is gone from every constructor ———
	Expect<Equal<"newtype" extends keyof typeof u64 ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof i64 ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof RawBytes ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof RawInterval ? true : false, false>>,
	// ——— closed(): handle constants are bare bigints; the weld is exact ———
	Expect<Equal<typeof Kind.Checking, bigint>>,
	Expect<Equal<typeof Grade.DirectPass, bigint>>,
	Expect<Equal<ReturnType<typeof Kind.fromId>, "Checking" | "Savings" | undefined>>,
	Expect<Equal<Parameters<typeof Kind.fromId>, [id: bigint]>>,
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
	// ——— closed relations are unwritable: no relation shape, no fact ———
	Expect<Equal<typeof Kind extends AnyRelation ? true : false, false>>,
	Expect<Equal<typeof Grade extends AnyRelation ? true : false, false>>,
	Expect<Equal<typeof Account extends AnyRelation ? true : false, true>>,
	Expect<Equal<"where" extends keyof typeof Kind ? true : false, false>>,
	Expect<Equal<"fields" extends keyof typeof Grade ? true : false, false>>
]

/**
 * The structural dividend, as a compile-must-PASS probe: a HolderId-domain
 * value IS an AccountId-domain value at the value level (both bare
 * `bigint`) — no cast, no mint, no brand assertion.
 */
function domainsShareTheValueLevel(holder: Infer<typeof HolderId>): Infer<typeof AccountId> {
	return holder
}

/** `.as` is a type-level absence on bool/str — Rust's `as` grammar refuses them. */
function asStaysOffBoolAndStr(): unknown[] {
	return [
		// @ts-expect-error — bool carries no reference domain, so `.as` does not exist on it
		bool.as("Flag"),
		// @ts-expect-error — str carries no reference domain, so `.as` does not exist on it
		str.as("Note")
	]
}

/** `.fresh` marks an engine-minted u64 key; every other kind refuses the mark. */
function freshStaysU64Only(): unknown[] {
	return [
		// @ts-expect-error — fresh is legal on u64 only, never i64
		i64.fresh,
		// @ts-expect-error — fresh is legal on u64 only, never a domained i64
		Cents.fresh,
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

/** The brand-era `.newtype` spelling has no successor alias: it is unwritable. */
function newtypeIsGone(): unknown[] {
	return [
		// @ts-expect-error — `.newtype` died with the brand era; the spelling is `.as`
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
 * surface's own operator typing (S3) — the engine refuses order on
 * bytes/intervals as the final authority.
 */
type OrderCases = [
	Expect<Equal<keyof Infer<typeof ActiveDuring>, "start" | "end">>,
	Expect<Equal<keyof Infer<typeof Stay>, "start" | "end">>
]

/** No comparator method exists on a bytes or interval value — a type-level absence. */
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
export { asStaysOffBoolAndStr, domainsShareTheValueLevel, freshStaysU64Only, newtypeIsGone, orderStaysRefused }
