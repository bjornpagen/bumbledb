/**
 * Type-level pins for the PRD-05 kernel and PRD-06 statement algebra,
 * compiled by the package typecheck. Negative space is asserted through
 * assignability probes (`X extends Y ? true : false` pinned to `false`)
 * rather than expect-error directives, so every line here is checked
 * positively. The one runtime test at the bottom proves the module loads.
 */

import assert from "node:assert/strict"
import { test } from "node:test"

import type {
	Abandon,
	AnyRelation,
	Axioms,
	BoolField,
	Brand,
	Db,
	FaceArityMismatch,
	FaceFields,
	Fact,
	FreshKeys,
	Infer,
	InsertFact,
	Interval,
	IntervalValue,
	KeyFact,
	OneOf,
	Prepared,
	QueryParams,
	QueryRow,
	ReadScope,
	RelationFields,
	SameArity,
	SelectionInput,
	Statement,
	TermInput,
	Tx,
	Var,
	Violation,
	WitnessedWriteResult,
	WriteResult
} from "#index.ts"
import {
	bool,
	bytes,
	closed,
	count,
	duration,
	i64,
	interval,
	is,
	match,
	on,
	pack,
	query,
	relation,
	schema,
	str,
	sum,
	u64
} from "#index.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
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
const HolderId = u64.newtype("HolderId")
const AccountId = u64.newtype("AccountId")
const EverythingId = u64.newtype("EverythingId")
const Cents = i64.newtype("Cents")
const Tag = bytes(4).newtype("Tag")
const ActiveDuring = interval(i64).newtype("ActiveDuring")

const Holder = relation("Holder", { id: HolderId.fresh, name: str })
const Account = relation("Account", {
	id: AccountId.fresh,
	holder: HolderId,
	kind: Kind.id,
	active: ActiveDuring
})

type AccountId = Infer<typeof AccountId>
type HolderId = Infer<typeof HolderId>

const oneField = on(Holder, "id")
const twoFields = on(Account, "id", "kind")

/** Every field type in one relation — the PRD-07 round-trip target. */
const Everything = relation("Everything", {
	id: EverythingId.fresh,
	flag: bool,
	note: str,
	tag: Tag,
	raw: u64,
	score: Cents,
	kind: Kind.id,
	at: interval(u64)
})

/** A keyless-by-type relation: no fresh field, so `KeyFact` falls back to the runtime rule. */
const Pair = relation("Pair", { a: u64, b: u64 })

const Vault = schema("Vault", { Kind, Grade, Holder, Account, Everything }, [])
type VaultRels = (typeof Vault)["relations"]

declare const vaultDb: Db<VaultRels>
declare const snap: ReadScope<VaultRels>
declare const tx: Tx<VaultRels>

test("type-level pins compile and the weld holds at runtime", function probeCompiled() {
	assert.equal(Kind.fromId(Kind.Checking), "Checking")
	assert.equal(Grade.fromId(Grade.Failed), "Failed")
})

/**
 * The pinned cases, exported so the compiler counts every probe as used.
 * Hover-quality pins lead: `Fact` and `InsertFact` must BE plain object
 * types with named brands — the `Equal` probe fails on any conditional
 * tangle that is not identical to the spelled-out object.
 */
