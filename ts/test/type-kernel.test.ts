/**
 * Runtime pins for the MINIMAL structural field kernel (K3): descriptors
 * honest at runtime (`{ kind, width?, element?, fresh? }` frozen plain
 * objects — PURE STRUCTURE, no `domain` slot, no `.as`: the type-lie sweep
 * below proves the absences own-property by own-property), closed
 * relations in both tiers (bare-bigint handle constants in declaration
 * order, the `fromId` weld, payload readback, the `__proto__`-safe
 * own-property minting), the field constructors' grammar bounds, `span()`'s
 * half-open nonempty law, and the selection-literal machine's roster
 * judgment — the runtime half of the two-boundary split (structural types
 * admit any bigint; the roster and the engine judge).
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"

import { closed } from "#closed.ts"
import { bool, bytes, i64, interval, literalOf, span, str, u64 } from "#fields.ts"
import { relation } from "#relation.ts"

function buildLedgerPieces() {
	const Kind = closed("Kind", ["Checking", "Savings"])
	const Grade = closed("Grade", { mastered: bool })({
		DirectPass: { mastered: true },
		Failed: { mastered: false }
	})
	const Holder = relation("Holder", { id: u64.fresh, name: str })
	const Account = relation("Account", {
		id: u64.fresh,
		holder: u64,
		kind: Kind.id,
		active: interval(i64)
	})
	return { Kind, Grade, Holder, Account }
}

describe("field descriptors", function describeDescriptors() {
	test("descriptors are honest frozen plain objects — the type IS the runtime shape", function probeDescriptorShape() {
		assert.equal(u64.kind, "u64")
		assert.deepStrictEqual(u64.fresh, { kind: "u64", fresh: true })
		assert.deepStrictEqual(i64, { kind: "i64" })
		assert.deepStrictEqual(bool, { kind: "bool" })
		assert.deepStrictEqual(str, { kind: "str" })
		assert.ok(Object.isFrozen(u64))
		assert.ok(Object.isFrozen(u64.fresh))
		assert.ok(Object.isFrozen(i64))
	})

	test("descriptors carry NO domain slot and NO .as — the type-lie sweep, every constructor output", function probeNoDomainSlot() {
		const { Kind, Grade } = buildLedgerPieces()
		const descriptors: ReadonlyArray<readonly [string, object]> = [
			["bool", bool],
			["str", str],
			["u64", u64],
			["u64.fresh", u64.fresh],
			["i64", i64],
			["bytes(4)", bytes(4)],
			["interval(u64)", interval(u64)],
			["interval(u64, 7n)", interval(u64, 7n)],
			["interval(i64)", interval(i64)],
			["Kind.id (closed reference)", Kind.id],
			["Grade.columns.mastered (payload column)", Grade.columns.mastered]
		]
		for (const [name, descriptor] of descriptors) {
			assert.equal(Object.hasOwn(descriptor, "domain"), false, `${name} must carry no runtime domain slot`)
			assert.equal(Object.hasOwn(descriptor, "as"), false, `${name} must carry no .as constructor`)
		}
	})

	test("bytes carries its width label at runtime and validates the 1..=64 grammar bound", function probeBytes() {
		const tag = bytes(32)
		assert.deepStrictEqual(tag, { kind: "bytes", width: 32 })
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
		assert.deepStrictEqual(fixed, { kind: "interval", element: "u64", width: 4n })
		const general = interval(i64)
		assert.deepStrictEqual(general, { kind: "interval", element: "i64", width: undefined })
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

	test("the id descriptor is pure structure plus the roster — no declared handle domain", function probeIdDescriptor() {
		const { Kind } = buildLedgerPieces()
		assert.deepStrictEqual(Kind.id, {
			kind: "u64",
			closed: { name: "Kind", handles: ["Checking", "Savings"] }
		})
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

	test("the minted value carries its columns at runtime — the typed carrier's honest twin", function probeColumnsCarrier() {
		const { Kind, Grade } = buildLedgerPieces()
		assert.ok(Object.hasOwn(Grade, "columns"), "the payload tier's columns record is an own runtime property")
		assert.ok(Object.isFrozen(Grade.columns))
		assert.deepStrictEqual(Object.keys(Grade.columns), ["mastered"])
		assert.equal(Grade.columns.mastered, bool, "the carrier holds the declared descriptor itself, by identity")
		assert.equal(Grade.data.columns[0]?.field, Grade.columns.mastered, "the lowering reads the same descriptors")
		// the bare tier declares no columns — the carrier is present and empty
		assert.ok(Object.hasOwn(Kind, "columns"), "the bare tier carries the empty columns record")
		assert.ok(Object.isFrozen(Kind.columns))
		assert.deepStrictEqual(Kind.columns, {})
		// the carrier's TYPE flows through the mint: a width label reads back as its literal
		const width: 8 = closed("Sev", { tag: bytes(8) })({ Info: { tag: new Uint8Array(8) } }).columns.tag.width
		assert.equal(width, 8)
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
