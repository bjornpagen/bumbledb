/**
 * The TS-render ⇄ manifest-render golden (TODO.md §7 item 3a): for every
 * statement construct the schema surface can utter — the FD key form
 * (scalar, composite pointwise-interval), containment (plain, σ-selected,
 * ψ-selected, closed-target, multi-field pointwise), the `==` bijection,
 * every legal capacity spelling (`{n}`, `{0}`, `{lo..hi}`, `{lo..*}`,
 * `{0..hi}`, the weighted bracket, the dependent bound, the Duration
 * pair), and the sub-vocabulary handle-set selection — the SDK's
 * `renderStatement` output equals, byte for byte, the engine-rendered
 * spelling the manifest ships for the same store
 * (`schema/render.rs::render_declared` via `dbManifest`). The engine's
 * materialized order is mirrored positionally (fresh auto-keys, closed
 * auto-keys, then declared statements, a `mirrors` occupying two slots), so
 * the golden also pins the implied-key spellings the SDK never utters.
 * Selection literals lean adversarial on purpose: the `char::escape_debug`
 * and `u8::escape_ascii` mirrors are exactly where the two renderers could
 * drift silently. The ψ-on-closed golden (PRD-K1) runs against its own
 * store: `Grade.where({ mastered: true })` as a statement face — created,
 * manifest-pinned, folded by the ENGINE at validate, and rejected with the
 * violation's canonical string equal to the manifest spelling byte for
 * byte.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import { duration, ref, weigh, within } from "#capacity.ts"
import { closed } from "#closed.ts"
import { Db } from "#db.ts"
import { on } from "#face.ts"
import { bool, bytes, i64, interval, span, str, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import type { DbHandle, Manifest, StatementKindTag } from "#native.ts"
import { native } from "#native.ts"
import { relation } from "#relation.ts"
import { type AnySchema, schema } from "#schema.ts"
import { capacity, contained, key, mirrors, renderStatement, type Statement } from "#statements.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-render-golden-"))
const storeDir = path.join(tmpRoot, "store")
const psiStoreDir = path.join(tmpRoot, "psi-store")
const psiDbDir = path.join(tmpRoot, "psi-db-store")

const Status = closed("Status", ["Active", "Frozen"])
const Kind = closed("Kind", ["Checking", "Savings", "Brokerage"])
const Holder = relation("Holder", { id: u64.fresh, name: str })
const Account = relation("Account", {
	id: u64.fresh,
	holder: u64,
	kind: Kind.id,
	status: Status.id,
	score: i64,
	weight: u64,
	label: str,
	flag: bool,
	tag: bytes(4),
	active: interval(i64)
})
const Booking = relation("Booking", { room: u64, during: interval(u64) })
const Slot = relation("Slot", { room: u64, during: interval(u64) })
const SavingsTerms = relation("SavingsTerms", { account: u64 })

/**
 * The escape gauntlet: a single quote, escaped double quotes, a tab, a
 * newline, a literal backslash, and a combining acute (grapheme-extending,
 * printable — `char::escape_debug` escapes it as `\u{301}` anyway).
 */
const gauntletLabel = 'it\'s "w\teird"\n\\e\u0301'

/** Bytes across `u8::escape_ascii`'s arms: printable, NUL, high bit, quote. */
const gauntletTag = new Uint8Array([0x62, 0x00, 0xff, 0x22])

/**
 * Every construct once, declaration order = materialized order (after the
 * implied keys). The closed-target containments double as the renderer's
 * handle-resolution walk: `schema/render.rs` prints a selection word as its
 * handle only when a declared containment names the field's closed target.
 */
