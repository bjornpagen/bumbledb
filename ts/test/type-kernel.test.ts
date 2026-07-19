/**
 * Runtime pins for the MINIMAL structural field kernel (K3): descriptors
 * honest at runtime (`{ kind, width?, element?, fresh? }` frozen plain
 * objects — PURE STRUCTURE, no `domain` slot, no `.as`: the type-lie sweep
 * below proves the absences own-property by own-property), closed
 * relations in both tiers (the roster-carrying `id` descriptor in
 * declaration order, payload readback, the `__proto__`-safe own-property
 * minting — handles are DATA on the roster, never properties of the
 * value), the field constructors' grammar bounds, `span()`'s half-open
 * nonempty law, and the selection-literal machine's roster judgment — the
 * runtime half of the two-boundary split (structural types admit what the
 * roster and the engine then judge).
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"

import { closed } from "#closed.ts"
import { bool, bytes, i64, interval, literalOf, span, str, u64 } from "#fields.ts"
import { relation } from "#relation.ts"

function buildLedgerPieces() {
	const Kind = closed("Kind", ["Checking", "Savings"])
	const Grade = closed(
		"Grade",
		{ mastered: bool },
		{
			DirectPass: { mastered: true },
			Failed: { mastered: false }
		}
	)
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
		const width: 8 = closed("Sev", { tag: bytes(8) }, { Info: { tag: new Uint8Array(8) } }).columns.tag.width
		assert.equal(width, 8)
	})

	test("duplicate handles are construction errors — no name is reserved (handles are data)", function probeHandleGuards() {
		assert.throws(function duplicateHandle() {
			closed("Kind", ["Checking", "Checking"])
		}, /duplicate handle Checking/)
		// H5: the reserved-name wall died with the handle constants — a
		// method-named handle is ordinary roster data in both tiers.
		const bare = closed("Kind", ["Checking", "match"])
		assert.deepStrictEqual(bare.data.handles, ["Checking", "match"])
		const payload = closed("Sev", { pages: bool }, { where: { pages: true } })
		assert.equal(payload.axioms.where.pages, true)
	})

	test("an empty payload roster is a construction error", function probeEmptyRoster() {
		assert.throws(function emptyAxioms() {
			closed("Sev", { pages: bool }, {})
		}, /at least one handle/)
	})

	test("integer-index column and handle names are rejected (declaration-order law)", function probeNumericNames() {
		assert.throws(function numericColumn() {
			closed("Bad", { "0": bool }, { X: { "0": true } })
		}, /integer index/)
		assert.throws(function numericHandle() {
			closed("Bad", { pages: bool }, { "7": { pages: true } })
		}, /integer index/)
	})

	test("axiom rows are minted as OWN properties for every admitted name", function probeProtoHandle() {
		/**
		 * "__proto__" is a legal identifier (the macro analog admits it), so
		 * the axiom row must land as an OWN property — own-property definition
		 * shadows the object-protocol accessor instead of silently riding it
		 * (which would swap the record's prototype instead of creating the
		 * row).
		 */
		const handles = ["Alpha", "__proto__"] as const
		const K = closed("K", handles)
		assert.deepEqual(
			Object.keys(K.axioms).toSorted(),
			[...handles].toSorted(),
			"the axioms record carries every handle row as an own enumerable property"
		)
		assert.equal(
			Object.getPrototypeOf(K.axioms),
			Object.prototype,
			"the __proto__ handle never rides the accessor — the record's prototype is untouched"
		)
		assert.deepStrictEqual(K.data.handles, [...handles], "the roster carries the names in declaration order")
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
	test("the roster judges what the structural type cannot: an out-of-roster handle name", function probeRosterMiss() {
		const { Kind } = buildLedgerPieces()
		assert.deepStrictEqual(literalOf(Kind.id, "Savings"), { kind: "handle", handle: "Savings" })
		assert.throws(function outOfRoster() {
			literalOf(Kind.id, "Frozen")
		}, /"Frozen" is not a handle of Kind/)
	})

	test("shape mismatches are typed construction errors on the one literal machine", function probeShapeErrors() {
		assert.throws(function stringOnU64() {
			literalOf(u64, "x")
		}, /expected bigint/)
		assert.throws(function halfInterval() {
			literalOf(interval(i64), { start: 1n })
		}, /interval/)
	})

	test("where() rides the same machine: an ill-typed forged spelling still faces the roster", function probeWhereRoster() {
		/**
		 * The two-boundary split, demonstrated: since H1 the TYPE tier already
		 * refuses a bigint and an out-of-roster string on a closed-reference
		 * field (the value type is the precise handle union), so forging one
		 * requires an ill-typed call — and the literal machine STILL refuses
		 * it at construction (the runtime belt under the type claim; the
		 * engine would refuse it again at commit).
		 */
		const { Account } = buildLedgerPieces()
		assert.throws(function bigintForged() {
			// @ts-expect-error — H1: a closed field's selection literal is the handle union; a bigint no longer typechecks
			Account.where({ kind: 7n })
		}, /expected a Kind handle name \(string\), got bigint/)
		assert.throws(function outOfRoster() {
			// @ts-expect-error — H1: "Frozen" is off the Kind roster — a wrong string is a compile error
			Account.where({ kind: "Frozen" })
		}, /"Frozen" is not a handle of Kind/)
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
