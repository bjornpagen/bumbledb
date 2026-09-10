import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { closed, closedId } from "#closed.ts"
import { bool, bytes, i64, interval, literalOf, str, u64 } from "#fields.ts"
import { relation } from "#relation.ts"
import { select } from "#selection.ts"

function buildLedgerPieces() {
	const Kind = closed("Kind", ["Checking", "Savings"])
	const Grade = closed(
		"Grade",
		["DirectPass", "Failed"],
		{ mastered: bool },
		{
			DirectPass: { mastered: true },
			Failed: { mastered: false }
		}
	)
	const Holder = relation("Holder", { id: u64, name: str })
	const Account = relation("Account", {
		id: u64,
		holder: u64,
		kind: closedId(Kind),
		active: interval(i64)
	})
	return { Kind, Grade, Holder, Account }
}

describe("field descriptors", function describeDescriptors() {
	test("descriptors are honest frozen plain objects — the type IS the runtime shape", function probeDescriptorShape() {
		assert.equal(u64.kind, "u64")
		assert.deepStrictEqual(u64, { kind: "u64" })
		assert.deepStrictEqual(i64, { kind: "i64" })
		assert.deepStrictEqual(bool, { kind: "bool" })
		assert.deepStrictEqual(str, { kind: "str" })
		assert.ok(Object.isFrozen(u64))
		assert.ok(Object.isFrozen(i64))
	})

	test("descriptors carry NO domain slot and NO .as — the type-lie sweep, every constructor output", function probeNoDomainSlot() {
		const { Kind, Grade } = buildLedgerPieces()
		const descriptors: ReadonlyArray<readonly [string, object]> = [
			["bool", bool],
			["str", str],
			["u64", u64],
			["i64", i64],
			["bytes(4)", bytes(4)],
			["interval(u64)", interval(u64)],
			["interval(u64, 7n)", interval(u64, 7n)],
			["interval(i64)", interval(i64)],
			["Kind.id (closed reference)", closedId(Kind)],
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
		}, /width must be a bigint in 1\.\.=u64::MAX/)
	})
})

describe("closed relations", function describeClosed() {
	test("the id descriptor is pure structure plus the roster — no declared handle domain", function probeIdDescriptor() {
		const { Kind } = buildLedgerPieces()
		assert.deepStrictEqual(closedId(Kind), {
			kind: "u64",
			closed: { name: "Kind", handles: ["Checking", "Savings"] }
		})
	})

	test("payload readback returns the declared axioms, bare and structural", function probeAxioms() {
		const { Grade } = buildLedgerPieces()
		assert.equal(Grade.axioms.DirectPass.mastered, true)
		assert.equal(Grade.axioms.Failed.mastered, false)
	})

	test("payload declarations have one field record and one ground-fact record", function probePayloadLowering() {
		const { Grade } = buildLedgerPieces()
		assert.deepStrictEqual(Grade.handles, ["DirectPass", "Failed"])
		assert.deepEqual(Object.keys(Grade.columns), ["mastered"])
		assert.deepStrictEqual(Grade.axioms, { DirectPass: { mastered: true }, Failed: { mastered: false } })
	})

	test("the minted value carries its columns at runtime — the typed carrier's honest twin", function probeColumnsCarrier() {
		const { Kind, Grade } = buildLedgerPieces()
		assert.ok(Object.hasOwn(Grade, "columns"), "the payload tier's columns record is an own runtime property")
		assert.ok(Object.isFrozen(Grade.columns))
		assert.deepStrictEqual(Object.keys(Grade.columns), ["mastered"])
		assert.deepEqual(Grade.columns.mastered, bool, "the descriptor is structural")

		assert.ok(Object.hasOwn(Kind, "columns"), "the bare tier carries the empty columns record")
		assert.ok(Object.isFrozen(Kind.columns))
		assert.deepStrictEqual(Kind.columns, {})

		const width: 8 = closed("Sev", ["Info"], { tag: bytes(8) }, { Info: { tag: new Uint8Array(8) } }).columns.tag.width
		assert.equal(width, 8)
	})

	test("duplicate handles are construction errors — no name is reserved (handles are data)", function probeHandleGuards() {
		assert.throws(function duplicateHandle() {
			closed("Kind", ["Checking", "Checking"])
		}, /duplicate handle Checking/)

		const bare = closed("Kind", ["Checking", "match"])
		assert.deepStrictEqual(bare.handles, ["Checking", "match"])
		const payload = closed("Sev", ["where"], { pages: bool }, { where: { pages: true } })
		assert.equal(payload.axioms.where.pages, true)
	})

	test("an empty payload roster is a construction error", function probeEmptyRoster() {
		assert.throws(function emptyAxioms() {
			// @ts-expect-error — an empty handle vector is not a roster
			closed("Sev", [], { pages: bool }, {})
		}, /nonempty handle tuple/)
	})

	test("column order is protected; handles use their explicit tuple order", function probeNumericNames() {
		assert.throws(function numericColumn() {
			closed("Bad", ["X"], { "0": bool }, { X: { "0": true } })
		}, /integer index/)
		const numericHandles = closed(
			"Numeric",
			["7", "2"],
			{ pages: bool },
			{ "2": { pages: false }, "7": { pages: true } }
		)
		assert.deepEqual(numericHandles.handles, ["7", "2"])
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
		assert.deepStrictEqual(K.handles, [...handles], "the roster carries the names in declaration order")
	})
})

