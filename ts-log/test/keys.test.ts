import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { storeKey } from "#keys.ts"

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
	})
})