const plainKey = key(SavingsTerms, ["account"])
const pointwiseKey = key(Booking, ["room", "during"])
const plainContainment = contained(on(Account, "holder"), on(Holder, "id"))
const kindClosedTarget = contained(on(Account, "kind"), on(Kind, "id"))
const statusClosedTarget = contained(on(Account, "status"), on(Status, "id"))
const pointwiseContainment = contained(on(Slot, ["room", "during"]), on(Booking, ["room", "during"]))
const sigmaContainment = contained(on(Account.where({ kind: "Savings" }), "holder"), on(Holder, "id"))
const psiContainment = contained(on(SavingsTerms, "account"), on(Account.where({ status: "Active" }), "id"))
const bijection = mirrors(on(Account.where({ kind: "Savings" }), "id"), on(SavingsTerms, "account"))
const ceilingCapacity = capacity(on(Holder, "id"), within(0n, 3n), on(Account, "holder"))
const exclusionCapacity = capacity(on(Holder, "id"), within(0n), on(Account.where({ flag: false }), "holder"))
const exactCapacity = capacity(on(Holder, "id"), within(2n), on(Account.where({ label: gauntletLabel }), "holder"))
const rangeCapacity = capacity(on(Holder, "id"), within(1n, 4n), on(Account.where({ score: -42n }), "holder"))
const floorCapacity = capacity(
	on(Holder, "id"),
	within(2n, "*"),
	on(Account.where({ kind: ["Checking", "Savings"] }), "holder")
)
const psiTargetCapacity = capacity(on(Account.where({ flag: true }), "id"), within(0n, 1n), on(SavingsTerms, "account"))
const literalGauntletCapacity = capacity(
	on(Holder, "id"),
	within(0n, 5n),
	on(Account.where({ weight: 7n, tag: gauntletTag, active: span(-3n, 9n) }), "holder")
)
/** The weighted rows (dossier § 4.4): the dependent bound and the calendar shape against the engine arm. */
const Pool = relation("Pool", { id: u64.fresh, supply: u64 })
const Device = relation("Device", { id: u64.fresh, pool: u64, watts: u64 })
const Hall = relation("Hall", { id: u64.fresh, span: interval(i64) })
const Session = relation("Session", { id: u64.fresh, hall: u64, booked: interval(i64) })
const devicePoolContainment = contained(on(Device, "pool"), on(Pool, "id"))
const sessionHallContainment = contained(on(Session, "hall"), on(Hall, "id"))
const powerBudget = capacity(on(Pool, "id"), weigh("watts"), within(0n, ref("supply")), on(Device, "pool"))
const weightedFloor = capacity(on(Pool, "id"), weigh("watts"), within(1n, "*"), on(Device, "pool"))
const calendarCapacity = capacity(
	on(Hall, "id"),
	weigh(duration("booked")),
	within(0n, duration("span")),
	on(Session, "hall")
)

const statements = [
	plainKey,
	pointwiseKey,
	plainContainment,
	kindClosedTarget,
	statusClosedTarget,
	pointwiseContainment,
	sigmaContainment,
	psiContainment,
	bijection,
	ceilingCapacity,
	exclusionCapacity,
	exactCapacity,
	rangeCapacity,
	floorCapacity,
	psiTargetCapacity,
	literalGauntletCapacity,
	devicePoolContainment,
	sessionHallContainment,
	powerBudget,
	weightedFloor,
	calendarCapacity
]

const Golden = schema(
	"Golden",
	{ Status, Kind, Holder, Account, Booking, Slot, SavingsTerms, Pool, Device, Hall, Session },
	statements
)

/**
 * The ψ-on-closed fixtures (PRD-K1): a payload-tier closed vocabulary
 * ψ-selected by its own column as a statement face — one containment and
 * one capacity law over `Grade(id | mastered == true)`, and NO handle literal
 * anywhere (a handle literal resolves through the law-computed newtype,
 * which lands in K4; the ψ selection itself is a plain bool literal and
 * resolves against the sealed columns today).
 */
const Grade = closed(
	"Grade",
	{ mastered: bool },
	{
		Failed: { mastered: false },
		DirectPass: { mastered: true }
	}
)
const Certificate = relation("Certificate", { id: u64.fresh, grade: Grade.id })
const closedPsiContainment = contained(on(Certificate, "grade"), on(Grade.where({ mastered: true }), "id"))
const closedPsiCapacity = capacity(on(Grade.where({ mastered: true }), "id"), within(0n, 1n), on(Certificate, "grade"))
const Mastery = schema("Mastery", { Grade, Certificate }, [closedPsiContainment, closedPsiCapacity])

/** One expected materialized slot: the form tag and the canonical spelling. */
interface Slot {
	readonly kind: StatementKindTag
	readonly spelling: string
}

