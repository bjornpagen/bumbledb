/**
 * PRD-S2 pins: the full Ledger example (key, containment, selected `==`,
 * window) lowers to its `SchemaSpec` shape with DOMAINS carried throughout;
 * the canonical-utterance ban table is enumerated one row at a time (each
 * banned LITERAL spelling a REAL `@ts-expect-error` — unwritable — and each
 * computed-bound escape a construction error naming the canonical form);
 * field references are checked in the type (existence and domain, read
 * structurally off the schema type); `schema()` enforces its
 * expansion-boundary checks including the handle-selection paste-back law;
 * and `renderStatement` emits the canonical `70-api.md` spellings exactly.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"

import { closed } from "#closed.ts"
import * as countModule from "#count.ts"
import { atLeast, atMost, between, exactly, none } from "#count.ts"
import type { Db } from "#db.ts"
import { on, oneOf } from "#face.ts"
import { i64, interval, span, str, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import { type InsertFact, relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained, key, mirrors, renderStatement, window } from "#statements.ts"

const HolderId = u64.as("HolderId")
const AccountId = u64.as("AccountId")
const RoomId = u64.as("RoomId")
const ActiveDuring = interval(i64).as("ActiveDuring")
const BookedDuring = interval(u64).as("BookedDuring")

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

/** The composite/pointwise fixtures: `on(R, ["a", "b"])` positions and the composite key. */
function buildCalendar() {
	const Booking = relation("Booking", { room: RoomId, during: BookedDuring })
	const Slot = relation("Slot", { room: RoomId, during: BookedDuring })
	const statements = [
		key(Booking, ["room", "during"]),
		contained(on(Slot, ["room", "during"]), on(Booking, ["room", "during"]))
	]
	const Calendar = schema("Calendar", { Booking, Slot }, statements)
	return { Booking, Slot, statements, Calendar }
}

