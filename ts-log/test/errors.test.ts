/**
 * One error vocabulary: `LogError = DbError | ProtocolError`. Core failures
 * cross the log surface unchanged — never rewrapped, renamed or copied —
 * and protocol reasons come only from the pinned roster. `.code` derives
 * from the reason tag; there is no second classification authority.
 * Maps to OPS-006 evidence rows and the API-04 certainty error carriage.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { DbError } from "@bjornpagen/bumbledb"
import { protocolErrorCodes } from "#codes.ts"
import { invalidInput, logFailure, ProtocolError, protocolError } from "#errors.ts"

describe("ProtocolError", function suite() {
	test("reason tags come from the pinned roster and drive .code", function reasons() {
		const error = protocolError("History.submit", "DatabaseFrozen")
		assert.equal(error._tag, "ProtocolError")
		assert.equal(error.code, "DatabaseFrozen")
		assert.equal(error.operation, "History.submit")
		assert.ok((protocolErrorCodes as readonly string[]).includes(error.code))
	})

	test("structured reasons retain their bounded payload", function structured() {
		const error = new ProtocolError({
			operation: "History.snapshot",
			reason: { _tag: "NotYetAvailable", requestedSeq: 9n, capturedSeq: 4n }
		})
		assert.equal(error.code, "NotYetAvailable")
		assert.ok(error.reason._tag === "NotYetAvailable")
		assert.equal(error.reason.requestedSeq, 9n)
	})

	test("MaintenanceRequired carries the exact tail envelope measures", function maintenance() {
		const decoded = logFailure("History.submit", {
			source: "protocol",
			reason: { _tag: "MaintenanceRequired", count: 4096n, bytes: 1048576n }
		})
		assert.ok(decoded instanceof ProtocolError)
		assert.equal(decoded.code, "MaintenanceRequired")
		assert.ok(decoded.reason._tag === "MaintenanceRequired")
		assert.equal(decoded.reason.count, 4096n)
		assert.equal(decoded.reason.bytes, 1048576n)
	})

	test("MaterializationStale surfaces the native recovery guidance as data", function stale() {
		// Hydration routing decision (P04R cross-lane note 3): the native side
		// owns hydration; this layer surfaces the typed refusal whose detail
		// routes the caller to reopen (history open / tenant acquire runs
		// recovery). No JS repair path exists to invoke.
		const decoded = logFailure("History.snapshot", {
			source: "protocol",
			reason: {
				_tag: "MaterializationStale",
				detail: "local materialization is behind the checkpoint base; reopen the history to hydrate"
			}
		})
		assert.ok(decoded instanceof ProtocolError)
		assert.equal(decoded.code, "MaterializationStale")
		assert.ok(decoded.reason._tag === "MaterializationStale")
		assert.match(decoded.reason.detail, /reopen/)
	})
})

describe("logFailure", function suite() {
	test("typed errors pass through untouched", function passthrough() {
		const core = invalidInput("x")
		assert.equal(logFailure("y", core), core)
		const protocol = protocolError("x", "Corruption")
		assert.equal(logFailure("y", protocol), protocol)
	})

	test("the protocol wire frame decodes to ProtocolError", function protocolFrame() {
		const decoded = logFailure("History.open", {
			source: "protocol",
			reason: { _tag: "DatabaseMissing" }
		})
		assert.ok(decoded instanceof ProtocolError)
		assert.equal(decoded.code, "DatabaseMissing")
	})

	test("plain and structured native failures preserve their detail in thrown and outcome decoding", () => {
		for (const reason of [
			{ _tag: "Misuse", detail: "command frame: Truncated { at: 32 }" },
			{ _tag: "Corruption", detail: "persisted command frame" },
			{ _tag: "Contention", attempts: 4, detail: "conditional head changed" },
			{ _tag: "MaintenanceRequired", count: 8n, bytes: 100n, detail: "tail capacity" }
		]) {
			const decoded = logFailure("boundary", { source: "protocol", reason })
			assert.ok(decoded instanceof ProtocolError)
			assert.deepEqual(decoded.reason, reason)
			assert.equal(logFailure("another boundary", decoded), decoded)
		}
	})

	test("resource measurements remain lossless core errors", () => {
		const reason = {
			_tag: "ResourceLimit",
			dimension: "change",
			used: 0n,
			requested: (1n << 64n) - 1n,
			limit: 1n << 26n
		}
		const error = logFailure("Command.decode", { source: "core", reason })
		assert.ok(error instanceof DbError)
		assert.deepEqual(error.reason, reason)
		assert.equal(logFailure("History.submit", error), error)
	})

	test("the core wire frame decodes to the exact core DbError", function coreFrame() {
		const decoded = logFailure("History.open", {
			source: "core",
			reason: { _tag: "DirectoryBusy" }
		})
		assert.ok(decoded instanceof DbError)
		assert.equal(decoded.code, "DirectoryBusy")
	})

	test("an unknown local throw becomes core Internal, never a fabricated protocol outcome", function unknownThrow() {
		const decoded = logFailure("History.open", new Error("boom"))
		assert.ok(decoded instanceof DbError)
		assert.equal(decoded.code, "Internal")
	})

	test("a malformed protocol reason does not forge a roster code", function malformed() {
		const decoded = logFailure("History.open", {
			source: "protocol",
			reason: { _tag: "NotARealCode" }
		})
		assert.ok(decoded instanceof DbError)
		assert.equal(decoded.code, "Internal")
	})
})

describe("roster hygiene", function suite() {
	test("the roster is unique and never respells a core code", function hygiene() {
		assert.equal(new Set(protocolErrorCodes).size, protocolErrorCodes.length)
		const coreCodes = [
			"RuntimeAlreadyLive",
			"ForeignRuntime",
			"ClosedHandle",
			"SpentHandle",
			"QueueFull",
			"InvalidArgument",
			"Internal",
			"DirectoryBusy",
			"InvalidPath",
			"Io",
			"ResourceLimit",
			"Cancelled",
			"DeadlineExceeded"
		]
		for (const code of coreCodes) {
			assert.ok(!(protocolErrorCodes as readonly string[]).includes(code), `core code respelled: ${code}`)
		}
	})
})