/** The SDK statement form tags mapped onto the manifest's vocabulary. */
function kindTag(statement: Statement): StatementKindTag {
	switch (statement.data.kind) {
		case "key":
			return "functionality"
		case "containment":
		case "mirrors":
			return "containment"
		case "capacity":
			return "capacity"
	}
}

/**
 * The expected manifest, slot by slot — the SDK's positional mirror of
 * `SchemaDescriptor::materialized_statements`: one fresh auto-key per
 * minted field (relation declaration order, then field order), one closed
 * auto-key per closed relation (declaration order), then the declared
 * statements, a `mirrors` filling two adjacent slots with the one `==`
 * spelling (the engine renders both partners identically, in the written
 * orientation). Implied-key spellings are spelled by hand here — the SDK
 * has no statement value for them, and the key form's canonical shape is
 * exactly this string.
 */
function expectedSlots(theory: AnySchema): Slot[] {
	const slots: Slot[] = []
	for (const member of Object.values(theory.relations)) {
		if ("handles" in member.data) {
			continue
		}
		for (const declared of member.data.fields) {
			if ("fresh" in declared.field && declared.field.fresh === true) {
				slots.push({
					kind: "functionality",
					spelling: `${member.name}(${declared.name}) -> ${member.name}`
				})
			}
		}
	}
	for (const member of Object.values(theory.relations)) {
		if ("handles" in member.data) {
			slots.push({ kind: "functionality", spelling: `${member.name}(id) -> ${member.name}` })
		}
	}
	for (const statement of theory.statements) {
		const slot: Slot = { kind: kindTag(statement), spelling: renderStatement(statement) }
		slots.push(slot)
		if (statement.data.kind === "mirrors") {
			slots.push(slot)
		}
	}
	return slots
}

let db: DbHandle | undefined
let psiDb: DbHandle | undefined