describe("the Ledger example", function describeLedger() {
	test("lowers to the SchemaSpec shape, declaration order and domains throughout", function probeLedgerLowering() {
		const { Ledger } = buildLedger()
		assert.deepStrictEqual(lower(Ledger), {
			relations: [
				{
					name: "Kind",
					newtype: "KindId",
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
						{ name: "kind", valueType: { kind: "u64" }, newtype: "KindId", fresh: false },
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

	test("the composite key and pointwise containment lower positionally", function probeCalendarLowering() {
		const { Calendar } = buildCalendar()
		assert.deepStrictEqual(lower(Calendar).statements, [
			{ kind: "fd", relation: "Booking", projection: ["room", "during"] },
			{
				kind: "containment",
				source: { relation: "Slot", projection: ["room", "during"], selection: [] },
				target: { relation: "Booking", projection: ["room", "during"], selection: [] },
				bidirectional: false
			}
		])
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

	test("composite positions render in written tuple order", function probeCompositeSpellings() {
		const { statements } = buildCalendar()
		assert.deepStrictEqual(statements.map(renderStatement), [
			"Booking(room, during) -> Booking",
			"Slot(room, during) <= Booking(room, during)"
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

describe("the ban table, one row at a time — literal spellings are UNWRITABLE", function describeBanTable() {
	test("no sixth constructor exists — the count vocabulary is exactly the five", function probeVocabulary() {
		assert.deepStrictEqual(Object.keys(countModule).sort(), ["atLeast", "atMost", "between", "exactly", "none"])
	})

	test("one-element literal sets are unwritable — oneOf demands two literals", function probeDegenerateSet() {
		const set = oneOf(1n, 2n)
		assert.equal(set.literals.length, 2)
		assert.equal(oneOf.length, 2)
	})
})

/**
 * The ban table's compile tier: every banned LITERAL spelling is a type
 * error naming the canonical form — there is no argument shape that
 * produces `{0}`-as-exactly, `{n..n}`, `{0..0}`, `{0..*}`, `{1..*}`, or a
 * negative bound. Each directive is REAL: removing it breaks compilation.
 */
function banTableIsUnwritable(): unknown[] {
	return [
		// @ts-expect-error — `{0}` is the exclusion: the spelling is `none`, exactly(0n) does not exist
		exactly(0n),
		// @ts-expect-error — window counts are u64: a negative exact count is out of domain
		exactly(-1n),
		// @ts-expect-error — `{0..0}` is the exclusion respelled: write none
		between(0n, 0n),
		// @ts-expect-error — `{n..n}` is the exact count respelled: write exactly(n)
		between(2n, 2n),
		// @ts-expect-error — window bounds are u64: a negative bound is out of domain
		between(-1n, 3n),
		// @ts-expect-error — `{0..*}` is vacuous: it provably says nothing, delete the statement
		atLeast(0n),
		// @ts-expect-error — `{1..*}` says only what the bare containment says: write contained(source, target)
		atLeast(1n),
		// @ts-expect-error — `{0..0}` is the exclusion respelled: write none
		atMost(0n),
		// @ts-expect-error — window counts are u64: a negative ceiling is out of domain
		atMost(-2n)
	]
}

describe("the ban table's construction tier — computed bounds the type cannot judge", function describeBelts() {
	/** A bound whose literal identity the type level has already lost. */
	const computed: (n: bigint) => bigint = function widen(n) {
		return n
	}

	test("a computed banned bound is a construction error naming the canonical form", function probeComputedBans() {
		assert.throws(function computedExactZero() {
			exactly(computed(0n))
		}, /use none/)
		assert.throws(function computedFloorOne() {
			atLeast(computed(1n))
		}, /says only what the bare containment says/)
		assert.throws(function computedVacuous() {
			atLeast(computed(0n))
		}, /vacuous — it provably says nothing/)
		assert.throws(function computedCeilingZero() {
			atMost(computed(0n))
		}, /use none/)
		assert.throws(function computedExactRange() {
			between(computed(2n), computed(2n))
		}, /an exact count is written `\{2\}`: use exactly\(2\)/)
		assert.throws(function computedZeroRange() {
			between(computed(0n), computed(0n))
		}, /the exclusion is written `\{0\}`: use none/)
		assert.throws(function computedNegative() {
			exactly(computed(-1n))
		}, /window counts are u64/)
	})

	test("an inverted window is unsatisfiable — bigint literals carry no type-level order", function probeInverted() {
		assert.throws(function bannedInverted() {
			between(3n, 1n)
		}, /inverted — no count satisfies it/)
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

	test("the paste-back law: a handle selection needs its resolving containment declared", function probePasteBack() {
		const { Kind, Holder, Account, SavingsTerms } = buildLedger()
		assert.throws(function unresolvedHandleSelection() {
			schema("Broken", { Kind, Holder, Account, SavingsTerms }, [
				mirrors(on(Account.where({ kind: Kind.Savings }), "id"), on(SavingsTerms, "account"))
			])
		}, /no declared containment resolves the closed reference/)
	})
})

// ————————————————————————————————————————————————————————————————————————
// The S2 compile probes: field references are checked in the TYPE —
// existence (names autocomplete, unknown field = type error) and DOMAIN
// compatibility (positionwise string-literal equality of the S1 labels,
// read structurally off the schema type — never a value brand). Each
// function is exported-but-uncalled; each directive is REAL.
// ————————————————————————————————————————————————————————————————————————

/** `on()` field references must exist on the source — existence is a type property. */
function fieldReferencesAreTypeChecked(): unknown[] {
	const { Kind, Account } = buildLedger()
	const { Booking } = buildCalendar()
	return [
		// @ts-expect-error — Account has no field `nope`
		on(Account, "nope"),
		// @ts-expect-error — a composite position field-checks every name
		on(Booking, ["room", "nope"]),
		// @ts-expect-error — the empty projection has no meaning in the statement grammar
		on(Booking, []),
		// @ts-expect-error — a closed relation's sealed shape holds `id` (plus payload columns) only
		on(Kind, "kind"),
		// @ts-expect-error — a key projection names declared fields only
		key(Account, ["id", "nope"])
	]
}

/** Cross-domain pairs are compile errors on every relating constructor. */
function domainsAreComparedStructurally(): unknown[] {
	const { Holder, Account, SavingsTerms } = buildLedger()
	const { Booking, Slot } = buildCalendar()
	return [
		// the legal pairs compile — same labels, positionwise
		contained(on(Account, "holder"), on(Holder, "id")),
		contained(on(Slot, ["room", "during"]), on(Booking, ["room", "during"])),
		// @ts-expect-error — HolderId vs AccountId: a cross-domain containment pair
		contained(on(Account, "holder"), on(SavingsTerms, "account")),
		// @ts-expect-error — an unlabeled field (undefined) links only unlabeled fields
		contained(on(Holder, "name"), on(Account, "holder")),
		// @ts-expect-error — KindId vs HolderId: a closed reference pairs only with its own handle domain
		contained(on(Account, "kind"), on(Holder, "id")),
		// @ts-expect-error — composite positions compare positionwise: [RoomId, BookedDuring] vs [BookedDuring, RoomId]
		contained(on(Slot, ["room", "during"]), on(Booking, ["during", "room"])),
		// @ts-expect-error — a mirrors bijection relates domains exactly as containment
		mirrors(on(Account, "id"), on(Holder, "id")),
		// @ts-expect-error — a window's grouping join relates domains exactly as containment
		window(on(Holder, "id"), atMost(3n), on(Account, "id")),
		// @ts-expect-error — arity mismatch: positional pairing requires equally many fields
		contained(on(Slot, ["room", "during"]), on(Booking, "room"))
	]
}

/** `where()` selections are typed: handles are the closed value's own constants. */
function selectionsAreTyped(): unknown[] {
	const { Kind, Account } = buildLedger()
	return [
		Account.where({ kind: Kind.Savings }),
		// @ts-expect-error — Nope is not a handle of Kind's vocabulary
		Account.where({ kind: Kind.Nope }),
		// @ts-expect-error — a closed reference selects by handle id (bigint), never by name string
		Account.where({ kind: "Savings" }),
		// @ts-expect-error — Account has no field `nope` to select on
		Account.where({ nope: 1n })
	]
}

/**
 * `schema()` carries its relation record as typestate: `Db` over one
 * schema's relations accepts exactly those relations — a schema-A fact
 * into a schema-B store is a compile error (relation identity is the
 * membership rule).
 */
function dbTypestateHoldsTheWall(
	ledgerDb: Db<ReturnType<typeof buildLedger>["Ledger"]["relations"]>,
	calendarDb: Db<ReturnType<typeof buildCalendar>["Calendar"]["relations"]>,
	account: InsertFact<ReturnType<typeof buildLedger>["Account"]>
): void {
	const { Account } = buildLedger()
	ledgerDb.write(function accepts(tx) {
		tx.insert(Account, account)
	})
	calendarDb.write(function rejects(tx) {
		// @ts-expect-error — a Ledger fact belongs to Db<Ledger>, never Db<Calendar>
		tx.insert(Account, account)
	})
}

export {
	banTableIsUnwritable,
	dbTypestateHoldsTheWall,
	domainsAreComparedStructurally,
	fieldReferencesAreTypeChecked,
	selectionsAreTyped
}
