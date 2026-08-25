import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as path from "node:path"
import { describe, test } from "node:test"

const srcRoot = path.resolve(import.meta.dirname, "../src")

function sourceOf(name: string): string {
	return fs.readFileSync(path.join(srcRoot, name), "utf8")
}

/** 70's temporal law: async ⟺ network. The pure protocol set is
 *  synchronous and store-blind; every exported async surface awaits a
 *  store verb on some path. */
describe("the temporal gate", function suite() {
	test("the pure protocol modules are synchronous and never import the store", function pure() {
		for (const name of ["codec.ts", "braids.ts", "value.ts", "bytes.ts", "descriptor.ts", "keys.ts", "document.ts"]) {
			const source = sourceOf(name)
			assert.ok(!source.includes("async "), `${name} declares an async function`)
			assert.ok(!source.includes("await "), `${name} awaits`)
			assert.ok(!source.includes('#store.ts"'), `${name} imports the store`)
		}
	})

	test("every store verb consumer awaits it; the verb set is exactly five", function verbs() {
		const verbPattern = /await\s+(?:core\.|options\.)?store\.(get|getIfChanged|putCreate|putSwap|delete)\(/g
		const replica = sourceOf("replica.ts")
		const writer = sourceOf("writer.ts")
		const replicaVerbs = new Set([...replica.matchAll(verbPattern)].map((hit) => hit[1]))
		const writerVerbs = new Set([...writer.matchAll(verbPattern)].map((hit) => hit[1]))
		assert.ok(replicaVerbs.has("get"), "the replica probes with get")
		assert.ok(replicaVerbs.has("getIfChanged"), "the heartbeat polls with getIfChanged")
		assert.ok(!replicaVerbs.has("putCreate"), "a replica never genesis-creates")
		assert.ok(writerVerbs.has("putCreate"), "the writer births the store and arbitrates slots")
		assert.ok(writerVerbs.has("putSwap"), "the id lease CAS swaps")
		const unknownVerb = /store\.(?!get|getIfChanged|putCreate|putSwap|delete)[a-zA-Z]+\(/
		assert.ok(!unknownVerb.test(replica) && !unknownVerb.test(writer), "a sixth verb appeared")
	})

	test("every exported async surface reaches a store verb on some path", function surfaces() {
		const replica = sourceOf("replica.ts")
		for (const surface of ["async refresh(", "async waitFor(", "async [Symbol.asyncDispose]("]) {
			assert.ok(replica.includes(surface), `replica surface missing: ${surface}`)
		}
		assert.ok(replica.includes("async function openCore"), "openReplica opens through the store")
		const writer = sourceOf("writer.ts")
		for (const surface of ["async function openWriter", "async commit(", "async commitSplit("]) {
			assert.ok(writer.includes(surface), `writer surface missing: ${surface}`)
		}
		const birthAt = writer.indexOf("await birthStore(")
		const openAt = writer.indexOf("await openReplica(")
		assert.ok(birthAt >= 0 && openAt > birthAt, "openWriter births the store, then opens the replica")
		const tenants = sourceOf("tenants.ts")
		assert.ok(tenants.includes("await openReplica("), "tenants.get awaits the replica open, which awaits the store")
	})

	test("the batch recorders are synchronous by law", function syncRecorders() {
		const writer = sourceOf("writer.ts")
		const recorder = writer.slice(writer.indexOf("const batch: Batch"), writer.indexOf("return { batch, recording }"))
		assert.ok(!recorder.includes("await"), "batch.insert/delete/reserve are pure recorders")
	})
})
