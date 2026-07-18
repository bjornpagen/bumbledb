/**
 * PRD-S1 runtime pins for the structural field & domain kernel: descriptors
 * honest at runtime (`{ kind, domain, fresh?, width?, element? }` frozen
 * plain objects), closed relations in both tiers (bare-bigint handle
 * constants in declaration order, the `fromId` weld, payload readback, the
 * `__proto__`-safe own-property minting), the field constructors' grammar
 * bounds, `span()`'s half-open nonempty law, and the selection-literal
 * machine's roster judgment — the runtime half of the two-boundary split
 * (structural types admit any bigint; the roster and the engine judge).
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"

import { closed } from "#closed.ts"
import { bool, bytes, i64, interval, literalOf, span, str, u64 } from "#fields.ts"
import { relation } from "#relation.ts"

const HolderId = u64.as("HolderId")
const AccountId = u64.as("AccountId")
const ActiveDuring = interval(i64).as("ActiveDuring")

function buildLedgerPieces() {
	const Kind = closed("Kind", ["Checking", "Savings"])
	const Grade = closed("Grade", { mastered: bool })({
		DirectPass: { mastered: true },
		Failed: { mastered: false }
	})
	const Holder = relation("Holder", { id: HolderId.fresh, name: str })
	const Account = relation("Account", {
		id: AccountId.fresh,
		holder: HolderId,
		kind: Kind.id,
		active: ActiveDuring
	})
	return { Kind, Grade, Holder, Account }
}

describe("field descriptors", function describeDescriptors() {
	test("descriptors are honest frozen plain objects — the type IS the runtime shape", function probeDescriptorShape() {
		assert.equal(HolderId.kind, "u64")
		assert.equal(HolderId.domain, "HolderId")
		assert.deepStrictEqual(HolderId.fresh, { kind: "u64", domain: "HolderId", fresh: true })
		assert.deepStrictEqual(u64.fresh, { kind: "u64", domain: undefined, fresh: true })
		assert.deepStrictEqual(i64.as("Cents"), { kind: "i64", domain: "Cents" })
		assert.equal(bool.kind, "bool")
		assert.equal(bool.domain, undefined)
		assert.equal(str.kind, "str")
		assert.equal(str.domain, undefined)
		assert.ok(Object.isFrozen(HolderId))
		assert.ok(Object.isFrozen(HolderId.fresh))
		assert.ok(Object.isFrozen(u64))
	})

	test("bytes carries its width label at runtime and validates the 1..=64 grammar bound", function probeBytes() {
		const tag = bytes(32).as("Tag")
		assert.equal(tag.kind, "bytes")
		assert.equal(tag.width, 32)
		assert.equal(tag.domain, "Tag")
		assert.equal(bytes(4).width, 4)
		assert.throws(function zeroBytes() {
			bytes(0)
		}, /1\.\.=64/)
		assert.throws(function wideBytes() {
			bytes(65)
		}, /1\.\.=64/)
	})

	test("interval carries element and width labels at runtime and validates w >= 1", function probeInterval() {
		const fixed = interval(u64, 4n)
		assert.equal(fixed.kind, "interval")
		assert.equal(fixed.element, "u64")
		assert.equal(fixed.width, 4n)
		assert.equal(fixed.domain, undefined)
		const general = interval(i64).as("ActiveDuring")
		assert.equal(general.element, "i64")
		assert.equal(general.width, undefined)
		assert.equal(general.domain, "ActiveDuring")
		assert.throws(function zeroWidth() {
			interval(u64, 0n)
		}, /width must be >= 1/)
	})
})

describe("closed relations", function describeClosed() {
	test("handle constants are bare bigints carrying declaration-order ids", function probeHandleIds() {
		const { Kind, Grade } = buildLedgerPieces()
		assert.equal(Kind.Checking, 0n)
		assert.equal(Kind.Savings, 1n)
		assert.equal(Grade.DirectPass, 0n)
		assert.equal(Grade.Failed, 1n)
		assert.equal(typeof Kind.Checking, "bigint")
	})

	test("fromId welds ids back to handles and misses beyond the roster — no forge needed", function probeWeld() {
		const { Kind } = buildLedgerPieces()
		assert.equal(Kind.fromId(Kind.Checking), "Checking")
		assert.equal(Kind.fromId(Kind.Savings), "Savings")
		assert.equal(Kind.fromId(99n), undefined)
	})

	test("the id descriptor carries the handle domain and the roster", function probeIdDescriptor() {
		const { Kind } = buildLedgerPieces()
		assert.equal(Kind.id.kind, "u64")
		assert.equal(Kind.id.domain, "KindId")
		assert.deepStrictEqual(Kind.id.closed, { name: "Kind", handles: ["Checking", "Savings"] })
	})

	test("payload readback returns the declared axioms, bare and structural", function probeAxioms() {
		const { Grade } = buildLedgerPieces()
		assert.equal(Grade.axioms.DirectPass.mastered, true)
		assert.equal(Grade.axioms.Failed.mastered, false)
	})

	test("payload tier lowers columns and ground axioms eagerly in declaration order", function probePayloadLowering() {
		const { Grade } = buildLedgerPieces()
		assert.deepStrictEqual(Grade.data.handles, ["DirectPass", "Failed"])
		assert.equal(Grade.data.columns.length, 1)
		assert.equal(Grade.data.columns[0]?.name, "mastered")
		assert.deepStrictEqual(Grade.data.rows, [
			{ handle: "DirectPass", values: [{ kind: "value", value: { kind: "bool", value: true } }] },
			{ handle: "Failed", values: [{ kind: "value", value: { kind: "bool", value: false } }] }
		])
	})

	test("duplicate and reserved handles are construction errors in both tiers", function probeHandleGuards() {
		assert.throws(function duplicateHandle() {
			closed("Kind", ["Checking", "Checking"])
		}, /duplicate handle Checking/)
		assert.throws(function reservedHandle() {
			closed("Kind", ["Checking", "fromId"])
		}, /collides with the closed value's own surface/)
		assert.throws(function reservedPayloadHandle() {
			closed("Sev", { pages: bool })({ fromId: { pages: true } })
		}, /collides with the closed value's own surface/)
	})

	test("an empty payload roster is a construction error", function probeEmptyRoster() {
		assert.throws(function emptyAxioms() {
			closed("Sev", { pages: bool })({})
		}, /at least one handle/)
	})

	test("integer-index column and handle names are rejected (declaration-order law)", function probeNumericNames() {
		assert.throws(function numericColumn() {
			closed("Bad", { "0": bool })
		}, /integer index/)
		assert.throws(function numericHandle() {
			closed("Bad", { pages: bool })({ "7": { pages: true } })
		}, /integer index/)
	})

	test("handle constants and axiom rows are minted as OWN properties for every admitted name", function probeProtoHandle() {
		/**
		 * "__proto__" is a legal identifier (the macro analog admits it), so
		 * the constant must work — own-property definition shadows the
		 * object-protocol accessor instead of silently riding it. The
		 * computed access below is deliberate: it is exactly how a host loops
		 * a roster.
		 */
		const handles = ["Alpha", "__proto__"] as const
		const K = closed("K", handles)
		for (const handle of handles) {
			assert.equal(
				typeof K[handle],
				"bigint",
				`the ${handle} handle constant must be a bigint, never an accessor no-op`
			)
			assert.equal(K.fromId(K[handle]), handle, "the weld agrees with the constant")
		}
		assert.deepEqual(
			Object.keys(K.axioms).toSorted(),
			[...handles].toSorted(),
			"the axioms record carries every handle row as an own enumerable property"
		)
	})
})

