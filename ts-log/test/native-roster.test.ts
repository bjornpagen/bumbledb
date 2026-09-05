/**
 * Pins the typed log wire declaration against the actual loaded addon: the
 * protocol error-code roster must match `logErrorCodes()` exactly (one
 * native speller), and every declared verb must exist as a function on the
 * one shared binding. Runs against the packaged native implementation in
 * F3 — a missing verb here is a real integration failure, not a skip.
 * Maps to OPS-006 and the C10 log-addition roster (FFI-02 layer side).
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { protocolErrorCodes } from "#codes.ts"
import { logNative } from "#native.ts"

const verbs = [
	"logErrorCodes",
	"logHistoryOpen",
	"logHistoryTake",
	"logHistoryCall",
	"logHistoryResult",
	"logHistoryClose",
	"logSnapshotClose",
	"logCommandSeal",
	"logCommandDecode",
	"logCommandTake",
	"logCommandEncode",
	"logBytesTake",
	"logCommandClose",
	"logCacheMake",
	"logCacheTake",
	"logCacheAcquire",
	"logBorrowTake",
	"logCacheInspect",
	"logCacheInspectTake",
	"logCacheEvict",
	"logCacheEvictTake",
	"logBorrowRelease",
	"logCacheClose",
	"logAdmin",
	"logAdminTake",
	// Shared with the runtime surface; cancellation joins one registry.
	"runtimeCancel"
] as const

describe("the one addon carries the declared log roster", function suite() {
	test("every declared verb is a function on the shared binding", function roster() {
		for (const verb of verbs) {
			assert.equal(typeof logNative[verb], "function", `missing native log verb: ${verb}`)
		}
	})

	test("the protocol code roster matches the native speller exactly", function codes() {
		assert.deepEqual([...logNative.logErrorCodes()], [...protocolErrorCodes])
	})
})
