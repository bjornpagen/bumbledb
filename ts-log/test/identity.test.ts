/**
 * Bounded boundary codecs for the log identity vocabulary. Session tokens
 * from HTTP are untrusted input: every parser refuses malformed widths,
 * uppercase, noncanonical integers and role confusion BEFORE any I/O.
 * Maps to API-06 (application IDs / request-role separation), API-08
 * (identity boundaries) and OPS-006 (scoped origin authority evidence).
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { Uuid } from "@bjornpagen/bumbledb"
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
	RequestId,
	RootId,
	renderCommandRef,
	renderDatabaseIdentity,
	renderDecisionStamp,
	renderStateStamp,
	sameCommandRef
} from "#identity.ts"

const DB = "0f0f0f0f-0f0f-0f0f-0f0f-0f0f0f0f0f0f"
const INC = "1e1e1e1e-1e1e-1e1e-1e1e-1e1e1e1e1e1e"
const SCHEMA = "2d".repeat(32)
const HASH = "3c".repeat(32)
const REQUEST = "4b4b4b4b-4b4b-4b4b-4b4b-4b4b4b4b4b4b"
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
	test("Uuid-backed roles accept exactly canonical UUID", function roles() {
		ok(DatabaseId.parse(DB))
		ok(IncarnationId.parse(INC))
		ok(RequestId.parse(REQUEST))
		ok(OperationId.parse(REQUEST))
		bad(DatabaseId.parse(DB.toUpperCase()))
		bad(DatabaseId.parse(DB.slice(2)))
		bad(DatabaseId.parse(`${DB.slice(2)}zz`))
		bad(RequestId.parse(""))
	})

	test("RequestId.from is a nominal conversion, not a scalar codec", function nominal() {
		// The canonical Uuid runtime value is its canonical hyphenated UUID string.
		const request = ok(RequestId.from(REQUEST))
		assert.equal(request, REQUEST)
		const core: Uuid = request
		ok(Uuid.toBytes(core))
		// A hostile structural forgery (non-string) refuses instead of casting.
		// @ts-expect-error — deliberately exercise the runtime boundary
		bad(RequestId.from(42))
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
		bad(parseDecisionStamp(`18446744073709551616:${HASH}`)) // > u64
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
		bad(parseCommandRef(`${identityToken}:1:${DIGEST}:${DIGEST}`)) // 64-hex in a UUID role
	})

	test("digest widths are role-exact: UUIDs never parse as digests", function widths() {
		bad(CommandDigest.fromHex(REQUEST))
		ok(CommandDigest.fromHex(DIGEST))
	})
})
