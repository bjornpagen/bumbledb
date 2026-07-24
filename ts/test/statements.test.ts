/**
 * Statement-algebra pins on the MINIMAL kernel (K3) under the LAW-TYPING
 * (K4): the full Ledger example (key, containment, selected `==`, window)
 * lowers to its `SchemaSpec` shape — every `newtype` slot carrying the
 * class name `schema()` COMPUTED from the statement list (the laws type
 * the columns; bare fields lower `undefined`); the canonical-utterance ban
 * table is enumerated one row at a time (each banned LITERAL spelling a
 * REAL `@ts-expect-error` — unwritable — and each computed-bound escape a
 * construction error naming the canonical form); field references are
 * checked in the type — existence AND structural shape (positionwise
 * kind/width/element, read off the schema type; the DOMAIN wall lives at
 * `schema()` — the one-generator-per-class law, pinned in
 * `law-typing.test.ts` — and at query joins, never at face construction);
 * `schema()` enforces its expansion-boundary checks including the
 * handle-selection paste-back law; ψ-selection over closed relations
 * (`Grade.where({ mastered: true })` as a face source) is typed, rendered,
 * and lowered PASS-THROUGH (the engine folds at validate); and
 * `renderStatement` emits the canonical `70-api.md` spellings exactly.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"

import { closed } from "#closed.ts"
import * as countModule from "#count.ts"
import { atLeast, atMost, between, exactly, none } from "#count.ts"
import { on } from "#face.ts"
import { bool, bytes, i64, interval, span, str, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained, key, mirrors, renderStatement, window } from "#statements.ts"

function buildLedger() {
	const Kind = closed("Kind", ["Checking", "Savings"])
	const Holder = relation("Holder", { id: u64.fresh, name: str })
	const Account = relation("Account", {
		id: u64.fresh,
		holder: u64,
		kind: Kind.id,
		active: interval(i64)
	})
	const SavingsTerms = relation("SavingsTerms", { account: u64 })
	const statements = [
		key(SavingsTerms, ["account"]),
		contained(on(Account, "holder"), on(Holder, "id")),
		contained(on(Account, "kind"), on(Kind, "id")),
		mirrors(on(Account.where({ kind: "Savings" }), "id"), on(SavingsTerms, "account")),
		window(on(Holder, "id"), atMost(3n), on(Account, "holder"))
	]
	const Ledger = schema("Ledger", { Kind, Holder, Account, SavingsTerms }, statements)
	return { Kind, Holder, Account, SavingsTerms, statements, Ledger }
}

/** The composite/pointwise fixtures: `on(R, ["a", "b"])` positions and the composite key. */
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

/**
 * The ψ fixtures: a payload-tier closed vocabulary selected by its own
 * columns (`Grade.where({ mastered: true })`) as a face source — Hole A of
 * ψ-selection closed. The selection is lowered AS-IS (pass-through — the
 * engine folds against the sealed extension at validate), so the two ψ
 * statement forms here are the exact shapes the macro's
 * `Grade(id | mastered == true)` lowers to.
 */
function buildMastery() {
	const Grade = closed(
		"Grade",
		{ mastered: bool, score: u64 },
		{
			Failed: { mastered: false, score: 0n },
			DirectPass: { mastered: true, score: 2n }
		}
	)
	const Certificate = relation("Certificate", { id: u64.fresh, grade: Grade.id })
	const psiContainment = contained(on(Certificate, "grade"), on(Grade.where({ mastered: true }), "id"))
	const psiWindow = window(on(Grade.where({ mastered: true }), "id"), atMost(1n), on(Certificate, "grade"))
	const Mastery = schema("Mastery", { Grade, Certificate }, [psiContainment, psiWindow])
	return { Grade, Certificate, psiContainment, psiWindow, Mastery }
}

/**
 * The closed-payload fixtures: a payload column facing a relation field of
 * the SAME structure — the pairing the typed `columns` carrier exists to
 * admit (its domain, if any, is law-born at `schema()`, never declared).
 */
