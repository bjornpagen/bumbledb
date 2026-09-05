import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect } from "effect"
import {
	AuthoringError,
	ErrAsyncCallback,
	ErrFingerprintMismatch,
	ErrForeignPrepared,
	ErrForeignWitness,
	ErrIrError,
	ErrNewtypeMismatch,
	ErrSchemaError,
	ErrSpentHandle,
	ErrUseAfterScope,
	NativeLoadError,
	NativeOperationError,
	NativeReportedError,
	span
} from "#index.ts"
import { bridged, bridgedAsync, errorFromThrow } from "#native.ts"

test("pure authoring failures are directly matchable Effect tagged errors", () => {
	assert.throws(
		() => span(4n, 4n),
		(error: unknown) => error instanceof AuthoringError && error._tag === "AuthoringError"
	)
	const error = new AuthoringError({ message: "bad authoring" })
	const recovered = Effect.runSync(
		Effect.fail(error).pipe(Effect.catchTag("AuthoringError", (failure) => Effect.succeed(failure.message)))
	)
	assert.equal(recovered, "bad authoring")
})

test("legacy Err exports are classes carrying semantic fields, not singleton sentinels", () => {
	const operation = "create"
	const path = "/tmp/example"
	const message = "refused"
	const failures = [
		new ErrAsyncCallback({ scope: "write", message }),
		new ErrSpentHandle({ handle: "witness", state: "disposed", message }),
		new ErrUseAfterScope({ handle: "readInstance", message }),
		new ErrForeignPrepared({ reason: "foreignStore", message }),
		new ErrForeignWitness({ reason: "notWitness", message }),
		new ErrNewtypeMismatch({ operation, path, message }),
		new ErrSchemaError({ operation, path, message }),
		new ErrFingerprintMismatch({ operation, path, message }),
		new ErrIrError({ operation: "prepare", message })
	]
	assert.equal(new Set(failures.map((failure) => failure._tag)).size, 9)
	for (const failure of failures) {
		assert.ok(failure instanceof Error)
		assert.equal(failure.message, message)
		assert.throws(
			() =>
				bridged("probe", () => {
					throw failure
				}),
			(caught: unknown) => caught === failure
		)
	}
})

test("native contexts preserve exact falsy and non-Error thrown values as causes", async () => {
	for (const cause of [undefined, null, false, 0, "", Symbol("host"), { raw: "host" }]) {
		assert.throws(
			() =>
				bridged("sync probe", () => {
					throw cause
				}),
			(error: unknown) =>
				error instanceof NativeOperationError && error.operation === "sync probe" && error.cause === cause
		)
		await assert.rejects(
			bridgedAsync("async probe", () => Promise.reject(cause)),
			(error: unknown) =>
				error instanceof NativeOperationError && error.operation === "async probe" && error.cause === cause
		)
		const described = errorFromThrow(cause)
		assert.ok(described instanceof NativeReportedError)
		assert.equal(described.cause, cause)
	}
})

test("bridge boundaries preserve existing Error and tagged-error identity", async () => {
	const cause = new Error("original host failure")
	assert.throws(
		() =>
			bridged("sync", () => {
				throw cause
			}),
		(caught: unknown) => caught === cause
	)
	await assert.rejects(
		bridgedAsync("async", () => Promise.reject(cause)),
		(caught: unknown) => caught === cause
	)
	assert.equal(errorFromThrow(cause), cause)
})

test("native error records and contextual failures keep causes without ad-hoc mutation", () => {
	const cause = { kind: "io", message: "disk unavailable", extra: 17 }
	const error = errorFromThrow(cause)
	assert.ok(error instanceof NativeReportedError)
	assert.equal(error.kind, "io")
	assert.equal(error.cause, cause)
	assert.match(error.message, /disk unavailable/)
	const outer = new NativeOperationError({ operation: "build write delta", cause: error })
	assert.equal(outer.cause, error)
	assert.match(outer.message, /build write delta.*disk unavailable/)
	const missing = new NativeLoadError({ package: "native-test", operation: "resolve", message: "missing", cause })
	assert.equal(missing._tag, "NativeLoadError")
	assert.equal(missing.package, "native-test")
	assert.equal(missing.cause, cause)
})
