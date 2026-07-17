/**
 * PRD-05 runtime pins: lowering shapes for relations and closed relations
 * (both tiers), declaration-order handle ids, the `fromId` weld, payload
 * readback, interval/bytes construction boundaries, and byte-stability of
 * the lowered fragments under serialization.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"

import * as errors from "@superbuilders/errors"
import type { Brand } from "#index.ts"
import { bool, bytes, closed, i64, interval, lowerClosed, lowerRelation, relation, span, str, u64 } from "#index.ts"

/**
 * Serializes lowered plain data for byte-stability comparison: bigints and
 * byte arrays gain JSON spellings; everything else is already plain.
 */
function stringify(value: unknown): string {
	return JSON.stringify(value, function replace(_key, entry: unknown) {
		if (typeof entry === "bigint") {
			return `${entry}n`
		}
		if (entry instanceof Uint8Array) {
			return Array.from(entry)
		}
		return entry
	})
}

/**
 * Forges a Kind-branded id for negative-space probes. The guard's runtime
 * check is deliberately weaker than the roster (bigint-ness only) — the
 * probes below need ids the roster does NOT hold, which no public
 * constructor can produce.
 */
function isForgedKindId(raw: bigint): raw is Brand<bigint, "Kind"> {
	return typeof raw === "bigint"
}

function forgeKindId(raw: bigint): Brand<bigint, "Kind"> {
	if (!isForgedKindId(raw)) {
		throw errors.new("unreachable: a bigint literal is a bigint")
	}
	return raw
}

const HolderId = u64.newtype("HolderId")
const AccountId = u64.newtype("AccountId")
const ActiveDuring = interval(i64).newtype("ActiveDuring")

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
	const Holder = relation("Holder", { id: HolderId.fresh, name: str })
	const Account = relation("Account", {
		id: AccountId.fresh,
		holder: HolderId,
		kind: Kind.id,
		active: ActiveDuring
	})
	return { Kind, Grade, Holder, Account }
}

describe("closed relations", function describeClosed() {
	test("handle constants carry declaration-order ids", function probeHandleIds() {
		const { Kind, Grade } = buildLedgerPieces()
		assert.equal(Kind.Checking, 0n)
		assert.equal(Kind.Savings, 1n)
		assert.equal(Grade.DirectPass, 0n)
		assert.equal(Grade.Failed, 1n)
	})

	test("fromId welds ids back to handles and misses beyond the roster", function probeWeld() {
		const { Kind } = buildLedgerPieces()
		assert.equal(Kind.fromId(Kind.Checking), "Checking")
		assert.equal(Kind.fromId(Kind.Savings), "Savings")
		assert.equal(Kind.fromId(forgeKindId(99n)), undefined)
	})

	test("payload readback returns the declared axioms", function probeAxioms() {
		const { Grade } = buildLedgerPieces()
		assert.equal(Grade.axioms.DirectPass.mastered, true)
		assert.equal(Grade.axioms.Failed.mastered, false)
	})

	test("duplicate and reserved handles are construction errors", function probeHandleGuards() {
		assert.throws(function duplicateHandle() {
			closed("Kind", ["Checking", "Checking"])
		}, /duplicate handle Checking/)
		assert.throws(function reservedHandle() {
			closed("Kind", ["Checking", "fromId"])
		}, /collides with the closed value's own surface/)
	})

	test("bare tier lowers with an empty column block and axiom rows", function probeBareLowering() {
		const { Kind } = buildLedgerPieces()
		assert.deepStrictEqual(lowerClosed(Kind), {
			name: "Kind",
			newtype: "Kind",
			fields: [],
			extension: [
				{ handle: "Checking", values: [] },
				{ handle: "Savings", values: [] }
			]
		})
	})

	test("payload tier lowers columns and ground axioms in declaration order", function probePayloadLowering() {
		const { Grade } = buildLedgerPieces()
		assert.deepStrictEqual(lowerClosed(Grade), {
			name: "Grade",
			newtype: "Grade",
			fields: [{ name: "mastered", valueType: { kind: "bool" }, newtype: undefined, fresh: false }],
			extension: [
				{ handle: "DirectPass", values: [{ kind: "value", value: { kind: "bool", value: true } }] },
				{ handle: "Failed", values: [{ kind: "value", value: { kind: "bool", value: false } }] }
			]
		})
	})
})

describe("relations", function describeRelations() {
	test("lowering emits fields in declaration order with newtypes and fresh marks", function probeRelationLowering() {
		const { Account } = buildLedgerPieces()
		assert.deepStrictEqual(lowerRelation(Account), {
			name: "Account",
			newtype: undefined,
			fields: [
				{ name: "id", valueType: { kind: "u64" }, newtype: "AccountId", fresh: true },
				{ name: "holder", valueType: { kind: "u64" }, newtype: "HolderId", fresh: false },
				{ name: "kind", valueType: { kind: "u64" }, newtype: "Kind", fresh: false },
				{
					name: "active",
					valueType: { kind: "interval", element: "i64", width: undefined },
					newtype: "ActiveDuring",
					fresh: false
				}
			],
			extension: undefined
		})
	})

	test("lowered fragments are byte-stable across independent constructions", function probeByteStability() {
		const first = buildLedgerPieces()
		const second = buildLedgerPieces()
		assert.equal(stringify(lowerRelation(first.Account)), stringify(lowerRelation(second.Account)))
		assert.equal(stringify(lowerClosed(first.Grade)), stringify(lowerClosed(second.Grade)))
	})

	test("integer-index field names are rejected (declaration-order law)", function probeNumericFieldName() {
		assert.throws(function numericField() {
			relation("Bad", { "0": u64 })
		}, /integer index/)
	})

	test("fixed-width and branded field constructors validate their grammar bounds", function probeFieldBounds() {
		assert.throws(function zeroBytes() {
			bytes(0)
		}, /1\.\.=64/)
		assert.throws(function wideBytes() {
			bytes(65)
		}, /1\.\.=64/)
		assert.throws(function zeroWidth() {
			interval(u64, 0n)
		}, /width must be >= 1/)
		const fixed = interval(u64, 4n)
		assert.deepStrictEqual(fixed.data.type, { kind: "interval", element: "u64", width: 4n })
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
	test("a closed handle out of roster is a construction error", function probeRosterMiss() {
		const { Kind, Account } = buildLedgerPieces()
		const forged = forgeKindId(7n)
		assert.equal(typeof Kind.Checking, "bigint")
		assert.throws(function outOfRoster() {
			Account.where({ kind: forged })
		}, /closed relation Kind has no handle with id 7/)
	})

	test("an empty selection is rejected naming the bare relation", function probeEmptySelection() {
		const { Account } = buildLedgerPieces()
		assert.throws(function emptyWhere() {
			Account.where({})
		}, /bare relation respelled/)
	})
})
