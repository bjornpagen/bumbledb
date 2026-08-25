/**
 * The chain sidecar in `dir/chain`: a floor cache of chain
 * position, written atomically (temp + rename, fsync). The chain is
 * Settled or Pending — generation is the vector sum, plus one exactly
 * when the value is Pending. The document is a binary v:3 record:
 * version byte 3, counted roster of braid / g / prev / ts, pending
 * tag. Wire pending is the batch bytes. The content address is blake3
 * of those bytes. Every integer is little-endian.
 */

import * as crypto from "node:crypto"
import * as fs from "node:fs/promises"
import * as path from "node:path"
import * as errors from "@superbuilders/errors"
import { ByteReader, ByteWriter, saturatingAddU64, U64_MAX } from "#bytes.ts"
import type { ChainEntry } from "#codec.ts"
import type { Braid } from "#descriptor.ts"
import { braidHex } from "#descriptor.ts"
import { refuse } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"
import { Vector } from "#vector.ts"

/** The sidecar's file name inside a replica directory. */
const CHAIN_FILE = "chain"

const VERSION = 3
const SETTLED = 0
const PENDING = 1
/** u32 braid + u64 g + 32 prev + u64 ts. */
const ENTRY_BYTES = 52n

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

function braidIdOf(id: Braid): number {
	return Number.parseInt(id.slice(1), 16)
}

function vectorOf(entries: ReadonlyMap<Braid, { readonly g: bigint }>): Vector {
	const counts = new Map<Braid, bigint>()
	for (const [braid, entry] of entries) {
		counts.set(braid, entry.g)
	}
	return Vector.from(counts)
}

function chainSum(chain: Chain): bigint {
	const sum = vectorOf(chain.entries).sum()
	return typeof sum === "bigint" ? sum : U64_MAX
}

function chainGeneration(chain: Chain): bigint {
	const sum = chainSum(chain)
	return chain.tag === "settled" ? sum : saturatingAddU64(sum, 1n)
}

/** A declared count the remaining bytes cannot open is Malformed
 *  before the loop. */
function refuseUnbacked(count: bigint, remaining: number, minItem: bigint, at: string): void {
	if (count === 0n) {
		return
	}
	if (minItem === 0n || BigInt(remaining) / minItem < count) {
		refuse({ kind: "Malformed", at: remaining }, `declared ${at} ${count} outruns the remaining ${remaining} bytes`)
	}
}

function renderSidecar(chain: Chain): Uint8Array {
	const out = new ByteWriter(64)
	out.u8(VERSION)
	const braids = [...chain.entries.keys()].sort()
	if (braids.length > 0xffffffff) {
		throw errors.new("sidecar chain count exceeds u32")
	}
	out.u32le(braids.length)
	for (const id of braids) {
		const entry = chain.entries.get(id)
		if (entry === undefined) {
			throw errors.new(`sidecar chain lost braid ${id}`)
		}
		out.u32le(braidIdOf(id))
		out.u64le(entry.g)
		out.bytes(entry.prev)
		out.u64le(entry.ts)
	}
	if (chain.tag === "settled") {
		out.u8(SETTLED)
		return out.finish()
	}
	out.u8(PENDING)
	out.u32le(braidIdOf(chain.batch.braid))
	out.u64le(chain.batch.gen)
	if (chain.batch.bytes.length > 0xffffffff) {
		throw errors.new("sidecar pending exceeds u32 length")
	}
	out.u32le(chain.batch.bytes.length)
	out.bytes(chain.batch.bytes)
	return out.finish()
}

function parseSidecar(bytes: Uint8Array, known?: ReadonlySet<Braid>): Chain {
	const reader = new ByteReader(bytes, {
		fail(what: string): never {
			refuse({ kind: "Malformed", at: bytes.length }, `sidecar truncated at ${what}`)
		}
	})
	const at = function offset(): number {
		return bytes.length - reader.remaining()
	}
	const version = reader.u8("version")
	if (version !== VERSION) {
		refuse({ kind: "Version", version }, `sidecar version ${version}, consumers refuse ≠ ${VERSION}`)
	}
	const count = BigInt(reader.u32le("chain count"))
	refuseUnbacked(count, reader.remaining(), ENTRY_BYTES, "chain count")
	const entries = new Map<Braid, ChainEntry>()
	let last: Braid | undefined
	for (let i = 0n; i < count; i++) {
		const raw = reader.u32le("braid")
		const name = braidHex(raw)
		if (known !== undefined && !known.has(name)) {
			refuse({ kind: "UnknownBraid", braid: raw }, `sidecar cites unknown braid ${name}`)
		}
		const g = reader.u64le("g")
		const prev = reader.array32("prev")
		const ts = reader.u64le("ts")
		if (last !== undefined && last >= name) {
			refuse({ kind: "Malformed", at: at() }, "sidecar chain is not strictly ascending")
		}
		entries.set(name, { g: generation(g), prev, ts })
		last = name
	}
	if (typeof vectorOf(entries).sum() !== "bigint") {
		refuse({ kind: "Overflow" }, "sidecar chain sum overflows u64")
	}
	const tag = reader.u8("pending")
	if (tag === SETTLED) {
		if (reader.remaining() !== 0) {
			refuse({ kind: "Malformed", at: reader.remaining() }, `${reader.remaining()} trailing bytes after the sidecar`)
		}
		return { tag: "settled", entries }
	}
	if (tag !== PENDING) {
		refuse({ kind: "Malformed", at: at() - 1 }, `sidecar pending tag ${tag}`)
	}
	const raw = reader.u32le("pending braid")
	const name = braidHex(raw)
	if (known !== undefined && !known.has(name)) {
		refuse({ kind: "UnknownBraid", braid: raw }, `sidecar pending cites unknown braid ${name}`)
	}
	const slot = reader.u64le("pending generation")
	const length = reader.u32le("pending length")
	const body = reader.bytes(length, "pending bytes")
	if (reader.remaining() !== 0) {
		refuse({ kind: "Malformed", at: reader.remaining() }, `${reader.remaining()} trailing bytes after the sidecar`)
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
export { CHAIN_FILE, chainGeneration, chainSum, parseSidecar, readSidecar, renderSidecar, writeSidecar }