type Cases = [
	Expect<
		Equal<
			Fact<typeof Account>,
			{
				id: Brand<bigint, "AccountId">
				holder: Brand<bigint, "HolderId">
				kind: Brand<bigint, "Kind">
				active: Interval<"ActiveDuring">
			}
		>
	>,
	Expect<
		Equal<
			InsertFact<typeof Account>,
			{
				holder: Brand<bigint, "HolderId">
				kind: Brand<bigint, "Kind">
				active: Interval<"ActiveDuring">
				id?: Brand<bigint, "AccountId"> | undefined
			}
		>
	>,
	Expect<Equal<FreshKeys<typeof Account>, "id">>,
	Expect<Equal<HolderId extends AccountId ? true : false, false>>,
	Expect<Equal<AccountId extends HolderId ? true : false, false>>,
	Expect<Equal<bigint extends AccountId ? true : false, false>>,
	Expect<Equal<typeof Grade.DirectPass extends typeof Kind.Checking ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof AccountId ? true : false, true>>,
	Expect<Equal<"fresh" extends keyof typeof Cents ? true : false, false>>,
	Expect<Equal<"fresh" extends keyof typeof bool ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof u64 ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof i64 ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof bool ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof str ? true : false, false>>,
	Expect<Equal<"as" extends keyof ReturnType<typeof bytes> ? true : false, false>>,
	Expect<Equal<"as" extends keyof ReturnType<typeof interval> ? true : false, false>>,
	Expect<Equal<"as" extends keyof typeof AccountId ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof u64 ? true : false, true>>,
	Expect<Equal<"newtype" extends keyof typeof i64 ? true : false, true>>,
	Expect<Equal<"newtype" extends keyof ReturnType<typeof bytes> ? true : false, true>>,
	Expect<Equal<"newtype" extends keyof ReturnType<typeof interval> ? true : false, true>>,
	Expect<Equal<"newtype" extends keyof typeof bool ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof str ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof AccountId ? true : false, false>>,
	Expect<Equal<"newtype" extends keyof typeof Kind.id ? true : false, false>>,
	Expect<Equal<Infer<typeof AccountId>, Brand<bigint, "AccountId">>>,
	Expect<Equal<Infer<typeof ActiveDuring>, Interval<"ActiveDuring">>>,
	Expect<Equal<Fact<typeof Holder>["id"], Fact<typeof Account>["holder"]>>,
	Expect<Equal<typeof Kind.Checking, Brand<bigint, "Kind">>>,
	Expect<Equal<ReturnType<typeof Kind.fromId>, "Checking" | "Savings" | undefined>>,
	Expect<
		Equal<
			Axioms<["DirectPass", "Failed"], { mastered: BoolField }>,
			{
				readonly DirectPass: { readonly mastered: boolean }
				readonly Failed: { readonly mastered: boolean }
			}
		>
	>,
	Expect<Equal<typeof Kind extends AnyRelation ? true : false, false>>,
	Expect<Equal<typeof Grade extends AnyRelation ? true : false, false>>,
	Expect<Equal<typeof Account extends AnyRelation ? true : false, true>>,
	Expect<Equal<FaceFields<typeof Account>, "id" | "holder" | "kind" | "active">>,
	Expect<Equal<FaceFields<typeof Kind>, "id">>,
	Expect<Equal<FaceFields<typeof Grade>, "id" | "mastered">>,
	Expect<Equal<SameArity<typeof oneField, typeof oneField>, unknown>>,
	Expect<Equal<SameArity<typeof twoFields, typeof twoFields>, unknown>>,
	Expect<Equal<SameArity<typeof oneField, typeof twoFields> extends FaceArityMismatch<1, 2> ? true : false, true>>,
	Expect<Equal<SameArity<typeof twoFields, typeof oneField> extends FaceArityMismatch<2, 1> ? true : false, true>>,
	Expect<
		Equal<
			SelectionInput<RelationFields<typeof Account>>["kind"],
			Brand<bigint, "Kind"> | OneOf<Brand<bigint, "Kind">> | undefined
		>
	>,
	Expect<
		Equal<
			Fact<typeof Everything>,
			{
				id: Brand<bigint, "EverythingId">
				flag: boolean
				note: string
				tag: Brand<Uint8Array, "Tag">
				raw: bigint
				score: Brand<bigint, "Cents">
				kind: Brand<bigint, "Kind">
				at: IntervalValue
			}
		>
	>,
	Expect<Equal<ReturnType<typeof snap.scan<typeof Everything>>, Fact<typeof Everything>[]>>,
	Expect<Equal<ReturnType<typeof snap.get<typeof Everything>>, Fact<typeof Everything> | undefined>>,
	Expect<Equal<Parameters<typeof snap.contains<typeof Everything>>[1], Fact<typeof Everything>>>,
	Expect<Equal<Parameters<typeof tx.insert<typeof Account>>[1], InsertFact<typeof Account>>>,
	Expect<Equal<ReturnType<typeof tx.insert<typeof Account>>, { id: Brand<bigint, "AccountId"> }>>,
	Expect<Equal<KeyFact<typeof Account>, { id: Brand<bigint, "AccountId"> }>>,
	Expect<Equal<KeyFact<typeof Pair>, Partial<Fact<typeof Pair>>>>,
	Expect<Equal<Extract<ReturnType<typeof vaultDb.write>, { ok: true }>["generation"], bigint>>,
	Expect<Equal<Violation<VaultRels>["statement"], Statement | undefined>>,
	Expect<
		Equal<Violation<VaultRels>["facts"][number]["relation"], "Kind" | "Grade" | "Holder" | "Account" | "Everything">
	>,
	Expect<Equal<Exclude<WitnessedWriteResult<VaultRels, Abandon<string>>, WriteResult<VaultRels>>["abandoned"], string>>,
	Expect<Equal<"close" extends keyof typeof vaultDb ? true : false, false>>,
	Expect<Equal<"snapshot" extends keyof typeof vaultDb ? true : false, false>>,
	Expect<
		Equal<
			keyof typeof vaultDb,
			"schema" | "read" | "scan" | "get" | "contains" | "execute" | "write" | "writeWitnessed" | "prepare"
		>
	>
]

