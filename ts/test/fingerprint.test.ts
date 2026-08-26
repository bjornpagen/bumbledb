import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { duration, ref, weigh, within } from "#capacity.ts"
import { closed } from "#closed.ts"
import { on } from "#face.ts"
import { bool, bytes, i64, interval, span, str, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import { dbClose, native } from "#native.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { capacity, contained, key, mirrors } from "#statements.ts"
import { put } from "#test/put.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-fingerprint-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const PIN = "588df888bd1f1a21057dbf0742af1d1223cc5c2e28ce265f803af989611f1418"

const RAY_END = 18446744073709551615n

const DIGEST = new TextEncoder().encode("0123456789abcdef")

const Status = closed("Status", ["Open", "Frozen"])
const Kind = closed(
	"Kind",
	["DirectPass", "Failed"],
	{ mastered: bool, weight: u64, span: interval(u64) },
	{
		DirectPass: { mastered: true, weight: 2n, span: span(1n, 3n) },
		Failed: { mastered: false, weight: 5n, span: span(3n, 5n) }
	}
)

/**
 * Fields are PURE STRUCTURE (the minimal kernel) — the Rust twin's declared
 * `as HolderId`/`as AccountId` sorts have no SDK spelling: here the classes
 * are LAW-COMPUTED from the statement list, and the fingerprint agrees
 * because newtypes are dropped before hashing on both hosts
 * (`bumbledb-schema-v5` hashes canonical descriptor bytes, never labels) —
 * the neutrality law this lock re-pins under class names.
 */
const Holder = relation("Holder", {
	id: u64.fresh,
	name: str,
	digest: bytes(16),
	at: interval(u64)
})
const Account = relation("Account", {
	id: u64.fresh,
	holder: u64,
	kind: Kind.id,
	status: Status.id,
	active: interval(i64),
	lease: interval(u64, 7n)
})
const SavingsTerms = relation("SavingsTerms", { account: u64, rate_bps: i64 })

const Pool = relation("Pool", { id: u64.fresh, supply: u64, open: interval(u64) })
const Device = relation("Device", { id: u64.fresh, pool: u64, watts: u64, ran: interval(u64) })
const AuditTrail = relation("AuditTrail", { account: u64, rate_bps: i64 })

const CrossHost = schema("CrossHost", { Status, Kind, Holder, Account, SavingsTerms, AuditTrail, Pool, Device }, [
	key(SavingsTerms, ["account"]),
	contained(on(Account, "holder"), on(Holder, "id")),
	contained(on(Account, "kind"), on(Kind, "id")),
	contained(on(Account, "status"), on(Status, "id")),
	mirrors(on(Account.where({ status: "Frozen" }), "id"), on(SavingsTerms, "account")),
	contained(on(Holder.where({ name: ["alpha", "beta"] }), "id"), on(Holder, "id")),
	contained(on(Holder.where({ at: span(5n, RAY_END), digest: DIGEST }), "id"), on(Holder, "id")),
	contained(on(SavingsTerms.where({ rate_bps: -3n }), "account"), on(SavingsTerms, "account")),
	capacity(on(Holder, "id"), within(0n, 3n), on(Account, "holder")),
	capacity(
		on(Holder, "id"),
		weigh(duration("active")),
		within(2n, "*"),
		on(Account.where({ status: "Frozen" }), "holder")
	),
	capacity(on(Holder, "id"), within(1n), on(Account.where({ status: "Open" }), "holder")),
	capacity(on(Holder, "id"), within(0n), on(Account.where({ kind: "Failed" }), "holder")),
	capacity(on(Holder, "id"), within(1n, 4n), on(Account.where({ kind: "DirectPass" }), "holder")),
	contained(on(Device, "pool"), on(Pool, "id")),
	capacity(on(Pool, "id"), weigh("watts"), within(0n, ref("supply")), on(Device, "pool")),
	capacity(on(Pool, "id"), weigh("watts"), within(0n, 100n), on(Device, "pool")),
	capacity(on(Pool, "id"), weigh("watts"), within(1n, "*"), on(Device, "pool")),
	capacity(on(Pool, "id"), weigh(duration("ran")), within(0n, duration("open")), on(Device, "pool")),
	contained(on(Account, "kind"), on(Kind.where({ mastered: true }), "id")),
	key(SavingsTerms, ["account", "rate_bps"]),
	key(AuditTrail, ["account", "rate_bps"]),
	mirrors(on(SavingsTerms, ["account", "rate_bps"]), on(AuditTrail, ["account", "rate_bps"]))
])

describe("the cross-host fingerprint lock", function suite() {
	test("a JS-created store carries the pinned fingerprint across the FFI", async function pin() {
		const created = await native.dbCreate(storeDir, lower(CrossHost))
		assert.equal(created.tag, "accepted", "the CrossHost theory admits")
		assert.equal(
			native.dbFingerprint(created.db),
			PIN,
			"the SDK-lowered theory must hash to the cross-host pin (crate/src/fingerprint_lock.rs carries the same constant)"
		)
		dbClose(created.db)
	})

	test("reopen verifies the stored fingerprint and reads the same identity back", async function reopen() {
		const reopened = await native.dbOpen(storeDir, lower(CrossHost))
		assert.ok(reopened.ok, "the identical theory reopens the store")
		assert.equal(native.dbFingerprint(reopened.db), PIN)
		dbClose(reopened.db)
	})

	test("a twisted twin is refused as fingerprintMismatch data", async function twisted() {
		const spec = lower(CrossHost)
		const refused = await native.dbOpen(storeDir, {
			relations: spec.relations,
			statements: spec.statements.slice(0, -1)
		})
		assert.ok(!refused.ok, "one statement fewer is a different theory")
		assert.equal(refused.kind, "fingerprintMismatch")
	})

	test("the store is inhabitable through the public surface", async function inhabit() {
		const { Db } = await import("#db.ts")
		const db = await Db.open(storeDir, CrossHost)
		const result = db.write(function seed(tx) {
			const ada = put(tx, Holder, {
				name: "ada",
				digest: DIGEST,
				at: span(5n, RAY_END)
			})
			const frozenA = put(tx, Account, {
				holder: ada.id,
				kind: "DirectPass",
				status: "Frozen",
				active: span(-5n, 5n),
				lease: span(0n, 7n)
			})
			const frozenB = put(tx, Account, {
				holder: ada.id,
				kind: "DirectPass",
				status: "Frozen",
				active: span(-1n, 1n),
				lease: span(7n, 14n)
			})
			put(tx, Account, {
				holder: ada.id,
				kind: "DirectPass",
				status: "Open",
				active: span(0n, 10n),
				lease: span(14n, 21n)
			})
			put(tx, SavingsTerms, { account: frozenA.id, rate_bps: -3n })
			put(tx, SavingsTerms, { account: frozenB.id, rate_bps: 25n })

			put(tx, AuditTrail, { account: frozenA.id, rate_bps: -3n })
			put(tx, AuditTrail, { account: frozenB.id, rate_bps: 25n })
		})
		assert.equal(result.tag, "accepted", "the seeded state satisfies every statement of the theory")
		assert.equal(db.read((i) => i.scan(Account)).length, 3)
		assert.equal(db.read((i) => i.scan(SavingsTerms)).length, 2)
		assert.equal(db.read((i) => i.scan(AuditTrail)).length, 2)
	})
})
