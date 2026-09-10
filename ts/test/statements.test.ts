import assert from "node:assert/strict"
import { describe, test } from "node:test"
import * as capacityModule from "#capacity.ts"
import { duration, ref, weigh, within } from "#capacity.ts"
import { closed, closedId } from "#closed.ts"
import { on } from "#face.ts"
import { bool, bytes, i64, interval, str, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { select } from "#selection.ts"
import { capacity, contained, key, mirrors, renderStatement } from "#statements.ts"

function buildLedger() {
	const Kind = closed("Kind", ["Checking", "Savings"])
	const Holder = relation("Holder", { id: u64, name: str })
	const Account = relation("Account", {
		id: u64,
		holder: u64,
		kind: closedId(Kind),
		active: interval(i64)
	})
	const SavingsTerms = relation("SavingsTerms", { account: u64 })
	const statements = [
		key(Holder, ["id"]),
		key(Account, ["id"]),
		key(SavingsTerms, ["account"]),
		contained(on(Account, "holder"), on(Holder, "id")),
		contained(on(Account, "kind"), on(Kind, "id")),
		mirrors(on(select(Account, { kind: "Savings" }), "id"), on(SavingsTerms, "account")),
		capacity(on(Holder, "id"), { from: on(Account, "holder"), within: within(0n, 3n) })
	]
	const Ledger = schema("Ledger", { Kind, Holder, Account, SavingsTerms }, statements)
	return { Kind, Holder, Account, SavingsTerms, statements, Ledger }
}

function buildCalendar() {
	const Booking = relation("Booking", { room: u64, during: interval(u64) })
	const Slot = relation("Slot", { room: u64, during: interval(u64) })
	const statements = [
		key(Booking, ["room", "during"]),
		contained(on(Slot, ["room", "during"]), on(Booking, ["room", "during"]))
	]
	const Calendar = schema("Calendar", { Booking, Slot }, statements)
	return { Booking, Slot, statements, Calendar }
}

function buildMastery() {
	const Grade = closed(
		"Grade",
		["Failed", "DirectPass"],
		{ mastered: bool, score: u64 },
		{
			Failed: { mastered: false, score: 0n },
			DirectPass: { mastered: true, score: 2n }
		}
	)
	const Certificate = relation("Certificate", { id: u64, grade: closedId(Grade) })
	const psiContainment = contained(on(Certificate, "grade"), on(select(Grade, { mastered: true }), "id"))
	const psiCapacity = capacity(on(select(Grade, { mastered: true }), "id"), {
		from: on(Certificate, "grade"),
		within: within(0n, 1n)
	})
	const Mastery = schema("Mastery", { Grade, Certificate }, [psiContainment, psiCapacity])
	return { Grade, Certificate, psiContainment, psiCapacity, Mastery }
}

function buildRacks() {
	const Pool = relation("Pool", { id: u64, supply: u64 })
	const Device = relation("Device", { id: u64, pool: u64, watts: u64 })
	const Room = relation("Room", { id: u64, span: interval(i64) })
	const Booking = relation("Booking", { id: u64, room: u64, booked: interval(i64) })
	return { Pool, Device, Room, Booking }
}

function buildSeverity() {
	const Sev = closed(
		"Sev",
		["Info", "Critical"],
		{ level: u64 },
		{
			Info: { level: 1n },
			Critical: { level: 5n }
		}
	)
	const Limit = relation("Limit", { level: u64, cap: u64 })
	const statements = [key(Limit, ["level"]), contained(on(Sev, "level"), on(Limit, "level"))]
	const Severity = schema("Severity", { Sev, Limit }, statements)
	return { Sev, Limit, statements, Severity }
}

describe("the Ledger example", function describeLedger() {
	test("lowers to the SchemaSpec shape, declaration order throughout, newtype slots carrying the law-computed class names", function probeLedgerLowering() {
		const { Ledger } = buildLedger()
		assert.deepStrictEqual(lower(Ledger), {
			relations: [
				{
					name: "Kind",
					fields: [],
					closed: {
						newtype: "Kind.id",
						rows: [
							{ handle: "Checking", values: [] },
							{ handle: "Savings", values: [] }
						]
					}
				},
				{
					name: "Holder",
					fields: [
						{ name: "id", valueType: { kind: "u64" }, newtype: "Holder.id" },
						{ name: "name", valueType: { kind: "string" }, newtype: undefined }
					],
					closed: undefined
				},
				{
					name: "Account",
					fields: [
						{ name: "id", valueType: { kind: "u64" }, newtype: "Account.id" },
						{ name: "holder", valueType: { kind: "u64" }, newtype: "Holder.id" },
						{ name: "kind", valueType: { kind: "u64" }, newtype: "Kind.id" },
						{
							name: "active",
							valueType: { kind: "interval", element: "i64", width: undefined },
							newtype: undefined
						}
					],
					closed: undefined
				},
				{
					name: "SavingsTerms",
					fields: [{ name: "account", valueType: { kind: "u64" }, newtype: "Account.id" }],
					closed: undefined
				}
			],
			statements: [
				{ kind: "fd", relation: "Holder", projection: ["id"] },
				{ kind: "fd", relation: "Account", projection: ["id"] },
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
					kind: "capacity",
					target: { relation: "Holder", projection: ["id"], selection: [] },
					weight: { kind: "unit" },
					window: {
						kind: "range",
						lo: { kind: "lit", value: 0n },
						hi: { kind: "lit", value: 3n }
					},
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

	test("a closed payload column lowers pure structure — the newtype slots carry its law-computed classes", function probeClosedPayloadLowering() {
		const { Severity } = buildSeverity()
		assert.deepStrictEqual(lower(Severity), {
			relations: [
				{
					name: "Sev",
					fields: [{ name: "level", valueType: { kind: "u64" }, newtype: "Sev.level" }],
					closed: {
						newtype: "Sev.id",
						rows: [
							{ handle: "Info", values: [{ kind: "value", value: { kind: "u64", value: 1n } }] },
							{ handle: "Critical", values: [{ kind: "value", value: { kind: "u64", value: 5n } }] }
						]
					}
				},
				{
					name: "Limit",
					fields: [
						{ name: "level", valueType: { kind: "u64" }, newtype: "Sev.level" },
						{ name: "cap", valueType: { kind: "u64" }, newtype: undefined }
					],
					closed: undefined
				}
			],
			statements: [
				{ kind: "fd", relation: "Limit", projection: ["level"] },
				{
					kind: "containment",
					source: { relation: "Sev", projection: ["level"], selection: [] },
					target: { relation: "Limit", projection: ["level"], selection: [] },
					bidirectional: false
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
			"Holder(id) -> Holder",
			"Account(id) -> Account",
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

	test("every legal capacity spelling renders canonically — unit strings byte-for-byte, the weighted rows beside them", function probeCapacitySpellings() {
		const { Holder, Account } = buildLedger()
		const target = on(Holder, "id")
		const source = on(Account, "holder")
		assert.equal(
			renderStatement(capacity(target, { from: source, within: within(1n) })),
			"Holder(id) <={1} Account(holder)"
		)
		assert.equal(
			renderStatement(capacity(target, { from: source, within: within(0n) })),
			"Holder(id) <={0} Account(holder)"
		)
		assert.equal(
			renderStatement(capacity(target, { from: source, within: within(1n, 3n) })),
			"Holder(id) <={1..3} Account(holder)"
		)
		assert.equal(
			renderStatement(capacity(target, { from: source, within: within(0n, 4n) })),
			"Holder(id) <={0..4} Account(holder)"
		)

		const { Pool, Device, Room, Booking } = buildRacks()
		assert.equal(
			renderStatement(
				capacity(on(Pool, "id"), { from: on(Device, "pool"), weight: weigh("watts"), within: within(0n, 20n) })
			),
			"Pool(id) <=[watts]{0..20} Device(pool)"
		)
		assert.equal(
			renderStatement(
				capacity(on(Pool, "id"), {
					from: on(Device, "pool"),
					weight: weigh("watts"),
					within: within(0n, ref("supply"))
				})
			),
			"Pool(id) <=[watts]{0..supply} Device(pool)"
		)
		assert.equal(
			renderStatement(
				capacity(on(Pool, "id"), { from: on(Device, "pool"), weight: weigh("watts"), within: within(1n, "*") })
			),
			"Pool(id) <=[watts]{1..*} Device(pool)"
		)
		assert.equal(
			renderStatement(
				capacity(on(Room, "id"), {
					from: on(Booking, "room"),
					weight: weigh(duration("booked")),
					within: within(0n, duration("span"))
				})
			),
			"Room(id) <=[Duration(booked)]{0..Duration(span)} Booking(room)"
		)
	})

	test("literal sets and interval literals render in macro notation", function probeSelectionRendering() {
		const { Account, SavingsTerms } = buildLedger()
		const setFace = on(select(Account, { kind: ["Checking", "Savings"] }), "id")
		const spanFace = on(select(Account, { active: { start: 0n, end: 10n } }), "id")
		const target = on(SavingsTerms, "account")
		assert.equal(
			renderStatement(contained(setFace, target)),
			"Account(id | kind == {Checking, Savings}) <= SavingsTerms(account)"
		)
		assert.equal(renderStatement(contained(spanFace, target)), "Account(id | active == 0..10) <= SavingsTerms(account)")
	})
})

describe("the ban table, one row at a time — literal spellings are UNWRITABLE", function describeBanTable() {
	test("the capacity mint vocabulary is exactly the roster — no sixth mint exists", function probeVocabulary() {
		assert.deepStrictEqual(Object.keys(capacityModule).sort(), [
			"capacityWeight",
			"capacityWindow",
			"duration",
			"ref",
			"unitWeight",
			"weigh",
			"within"
		])
	})

	test("degenerate literal sets refuse — a membership array needs two DISTINCT members, and the refusal locates itself", function probeDegenerateSet() {
		const { Account } = buildLedger()
		// Every refusal names the relation and field (`relation Account.kind:`)

		assert.throws(function emptySet() {
			select(Account, { kind: [] })
		}, /relation Account\.kind: an empty literal set selects nothing/)
		assert.throws(function oneElementSet() {
			select(Account, { kind: ["Checking"] })
		}, /relation Account\.kind: a one-element literal set is the bare literal respelled/)
		// A duplicate member is the banned one-element set respelled — refused

		assert.throws(function duplicateMember() {
			select(Account, { kind: ["Checking", "Checking"] })
		}, /relation Account\.kind: the literal set spells Checking twice — write it once/)

		assert.throws(function duplicateOrdinary() {
			const { Holder } = buildLedger()
			select(Holder, { name: ["a", "b", "a"] })
		}, /relation Holder\.name: the literal set spells "a" twice — write it once/)
	})

	test("a duplicate field in a key() projection refuses at the mint — the engine's FieldSet duplicate, canonical voice", function probeDuplicateKeyProjection() {
		const { Holder } = buildLedger()

		// without the mint refusal it could set-match a 1-field target

		assert.throws(function duplicateProjection() {
			key(Holder, ["name", "name"])
		}, /key\(Holder, \.\.\.\): the projection spells name twice/)
	})

	test("a plain u64 face never pairs a closed [id] face — closedness rides the descriptor (both tiers)", function probeRosterWall() {
		const { Sev, Limit } = buildSeverity()

		assert.throws(function aliasContainment() {
			// @ts-expect-error — a plain u64 column cannot alias a closed vocabulary through a containment
			contained(on(Limit, "cap"), on(Sev, "id"))
		}, /Limit\.cap is a bare column but Sev\.id is a Sev reference — closedness rides the descriptor/)
		assert.throws(function aliasReversed() {
			// @ts-expect-error — the reverse orientation is the same wall (pairing is symmetric)
			mirrors(on(Sev, "id"), on(Limit, "cap"))
		}, /Sev\.id is a Sev reference but Limit\.cap is a bare column/)
		assert.throws(function aliasCapacity() {
			// @ts-expect-error — a capacity statement's grouping join holds the roster wall exactly as containment
			capacity(on(Sev, "id"), { from: on(Limit, "cap"), within: within(0n, 1n) })
		}, /Limit\.cap is a bare column but Sev\.id is a Sev reference/)

		const Alert = relation("Alert", { sev: closedId(Sev) })
		assert.equal(renderStatement(contained(on(Alert, "sev"), on(Sev, "id"))), "Alert(sev) <= Sev(id)")
	})

	test("an arity-mismatched pairing is a construction error — the SameArity runtime twin (untyped path)", function probeArityWall() {
		/**
		 * Ruling 9 (cleanup-0.5.0): SameArity's runtime seat. The type tier
		 * already refuses these (the directives are real); before the twin an
		 * UNTYPED caller's mismatch silently truncated to the shorter
		 * projection (the positionwise walks skip unpaired positions) until
		 * Db.create's colder engine refusal — now the statement itself judges.
		 */
		const { Booking, Slot } = buildCalendar()
		assert.throws(function truncatedContainment() {
			// @ts-expect-error — SameArity refuses the pairing at the type tier; this is its construction-time twin
			contained(on(Booking, ["room", "during"]), on(Slot, "room"))
		}, /Booking\(room, during\) and Slot\(room\) project 2 vs 1 fields — positional pairing requires both faces to project equally many/)
		assert.throws(function truncatedMirrors() {
			// @ts-expect-error — the == abbreviation holds the same arity wall
			mirrors(on(Slot, "room"), on(Booking, ["room", "during"]))
		}, /Slot\(room\) and Booking\(room, during\) project 1 vs 2 fields/)
		assert.throws(function truncatedCapacity() {
			// @ts-expect-error — a capacity statement's grouping join holds the arity wall exactly as containment
			capacity(on(Slot, "room"), { from: on(Booking, ["room", "during"]), within: within(0n, 1n) })
		}, /Booking\(room, during\) and Slot\(room\) project 2 vs 1 fields/)

		assert.equal(
			renderStatement(contained(on(Slot, ["room", "during"]), on(Booking, ["room", "during"]))),
			"Slot(room, during) <= Booking(room, during)"
		)
	})
})

function banTableIsUnwritable(): unknown[] {
	const { Room, Booking } = buildRacks()
	return [
		// @ts-expect-error — capacity bounds are u64: a negative exact measure is out of domain
		within(-1n),
		// @ts-expect-error — capacity bounds are u64: a negative bound is out of domain
		within(-1n, 3n),
		// @ts-expect-error — capacity bounds are u64: a negative ceiling is out of domain
		within(1n, -3n),
		// @ts-expect-error — a count of facts bounded by a span of time mixes dimensions (C18): the duration() bound is banned on the UNIT instance
		capacity(on(Room, "id"), { from: on(Booking, "room"), within: within(0n, duration("span")) }),
		// @ts-expect-error — a path weight is refused: the vocabulary is closed at the row — pin the column
		weigh("model.watts"),
		// @ts-expect-error — a path bound is refused the same way: bounds name the target's own row
		ref("model.supply"),
		// @ts-expect-error — a Duration path is refused the same way
		duration("model.span")
	]
}

/**
 * The spelling-ban tables are DELETED (C01, chapter 34): `{n..n}`, `{0..0}`,
 * unit floors `{1..*}`/`{N..*}` and the vacuous `{0..*}` are harmless
 * equivalent spellings that LOWER to the one canonical `(lo, hi)` law at
 * the mint, preserving authored-statement attribution. Compiling AND the
 * canonical render are the pins. Genuinely different semantics (negative,
 * inverted, C18 dimension mixing) still refuse above.
 */
function equivalentSpellingsLowerToTheCanonicalLaw(): unknown[] {
	const { Pool, Device } = buildRacks()
	return [
		within(0n, 0n),
		within(2n, 2n),
		within(0n, "*"),
		capacity(on(Pool, "id"), { from: on(Device, "pool"), within: within(1n, "*") }),
		capacity(on(Pool, "id"), { from: on(Device, "pool"), within: within(2n, "*") }),
		capacity(on(Pool, "id"), { from: on(Device, "pool"), within: within(0n, "*") })
	]
}

/**
 * The weight-sensitive split's POSITIVE probe (design § 6): `<=[w]{1..*}`
 * COMPILES — on a weighted statement "positive total" admits zero-weight
 * rows and is a different, weaker law than containment.
 * Exported-but-uncalled; its compiling IS the pin.
 */
function weightedFloorOneCompiles(): unknown {
	const { Pool, Device } = buildRacks()
	return capacity(on(Pool, "id"), { from: on(Device, "pool"), weight: weigh("watts"), within: within(1n, "*") })
}

function capacityWallsAreTyped(): unknown[] {
	const { Pool, Device, Room, Booking } = buildRacks()
	return [
		capacity(on(Pool, "id"), { from: on(Device, "pool"), weight: weigh("watts"), within: within(0n, ref("supply")) }),
		capacity(on(Room, "id"), {
			from: on(Booking, "room"),
			weight: weigh(duration("booked")),
			within: within(0n, duration("span"))
		}),
		// @ts-expect-error — the weight names a field of the SOURCE's own row: Device has no field `nope`
		capacity(on(Pool, "id"), { from: on(Device, "pool"), weight: weigh("nope"), within: within(0n, 3n) }),
		// @ts-expect-error — a weight is u64-encoded: an interval field needs the Duration(...) spelling
		capacity(on(Room, "id"), { from: on(Room, "id"), weight: weigh("span"), within: within(0n, 3n) }),
		// @ts-expect-error — weigh(duration(...)) weighs an interval field: pool is a u64
		capacity(on(Pool, "id"), { from: on(Device, "pool"), weight: weigh(duration("pool")), within: within(0n, 3n) }),
		// @ts-expect-error — a dependent bound names a field of the TARGET's own row: Pool has no field `nope`
		capacity(on(Pool, "id"), { from: on(Device, "pool"), weight: weigh("watts"), within: within(0n, ref("nope")) }),
		capacity(on(Room, "id"), {
			from: on(Booking, "room"),
			weight: weigh(duration("booked")),
			// @ts-expect-error — ref() reads u64, not an interval; use duration("span")
			within: within(0n, ref("span"))
		}),
		capacity(on(Pool, "id"), {
			from: on(Device, "pool"),
			weight: weigh("watts"),
			// @ts-expect-error — duration() reads an interval, not u64; use ref("supply")
			within: within(0n, duration("supply"))
		})
	]
}

describe("the ban table's construction tier — computed bounds the type cannot judge", function describeBelts() {
	const computed: (n: bigint) => bigint = function widen(n) {
		return n
	}

	const computedName: (name: string) => string = function widenName(name) {
		return name
	}

	test("computed equivalent spellings lower to the canonical law; only negatives refuse", function probeComputedBans() {
		// `{0..*}` (vacuous) and `{n..n}`/`{0..0}` are ACCEPTED canonical
		// spellings now — normalization preserves the authored statement
		// instead of policing the style (C01, chapter 34).
		assert.deepEqual(within(computed(0n), "*"), {
			kind: "floor",
			lo: { kind: "lit", value: 0n }
		})
		assert.deepEqual(within(computed(2n), computed(2n)), {
			kind: "exact",
			n: { kind: "lit", value: 2n }
		})
		assert.deepEqual(within(computed(0n), computed(0n)), {
			kind: "exact",
			n: { kind: "lit", value: 0n }
		})
		assert.throws(function computedNegative() {
			within(computed(-1n))
		}, /u64 bigint in range/)
	})

	test("an inverted window is unsatisfiable — bigint literals carry no type-level order", function probeInverted() {
		assert.throws(function bannedInverted() {
			within(3n, 1n)
		}, /inverted — no measure satisfies it/)
	})

	test("unit floors are accepted canonical laws now — the spelling police is deleted", function probeComputedFloorOne() {
		const { Pool, Device } = buildRacks()
		const unitFloorOne = capacity(on(Pool, "id"), {
			from: on(Device, "pool"),
			within: within(computed(1n), "*")
		})
		assert.equal(renderStatement(unitFloorOne), "Pool(id) <={1..*} Device(pool)")
		const unitFloorTwo = capacity(on(Pool, "id"), {
			from: on(Device, "pool"),
			within: within(computed(2n), "*")
		})
		assert.equal(renderStatement(unitFloorTwo), "Pool(id) <={2..*} Device(pool)")

		const weighted = capacity(on(Pool, "id"), {
			from: on(Device, "pool"),
			weight: weigh("watts"),
			within: within(computed(1n), "*")
		})
		assert.equal(renderStatement(weighted), "Pool(id) <=[watts]{1..*} Device(pool)")
	})

	test("the C18 dimension gate fires at the capacity() call — a unit window never takes a duration() bound (both tiers)", function probeUnitDimensionGate() {
		const { Room, Booking } = buildRacks()
		assert.throws(function unitDurationBound() {
			// @ts-expect-error — a count of facts bounded by a span of time mixes dimensions (C18): the type tier's ban row, with the construction wall behind it
			return capacity(on(Room, "id"), { from: on(Booking, "room"), within: within(0n, duration("span")) })
		}, /mixes dimensions \(C18\)/)

		const weighted = capacity(on(Room, "id"), {
			from: on(Booking, "room"),
			weight: weigh(duration("booked")),
			within: within(0n, duration("span"))
		})
		assert.equal(renderStatement(weighted), "Room(id) <=[Duration(booked)]{0..Duration(span)} Booking(room)")
	})

	test("a computed path weight or bound is a construction refusal naming the pinned-column idiom", function probeComputedPaths() {
		assert.throws(function computedPathWeight() {
			weigh(computedName("model.watts"))
		}, /closed at the row .* pin the column/)
		assert.throws(function computedPathBound() {
			ref(computedName("model.supply"))
		}, /closed at the row .* pin the column/)
	})

	test("the weight and dependent-bound walls hold at construction for untyped callers", function probeCapacityRuntimeTwins() {
		const { Pool, Device, Room, Booking } = buildRacks()
		assert.throws(function weightOffRoster() {
			capacity(on(Pool, "id"), {
				from: on(Device, "pool"),
				weight: weigh(computedName("nope")),
				within: within(0n, 3n)
			})
		}, /Device has no field nope — a weight names a field of the SOURCE's own row/)
		assert.throws(function weightNotU64() {
			capacity(on(Room, "id"), { from: on(Room, "id"), weight: weigh(computedName("span")), within: within(0n, 3n) })
		}, /Room\.span is interval, not u64 — a weight is u64-encoded/)
		assert.throws(function durationWeightNotInterval() {
			capacity(on(Pool, "id"), {
				from: on(Device, "pool"),
				weight: weigh(duration(computedName("pool"))),
				within: within(0n, 3n)
			})
		}, /Device\.pool is u64, not an interval — Duration\(\.\.\.\) weighs/)
		assert.throws(function boundOffRoster() {
			capacity(on(Pool, "id"), {
				from: on(Device, "pool"),
				weight: weigh("watts"),
				within: within(0n, ref(computedName("nope")))
			})
		}, /Pool has no field nope — a dependent bound names a field of the TARGET's own row/)
		assert.throws(function boundNotU64() {
			capacity(on(Room, "id"), {
				from: on(Booking, "room"),
				weight: weigh(duration("booked")),
				within: within(0n, ref(computedName("span")))
			})
		}, /Room\.span is interval, not u64 — a dependent bound reads a u64 field/)
		assert.throws(function durationBoundNotInterval() {
			capacity(on(Pool, "id"), {
				from: on(Device, "pool"),
				weight: weigh("watts"),
				within: within(0n, duration(computedName("supply")))
			})
		}, /Pool\.supply is u64, not an interval — Duration\(\.\.\.\) bounds/)
	})

	test("generated capacity values use the same validation as constructors", () => {
		const { Pool, Device } = buildRacks()
		const window = { kind: "range", lo: { kind: "lit", value: 0n }, hi: { kind: "lit", value: 3n } } as const
		const weight = { kind: "field", field: "watts" } as const
		assert.deepEqual(
			capacity(on(Pool, "id"), { from: on(Device, "pool"), weight, within: window }),
			capacity(on(Pool, "id"), { from: on(Device, "pool"), weight: weigh("watts"), within: within(0n, 3n) })
		)
		for (const invalid of [
			{ ...window, lo: { kind: "lit", value: -1n } },
			{ ...window, hi: { kind: "lit", value: 2n ** 64n } },
			{ ...window, extra: true },
			{ ...window, hi: { kind: "lit", value: -1n } }
		])
			assert.throws(() => capacity(on(Pool, "id"), { from: on(Device, "pool"), within: invalid as never }))
		assert.throws(() =>
			capacity(on(Pool, "id"), {
				from: on(Device, "pool"),
				weight: { ...weight, extra: true } as never,
				within: window
			})
		)
	})
})

describe("schema() construction boundary", function describeSchemaBoundary() {
	test("a statement over an undeclared relation is rejected with the statement rendered", function probeMembership() {
		const { Kind, Holder, Account } = buildLedger()
		assert.throws(function undeclaredRelation() {
			// @ts-expect-error — undeclared targets also have no declared target key
			schema("Broken", { Kind, Account }, [contained(on(Account, "holder"), on(Holder, "id"))])
		}, /relation Holder is not declared in this schema — Account\(holder\) <= Holder\(id\)/)
	})

	test("same-named relations resolve structurally, including field order", () => {
		const declared = relation("Holder", { id: u64 })
		const equivalent = relation("Holder", { id: u64 })
		assert.doesNotThrow(() =>
			schema("Equivalent", { Holder: declared }, [
				key(declared, ["id"]),
				contained(on(equivalent, "id"), on(declared, "id"))
			])
		)
		const conflicting = relation("Holder", { id: u64, other: u64 })
		assert.throws(
			() =>
				schema("Broken", { Holder: declared }, [
					key(declared, ["id"]),
					contained(on(conflicting, "id"), on(declared, "id"))
				]),
			/different relation declaration named Holder/
		)
	})

	test("declared id keys are ordinary statements now — the fresh-implied key is deleted (E-NO-RESERVE)", function probeDeclaredIdKey() {
		const { Kind, Holder, Account, SavingsTerms } = buildLedger()
		// With no fresh mint there is no implied key to duplicate: an
		// explicit `Account(id) -> Account` is the ONE way to key an id.
		const keyed = schema("Keyed", { Kind, Holder, Account, SavingsTerms }, [key(Account, ["id"])])
		assert.deepStrictEqual(lower(keyed).statements, [{ kind: "fd", relation: "Account", projection: ["id"] }])
		// A closed relation's key stays engine-materialized; an explicit one
		// is still the duplicate it always was.
		assert.throws(function duplicateClosed() {
			// @ts-expect-error — a closed relation is not a key() target; the construction wall agrees
			key(Kind, ["id"])
		}, /closedness already materializes/)
	})

	test("duplicate statements are rejected via their canonical rendering", function probeDuplicate() {
		const { Kind, Holder, Account, SavingsTerms } = buildLedger()
		assert.throws(function duplicateStatement() {
			schema("Broken", { Kind, Holder, Account, SavingsTerms }, [
				key(Holder, ["id"]),
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
				key(SavingsTerms, ["account"]),
				mirrors(on(select(Account, { kind: "Savings" }), "id"), on(SavingsTerms, "account"))
			])
		}, /no declared containment resolves the closed reference/)
	})

	test("generated statements are checked semantically without an admission token", () => {
		const { Kind, Holder, Account, SavingsTerms } = buildLedger()
		const valid = { kind: "containment", source: on(Account, "holder"), target: on(Holder, "id") } as const
		assert.deepEqual(
			lower(schema("Generated", { Kind, Holder, Account, SavingsTerms }, [key(Holder, ["id"]), valid])),
			lower(
				schema("Built", { Kind, Holder, Account, SavingsTerms }, [
					key(Holder, ["id"]),
					contained(valid.source, valid.target)
				])
			)
		)
		assert.throws(
			() =>
				schema("WrongRoster", { Kind, Holder, Account, SavingsTerms }, [
					{ kind: "containment", source: on(Account, "holder"), target: on(Kind, "id") }
				]),
			/closedness rides the descriptor/
		)
		assert.throws(
			() => schema("Extra", { Holder }, [{ ...key(Holder, ["id"]), extra: true }]),
			/only the declared fields/
		)
	})
})

describe("ψ statements over closed relations — closed().where() as a face source", function describePsi() {
	test("a ψ-selected closed face renders canonically and schema() admits both forms", function probePsiCanonical() {
		const { psiContainment, psiCapacity, Mastery } = buildMastery()
		assert.equal(renderStatement(psiContainment), "Certificate(grade) <= Grade(id | mastered == true)")
		assert.equal(renderStatement(psiCapacity), "Grade(id | mastered == true) <={0..1} Certificate(grade)")
		assert.equal(Mastery.statements.length, 2)
	})

	test("ψ lowers PASS-THROUGH — the selection rides the SideSpec, never pre-folded into ids", function probePsiLowering() {
		const { Mastery } = buildMastery()
		const psiTarget = {
			relation: "Grade",
			projection: ["id"],
			selection: [["mastered", { kind: "one", literal: { kind: "value", value: { kind: "bool", value: true } } }]]
		}
		assert.deepStrictEqual(lower(Mastery).statements, [
			{
				kind: "containment",
				source: { relation: "Certificate", projection: ["grade"], selection: [] },
				target: psiTarget,
				bidirectional: false
			},
			{
				kind: "capacity",
				target: psiTarget,
				weight: { kind: "unit" },
				window: {
					kind: "range",
					lo: { kind: "lit", value: 0n },
					hi: { kind: "lit", value: 1n }
				},
				source: { relation: "Certificate", projection: ["grade"], selection: [] }
			}
		])
	})

	test("the closed where() speaks the ordinary selection vocabulary — literal sets and written order", function probePsiVocabulary() {
		const { Grade, Certificate } = buildMastery()
		assert.equal(
			renderStatement(contained(on(Certificate, "grade"), on(select(Grade, { score: [0n, 2n] }), "id"))),
			"Certificate(grade) <= Grade(id | score == {0, 2})"
		)
		assert.deepStrictEqual(
			select(Grade, { score: 2n, mastered: true }).selection.map(function fieldOf(binding) {
				return binding.field
			}),
			["score", "mastered"]
		)
	})

	test("the empty ψ is the bare closed relation respelled and rejected (canonical utterance)", function probeEmptyPsi() {
		const { Grade } = buildMastery()
		assert.throws(function emptySelection() {
			select(Grade, {})
		}, /an empty selection is the bare relation respelled/)
	})

	test("the compile walls carry runtime twins through the one selection machine", function probePsiRuntimeTwins() {
		const { Grade } = buildMastery()
		assert.throws(function unknownColumn() {
			// @ts-expect-error — Grade has no column `nope` (the runtime twin of the compile wall)
			select(Grade, { nope: true })
		}, /relation Grade has no field nope/)
		assert.throws(function idExcluded() {
			// @ts-expect-error — synthetic ids use the roster handle, never its encoded ordinal
			select(Grade, { id: 0n })
		}, /a Grade handle/)
		assert.throws(function wrongLiteral() {
			// @ts-expect-error — mastered is a bool column: a bigint literal is out of shape
			select(Grade, { mastered: 1n })
		}, /expected boolean/)
	})

	test("a handle named `where` is ordinary roster data — NO name is reserved, both tiers", function probeNoReservedNames() {
		const bare = closed("Fine", ["where"])
		assert.deepEqual(bare.handles, ["where"])
		const payload = closed("AlsoFine", ["where"], { pages: bool }, { where: { pages: true } })
		assert.deepEqual(payload.handles, ["where"])
		assert.equal(payload.axioms.where.pages, true)
		const selected = select(payload, { pages: true })
		assert.deepEqual(selected.relation, payload)
		assert.ok(Object.isFrozen(selected.relation), "immutable declarations may be shared")
	})
})

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

function facesArePairedStructurally(): unknown[] {
	const { Kind, Holder, Account } = buildLedger()
	const { Booking, Slot } = buildCalendar()
	const Vault = relation("Vault", { tag: bytes(32), stamp: bytes(16) })
	return [
		contained(on(Account, "holder"), on(Holder, "id")),
		contained(on(Slot, ["room", "during"]), on(Booking, ["room", "during"])),
		contained(on(Account, "kind"), on(Kind, "id")),
		// @ts-expect-error — a u64 face never pairs a str face (kind mismatch)
		contained(on(Holder, "name"), on(Account, "holder")),
		// @ts-expect-error — bytes(32) never pairs bytes(16) (width mismatch)
		contained(on(Vault, "tag"), on(Vault, "stamp")),
		// @ts-expect-error — interval(i64) never pairs interval(u64) (element mismatch)
		contained(on(Account, "active"), on(Booking, "during")),
		// @ts-expect-error — composite positions compare positionwise: [u64, interval] vs [interval, u64]
		contained(on(Slot, ["room", "during"]), on(Booking, ["during", "room"])),
		// @ts-expect-error — a mirrors bijection pairs structure exactly as containment (u64 vs interval)
		mirrors(on(Account, "id"), on(Account, "active")),
		// @ts-expect-error — a capacity statement's grouping join pairs structure exactly as containment (u64 vs interval)
		capacity(on(Holder, "id"), { from: on(Account, "active"), within: within(0n, 3n) }),
		// @ts-expect-error — arity mismatch: positional pairing requires equally many fields
		contained(on(Slot, ["room", "during"]), on(Booking, "room"))
	]
}

function closedPayloadColumnsPairStructurally(): unknown[] {
	const { Sev, Limit } = buildSeverity()
	const { Holder, Account } = buildLedger()
	const Alert = relation("Alert", { sev: closedId(Sev) })
	return [
		contained(on(Sev, "level"), on(Limit, "level")),
		contained(on(Limit, "level"), on(Sev, "level")),

		contained(on(Alert, "sev"), on(Sev, "id")),
		// @ts-expect-error — a plain u64 never pairs a closed [id]: closedness rides the descriptor (the roster slot)
		contained(on(Limit, "cap"), on(Sev, "id")),
		// @ts-expect-error — a payload column pairs by structure: u64 never pairs str
		contained(on(Sev, "level"), on(Holder, "name")),
		// @ts-expect-error — the synthetic id is a u64: it never pairs an interval face
		contained(on(Sev, "id"), on(Account, "active"))
	]
}

function selectionsAreTyped(): unknown[] {
	const { Account } = buildLedger()
	return [
		select(Account, { kind: "Savings" }),
		select(Account, { kind: ["Checking", "Savings"] }),
		// @ts-expect-error — "Nope" is not a handle of Kind's vocabulary (the union refuses)
		select(Account, { kind: "Nope" }),
		// @ts-expect-error — a closed reference selects by handle name, never by raw id: bigint left the closed surface
		select(Account, { kind: 1n }),
		// @ts-expect-error — Account has no field `nope` to select on
		select(Account, { nope: 1n })
	]
}

function closedSelectionsAreTyped(): unknown[] {
	const { Kind } = buildLedger()
	const { Grade } = buildMastery()
	return [
		select(Grade, { mastered: true }),
		select(Grade, { score: [0n, 2n] }),
		select(Kind, { id: "Checking" }),
		// @ts-expect-error — Grade has no column `nope`
		select(Grade, { nope: true }),
		// @ts-expect-error — mastered is a bool column: a bigint literal is out of shape
		select(Grade, { mastered: 1n }),
		// @ts-expect-error — a synthetic id selects by named handle, never by ordinal
		select(Grade, { id: 0n })
	]
}

function psiFacesArePairedStructurally(): unknown[] {
	const { Grade, Certificate } = buildMastery()
	return [
		contained(on(Certificate, "grade"), on(select(Grade, { mastered: true }), "id")),
		capacity(on(select(Grade, { mastered: true }), "id"), { from: on(Certificate, "grade"), within: within(0n, 1n) }),
		contained(on(select(Grade, { mastered: true }), "score"), on(Certificate, "id")),
		// @ts-expect-error — a ψ face's projected shapes still hold the wall: bool never pairs u64
		contained(on(select(Grade, { score: 2n }), "mastered"), on(Certificate, "grade")),
		// @ts-expect-error — an unknown field is not projectable through a ψ-selected closed source
		on(select(Grade, { mastered: true }), "nope")
	]
}

export {
	banTableIsUnwritable,
	capacityWallsAreTyped,
	closedPayloadColumnsPairStructurally,
	closedSelectionsAreTyped,
	equivalentSpellingsLowerToTheCanonicalLaw,
	facesArePairedStructurally,
	fieldReferencesAreTypeChecked,
	psiFacesArePairedStructurally,
	selectionsAreTyped,
	weightedFloorOneCompiles
}
