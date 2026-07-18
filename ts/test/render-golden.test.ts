/**
 * The TS-render ⇄ manifest-render golden (TODO.md §7 item 3a): for every
 * statement construct the schema surface can utter — the FD key form
 * (scalar, composite pointwise-interval), containment (plain, σ-selected,
 * ψ-selected, closed-target, multi-field pointwise), the `==` bijection,
 * every legal window spelling (`{n}`, `{0}`, `{lo..hi}`, `{lo..*}`,
 * `{0..hi}`), and the sub-vocabulary handle-set selection — the SDK's
 * `renderStatement` output equals, byte for byte, the engine-rendered
 * spelling the manifest ships for the same store
 * (`schema/render.rs::render_declared` via `dbManifest`). The engine's
 * materialized order is mirrored positionally (fresh auto-keys, closed
 * auto-keys, then declared statements, a `mirrors` occupying two slots), so
 * the golden also pins the implied-key spellings the SDK never utters.
 * Selection literals lean adversarial on purpose: the `char::escape_debug`
 * and `u8::escape_ascii` mirrors are exactly where the two renderers could
 * drift silently.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import { closed } from "#closed.ts"
import { atLeast, atMost, between, exactly, none } from "#count.ts"
import { on, oneOf } from "#face.ts"
import { bool, bytes, i64, interval, span, str, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import type { DbHandle, Manifest, StatementKindTag } from "#native.ts"
import { native } from "#native.ts"
import { relation } from "#relation.ts"
import { type AnySchema, schema } from "#schema.ts"
import { contained, key, mirrors, renderStatement, type Statement, window } from "#statements.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-render-golden-"))
const storeDir = path.join(tmpRoot, "store")

const HolderId = u64.as("HolderId")
const AccountId = u64.as("AccountId")
const RoomId = u64.as("RoomId")
const ActiveDuring = interval(i64).as("ActiveDuring")
const BookedDuring = interval(u64).as("BookedDuring")

const Status = closed("Status", ["Active", "Frozen"])
const Kind = closed("Kind", ["Checking", "Savings", "Brokerage"])
const Holder = relation("Holder", { id: HolderId.fresh, name: str })
const Account = relation("Account", {
	id: AccountId.fresh,
	holder: HolderId,
	kind: Kind.id,
	status: Status.id,
	score: i64,
	weight: u64,
	label: str,
	flag: bool,
	tag: bytes(4),
	active: ActiveDuring
})
const Booking = relation("Booking", { room: RoomId, during: BookedDuring })
const Slot = relation("Slot", { room: RoomId, during: BookedDuring })
const SavingsTerms = relation("SavingsTerms", { account: AccountId })

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
const sigmaContainment = contained(on(Account.where({ kind: Kind.Savings }), "holder"), on(Holder, "id"))
const psiContainment = contained(on(SavingsTerms, "account"), on(Account.where({ status: Status.Active }), "id"))
const bijection = mirrors(on(Account.where({ kind: Kind.Savings }), "id"), on(SavingsTerms, "account"))
const ceilingWindow = window(on(Holder, "id"), atMost(3n), on(Account, "holder"))
const exclusionWindow = window(on(Holder, "id"), none, on(Account.where({ flag: false }), "holder"))
const exactWindow = window(on(Holder, "id"), exactly(2n), on(Account.where({ label: gauntletLabel }), "holder"))
const rangeWindow = window(on(Holder, "id"), between(1n, 4n), on(Account.where({ score: -42n }), "holder"))
const floorWindow = window(
	on(Holder, "id"),
	atLeast(2n),
	on(Account.where({ kind: oneOf(Kind.Checking, Kind.Savings) }), "holder")
)
const psiTargetWindow = window(on(Account.where({ flag: true }), "id"), atMost(1n), on(SavingsTerms, "account"))
const literalGauntletWindow = window(
	on(Holder, "id"),
	atMost(5n),
	on(Account.where({ weight: 7n, tag: gauntletTag, active: span(-3n, 9n) }), "holder")
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
	ceilingWindow,
	exclusionWindow,
	exactWindow,
	rangeWindow,
	floorWindow,
	psiTargetWindow,
	literalGauntletWindow
]

const Golden = schema("Golden", { Status, Kind, Holder, Account, Booking, Slot, SavingsTerms }, statements)

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
			return "containment"
		case "window":
			return "cardinality"
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
		if (statement.data.kind === "containment" && statement.data.bidirectional) {
			slots.push(slot)
		}
	}
	return slots
}

let db: DbHandle | undefined

after(function cleanup() {
	if (db !== undefined) {
		native.dbClose(db)
	}
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

describe("the TS-render ⇄ manifest-render golden", function suite() {
	test("the exotic spellings are pinned as literals, pre-open", function pins() {
		assert.equal(renderStatement(pointwiseKey), "Booking(room, during) -> Booking")
		assert.equal(renderStatement(pointwiseContainment), "Slot(room, during) <= Booking(room, during)")
		assert.equal(renderStatement(psiContainment), "SavingsTerms(account) <= Account(id | status == Active)")
		assert.equal(renderStatement(bijection), "Account(id | kind == Savings) == SavingsTerms(account)")
		assert.equal(renderStatement(ceilingWindow), "Holder(id) <={0..3} Account(holder)")
		assert.equal(renderStatement(exclusionWindow), "Holder(id) <={0} Account(holder | flag == false)")
		assert.equal(
			renderStatement(exactWindow),
			'Holder(id) <={2} Account(holder | label == "it\\\'s \\"w\\teird\\"\\n\\\\e\\u{301}")'
		)
		assert.equal(renderStatement(rangeWindow), "Holder(id) <={1..4} Account(holder | score == -42)")
		assert.equal(renderStatement(floorWindow), "Holder(id) <={2..*} Account(holder | kind == {Checking, Savings})")
		assert.equal(renderStatement(psiTargetWindow), "Account(id | flag == true) <={0..1} SavingsTerms(account)")
		assert.equal(
			renderStatement(literalGauntletWindow),
			'Holder(id) <={0..5} Account(holder | weight == 7, tag == b"b\\x00\\xff\\"", active == -3..9)'
		)
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
