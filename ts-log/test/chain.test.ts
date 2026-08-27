import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { LogCodecHandle } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { digest32, digest32FromHex, toHex } from "#bytes.ts"
import type { Chain } from "#chain.ts"
import { parseSidecar, renderSidecar } from "#chain.ts"
import { braid, descriptorOf } from "#descriptor.ts"
import { ErrRefused, refusalOf } from "#errors.ts"
import { generation } from "#keys.ts"
import { Grid, Ledger } from "#test/fixtures.ts"

const HOME = braid("c00000000")
const NOTES = braid("c00000002")
const ZERO = digest32(new Uint8Array(32))
const ZERO_HEX = "0".repeat(64)

/** The sealed handle is the braid authority: parse and render walk it. */
const LEDGER = descriptorOf(Ledger).codec
/** Grid's decomposition mints one braid; Ledger's Note braid is foreign to it. */
const GRID = descriptorOf(Grid).codec

function genesis(): Chain {
	return {
		tag: "settled",
		entries: new Map([[HOME, { g: generation(0n), prev: ZERO, ts: 0n }]])
	}
}

function refuseKind(codec: LogCodecHandle, bytes: Uint8Array): string {
	const ran = errors.trySync(function parseIt() {
		return parseSidecar(codec, bytes)
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
		const bytes = renderSidecar(LEDGER, chain)
		const parsed = parseSidecar(LEDGER, bytes)
		const entry = parsed.entries.get(HOME)
		assert.ok(entry !== undefined)
		assert.ok(entry.prev instanceof Uint8Array)
		assert.equal(entry.prev.length, 32)
		assert.deepEqual(entry.prev, digest32FromHex(ZERO_HEX))
		assert.equal(toHex(renderSidecar(LEDGER, parsed)), toHex(bytes))
	})

	test("a leading byte other than 3 is Version", function version() {
		const bytes = renderSidecar(LEDGER, genesis())
		bytes[0] = 2
		assert.equal(refuseKind(LEDGER, bytes), "Version")
	})

	test("a braid outside the handle's decomposition refuses", function unknownBraid() {
		const bytes = renderSidecar(LEDGER, {
			tag: "settled",
			entries: new Map([[NOTES, { g: generation(0n), prev: ZERO, ts: 0n }]])
		} satisfies Chain)
		assert.equal(refuseKind(GRID, bytes), "UnknownBraid")
	})

	test("trailing bytes refuse", function trailing() {
		const bytes = renderSidecar(LEDGER, genesis())
		const padded = new Uint8Array(bytes.length + 1)
		padded.set(bytes)
		assert.equal(refuseKind(LEDGER, padded), "Malformed")
	})
})