after(function cleanup() {
	if (db !== undefined) {
		native.dbClose(db)
	}
	if (psiDb !== undefined) {
		native.dbClose(psiDb)
	}
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

describe("the TS-render ⇄ manifest-render golden", function suite() {
	test("the exotic spellings are pinned as literals, pre-open", function pins() {
		assert.equal(renderStatement(pointwiseKey), "Booking(room, during) -> Booking")
		assert.equal(renderStatement(pointwiseContainment), "Slot(room, during) <= Booking(room, during)")
		assert.equal(renderStatement(psiContainment), "SavingsTerms(account) <= Account(id | status == Active)")
		assert.equal(renderStatement(bijection), "Account(id | kind == Savings) == SavingsTerms(account)")
		assert.equal(renderStatement(ceilingCapacity), "Holder(id) <={0..3} Account(holder)")
		assert.equal(renderStatement(exclusionCapacity), "Holder(id) <={0} Account(holder | flag == false)")
		assert.equal(
			renderStatement(exactCapacity),
			'Holder(id) <={2} Account(holder | label == "it\\\'s \\"w\\teird\\"\\n\\\\e\\u{301}")'
		)
		assert.equal(renderStatement(rangeCapacity), "Holder(id) <={1..4} Account(holder | score == -42)")
		assert.equal(renderStatement(floorCapacity), "Holder(id) <={2..*} Account(holder | kind == {Checking, Savings})")
		assert.equal(renderStatement(psiTargetCapacity), "Account(id | flag == true) <={0..1} SavingsTerms(account)")
		assert.equal(
			renderStatement(literalGauntletCapacity),
			'Holder(id) <={0..5} Account(holder | weight == 7, tag == b"b\\x00\\xff\\"", active == -3..9)'
		)
		assert.equal(renderStatement(powerBudget), "Pool(id) <=[watts]{0..supply} Device(pool)")
		assert.equal(renderStatement(weightedFloor), "Pool(id) <=[watts]{1..*} Device(pool)")
		assert.equal(renderStatement(calendarCapacity), "Hall(id) <=[Duration(booked)]{0..Duration(span)} Session(hall)")
	})

	test("every materialized slot's manifest spelling equals the SDK render", function golden() {
		const created = native.dbCreate(storeDir, lower(Golden))
		if (!created.ok) {
			assert.fail(`dbCreate refused the golden theory (${created.kind}): ${created.message}`)
		}
		db = created.db
		const manifest: Manifest = native.dbManifest(db)

		const expected = expectedSlots(Golden)
		assert.equal(
			manifest.statements.length,
			expected.length,
			"the SDK mirror and the engine agree on the materialized statement count"
		)
		manifest.statements.forEach(function verifySlot(statement, index) {
			const slot = expected[index]
			assert.ok(slot, `expected slot ${index} exists`)
			assert.equal(statement.id, index, "statement ids are the materialized indices")
			assert.equal(statement.kind, slot.kind, `slot ${index} form tag`)
			assert.equal(
				statement.spelling,
				slot.spelling,
				`slot ${index}: the manifest's engine spelling equals the SDK's canonical render`
			)
		})
	})
})

describe("the ψ-on-closed golden: manifest spelling, engine folding, violation paste-back", function psiSuite() {
	test("every Mastery slot's manifest spelling equals the SDK render", function psiGolden() {
		const created = native.dbCreate(psiStoreDir, lower(Mastery))
		if (!created.ok) {
			assert.fail(`dbCreate refused the Mastery theory (${created.kind}): ${created.message}`)
		}
		psiDb = created.db
		const manifest: Manifest = native.dbManifest(psiDb)
		const expected = expectedSlots(Mastery)
		assert.equal(
			manifest.statements.length,
			expected.length,
			"the SDK mirror and the engine agree on the materialized statement count"
		)
		manifest.statements.forEach(function verifyPsiSlot(statement, index) {
			const slot = expected[index]
			assert.ok(slot, `expected slot ${index} exists`)
			assert.equal(statement.kind, slot.kind, `slot ${index} form tag`)
			assert.equal(
				statement.spelling,
				slot.spelling,
				`slot ${index}: the manifest's engine spelling equals the SDK's canonical render`
			)
		})
	})

	test("the ENGINE folds ψ at validate: a member commit lands, a non-member commit pastes the manifest spelling back", function psiCommit() {
		const handle = psiDb
		assert.ok(handle !== undefined, "the ψ store is open")
		const manifest = native.dbManifest(handle)
		const certificate = manifest.relations.find(function byName(candidate) {
			return candidate.name === "Certificate"
		})
		assert.ok(certificate, "the manifest names Certificate")

		const passing = native.dbWriteBegin(handle)
		assert.equal(native.txInsert(passing, certificate.id, [1n, 1n]), true)
		const landed = native.txCommit(passing)
		assert.ok(landed.ok, "a certificate over a ψ-member grade commits")

		const violating = native.dbWriteBegin(handle)
		assert.equal(native.txInsert(violating, certificate.id, [2n, 0n]), true)
		const rejected = native.txCommit(violating)
		assert.ok(!rejected.ok, "a certificate over a non-member grade is rejected")
		assert.equal(rejected.violations.length, 1, "exactly the ψ containment is violated")
		const violation = rejected.violations[0]
		assert.ok(violation, "the violation is present")
		assert.equal(violation.kind, "containment")
		assert.equal(violation.canonical, renderStatement(closedPsiContainment))
		assert.equal(violation.canonical, "Certificate(grade) <= Grade(id | mastered == true)")
		const manifestSlot = manifest.statements.find(function bySpelling(statement) {
			return statement.spelling === violation.canonical
		})
		assert.ok(manifestSlot, "the violation's canonical string IS a manifest spelling, byte for byte")
	})

	test("Db.create accepts the ψ theory and the violation IS the statement value", async function psiDbRuntime() {
		const masteryDb = await Db.create(psiDbDir, Mastery)
		const rejected = masteryDb.write(function violate(tx) {
			tx.insert(Certificate, { grade: "Failed" })
		})
		assert.ok(!rejected.ok, "the ψ containment rejects the non-member grade")
		assert.equal(rejected.violations.length, 1)
		const violation = rejected.violations[0]
		assert.ok(violation, "the violation is present")
		assert.strictEqual(violation.statement, closedPsiContainment)
		assert.equal(violation.canonical, renderStatement(closedPsiContainment))
	})
})
