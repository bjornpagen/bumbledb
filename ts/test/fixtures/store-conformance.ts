import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import { Effect } from "effect"
import type { ObjectStore } from "../../src/database/io.ts"

const bytes = (text: string) => new TextEncoder().encode(text)

/**
 * The create-only contract every `ObjectStore` must meet, run against `store()`. Keys are unique
 * per call of this suite (`run`), so it can run against a shared real bucket.
 */
export function storeConformance(name: string, store: () => ObjectStore, run: string): void {
	const key = (suffix: string) => `${run}/${suffix}`
	const scratch = () => fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-store-"))

	test(`${name}: a key is created once; later creates are refused and keep the first bytes`, async () => {
		const s = store()
		const first = await Effect.runPromise(s.putIfAbsent("Log", key("log/1"), { _tag: "Bytes", bytes: bytes("one") }))
		assert.equal(first.result._tag, "Created")
		const second = await Effect.runPromise(s.putIfAbsent("Log", key("log/1"), { _tag: "Bytes", bytes: bytes("two") }))
		assert.equal(second.result._tag, "Occupied")
		const read = await Effect.runPromise(s.get("Log", key("log/1"), { _tag: "Memory" }))
		assert.equal(read.result._tag, "Body")
		if (read.result._tag === "Body") {
			assert.deepEqual(read.result.bytes, bytes("one"))
			assert.ok(read.result.lastModified > 0n)
		}
		const missing = await Effect.runPromise(s.get("Log", key("log/2"), { _tag: "Memory" }))
		assert.equal(missing.result._tag, "Missing")
	})

	test(`${name}: racing creates of one key have exactly one winner, whose bytes are stored`, async () => {
		const s = store()
		const results = await Effect.runPromise(
			Effect.forEach(
				Array.from({ length: 32 }, (_, index) => index),
				(index) =>
					Effect.map(
						s.putIfAbsent("Log", key("log/race"), { _tag: "Bytes", bytes: bytes(`writer ${index}`) }),
						(reply) => ({ index, tag: reply.result._tag })
					),
				{ concurrency: "unbounded" }
			)
		)
		const winners = results.filter((result) => result.tag === "Created")
		assert.equal(winners.length, 1)
		assert.ok(results.every((result) => result.tag === "Created" || result.tag === "Occupied"))
		const read = await Effect.runPromise(s.get("Log", key("log/race"), { _tag: "Memory" }))
		assert.ok(read.result._tag === "Body")
		assert.deepEqual(read.result.bytes, bytes(`writer ${winners[0]?.index}`))
	})

	test(`${name}: files go up and come down byte for byte`, async () => {
		const s = store()
		const dir = scratch()
		try {
			const source = path.join(dir, "image")
			const payload = new Uint8Array(300_000).map((_, index) => index % 251)
			fs.writeFileSync(source, payload)
			const put = await Effect.runPromise(
				s.putIfAbsent("Checkpoints", key("files/ckpt/image"), { _tag: "File", path: source })
			)
			assert.equal(put.result._tag, "Created")
			const target = path.join(dir, "download")
			const got = await Effect.runPromise(s.get("Checkpoints", key("files/ckpt/image"), { _tag: "File", path: target }))
			assert.equal(got.result._tag, "Saved")
			assert.deepEqual(new Uint8Array(fs.readFileSync(target)), payload)
		} finally {
			fs.rmSync(dir, { recursive: true, force: true })
		}
	})

	test(`${name}: checkpoint listing is lexicographic, paged and prefix-bounded; delete removes`, async () => {
		const s = store()
		for (const suffix of ["c", "a", "b", "d"]) {
			await Effect.runPromise(
				s.putIfAbsent("Checkpoints", key(`listing/ckpt/${suffix}`), { _tag: "Bytes", bytes: bytes(suffix) })
			)
		}
		await Effect.runPromise(s.putIfAbsent("Checkpoints", key("listing/mig/a"), { _tag: "Bytes", bytes: bytes("m") }))
		const listed = await Effect.runPromise(s.list(key("listing/ckpt/"), null, 3))
		assert.deepEqual(listed.result.keys, [key("listing/ckpt/a"), key("listing/ckpt/b"), key("listing/ckpt/c")])
		const next = await Effect.runPromise(s.list(key("listing/ckpt/"), key("listing/ckpt/b"), 10))
		assert.deepEqual(next.result.keys, [key("listing/ckpt/c"), key("listing/ckpt/d")])
		await Effect.runPromise(s.delete(key("listing/ckpt/a")))
		await Effect.runPromise(s.delete(key("listing/ckpt/never")))
		const after = await Effect.runPromise(s.list(key("listing/ckpt/"), null, 10))
		assert.deepEqual(after.result.keys, [key("listing/ckpt/b"), key("listing/ckpt/c"), key("listing/ckpt/d")])
	})
}
