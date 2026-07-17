/**
 * PRD-06 runtime pins: the full Ledger example (key, containment, selected
 * `==`, window) lowers to its `SchemaSpec` shape; the canonical-utterance
 * ban table is enumerated one row at a time (each row a construction error
 * naming the canonical form, or unwritable at the type level); `schema()`
 * enforces its expansion-boundary checks; and `renderStatement` emits the
 * canonical `70-api.md` spellings exactly.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"

import {
	atLeast,
	atMost,
	between,
	closed,
	contained,
	exactly,
	i64,
	interval,
	key,
	lower,
	mirrors,
	none,
	on,
	oneOf,
	relation,
	renderStatement,
	schema,
	span,
	str,
	u64,
	window
} from "#index.ts"

const HolderId = u64.newtype("HolderId")
const AccountId = u64.newtype("AccountId")
const ActiveDuring = interval(i64).newtype("ActiveDuring")

function buildLedger() {
	const Kind = closed("Kind", ["Checking", "Savings"])
	const Holder = relation("Holder", { id: HolderId.fresh, name: str })
	const Account = relation("Account", {
		id: AccountId.fresh,
		holder: HolderId,
		kind: Kind.id,
		active: ActiveDuring
	})
	const SavingsTerms = relation("SavingsTerms", { account: AccountId })
	const statements = [
		key(SavingsTerms, ["account"]),
		contained(on(Account, "holder"), on(Holder, "id")),
		contained(on(Account, "kind"), on(Kind, "id")),
		mirrors(on(Account.where({ kind: Kind.Savings }), "id"), on(SavingsTerms, "account")),
		window(on(Holder, "id"), atMost(3n), on(Account, "holder"))
	]
	const Ledger = schema("Ledger", { Kind, Holder, Account, SavingsTerms }, statements)
	return { Kind, Holder, Account, SavingsTerms, statements, Ledger }
}

describe("the Ledger example", function describeLedger() {
	test("lowers to the SchemaSpec shape, declaration order throughout", function probeLedgerLowering() {
		const { Ledger } = buildLedger()
		assert.deepStrictEqual(lower(Ledger), {
			relations: [
				{
					name: "Kind",
					newtype: "Kind",
					fields: [],
					extension: [
						{ handle: "Checking", values: [] },
						{ handle: "Savings", values: [] }
					]
				},
				{
					name: "Holder",
					newtype: undefined,
					fields: [
						{ name: "id", valueType: { kind: "u64" }, newtype: "HolderId", fresh: true },
						{ name: "name", valueType: { kind: "string" }, newtype: undefined, fresh: false }
					],
					extension: undefined
				},
				{
					name: "Account",
					newtype: undefined,
					fields: [
						{ name: "id", valueType: { kind: "u64" }, newtype: "AccountId", fresh: true },
						{ name: "holder", valueType: { kind: "u64" }, newtype: "HolderId", fresh: false },
						{ name: "kind", valueType: { kind: "u64" }, newtype: "Kind", fresh: false },
						{
							name: "active",
							valueType: { kind: "interval", element: "i64", width: undefined },
							newtype: "ActiveDuring",
							fresh: false
						}
					],
					extension: undefined
				},
				{
					name: "SavingsTerms",
					newtype: undefined,
					fields: [{ name: "account", valueType: { kind: "u64" }, newtype: "AccountId", fresh: false }],
					extension: undefined
				}
			],
			statements: [
				{ kind: "fd", relation: "SavingsTerms", projection: ["account"] },
				{
					kind: "containment",
					source: { relation: "Account", projection: ["holder"], selection: [] },
					target: { relation: "Holder", projection: ["id"], selection: [] },
					bidirectional: false
				},
				{
					kind: "containment",
					source: { relation: "Account", projection: ["kind"], selection: [] },
					target: { relation: "Kind", projection: ["id"], selection: [] },
					bidirectional: false
				},
				{
					kind: "containment",
					source: {
						relation: "Account",
						projection: ["id"],
						selection: [["kind", { kind: "one", literal: { kind: "handle", handle: "Savings" } }]]
					},
					target: { relation: "SavingsTerms", projection: ["account"], selection: [] },
					bidirectional: true
				},
				{
					kind: "cardinality",
					target: { relation: "Holder", projection: ["id"], selection: [] },
					window: { kind: "range", lo: 0n, hi: 3n },
					source: { relation: "Account", projection: ["holder"], selection: [] }
				}
			]
		})
	})

	test("lowering is deterministic across independent constructions", function probeDeterminism() {
		const first = JSON.stringify(lower(buildLedger().Ledger), function replace(_key, entry: unknown) {
			return typeof entry === "bigint" ? `${entry}n` : entry
		})
		const second = JSON.stringify(lower(buildLedger().Ledger), function replace(_key, entry: unknown) {
			return typeof entry === "bigint" ? `${entry}n` : entry
		})
		assert.equal(first, second)
	})
})

describe("renderStatement", function describeRender() {
	test("each statement form renders its canonical 70-api spelling", function probeCanonicalSpellings() {
		const { statements } = buildLedger()
		assert.deepStrictEqual(statements.map(renderStatement), [
			"SavingsTerms(account) -> SavingsTerms",
			"Account(holder) <= Holder(id)",
			"Account(kind) <= Kind(id)",
			"Account(id | kind == Savings) == SavingsTerms(account)",
			"Holder(id) <={0..3} Account(holder)"
		])
	})

	test("every legal window spelling renders canonically", function probeWindowSpellings() {
		const { Holder, Account } = buildLedger()
		const target = on(Holder, "id")
		const source = on(Account, "holder")
		assert.equal(renderStatement(window(target, exactly(1n), source)), "Holder(id) <={1} Account(holder)")
		assert.equal(renderStatement(window(target, none, source)), "Holder(id) <={0} Account(holder)")
		assert.equal(renderStatement(window(target, between(1n, 3n), source)), "Holder(id) <={1..3} Account(holder)")
		assert.equal(renderStatement(window(target, atLeast(2n), source)), "Holder(id) <={2..*} Account(holder)")
		assert.equal(renderStatement(window(target, atMost(4n), source)), "Holder(id) <={0..4} Account(holder)")
	})

	test("literal sets and interval literals render in macro notation", function probeSelectionRendering() {
		const { Kind, Account, SavingsTerms } = buildLedger()
		const setFace = on(Account.where({ kind: oneOf(Kind.Checking, Kind.Savings) }), "id")
		const spanFace = on(Account.where({ active: span(0n, 10n) }), "id")
		const target = on(SavingsTerms, "account")
		assert.equal(
			renderStatement(contained(setFace, target)),
			"Account(id | kind == {Checking, Savings}) <= SavingsTerms(account)"
		)
		assert.equal(renderStatement(contained(spanFace, target)), "Account(id | active == 0..10) <= SavingsTerms(account)")
	})
})

describe("the ban table, one row at a time", function describeBanTable() {
	test("{1..*} — atLeast(1) names the bare containment", function probeContainmentRespelled() {
		assert.throws(function bannedFloorOne() {
			atLeast(1n)
		}, /says only what the bare containment says/)
	})

	test("{n..n} — between(n, n) names exactly(n)", function probeExactRespelled() {
		assert.throws(function bannedExactRange() {
			between(2n, 2n)
		}, /an exact count is written `\{2\}`: use exactly\(2\)/)
	})

	test("{0..0} — between(0, 0), exactly(0), atMost(0) all name none", function probeExclusionRespelled() {
		assert.throws(function bannedZeroRange() {
			between(0n, 0n)
		}, /the exclusion is written `\{0\}`: use none/)
		assert.throws(function bannedExactZero() {
			exactly(0n)
		}, /use none/)
		assert.throws(function bannedCeilingZero() {
			atMost(0n)
		}, /use none/)
	})

	test("{0..*} — atLeast(0) is vacuous", function probeVacuous() {
		assert.throws(function bannedVacuous() {
			atLeast(0n)
		}, /vacuous — it provably says nothing/)
	})

	test("{hi..lo} — inverted windows are unsatisfiable", function probeInverted() {
		assert.throws(function bannedInverted() {
			between(3n, 1n)
		}, /inverted — no count satisfies it/)
	})

	test("one-element literal sets are unwritable — oneOf demands two literals", function probeDegenerateSet() {
		const set = oneOf(1n, 2n)
		assert.equal(set.literals.length, 2)
		assert.equal(oneOf.length, 2)
	})
})

describe("schema() construction boundary", function describeSchemaBoundary() {
	test("a statement over an undeclared relation is rejected with the statement rendered", function probeMembership() {
		const { Kind, Holder, Account } = buildLedger()
		assert.throws(function undeclaredRelation() {
			schema("Broken", { Kind, Account }, [contained(on(Account, "holder"), on(Holder, "id"))])
		}, /relation Holder is not declared in this schema — Account\(holder\) <= Holder\(id\)/)
	})

	test("a same-named but different relation value is rejected", function probeIdentity() {
		const impostor = relation("Holder", { id: HolderId.fresh })
		const declared = relation("Holder", { id: HolderId.fresh })
		assert.throws(function differentValue() {
			schema("Broken", { Holder: declared }, [contained(on(impostor, "id"), on(declared, "id"))])
		}, /different relation value named Holder/)
	})

	test("an explicit duplicate of the fresh-implied key is rejected (macro parity)", function probeImpliedDuplicate() {
		const { Kind, Holder, Account, SavingsTerms } = buildLedger()
		assert.throws(function duplicateImplied() {
			schema("Broken", { Kind, Holder, Account, SavingsTerms }, [key(Account, ["id"])])
		}, /Account\(id\) -> Account is redundant here .* rejected as a duplicate/)
	})

	test("duplicate statements are rejected via their canonical rendering", function probeDuplicate() {
		const { Kind, Holder, Account, SavingsTerms } = buildLedger()
		assert.throws(function duplicateStatement() {
			schema("Broken", { Kind, Holder, Account, SavingsTerms }, [
				contained(on(Account, "holder"), on(Holder, "id")),
				contained(on(Account, "holder"), on(Holder, "id"))
			])
		}, /duplicate statement — Account\(holder\) <= Holder\(id\)/)
	})

	test("a record key must equal its relation's declared name", function probeRecordKey() {
		const { Account } = buildLedger()
		assert.throws(function mismatchedKey() {
			schema("Broken", { Acct: Account }, [])
		}, /record key Acct holds relation Account/)
	})
})