/**
 * PRD-08 query-surface pins: inert query values built at module load (no
 * store is touched), their inferred Row/Params objects asserted exactly,
 * and the nominal join discipline asserted through assignability probes —
 * a Holder-branded var is NOT placeable at a Kind-branded position.
 */
const holderAccounts = query(Vault, function build($) {
	const acct = $.var(Account.fields.id)
	const holder = $.var(Holder.fields.id)
	const root = $.param("root", Holder.fields.id)
	return {
		rules: [[match(Account, { id: acct, holder }), is(holder, root)]],
		select: { acct, holder }
	}
})

/** Measure, fold, and nullary-count entries in one select. */
const measured = query(Vault, function build($) {
	const id = $.var(Everything.fields.id)
	const at = $.var(Everything.fields.at)
	const score = $.var(Everything.fields.score)
	return {
		rules: [[match(Everything, { id, at, score })]],
		select: { id, d: duration(at), total: sum(score), n: count() }
	}
})

/** The relation-shaped coalescing fold: the packed column is interval-typed. */
const packed = query(Vault, function build($) {
	const kind = $.var(Everything.fields.kind)
	const at = $.var(Everything.fields.at)
	return { rules: [[match(Everything, { kind, at })]], select: { kind, cover: pack(at) } }
})

declare const holderVar: Var<Brand<bigint, "HolderId">>
declare const preparedAccounts: Prepared<VaultRels, QueryRow<typeof holderAccounts>, QueryParams<typeof holderAccounts>>

type QueryCases = [
	Expect<
		Equal<
			QueryRow<typeof holderAccounts>,
			{
				readonly acct: Brand<bigint, "AccountId">
				readonly holder: Brand<bigint, "HolderId">
			}
		>
	>,
	Expect<Equal<QueryParams<typeof holderAccounts>, { readonly root: Brand<bigint, "HolderId"> }>>,
	Expect<
		Equal<
			QueryRow<typeof measured>,
			{
				readonly id: Brand<bigint, "EverythingId">
				readonly d: bigint
				readonly total: Brand<bigint, "Cents">
				readonly n: bigint
			}
		>
	>,
	Expect<Equal<QueryParams<typeof measured>, Record<never, never>>>,
	Expect<Equal<QueryRow<typeof packed>, { readonly kind: Brand<bigint, "Kind">; readonly cover: IntervalValue }>>,
	Expect<Equal<typeof holderVar extends TermInput<Brand<bigint, "Kind">> ? true : false, false>>,
	Expect<Equal<typeof holderVar extends TermInput<Brand<bigint, "HolderId">> ? true : false, true>>,
	Expect<Equal<"execute" extends keyof typeof preparedAccounts ? true : false, false>>,
	Expect<Equal<"close" extends keyof typeof preparedAccounts ? true : false, false>>,
	Expect<
		Equal<
			Parameters<typeof snap.execute<QueryRow<typeof holderAccounts>, QueryParams<typeof holderAccounts>>>[1],
			{ readonly root: Brand<bigint, "HolderId"> }
		>
	>,
	Expect<
		Equal<
			ReturnType<typeof vaultDb.execute<QueryRow<typeof holderAccounts>, QueryParams<typeof holderAccounts>>>,
			QueryRow<typeof holderAccounts>[]
		>
	>,
	Expect<
		Equal<
			Parameters<typeof vaultDb.execute<QueryRow<typeof holderAccounts>, QueryParams<typeof holderAccounts>>>[0],
			typeof preparedAccounts
		>
	>
]

export type { Cases, QueryCases }
