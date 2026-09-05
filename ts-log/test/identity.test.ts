/**
 * Bounded boundary codecs for the log identity vocabulary. Session tokens
 * from HTTP are untrusted input: every parser refuses malformed widths,
 * uppercase, noncanonical integers and role confusion BEFORE any I/O.
 * Maps to API-06 (application IDs / request-role separation), API-08
 * (identity boundaries) and OPS-006 (scoped origin authority evidence).
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { Result } from "effect"
import {
	CommandDigest,
	DatabaseId,
	IncarnationId,
	OperationId,
	parseCommandRef,
	parseDatabaseIdentity,
	parseDecisionStamp,
	parseStateStamp,
	ReceiptEpoch,
	renderCommandRef,
	renderDatabaseIdentity,
	renderDecisionStamp,
	renderStateStamp,
	RequestId,
	RootId,
	sameCommandRef
} from "#identity.ts"

const DB = "0f".repeat(16)
const INC = "1e".repeat(16)
const SCHEMA = "2d".repeat(32)
const HASH = "3c".repeat(32)
const REQUEST = "4b".repeat(16)
const DIGEST = "5a".repeat(32)

function ok<A, E>(result: Result.Result<A, E>): A {
	assert.ok(Result.isSuccess(result), "expected success")
	return result.success
}

function bad<A, E>(result: Result.Result<A, E>): E {
	assert.ok(Result.isFailure(result), "expected refusal")
	return result.failure
}

describe("identity roles", function suite() {
	test("Id128-backed roles accept exactly 32 lowercase hex", function roles() {
		ok(DatabaseId.fromHex(DB))
		ok(IncarnationId.fromHex(INC))
		ok(RequestId.fromHex(REQUEST))
		ok(OperationId.fromHex(REQUEST))
		bad(DatabaseId.fromHex(DB.toUpperCase()))
		bad(DatabaseId.fromHex(DB.slice(2)))
		bad(DatabaseId.fromHex(`${DB.slice(2)}zz`))
		bad(RequestId.fromHex(""))
	})

	test("RequestId.from is a nominal conversion, not a scalar codec", function nominal() {
		// The canonical Id128 runtime value is its 32-lowercase-hex string.
		const id = REQUEST as unknown as Parameters<typeof RequestId.from>[0]
		const request = ok(RequestId.from(id))
		assert.equal(request, REQUEST)
		// A hostile structural forgery (non-string) refuses instead of casting.
		bad(RequestId.from(42 as unknown as Parameters<typeof RequestId.from>[0]))
	})

	test("receipt epochs are positive u64", function epochs() {
		ok(ReceiptEpoch.from(1n))
		bad(ReceiptEpoch.from(0n))
		bad(ReceiptEpoch.from(-1n))
		bad(ReceiptEpoch.from(0x1_0000_0000_0000_0000_0n))
	})

	test("root IDs are bounded lowercase names", function roots() {
		ok(RootId.fromString("restore-2026-09-04"))
		bad(RootId.fromString(""))
		bad(RootId.fromString("UPPER"))
		bad(RootId.fromString("a".repeat(129)))
	})
})

describe("stamp and ref tokens", function suite() {
	test("decision stamp round-trips and refuses noncanonical forms", function stamps() {
		const stamp = ok(parseDecisionStamp(`7:${HASH}`))
		assert.equal(stamp.seq, 7n)
		assert.equal(renderDecisionStamp(stamp), `7:${HASH}`)
		bad(parseDecisionStamp(`07:${HASH}`)) // leading zero is noncanonical
		bad(parseDecisionStamp(`7:${HASH.toUpperCase()}`))
		bad(parseDecisionStamp(`7:${HASH}:extra`))
		bad(parseDecisionStamp("18446744073709551616:" + HASH)) // > u64
	})

	test("state stamp round-trips", function states() {
		const stamp = ok(parseStateStamp(`${INC}:4`))
		assert.equal(stamp.dataRevision, 4n)
		assert.equal(renderStateStamp(stamp), `${INC}:4`)
		bad(parseStateStamp(`${INC}:`))
		bad(parseStateStamp(`${INC.slice(1)}:4`))
	})

	test("database identity and command ref round-trip", function refs() {
		const identityToken = `${DB}:${INC}:${SCHEMA}`
		const identity = ok(parseDatabaseIdentity(identityToken))
		assert.equal(renderDatabaseIdentity(identity), identityToken)

		const refToken = `${identityToken}:1:${REQUEST}:${DIGEST}`
		const ref = ok(parseCommandRef(refToken))
		assert.equal(renderCommandRef(ref), refToken)
		assert.ok(sameCommandRef(ref, ok(parseCommandRef(refToken))))

		bad(parseCommandRef(`${identityToken}:0:${REQUEST}:${DIGEST}`)) // epoch 0
		bad(parseCommandRef(`${identityToken}:1:${REQUEST}`)) // missing digest
		bad(parseCommandRef(`${identityToken}:1:${DIGEST}:${DIGEST}`)) // 64-hex in a 32-hex role
	})

	test("digest widths are role-exact: 32-hex ids never parse as digests", function widths() {
		bad(CommandDigest.fromHex(REQUEST))
		ok(CommandDigest.fromHex(DIGEST))
	})
})
