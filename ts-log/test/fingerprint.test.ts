import assert from "node:assert/strict"
import { describe, test } from "node:test"
import {
	bool,
	bytes,
	capacity,
	closed,
	contained,
	duration,
	i64,
	interval,
	key,
	mirrors,
	on,
	ref,
	relation,
	schema,
	span,
	str,
	u64,
	weigh,
	within
} from "@bjornpagen/bumbledb"
import { descriptorOf } from "#descriptor.ts"
import { Ledger, Vocab } from "#test/fixtures.ts"

/**
 * The exact CrossHost theory the engine SDK pins against the Rust
 * fingerprint (ts/test/fingerprint.test.ts): if our pure-TS canonical
 * encoding mirror produces this hash, the whole descriptor parse —
 * sealed fields, closed extensions, materialized statement order, side
 * selections, handle resolution, capacity windows — agrees with the
 * engine byte for byte.
 */
const PIN = "588df888bd1f1a21057dbf0742af1d1223cc5c2e28ce265f803af989611f1418"

const RAY_END = 18446744073709551615n
const DIGEST = new TextEncoder().encode("0123456789abcdef")

const Status = closed("Status", ["Open", "Frozen"])
const Kind = closed(
	"Kind",
	{ mastered: bool, weight: u64, span: interval(u64) },
	{
		DirectPass: { mastered: true, weight: 2n, span: span(1n, 3n) },
		Failed: { mastered: false, weight: 5n, span: span(3n, 5n) }
	}
)

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

describe("the fingerprint mirror", function suite() {
	test("the CrossHost theory hashes to the engine's cross-host pin", function pinned() {
		assert.equal(descriptorOf(CrossHost).fingerprint, PIN)
	})

	test("the descriptor parse is cached per theory value", function cached() {
		assert.equal(descriptorOf(Ledger), descriptorOf(Ledger))
	})

	test("closed rosters resolve handles to declaration-order row ids", function handles() {
		const descriptor = descriptorOf(Vocab)
		const status = descriptor.relationByName.get("Status")
		assert.ok(status)
		assert.deepEqual(status.handles, ["Open", "Frozen"])
		assert.equal(status.fields[0]?.name, "id")
		assert.equal(status.fields[0]?.closedRef, "Status")
	})
})
