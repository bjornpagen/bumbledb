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
import { hex32, saturatingAddU64 } from "#bytes.ts"
import type { ChainEntry } from "#codec.ts"
import type { Braid } from "#descriptor.ts"
import { braid } from "#descriptor.ts"
import { DOC_VERSION, pendingHex, Text } from "#document.ts"
import { refuse } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"

interface Pending {
	readonly braid: Braid
	readonly gen: Generation
	readonly bytes: Uint8Array
}

interface Sidecar {
	readonly chain: ReadonlyMap<Braid, ChainEntry>
	readonly pending: Pending | null
	readonly sum?: bigint
}

function renderSidecar(sidecar: Sidecar): string {
	const braids = [...sidecar.chain.keys()].sort()
	const chain = braids
		.map(function renderEntry(id) {
			const entry = sidecar.chain.get(id)
			if (entry === undefined) {
				throw errors.new(`sidecar chain lost braid ${id}`)
			}
			return `"${id}":{"g":"${entry.g}","prev":"${entry.prev}","ts":"${entry.ts}"}`
		})
		.join(",")
	const pending =
		sidecar.pending === null
			? "null"
			: `{"braid":"${sidecar.pending.braid}","gen":"${sidecar.pending.gen}","bytes":"${pendingHex(sidecar.pending.bytes)}"}`
	return `{"v":${DOC_VERSION},"chain":{${chain}},"pending":${pending}}`
}

function parseSidecar(bytes: Uint8Array, known?: ReadonlySet<Braid>): Sidecar {
	const text = new Text(bytes)
	if (!text.lit('{"v":')) {
		refuse({ kind: "SidecarShape" }, "sidecar is not the canonical template")
	}
	const version = text.u64()
	if (version === undefined) {
		refuse({ kind: "SidecarShape" }, "sidecar version is not a canonical u64")
	}
	if (version !== DOC_VERSION) {
		refuse({ kind: "Version", version: Number(version) }, `sidecar version ${version}, consumers refuse ≠ ${DOC_VERSION}`)
	}
	if (!text.lit(',"chain":{')) {
		refuse({ kind: "SidecarShape" }, "sidecar chain field is absent")
	}
	const chain = new Map<Braid, ChainEntry>()
	let first = true
	let sum = 0n
	while (!text.peek("}")) {
		if (!first && !text.lit(",")) {
			refuse({ kind: "SidecarShape" }, "sidecar chain is not comma-separated")
		}
		first = false
		if (!text.lit('"c')) {
			refuse({ kind: "SidecarShape" }, "sidecar braid id is not a c-prefixed hex")
		}
		const raw = text.hexU32()
		if (raw === undefined) {
			refuse({ kind: "SidecarShape" }, "sidecar braid id is not 8 hex")
		}
		const name = braid(`c${raw.toString(16).padStart(8, "0")}`)
		if (known !== undefined && !known.has(name)) {
			refuse({ kind: "SidecarShape" }, `sidecar cites unknown braid ${name}`)
		}
		if (!text.lit('":{"g":')) {
			refuse({ kind: "SidecarShape" }, `sidecar braid ${name} entry is malformed`)
		}
		const g = text.quotedU64()
		if (g === undefined || !text.lit(',"prev":"')) {
			refuse({ kind: "SidecarShape" }, `sidecar braid ${name} generation is not a quoted decimal u64`)
		}
		const prev = text.hex32()
		if (prev === undefined || !text.lit('","ts":')) {
			refuse({ kind: "SidecarShape" }, `sidecar braid ${name} prev is not 32 bytes`)
		}
		const ts = text.quotedU64()
		if (ts === undefined || !text.lit("}")) {
			refuse({ kind: "SidecarShape" }, `sidecar braid ${name} timestamp is not a quoted decimal u64`)
		}
		const last = [...chain.keys()].at(-1)
		if (last !== undefined && last >= name) {
			refuse({ kind: "SidecarShape" }, "sidecar chain is not strictly ascending")
		}
		chain.set(name, { g: generation(g), prev: hex32(prev), ts })
		sum = saturatingAddU64(sum, g)
	}
	if (!text.lit('},"pending":')) {
		refuse({ kind: "SidecarShape" }, "sidecar pending field is absent")
	}
	let pending: Pending | null
	if (text.peek("null")) {
		if (!text.lit("null")) {
			refuse({ kind: "SidecarShape" }, "sidecar pending null arm failed")
		}
		pending = null
	} else {
		if (!text.lit('{"braid":"c')) {
			refuse({ kind: "SidecarShape" }, "sidecar pending is not the canonical object")
		}
		const raw = text.hexU32()
		if (raw === undefined) {
			refuse({ kind: "SidecarShape" }, "sidecar pending braid is not 8 hex")
		}
		const name = braid(`c${raw.toString(16).padStart(8, "0")}`)
		if (known !== undefined && !known.has(name)) {
			refuse({ kind: "SidecarShape" }, `sidecar pending cites unknown braid ${name}`)
		}
		if (!text.lit('","gen":')) {
			refuse({ kind: "SidecarShape" }, "sidecar pending gen field is absent")
		}
		const slot = text.quotedU64()
		if (slot === undefined || !text.lit(',"bytes":"')) {
			refuse({ kind: "SidecarShape" }, "sidecar pending generation is not a quoted decimal u64")
		}
		const body = text.hexBytes()
		if (body === undefined || !text.lit('"}')) {
			refuse({ kind: "SidecarShape" }, "sidecar pending bytes are not lowercase hex")
		}
		pending = { braid: name, gen: generation(slot), bytes: body }
	}
	if (!text.lit("}") || !text.finished()) {
		refuse({ kind: "SidecarShape" }, "sidecar is not the canonical single-line rendering")
	}
	return { chain, pending, sum }
}

async function readSidecar(file: string, known?: ReadonlySet<Braid>): Promise<Sidecar | null> {
	const read = await errors.try(fs.readFile(file))
	if (read.error) {
		return null
	}
	return parseSidecar(read.data, known)
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
export { parseSidecar, readSidecar, renderSidecar, writeSidecar }