function buildSeverity() {
	const Sev = closed(
		"Sev",
		{ level: u64 },
		{
			Info: { level: 1n },
			Critical: { level: 5n }
		}
	)
	const Limit = relation("Limit", { level: u64, cap: u64 })
	const statements = [contained(on(Sev, "level"), on(Limit, "level"))]
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
						{ name: "id", valueType: { kind: "u64" }, newtype: "Holder.id", fresh: true },
						{ name: "name", valueType: { kind: "string" }, newtype: undefined, fresh: false }
					],
					closed: undefined
				},
				{
					name: "Account",
					fields: [
						{ name: "id", valueType: { kind: "u64" }, newtype: "Account.id", fresh: true },
						{ name: "holder", valueType: { kind: "u64" }, newtype: "Holder.id", fresh: false },
						{ name: "kind", valueType: { kind: "u64" }, newtype: "Kind.id", fresh: false },
						{
							name: "active",
							valueType: { kind: "interval", element: "i64", width: undefined },
							newtype: undefined,
							fresh: false
						}
					],
					closed: undefined
				},
				{
					name: "SavingsTerms",
					fields: [{ name: "account", valueType: { kind: "u64" }, newtype: "Account.id", fresh: false }],
					closed: undefined
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

	test("a closed payload column lowers pure structure — the newtype slots carry its law-computed classes", function probeClosedPayloadLowering() {
		const { Severity } = buildSeverity()
		assert.deepStrictEqual(lower(Severity), {
			relations: [
				{
					name: "Sev",
					fields: [{ name: "level", valueType: { kind: "u64" }, newtype: "Sev.level", fresh: false }],
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
						{ name: "level", valueType: { kind: "u64" }, newtype: "Sev.level", fresh: false },
						{ name: "cap", valueType: { kind: "u64" }, newtype: undefined, fresh: false }
					],
					closed: undefined
				}
			],
			statements: [
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
		const { Account, SavingsTerms } = buildLedger()
		const setFace = on(Account.where({ kind: ["Checking", "Savings"] }), "id")
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

	test("degenerate literal sets refuse — a membership array needs two DISTINCT members, and the refusal locates itself", function probeDegenerateSet() {
		const { Account } = buildLedger()
		// Every refusal names the relation and field (`relation Account.kind:`)
		// — self-locating, the same texture as the query tier's membershipSet.
		assert.throws(function emptySet() {
			Account.where({ kind: [] })
		}, /relation Account\.kind: an empty literal set selects nothing/)
		assert.throws(function oneElementSet() {
			Account.where({ kind: ["Checking"] })
		}, /relation Account\.kind: a one-element literal set is the bare literal respelled/)
		// A duplicate member is the banned one-element set respelled — refused
		// HERE with the canonical-utterance voice, so the engine's index-speak
		// duplicate error at Db.create is unreachable from this surface.
		assert.throws(function duplicateMember() {
			Account.where({ kind: ["Checking", "Checking"] })
		}, /relation Account\.kind: the literal set spells Checking twice — write it once/)
		// The ordinary-field twin, same voice (the one selection machine).
		assert.throws(function duplicateOrdinary() {
			const { Holder } = buildLedger()
			Holder.where({ name: ["a", "b", "a"] })
		}, /relation Holder\.name: the literal set spells "a" twice — write it once/)
	})

	test("a plain u64 face never pairs a closed [id] face — closedness rides the descriptor (both tiers)", function probeRosterWall() {
		const { Sev, Limit } = buildSeverity()
		// The alias spelling — a bare u64 column into the vocabulary's [id] —
		// dies at statement construction: the vocabulary's own descriptor
		// (`Sev.id`) is the ONE spelling of a closed reference, so every
		// descriptor-keyed closed judgment (the orderable ban, the name↔id
		// marshal, answer decode) stays sound. The directives are real: the
		// roster is the fourth slot of the face shape.
		assert.throws(function aliasContainment() {
			// @ts-expect-error — a plain u64 column cannot alias a closed vocabulary through a containment
			contained(on(Limit, "cap"), on(Sev, "id"))
		}, /Limit\.cap is a bare column but Sev\.id is a Sev reference — closedness rides the descriptor/)
		assert.throws(function aliasReversed() {
			// @ts-expect-error — the reverse orientation is the same wall (pairing is symmetric)
			mirrors(on(Sev, "id"), on(Limit, "cap"))
		}, /Sev\.id is a Sev reference but Limit\.cap is a bare column/)
		assert.throws(function aliasWindow() {
			// @ts-expect-error — a window's grouping join holds the roster wall exactly as containment
			window(on(Sev, "id"), atMost(1n), on(Limit, "cap"))
		}, /Limit\.cap is a bare column but Sev\.id is a Sev reference/)
		// The one spelling still constructs and renders canonically.
		const Alert = relation("Alert", { sev: Sev.id })
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
		assert.throws(function truncatedWindow() {
			// @ts-expect-error — a window's grouping join holds the arity wall exactly as containment
			window(on(Slot, "room"), atMost(1n), on(Booking, ["room", "during"]))
		}, /Booking\(room, during\) and Slot\(room\) project 2 vs 1 fields/)
		// The paired spelling still constructs.
		assert.equal(
			renderStatement(contained(on(Slot, ["room", "during"]), on(Booking, ["room", "during"]))),
			"Slot(room, during) <= Booking(room, during)"
		)
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
		}, /`\{0\}` is the exclusion — write none/)
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
		const impostor = relation("Holder", { id: u64.fresh })
		const declared = relation("Holder", { id: u64.fresh })
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
				mirrors(on(Account.where({ kind: "Savings" }), "id"), on(SavingsTerms, "account"))
			])
		}, /no declared containment resolves the closed reference/)
	})

	test("a forged structural statement is refused at BOTH tiers — the admission brand (062)", function probeForgery() {
		const { Kind, Holder, Account, SavingsTerms } = buildLedger()
		// A statement pairing a bare u64 with the closed id — the exact shape the
		// roster wall exists to refuse, spelled structurally to skip the constructors.
		const forgedData = {
			kind: "containment" as const,
			source: on(Account, "holder").data,
			target: on(Kind, "id").data,
			bidirectional: false
		}
		assert.throws(function forgedIntoSchema() {
			schema("Forge", { Kind, Holder, Account, SavingsTerms }, [
				// @ts-expect-error — 062: Statement carries the module-private admission brand, so a structural literal is not a Statement
				{ data: forgedData }
			])
		}, /a statement is minted only by key\/contained\/mirrors\/window/)
	})
})

