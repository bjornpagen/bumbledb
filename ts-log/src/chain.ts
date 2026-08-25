/**
 * The chain sidecar in `dir/chain.json`: a floor cache of chain
 * position, written atomically (temp + rename, fsync). The chain is
 * Settled or Pending — generation is the vector sum, plus one exactly
 * when the value is Pending. Each entry's prev is Digest32; the v3
 * document renders it as 64 lowercase hex. Pending batch bytes are
 * lowercase hex. Every numeric field is a bigint u64. The document
 * version is 3.
 */

import * as crypto from "node:crypto"
import * as fs from "node:fs/promises"
import * as path from "node:path"
import * as errors from "@superbuilders/errors"
import { checkedAddU64, digest32, hex32, saturatingAddU64 } from "#bytes.ts"
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

type Chain =
	| { readonly tag: "settled"; readonly entries: ReadonlyMap<Braid, ChainEntry> }
	| { readonly tag: "pending"; readonly entries: ReadonlyMap<Braid, ChainEntry>; readonly batch: Pending }

type SidecarRead =
	| { readonly tag: "absent" }
	| { readonly tag: "fault"; readonly io: Error }
	| { readonly tag: "corrupt"; readonly parse: Error }
	| { readonly tag: "read"; readonly chain: Chain }

function codeOf(error: Error): string | undefined {
	return (error as NodeJS.ErrnoException).code
}

function chainSum(chain: Chain): bigint {
	let sum = 0n
	for (const entry of chain.entries.values()) {
		sum = saturatingAddU64(sum, entry.g)
	}
	return sum
}

function chainGeneration(chain: Chain): bigint {
	const sum = chainSum(chain)
	return chain.tag === "settled" ? sum : saturatingAddU64(sum, 1n)
}

function renderSidecar(chain: Chain): string {
	const braids = [...chain.entries.keys()].sort()
	const body = braids
		.map(function renderEntry(id) {
			const entry = chain.entries.get(id)
			if (entry === undefined) {
				throw errors.new(`sidecar chain lost braid ${id}`)
			}
			return `"${id}":{"g":"${entry.g}","prev":"${hex32(entry.prev)}","ts":"${entry.ts}"}`
		})
		.join(",")
	const pending =
		chain.tag === "settled"
			? "null"
			: `{"braid":"${chain.batch.braid}","gen":"${chain.batch.gen}","bytes":"${pendingHex(chain.batch.bytes)}"}`
	return `{"v":${DOC_VERSION},"chain":{${body}},"pending":${pending}}`
}

function malformed(text: Text, detail: string): never {
	refuse({ kind: "Malformed", at: text.offset() }, detail)
}

