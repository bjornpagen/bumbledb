import assert from "node:assert/strict"
import { describe, test } from "node:test"
import {
	CKPT_SCRATCH_LEASE,
	encodeCkptScratch,
	LEASE_NAMESPACE,
	parseCkptScratch,
	reservedName,
	scratchCkptName,
	storeKey,
	TEMP_NAMESPACE
} from "#keys.ts"

describe("the StoreKey grammar", function suite() {
	test("empty is refused", function empty() {
		assert.throws(function emptyKey() {
			storeKey("")
		})
	})

	test("tilde-family, lock-suffix, and format/separator attacks are unrepresentable", function attacks() {
		assert.throws(function fullwidthTmp() {
			storeKey("\uFF5Etmp/x")
		})
		assert.throws(function fullwidthLease() {
			storeKey("\uFF5Elease/manifest.json")
		})
		assert.throws(function lockZwsp() {
			storeKey("manifest.json.lock\u200B")
		})
		assert.throws(function lineSeparator() {
			storeKey("log/\u2028/1")
		})
		assert.throws(function tmpIsNotAKey() {
			storeKey(`${TEMP_NAMESPACE}/ckpt/${"ab".repeat(32)}`)
		})
		assert.throws(function leaseIsNotAKey() {
			storeKey(`${LEASE_NAMESPACE}/${CKPT_SCRATCH_LEASE}`)
		})
	})

	test("the scratch lease is ~lease/ckpt-scratch", function scratch() {
		const digest = "ab".repeat(32)
		assert.equal(scratchCkptName(), `${LEASE_NAMESPACE}/${CKPT_SCRATCH_LEASE}`)
		assert.equal(parseCkptScratch(encodeCkptScratch(digest)), digest)
		assert.equal(parseCkptScratch(new TextEncoder().encode("nope")), null)
		assert.throws(function honest() {
			reservedName("ckpt/head")
		})
	})
})
