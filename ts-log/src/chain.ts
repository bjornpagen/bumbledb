/**
 * The chain sidecar (50): the per-braid split and chain position in
 * `dir/chain.json`, written atomically (temp + rename, fsync). A floor
 * cache, never a truth the store reconciles against — recovery is the
 * catch-up loop, and the one wholeness check lives at the replica.
 * `pending` is the writer's one extra field: the encoded batch bytes a
 * local commit owes the log, present until its slot exists.
 */

import * as crypto from "node:crypto"
import * as fs from "node:fs/promises"
import * as path from "node:path"
import * as errors from "@superbuilders/errors"
import { utf8StrictDecoder } from "#bytes.ts"
import type { ChainEntry } from "#codec.ts"

interface Pending {
	readonly braid: string
	readonly gen: bigint
	readonly bytes: Uint8Array
}

interface Sidecar {
	readonly chain: ReadonlyMap<string, ChainEntry>
	readonly pending: Pending | null
}

function renderSidecar(sidecar: Sidecar): string {
	const braids = [...sidecar.chain.keys()].sort()
	const chain = braids
		.map(function renderEntry(braid) {
			const entry = sidecar.chain.get(braid)
			if (entry === undefined) {
				throw errors.new(`sidecar chain lost braid ${braid}`)
			}
			return `"${braid}":{"g":${entry.g},"prev":"${entry.prev}","ts":${entry.ts}}`
		})
		.join(",")
	const pending =
		sidecar.pending === null
			? "null"
			: `{"braid":"${sidecar.pending.braid}","gen":${sidecar.pending.gen},"bytes":"${Buffer.from(sidecar.pending.bytes).toString("base64")}"}`
	return `{"v":2,"chain":{${chain}},"pending":${pending}}`
}

function parseSidecar(text: string): Sidecar {
	const parsed = JSON.parse(text) as {
		v: number
		chain: Record<string, { g: number; prev: string; ts: number }>
		pending: { braid: string; gen: number; bytes: string } | null
	}
	if (parsed.v !== 2 || typeof parsed.chain !== "object" || parsed.chain === null) {
		throw errors.new("sidecar is not a v2 chain file")
	}
	const chain = new Map<string, ChainEntry>()
	for (const [braid, entry] of Object.entries(parsed.chain)) {
		chain.set(braid, { g: BigInt(entry.g), prev: entry.prev, ts: BigInt(entry.ts) })
	}
	const pending =
		parsed.pending === null
			? null
			: {
					braid: parsed.pending.braid,
					gen: BigInt(parsed.pending.gen),
					bytes: new Uint8Array(Buffer.from(parsed.pending.bytes, "base64"))
				}
	return { chain, pending }
}

async function readSidecar(file: string): Promise<Sidecar | null> {
	const read = await errors.try(fs.readFile(file))
	if (read.error) {
		return null
	}
	return parseSidecar(utf8StrictDecoder.decode(read.data))
}

async function writeSidecar(file: string, sidecar: Sidecar): Promise<void> {
	const dir = path.dirname(file)
	await fs.mkdir(dir, { recursive: true })
	const temp = path.join(dir, `.chain-${process.pid}-${crypto.randomBytes(4).toString("hex")}`)
	const handle = await fs.open(temp, "wx")
	const written = await errors.try(
		(async function writeAll() {
			await handle.writeFile(renderSidecar(sidecar))
			await handle.sync()
		})()
	)
	await handle.close()
	if (written.error) {
		await fs.rm(temp, { force: true })
		throw errors.wrap(written.error, `write sidecar ${file}`)
	}
	await fs.rename(temp, file)
	const dirHandle = await fs.open(dir, "r")
	const synced = await errors.try(dirHandle.sync())
	await dirHandle.close()
	if (synced.error) {
		throw errors.wrap(synced.error, `fsync sidecar directory ${dir}`)
	}
}

export type { ChainEntry, Pending, Sidecar }
export { readSidecar, renderSidecar, writeSidecar }
