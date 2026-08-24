/**
 * The interop lane's Node half: driven by the Rust orchestrator in
 * crates/bumbledb-log/tests/lane_b_interop.rs over one FsStore prefix.
 * Roles print structured `INTEROP {...}` lines; the parent asserts hard.
 *
 * Usage: node interop-child.ts <role> <root> [args...]
 *   write <root>                     — putCreate the shared corpus, report etags
 *   read <root> <key...>             — get each key, report bytes hex and etag
 *   race-create <root> <id> <slots>  — contend on every slot after the go barrier
 *   race-swap <root> <id> <count>    — land <count> CAS increments on the counter
 *   read s3 <prefix> <key...>        — same read over s3Store; vendor etag, not blake3
 */

import * as fs from "node:fs"
import * as path from "node:path"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { toHex } from "#bytes.ts"
import { storeKey } from "#keys.ts"
import type { ObjectStore } from "#store.ts"
import { fsStore, s3Store } from "#store.ts"

/** The shared corpus rule, implemented identically in the Rust test:
 *  body[j] of object i is (i * 31 + j * 7) mod 256. */
const CORPUS_SIZES = [0, 1, 3, 256, 4096, 65536]

function corpusKey(index: number) {
	return storeKey(`interop/obj-${index}`)
}

function corpusBody(index: number): Uint8Array {
	const size = CORPUS_SIZES[index]
	if (size === undefined) {
		throw errors.new(`corpus has no object ${index}`)
	}
	const body = new Uint8Array(size)
	for (let j = 0; j < size; j++) {
		body[j] = (index * 31 + j * 7) % 256
	}
	return body
}

function blake3Hex(bytes: Uint8Array): string {
	return toHex(new Uint8Array(internalBlake3(bytes)))
}

function report(line: Record<string, unknown>): void {
	process.stdout.write(`INTEROP ${JSON.stringify(line)}\n`)
}

async function waitForGo(root: string): Promise<void> {
	const go = path.join(root, "..", "go")
	const deadline = Date.now() + 20_000
	while (!fs.existsSync(go)) {
		if (Date.now() > deadline) {
			throw errors.new("start barrier never appeared")
		}
		await new Promise(function later(resolve) {
			setTimeout(resolve, 2)
		})
	}
}

async function main(): Promise<void> {
	const [role, root, ...rest] = process.argv.slice(2)
	if (role === undefined || root === undefined) {
		throw errors.new("usage: interop-child.ts <role> <root> [args...]")
	}
	const s3 = root === "s3"
	const store: ObjectStore = s3 ? s3FromEnv(rest.shift() ?? "") : fsStore(root)

	if (role === "write") {
		for (let i = 0; i < CORPUS_SIZES.length; i++) {
			const body = corpusBody(i)
			const outcome = await store.putCreate(corpusKey(i), body)
			if (outcome.tag !== "created") {
				throw errors.new(`corpus object ${i} was not created`)
			}
			report({ role, key: corpusKey(i), etag: outcome.etag })
		}
		return
	}

	if (role === "read") {
		for (const key of rest) {
			const fetched = await store.get(storeKey(key))
			if (fetched === null) {
				throw errors.new(`object ${key} is absent`)
			}
			if (!s3 && fetched.etag !== blake3Hex(fetched.bytes)) {
				throw errors.new(`object ${key}: the reported etag is not the blake3 of the bytes`)
			}
			report({ role, key, hex: toHex(fetched.bytes), etag: fetched.etag })
		}
		return
	}

	if (role === "race-create") {
		const [id, slots] = rest
		if (id === undefined || slots === undefined) {
			throw errors.new("race-create needs <id> <slots>")
		}
		await waitForGo(root)
		for (let s = 0; s < Number.parseInt(slots, 10); s++) {
			const body = new TextEncoder().encode(`ts-${id}-slot-${s}`)
			const outcome = await store.putCreate(storeKey(`race/slot-${s}`), body)
			report({
				role,
				id,
				slot: s,
				outcome: outcome.tag,
				etag: createdEtag(outcome, body, s3)
			})
		}
		return
	}

	if (role === "race-swap") {
		const [id, count] = rest
		if (id === undefined || count === undefined) {
			throw errors.new("race-swap needs <id> <count>")
		}
		await waitForGo(root)
		let swapped = 0
		let moved = 0
		const target = Number.parseInt(count, 10)
		while (swapped < target) {
			const current = await store.get(storeKey("race/counter"))
			if (current === null) {
				throw errors.new("the counter is absent")
			}
			const value = Number.parseInt(new TextDecoder().decode(current.bytes), 10)
			const next = new TextEncoder().encode(String(value + 1))
			const outcome = await store.putSwap(storeKey("race/counter"), next, current.etag)
			if (outcome.tag === "swapped") {
				if (!s3 && outcome.etag !== blake3Hex(next)) {
					throw errors.new("a swapped etag is not the blake3 of the written bytes")
				}
				swapped += 1
			} else {
				moved += 1
			}
		}
		report({ role, id, swapped, moved })
		return
	}

	throw errors.new(`unknown role: ${role}`)
}

function createdEtag(outcome: { tag: string; etag?: string }, body: Uint8Array, s3: boolean): string {
	if (outcome.tag === "created" && outcome.etag !== undefined) {
		return outcome.etag
	}
	return s3 ? "" : blake3Hex(body)
}

function s3FromEnv(prefix: string): ObjectStore {
	const bucket = process.env.BUMBLEDB_S3_SMOKE_BUCKET
	const accessKeyId = process.env.AWS_ACCESS_KEY_ID
	const secretAccessKey = process.env.AWS_SECRET_ACCESS_KEY
	if (bucket === undefined || accessKeyId === undefined || secretAccessKey === undefined) {
		throw errors.new("s3 interop needs BUMBLEDB_S3_SMOKE_BUCKET and AWS keys")
	}
	const region = process.env.BUMBLEDB_S3_SMOKE_REGION ?? "us-east-1"
	const endpoint = process.env.BUMBLEDB_S3_SMOKE_ENDPOINT
	const sessionToken = process.env.AWS_SESSION_TOKEN
	return s3Store({
		region,
		bucket,
		credentials: {
			accessKeyId,
			secretAccessKey,
			...(sessionToken === undefined ? {} : { sessionToken })
		},
		prefix,
		...(endpoint === undefined ? {} : { endpoint })
	})
}

await main()
