import assert from "node:assert/strict"
import { describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import { digest32, digest32FromHex, hex32, toHex } from "#bytes.ts"
import type { Chain } from "#chain.ts"
import { parseSidecar, renderSidecar } from "#chain.ts"
import type { Braid } from "#descriptor.ts"
import { braid } from "#descriptor.ts"
import { ErrRefused, refusalOf } from "#errors.ts"
import { generation } from "#keys.ts"

const HOME = braid("c00000000")
const ZERO = digest32(new Uint8Array(32))
const ZERO_HEX = "0".repeat(64)

function genesis(): Chain {
	return {
		tag: "settled",
		entries: new Map([[HOME, { g: generation(0n), prev: ZERO, ts: 0n }]])
	}
}

function refuseKind(bytes: Uint8Array, known?: ReadonlySet<Braid>): string {
	const ran = errors.trySync(function parseIt() {
		return parseSidecar(bytes, known)
	})
	assert.ok(ran.error, "expected a refusal")
	assert.ok(errors.is(ran.error, ErrRefused), `expected ErrRefused, got: ${ran.error.message}`)
	const cause = refusalOf(ran.error)
	assert.ok(cause !== undefined, "refusal carries its cause")
	return cause.kind
}

describe("the chain sidecar", function suite() {
	test("prev is 32 raw bytes", function digestPrev() {
		const chain = genesis()
		const bytes = renderSidecar(chain)
		const parsed = parseSidecar(bytes)
		const entry = parsed.entries.get(HOME)
		assert.ok(entry !== undefined)
		assert.ok(entry.prev instanceof Uint8Array)
		assert.equal(entry.prev.length, 32)
		assert.equal(hex32(entry.prev), ZERO_HEX)
		assert.deepEqual(entry.prev, digest32FromHex(ZERO_HEX))
		assert.equal(toHex(renderSidecar(parsed)), toHex(bytes))
	})

	test("a leading byte other than 3 is Version", function version() {
		const bytes = renderSidecar(genesis())
		bytes[0] = 2
		assert.equal(refuseKind(bytes), "Version")
	})

	test("an unknown braid refuses", function unknownBraid() {
		const foreign = braid("c0000ffff")
		const bytes = renderSidecar({
			tag: "settled",
			entries: new Map([[foreign, { g: generation(0n), prev: ZERO, ts: 0n }]])
		} satisfies Chain)
		assert.equal(refuseKind(bytes, new Set([HOME])), "UnknownBraid")
	})

	test("trailing bytes refuse", function trailing() {
		const bytes = renderSidecar(genesis())
		const padded = new Uint8Array(bytes.length + 1)
		padded.set(bytes)
		assert.equal(refuseKind(padded), "TrailingBytes")
	})
})
