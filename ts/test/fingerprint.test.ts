/**
 * The cross-host fingerprint lock (bumbledb TODO.md §7, the pin the SDK
 * owes): the ONE theory exercising every schema construct — fresh keys,
 * `str`, `bytes<N>`, general and fixed-width intervals INCLUDING a ray
 * literal, both closed tiers, containment with σ on both faces, `==`
 * mirrors, and every legal window spelling — built here through the SDK's
 * constructors and, in `crate/src/fingerprint_lock.rs`, through the
 * engine's `schema!` macro. Each side independently asserts its
 * engine-computed fingerprint equals the ONE pinned constant, so
 * `node --test` and `cargo test` each run standalone while jointly proving
 * the cross-host bond: identical fingerprints mean `Db::open` on either
 * side admits the other side's store (the fingerprint is open's whole
 * schema gate beyond format version and store kind), and neither surface
 * can fake the pin — this side's hex arrives from the engine ACROSS THE
 * FFI after a real `dbCreate`, the Rust side's through real macro
 * expansion and validation.
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
import { native } from "#native.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained, key, mirrors, window } from "#statements.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-fingerprint-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

/**
 * The pinned cross-host fingerprint of the `CrossHost` theory. The SAME
 * constant is baked into `crate/src/fingerprint_lock.rs`; a change here
 * without the twin change there (or vice versa) is exactly the drift this
 * lock exists to catch.
 */
const PIN = "6120cb184faaacec8f4e146f7d43b5b9c59053f7b560d037754d7cad41401508"

/** `u64::MAX` — an interval ending here is the unbounded ray `[start, ∞)`. */
const RAY_END = 18446744073709551615n

/** The 16 bytes of the Rust twin's `b"0123456789abcdef"` selection literal. */
const DIGEST = new TextEncoder().encode("0123456789abcdef")

const HolderId = u64.as("HolderId")
const AccountId = u64.as("AccountId")
const ActiveDuring = interval(i64).as("ActiveDuring")
const Lease = interval(u64, 7n).as("Lease")

const Status = closed("Status", ["Open", "Frozen"])
const Kind = closed("Kind", { mastered: bool, weight: u64, span: interval(u64) })({
	DirectPass: { mastered: true, weight: 2n, span: span(1n, 3n) },
	Failed: { mastered: false, weight: 5n, span: span(3n, 5n) }
})

const Holder = relation("Holder", {
	id: HolderId.fresh,
	name: str,
	digest: bytes(16),
	at: interval(u64)
})
const Account = relation("Account", {
	id: AccountId.fresh,
	holder: HolderId,
	kind: Kind.id,
	status: Status.id,
	active: ActiveDuring,
	lease: Lease
})
const SavingsTerms = relation("SavingsTerms", { account: AccountId, rate_bps: i64 })

/**
 * Statement for statement the Rust twin's declaration order — order is
 * fingerprint identity (materialized order pins statement ids).
 */
const CrossHost = schema("CrossHost", { Status, Kind, Holder, Account, SavingsTerms }, [
	key(SavingsTerms, ["account"]),
	contained(on(Account, "holder"), on(Holder, "id")),
	contained(on(Account, "kind"), on(Kind, "id")),
	contained(on(Account, "status"), on(Status, "id")),
	mirrors(on(Account.where({ status: Status.Frozen }), "id"), on(SavingsTerms, "account")),
	contained(on(Holder.where({ name: oneOf("alpha", "beta") }), "id"), on(Holder, "id")),
	contained(on(Holder.where({ at: span(5n, RAY_END), digest: DIGEST }), "id"), on(Holder, "id")),
	contained(on(SavingsTerms.where({ rate_bps: -3n }), "account"), on(SavingsTerms, "account")),
	window(on(Holder, "id"), atMost(3n), on(Account, "holder")),
	window(on(Holder, "id"), atLeast(2n), on(Account.where({ status: Status.Frozen }), "holder")),
	window(on(Holder, "id"), exactly(1n), on(Account.where({ status: Status.Open }), "holder")),
	window(on(Holder, "id"), none, on(Account.where({ kind: Kind.Failed }), "holder")),
	window(on(Holder, "id"), between(1n, 4n), on(Account.where({ kind: Kind.DirectPass }), "holder"))
])

describe("the cross-host fingerprint lock", function suite() {
	test("a JS-created store carries the pinned fingerprint across the FFI", function pin() {
		const created = native.dbCreate(storeDir, lower(CrossHost))
		assert.ok(created.ok, "the CrossHost theory admits")
		assert.equal(
			native.dbFingerprint(created.db),
			PIN,
			"the SDK-lowered theory must hash to the cross-host pin (crate/src/fingerprint_lock.rs carries the same constant)"
		)
		native.dbClose(created.db)
	})

	test("reopen verifies the stored fingerprint and reads the same identity back", function reopen() {
		const reopened = native.dbOpen(storeDir, lower(CrossHost))
		assert.ok(reopened.ok, "the identical theory reopens the store")
		assert.equal(native.dbFingerprint(reopened.db), PIN)
		native.dbClose(reopened.db)
	})

	test("a twisted twin is refused as fingerprintMismatch data", function twisted() {
		const spec = lower(CrossHost)
		const refused = native.dbOpen(storeDir, {
			relations: spec.relations,
			statements: spec.statements.slice(0, -1)
		})
		assert.ok(!refused.ok, "one statement fewer is a different theory")
		assert.equal(refused.kind, "fingerprintMismatch")
	})

	test("the store is inhabitable through the public surface", async function inhabit() {
		// Loaded lazily: the `Db` runtime is S4's structural rewrite — until it
		// lands, this import (not the fingerprint pins above) is the red part.
		const { Db } = await import("#db.ts")
		const db = await Db.open(storeDir, CrossHost)
		const result = db.write(function seed(tx) {
			const ada = tx.insert(Holder, {
				name: "ada",
				digest: DIGEST,
				at: span(5n, RAY_END)
			})
			const frozenA = tx.insert(Account, {
				holder: ada.id,
				kind: Kind.DirectPass,
				status: Status.Frozen,
				active: span(-5n, 5n),
				lease: span(0n, 7n)
			})
			const frozenB = tx.insert(Account, {
				holder: ada.id,
				kind: Kind.DirectPass,
				status: Status.Frozen,
				active: span(-1n, 1n),
				lease: span(7n, 14n)
			})
			tx.insert(Account, {
				holder: ada.id,
				kind: Kind.DirectPass,
				status: Status.Open,
				active: span(0n, 10n),
				lease: span(14n, 21n)
			})
			tx.insert(SavingsTerms, { account: frozenA.id, rate_bps: -3n })
			tx.insert(SavingsTerms, { account: frozenB.id, rate_bps: 25n })
		})
		assert.ok(result.ok, "the seeded state satisfies every statement of the theory")
		assert.equal(db.scan(Account).length, 3)
		assert.equal(db.scan(SavingsTerms).length, 2)
	})
})