describe("ψ statements over closed relations — closed().where() as a face source", function describePsi() {
	test("a ψ-selected closed face renders canonically and schema() admits both forms", function probePsiCanonical() {
		const { psiContainment, psiWindow, Mastery } = buildMastery()
		assert.equal(renderStatement(psiContainment), "Certificate(grade) <= Grade(id | mastered == true)")
		assert.equal(renderStatement(psiWindow), "Grade(id | mastered == true) <={0..1} Certificate(grade)")
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
				kind: "cardinality",
				target: psiTarget,
				window: { kind: "range", lo: 0n, hi: 1n },
				source: { relation: "Certificate", projection: ["grade"], selection: [] }
			}
		])
	})

	test("the closed where() speaks the ordinary selection vocabulary — literal sets and written order", function probePsiVocabulary() {
		const { Grade, Certificate } = buildMastery()
		assert.equal(
			renderStatement(contained(on(Certificate, "grade"), on(Grade.where({ score: [0n, 2n] }), "id"))),
			"Certificate(grade) <= Grade(id | score == {0, 2})"
		)
		assert.deepStrictEqual(
			Grade.where({ score: 2n, mastered: true }).selection.map(function fieldOf(binding) {
				return binding.field
			}),
			["score", "mastered"]
		)
	})

	test("the empty ψ is the bare closed relation respelled and rejected (canonical utterance)", function probeEmptyPsi() {
		const { Grade } = buildMastery()
		assert.throws(function emptySelection() {
			Grade.where({})
		}, /an empty selection is the bare relation respelled/)
	})

	test("the compile walls carry runtime twins through the one selection machine", function probePsiRuntimeTwins() {
		const { Grade } = buildMastery()
		assert.throws(function unknownColumn() {
			// @ts-expect-error — Grade has no column `nope` (the runtime twin of the compile wall)
			Grade.where({ nope: true })
		}, /relation Grade has no field nope/)
		assert.throws(function idExcluded() {
			// @ts-expect-error — the synthetic id is not selectable through where() (handle literals on the referencing side are the spelling)
			Grade.where({ id: 0n })
		}, /relation Grade has no field id/)
		assert.throws(function wrongLiteral() {
			// @ts-expect-error — mastered is a bool column: a bigint literal is out of shape
			Grade.where({ mastered: 1n })
		}, /expected boolean/)
	})

	test("a handle named `where` is ordinary roster data — NO name is reserved, both tiers", function probeNoReservedNames() {
		/**
		 * H5: handles are pure DATA, never properties of the value — the
		 * axioms record and the roster are their own namespaces, so a
		 * vocabulary may legally contain handles named like the value's own
		 * methods, and the payload tier's ψ surface is untouched by them.
		 */
		const bare = closed("Fine", ["where"])
		assert.deepEqual(bare.data.handles, ["where"])
		const payload = closed("AlsoFine", { pages: bool }, { where: { pages: true } })
		assert.deepEqual(payload.data.handles, ["where"])
		assert.equal(payload.axioms.where.pages, true)
		const selected = payload.where({ pages: true })
		assert.equal(selected.relation, payload, "the ψ surface is the value's own method, untouched by roster data")
	})
})

