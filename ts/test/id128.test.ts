/**
 * Id128 — the ordinary application-owned 128-bit identity (API-06 typed
 * half): pure fixed-size parsing with `Result`, effectful cryptographic
 * entropy with NO module-import side effect, canonical-spelling refusals,
 * and byte round-trips. No allocator, reservation, FreshRef or ID-burn
 * surface exists to test — their absence is pinned in types.test.ts.
 */
import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Result } from "effect"
import { Id128 } from "#id128.ts"

const CANONICAL = "00112233445566778899aabbccddeeff"

test("fromHex admits exactly the canonical 32-lowercase-hex spelling", function fromHexCanonical() {
	const parsed = Id128.fromHex(CANONICAL)
	assert.ok(Result.isSuccess(parsed))
	assert.equal(parsed.success, CANONICAL)

	for (const wrong of [
		"00112233445566778899AABBCCDDEEFF", // uppercase
		"00112233-4455-6677-8899-aabbccddeeff", // UUID punctuation
		"0x112233445566778899aabbccddeeff", // 0x prefix
		"00112233445566778899aabbccddeef", // 31 chars
		"00112233445566778899aabbccddeeff0", // 33 chars
		"g0112233445566778899aabbccddeeff", // non-hex
		"" // empty
	]) {
		const refused = Id128.fromHex(wrong)
		assert.ok(Result.isFailure(refused), `${JSON.stringify(wrong)} must refuse`)
		assert.equal(refused.failure.reason._tag, "InvalidArgument")
	}
})

test("fromBytes/toBytes round-trip the sixteen exact bytes; wrong widths refuse", function bytesRoundTrip() {
	const bytes = Uint8Array.from({ length: 16 }, (_, index) => index * 16 + index)
	const parsed = Id128.fromBytes(bytes)
	assert.ok(Result.isSuccess(parsed))
	const back = Id128.toBytes(parsed.success)
	assert.deepEqual([...back], [...bytes])
	// toBytes yields a FRESH owned copy each call — caller mutation cannot
	// alias a later read.
	back[0] = 0xff
	assert.deepEqual([...Id128.toBytes(parsed.success)], [...bytes])

	for (const wrong of [new Uint8Array(15), new Uint8Array(17), new Uint8Array(0)]) {
		assert.ok(Result.isFailure(Id128.fromBytes(wrong)))
	}
})

test("random is a LAZY effect of cryptographic entropy — construction generates nothing", async function randomIsLazy() {
	const effect = Id128.random()
	// Two executions of one constructed effect are two generations: no
	// hidden memoization of database-adjacent calls (chapter 35 laziness).
	const first = await Effect.runPromise(effect)
	const second = await Effect.runPromise(effect)
	assert.ok(Id128.isId128(first))
	assert.ok(Id128.isId128(second))
	assert.notEqual(first, second, "two runs, two identities (collision is 2^-128)")
})

test("isId128 is the runtime brand test over the canonical spelling", function brandTest() {
	assert.ok(Id128.isId128(CANONICAL))
	assert.equal(Id128.isId128("not-an-id"), false)
	assert.equal(Id128.isId128(123), false)
	assert.equal(Id128.isId128(CANONICAL.toUpperCase()), false)
})