function parseSidecar(bytes: Uint8Array, known?: ReadonlySet<Braid>): Chain {
	const text = new Text(bytes)
	if (!text.lit('{"v":')) {
		malformed(text, "sidecar is not the canonical template")
	}
	const version = text.u64()
	if (version === undefined) {
		malformed(text, "sidecar version is not a canonical u64")
	}
	if (version !== DOC_VERSION) {
		refuse(
			{ kind: "Version", version: Number(version) },
			`sidecar version ${version}, consumers refuse ≠ ${DOC_VERSION}`
		)
	}
	if (!text.lit(',"chain":{')) {
		malformed(text, "sidecar chain field is absent")
	}
	const entries = new Map<Braid, ChainEntry>()
	let first = true
	let sum = 0n
	while (!text.peek("}")) {
		if (!first && !text.lit(",")) {
			malformed(text, "sidecar chain is not comma-separated")
		}
		first = false
		if (!text.lit('"c')) {
			malformed(text, "sidecar braid id is not a c-prefixed hex")
		}
		const raw = text.hexU32()
		if (raw === undefined) {
			malformed(text, "sidecar braid id is not 8 hex")
		}
		const name = braid(`c${raw.toString(16).padStart(8, "0")}`)
		if (known !== undefined && !known.has(name)) {
			refuse({ kind: "UnknownBraid", braid: raw }, `sidecar cites unknown braid ${name}`)
		}
		if (!text.lit('":{"g":')) {
			malformed(text, `sidecar braid ${name} entry is malformed`)
		}
		const g = text.quotedU64()
		if (g === undefined || !text.lit(',"prev":"')) {
			malformed(text, `sidecar braid ${name} generation is not a quoted decimal u64`)
		}
		const prev = text.hex32()
		if (prev === undefined || !text.lit('","ts":')) {
			malformed(text, `sidecar braid ${name} prev is not 32 bytes`)
		}
		const ts = text.quotedU64()
		if (ts === undefined || !text.lit("}")) {
			malformed(text, `sidecar braid ${name} timestamp is not a quoted decimal u64`)
		}
		const last = [...entries.keys()].at(-1)
		if (last !== undefined && last >= name) {
			malformed(text, "sidecar chain is not strictly ascending")
		}
		const next = checkedAddU64(sum, g)
		if (next === undefined) {
			refuse({ kind: "Overflow" }, "sidecar chain sum overflows u64")
		}
		entries.set(name, { g: generation(g), prev: digest32(prev), ts })
		sum = next
	}
	if (!text.lit('},"pending":')) {
		malformed(text, "sidecar pending field is absent")
	}
	if (text.peek("null")) {
		if (!text.lit("null")) {
			malformed(text, "sidecar pending null arm failed")
		}
		if (!text.lit("}") || !text.finished()) {
			malformed(text, "sidecar is not the canonical single-line rendering")
		}
		return { tag: "settled", entries }
	}
	if (!text.lit('{"braid":"c')) {
		malformed(text, "sidecar pending is not the canonical object")
	}
	const raw = text.hexU32()
	if (raw === undefined) {
		malformed(text, "sidecar pending braid is not 8 hex")
	}
	const name = braid(`c${raw.toString(16).padStart(8, "0")}`)
	if (known !== undefined && !known.has(name)) {
		refuse({ kind: "UnknownBraid", braid: raw }, `sidecar pending cites unknown braid ${name}`)
	}
	if (!text.lit('","gen":')) {
		malformed(text, "sidecar pending gen field is absent")
	}
	const slot = text.quotedU64()
	if (slot === undefined || !text.lit(',"bytes":"')) {
		malformed(text, "sidecar pending generation is not a quoted decimal u64")
	}
	const body = text.hexBytes()
	if (body === undefined || !text.lit('"}')) {
		malformed(text, "sidecar pending bytes are not lowercase hex")
	}
	if (!text.lit("}") || !text.finished()) {
		malformed(text, "sidecar is not the canonical single-line rendering")
	}
	return {
		tag: "pending",
		entries,
		batch: { braid: name, gen: generation(slot), bytes: body }
	}
}

async function readSidecar(file: string, known?: ReadonlySet<Braid>): Promise<SidecarRead> {
	const read = await errors.try(fs.readFile(file))
	if (read.error) {
		if (codeOf(read.error) === "ENOENT") {
			return { tag: "absent" }
		}
		return { tag: "fault", io: read.error }
	}
	const parsed = errors.trySync(function parse() {
		return parseSidecar(read.data, known)
	})
	if (parsed.error) {
		return { tag: "corrupt", parse: parsed.error }
	}
	return { tag: "read", chain: parsed.data }
}

async function writeSidecar(file: string, chain: Chain): Promise<void> {
	const dir = path.dirname(file)
	await fs.mkdir(dir, { recursive: true })
	const temp = path.join(dir, `.chain-${process.pid}-${crypto.randomBytes(4).toString("hex")}`)
	const handle = await fs.open(temp, "wx")
	const written = await errors.try(
		(async function writeAll() {
			await handle.writeFile(renderSidecar(chain))
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

export type { Chain, ChainEntry, Pending, SidecarRead }
export { chainGeneration, chainSum, parseSidecar, readSidecar, renderSidecar, writeSidecar }