// ————————————————————————————————————————————————————————————————————————
// The construction compile probes: field references are checked in the
// TYPE — existence (names autocomplete, unknown field = type error) and
// STRUCTURAL compatibility (positionwise kind/width/element, read off the
// schema type). The old cross-DOMAIN construction probes are gone from
// here BY DESIGN: at construction there is no domain to compare — the
// laws are self-defining, and the domain wall is re-homed at the schema
// level (K4's one-generator-per-class check) and at query joins. Each
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

/** Structurally mismatched pairs are compile errors on every relating constructor. */
function facesArePairedStructurally(): unknown[] {
	const { Kind, Holder, Account } = buildLedger()
	const { Booking, Slot } = buildCalendar()
	const Vault = relation("Vault", { tag: bytes(32), stamp: bytes(16) })
	return [
		// the legal pairs compile — positionwise-equal kind/width/element
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
		// @ts-expect-error — a window's grouping join pairs structure exactly as containment (u64 vs interval)
		window(on(Holder, "id"), atMost(3n), on(Account, "active")),
		// @ts-expect-error — arity mismatch: positional pairing requires equally many fields
		contained(on(Slot, ["room", "during"]), on(Booking, "room"))
	]
}

/**
 * A closed relation's payload columns pair by their declared descriptors'
 * STRUCTURE through the typed `columns` carrier (whose runtime twin is the
 * frozen `columns` record the mint carries), exactly as an ordinary
 * relation's fields do; the synthetic `id` is a u64 CARRYING ITS ROSTER —
 * it pairs only a column spelled with the vocabulary's own descriptor
 * (the roster slot of the face shape; a plain u64 cannot alias it).
 */