describe("intervals", function describeIntervals() {
	test("span constructs half-open nonempty intervals and rejects the rest", function probeSpan() {
		const active = span(0n, 10n)
		assert.equal(active.start, 0n)
		assert.equal(active.end, 10n)
		assert.throws(function emptySpan() {
			span(5n, 5n)
		}, /start must be < end/)
		assert.throws(function invertedSpan() {
			span(6n, 5n)
		}, /start must be < end/)
	})

	test("the ray is representable", function probeRay() {
		const ray = span(7n, 2n ** 64n)
		assert.equal(ray.end, 2n ** 64n)
	})
})

describe("selection literal resolution", function describeSelections() {
	test("the roster judges what the structural type cannot: an out-of-roster handle id", function probeRosterMiss() {
		const { Kind } = buildLedgerPieces()
		assert.deepStrictEqual(literalOf(Kind.id, Kind.Savings), { kind: "handle", handle: "Savings" })
		assert.throws(function outOfRoster() {
			literalOf(Kind.id, 7n)
		}, /closed relation Kind has no handle with id 7/)
	})

	test("shape mismatches are typed construction errors on the one literal machine", function probeShapeErrors() {
		assert.throws(function stringOnU64() {
			literalOf(u64, "x")
		}, /expected bigint/)
		assert.throws(function halfInterval() {
			literalOf(interval(i64), { start: 1n })
		}, /interval/)
	})

	test("where() rides the same machine: a well-TYPED bare bigint still faces the roster", function probeWhereRoster() {
		/**
		 * The two-boundary split, demonstrated: structurally, 7n is a legal
		 * selection literal for a closed-reference field (no brand blocks
		 * it) — the roster refuses it at construction, and the engine would
		 * refuse it again at commit.
		 */
		const { Account } = buildLedgerPieces()
		assert.throws(function outOfRoster() {
			Account.where({ kind: 7n })
		}, /closed relation Kind has no handle with id 7/)
		assert.throws(function emptyWhere() {
			Account.where({})
		}, /bare relation respelled/)
	})

	test("integer-index field names are rejected (declaration-order law)", function probeNumericFieldName() {
		assert.throws(function numericField() {
			relation("Bad", { "0": u64 })
		}, /integer index/)
	})
})
