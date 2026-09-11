import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime } from "effect"
import * as sdk from "#index.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"
import { releaseCompatibility } from "#test/fixtures/release-compat.ts"
import released from "./fixtures/release-1.2.1.json" with { type: "json" }

test("unchanged schema identity and query results match the actual 1.2.1 release artifacts", async () => {
	const runtime = ManagedRuntime.make(sdk.NativeRuntime.layer(runtimeOptions))
	try {
		const path = storeDir("release-compat")
		assert.deepEqual(await runtime.runPromise(releaseCompatibility(sdk, path)), released.result)
		assert.deepEqual(await runtime.runPromise(releaseCompatibility(sdk, path, true)), released.result)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})
