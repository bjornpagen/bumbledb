import assert from "node:assert/strict"
import { describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import { digest32FromHex, hex32, toHex } from "#bytes.ts"
import { parseSidecar, renderSidecar } from "#chain.ts"
import { braid } from "#descriptor.ts"
import { ErrRefused, refusalOf } from "#errors.ts"

const ZERO_HEX = "0".repeat(64)

function utf8(text: string): Uint8Array {
	return new TextEncoder().encode(text)
}

function sidecar(prev: string): string {
	return `{"v":3,"chain":{"c00000000":{"g":"0","prev":"${prev}","ts":"0"}},"pending":null}`
}

function refuseKind(prev: string): string {
	const ran = errors.trySync(function parseIt() {
		return parseSidecar(utf8(sidecar(prev)))
	})
	assert.ok(ran.error, "expected a refusal")
	assert.ok(errors.is(ran.error, ErrRefused), `expected ErrRefused, got: ${ran.error.message}`)
	const cause = refusalOf(ran.error)
	assert.ok(cause !== undefined, "refusal carries its cause")
	return cause.kind
}

describe("the chain sidecar", function suite() {
	test("prev is Digest32 in memory and lowercase hex on the wire", function digestPrev() {
		const bytes = utf8(sidecar(ZERO_HEX))
		const chain = parseSidecar(bytes)
		const entry = chain.entries.get(braid("c00000000"))
		assert.ok(entry !== undefined)
		assert.ok(entry.prev instanceof Uint8Array)
		assert.equal(entry.prev.length, 32)
		assert.equal(hex32(entry.prev), ZERO_HEX)
		assert.deepEqual(entry.prev, digest32FromHex(ZERO_HEX))
		assert.equal(toHex(utf8(renderSidecar(chain))), toHex(bytes))
	})

	test("a short prev refuses", function shortPrev() {
		assert.equal(refuseKind("aabb"), "Malformed")
	})

	test("an odd-length prev refuses", function oddPrev() {
		assert.equal(refuseKind("0".repeat(63)), "Malformed")
	})

	test("an uppercase prev refuses", function uppercasePrev() {
		assert.equal(refuseKind("A".repeat(64)), "Malformed")
	})
})