describe("intervals", function describeIntervals() {
	test("plain interval values are validated by their declared field", () => {
		assert.deepEqual(literalOf(interval(i64), { start: 0n, end: 10n }), {
			kind: "value",
			value: { kind: "intervalI64", start: 0n, end: 10n }
		})
		for (const value of [
			{ start: 5n, end: 5n },
			{ start: 6n, end: 5n },
			{ start: 7n, end: 2n ** 64n }
		]) {
			assert.throws(() => literalOf(interval(u64), value))
		}
	})

	test("the native ray endpoint is the element maximum, not maximum plus one", () => {
		const end = 2n ** 64n - 1n
		assert.deepEqual(literalOf(interval(u64), { start: 7n, end }), {
			kind: "value",
			value: { kind: "intervalU64", start: 7n, end }
		})
		assert.throws(() => literalOf(interval(u64, 1n), { start: end - 1n, end }))
	})
})

describe("selection literal resolution", function describeSelections() {
	test("the roster judges what the structural type cannot: an out-of-roster handle name", function probeRosterMiss() {
		const { Kind } = buildLedgerPieces()
		assert.deepStrictEqual(literalOf(closedId(Kind), "Savings"), { kind: "handle", handle: "Savings" })
		assert.throws(function outOfRoster() {
			literalOf(closedId(Kind), "Frozen")
		}, /expected a Kind handle/)
	})

	test("shape mismatches are typed construction errors on the one literal machine", function probeShapeErrors() {
		assert.throws(function stringOnU64() {
			literalOf(u64, "x")
		}, /expected u64 bigint/)
		assert.throws(function halfInterval() {
			literalOf(interval(i64), { start: 1n })
		}, /required own field/)
	})

	test("where() rides the same machine: an ill-typed forged spelling still faces the roster", function probeWhereRoster() {
		const { Account } = buildLedgerPieces()
		assert.throws(function bigintForged() {
			// @ts-expect-error — H1: a closed field's selection literal is the handle union; a bigint no longer typechecks
			select(Account, { kind: 7n })
		}, /expected a Kind handle/)
		assert.throws(function outOfRoster() {
			// @ts-expect-error — H1: "Frozen" is off the Kind roster — a wrong string is a compile error
			select(Account, { kind: "Frozen" })
		}, /expected a Kind handle/)
		assert.throws(function emptyWhere() {
			select(Account, {})
		}, /bare relation respelled/)
	})

	test("integer-index field names are rejected (declaration-order law)", function probeNumericFieldName() {
		assert.throws(function numericField() {
			relation("Bad", { "0": u64 })
		}, /integer index/)
	})
})