function closedPayloadColumnsPairStructurally(): unknown[] {
	const { Sev, Limit } = buildSeverity()
	const { Holder, Account } = buildLedger()
	const Alert = relation("Alert", { sev: Sev.id })
	return [
		// the legal pairs compile — u64 against u64, whichever side is closed
		contained(on(Sev, "level"), on(Limit, "level")),
		contained(on(Limit, "level"), on(Sev, "level")),
		// the closed [id] pairs the vocabulary's OWN descriptor — the one spelling
		contained(on(Alert, "sev"), on(Sev, "id")),
		// @ts-expect-error — a plain u64 never pairs a closed [id]: closedness rides the descriptor (the roster slot)
		contained(on(Limit, "cap"), on(Sev, "id")),
		// @ts-expect-error — a payload column pairs by structure: u64 never pairs str
		contained(on(Sev, "level"), on(Holder, "name")),
		// @ts-expect-error — the synthetic id is a u64: it never pairs an interval face
		contained(on(Sev, "id"), on(Account, "active"))
	]
}

/** `where()` selections are typed: a closed reference selects by handle NAME (the precise union). */
function selectionsAreTyped(): unknown[] {
	const { Account } = buildLedger()
	return [
		Account.where({ kind: "Savings" }),
		Account.where({ kind: ["Checking", "Savings"] }),
		// @ts-expect-error — "Nope" is not a handle of Kind's vocabulary (the union refuses)
		Account.where({ kind: "Nope" }),
		// @ts-expect-error — a closed reference selects by handle name, never by raw id: bigint left the closed surface
		Account.where({ kind: 1n }),
		// @ts-expect-error — Account has no field `nope` to select on
		Account.where({ nope: 1n })
	]
}

/**
 * The closed `where()` is typed exactly like the ordinary one — payload
 * columns only (the synthetic `id` is excluded: an id selection is spelled
 * only as handle literals on the referencing side), the same literal/one-of
 * vocabulary — and it exists ONLY on the payload tier: the bare tier has no
 * payload, so `.where` is a type-level ABSENCE there, not an uncallable
 * method.
 */
function closedSelectionsAreTyped(): unknown[] {
	const { Kind } = buildLedger()
	const { Grade } = buildMastery()
	return [
		Grade.where({ mastered: true }),
		Grade.where({ score: [0n, 2n] }),
		// @ts-expect-error — the bare tier has no payload columns: `.where` does not exist there
		Kind.where({}),
		// @ts-expect-error — Grade has no column `nope`
		Grade.where({ nope: true }),
		// @ts-expect-error — mastered is a bool column: a bigint literal is out of shape
		Grade.where({ mastered: 1n }),
		// @ts-expect-error — the synthetic id is not selectable through where(): spell handle literals on the referencing side
		Grade.where({ id: 0n })
	]
}

/**
 * A ψ-selected closed face pairs by STRUCTURE exactly as a bare closed
 * face — the selection changes nothing about the projected shapes: the
 * synthetic `id` contributes the u64 triple, payload columns their
 * declared descriptors' triples through the typed `columns` carrier.
 */
function psiFacesArePairedStructurally(): unknown[] {
	const { Grade, Certificate } = buildMastery()
	return [
		// the legal pairs compile — same-label ψ pairing and the ψ window target
		contained(on(Certificate, "grade"), on(Grade.where({ mastered: true }), "id")),
		window(on(Grade.where({ mastered: true }), "id"), atMost(1n), on(Certificate, "grade")),
		contained(on(Grade.where({ mastered: true }), "score"), on(Certificate, "id")),
		// @ts-expect-error — a ψ face's projected shapes still hold the wall: bool never pairs u64
		contained(on(Grade.where({ score: 2n }), "mastered"), on(Certificate, "grade")),
		// @ts-expect-error — an unknown field is not projectable through a ψ-selected closed source
		on(Grade.where({ mastered: true }), "nope")
	]
}

export {
	banTableIsUnwritable,
	closedPayloadColumnsPairStructurally,
	closedSelectionsAreTyped,
	facesArePairedStructurally,
	fieldReferencesAreTypeChecked,
	psiFacesArePairedStructurally,
	selectionsAreTyped
}
