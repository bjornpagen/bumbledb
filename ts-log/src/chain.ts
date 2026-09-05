/**
 * The chain sidecar in `dir/chain`: a floor cache of chain position,
 * written atomically (temp + rename, fsync). The chain is Settled or
 * Pending — generation is the vector sum, plus one exactly when the
 * value is Pending. The byte grammar has one implementation —
 * `crates/bumbledb-log` behind the napi bridge — so parse and render
 * are marshal walks over the sealed codec handle; the file IO half
 * lives here. The content address is blake3 of the rendered bytes.
 */
import * as crypto from "node:crypto"
import * as fs from "node:fs/promises"
import * as path from "node:path"
import type { LogChain, LogCodecHandle, LogSidecarKind } from "@bjornpagen/bumbledb"
import { internalLogParseSidecar, internalLogRenderSidecar } from "@bjornpagen/bumbledb"
import { Result } from "effect"
import type { Digest32 } from "#bytes.ts"
import { digest32, saturatingAddU64, U64_MAX } from "#bytes.ts"
import type { Braid } from "#descriptor.ts"
import { braidHex } from "#descriptor.ts"
import { LogOperationError, refuse } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"
import { Vector } from "#vector.ts"

/** The sidecar's file name inside a replica directory. */
const CHAIN_FILE = "chain"
/** One braid's chain coordinate: the applied count, the applied
 *  batch's content address, its timestamp. */
interface ChainEntry {
	readonly g: Generation
	readonly prev: Digest32
	readonly ts: bigint
}
interface Pending {
	readonly braid: Braid
	readonly slot: Generation
	readonly bytes: Uint8Array
}
type Chain =
	| {
			readonly tag: "settled"
			readonly entries: ReadonlyMap<Braid, ChainEntry>
	  }
	| {
			readonly tag: "pending"
			readonly entries: ReadonlyMap<Braid, ChainEntry>
			readonly batch: Pending
	  }
type SidecarRead =
	| {
			readonly tag: "absent"
	  }
	| {
			readonly tag: "fault"
			readonly io: unknown
	  }
	| {
			readonly tag: "corrupt"
			readonly parse: unknown
	  }
	| {
			readonly tag: "read"
			readonly chain: Chain
	  }
function codeOf(error: unknown): string | undefined {
	return typeof error === "object" && error !== null && "code" in error && typeof error.code === "string"
		? error.code
		: undefined
}
function braidIdOf(id: Braid): number {
	return Number.parseInt(id.slice(1), 16)
}
function vectorOf(
	entries: ReadonlyMap<
		Braid,
		{
			readonly g: bigint
		}
	>
): Vector {
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
/**
 * Remints a bridge refusal row as the driver's typed refusal. The
 * boundary carries `{ kind, message }` only: the kind is the log
 * core's own identity string, so the cause payload holds the data this
 * side owns — the document's length, its version byte (byte 0 of every
 * v:3 document). The raw braid id of an `UnknownBraid` rides the
 * message.
 */
function refuseBridged(kind: LogSidecarKind, message: string, bytes: Uint8Array): never {
	switch (kind) {
		case "Version":
			return refuse({ kind: "Version", version: bytes[0] ?? 0 }, message)
		case "Overflow":
			return refuse({ kind: "Overflow" }, message)
		case "UnknownBraid":
			return refuse({ kind: "UnknownBraid" }, message)
		case "Malformed":
			return refuse({ kind: "Malformed", at: bytes.length }, message)
	}
}
function renderSidecar(codec: LogCodecHandle, chain: Chain): Uint8Array {
	const entries = [...chain.entries.entries()]
		.sort(function ascending(a, b) {
			return braidIdOf(a[0]) - braidIdOf(b[0])
		})
		.map(function entryOf([id, entry]) {
			return { braid: braidIdOf(id), g: entry.g, prev: entry.prev, ts: entry.ts }
		})
	const doc: LogChain =
		chain.tag === "settled"
			? { entries }
			: {
					entries,
					pending: { braid: braidIdOf(chain.batch.braid), slot: chain.batch.slot, bytes: chain.batch.bytes }
				}
	return internalLogRenderSidecar(codec, doc)
}
function parseSidecar(codec: LogCodecHandle, bytes: Uint8Array): Chain {
	const parsed = internalLogParseSidecar(codec, bytes)
	if (!parsed.ok) {
		refuseBridged(parsed.kind, parsed.message, bytes)
	}
	const entries = new Map<Braid, ChainEntry>()
	for (const entry of parsed.value.entries) {
		entries.set(braidHex(entry.braid), { g: generation(entry.g), prev: digest32(entry.prev), ts: entry.ts })
	}
	const pending = parsed.value.pending
	if (pending === undefined) {
		return { tag: "settled", entries }
	}
	return {
		tag: "pending",
		entries,
		batch: { braid: braidHex(pending.braid), slot: generation(pending.slot), bytes: pending.bytes }
	}
}
async function readSidecar(codec: LogCodecHandle, file: string): Promise<SidecarRead> {
	const read = await Promise.resolve(fs.readFile(file)).then(Result.succeed, (cause: unknown) => Result.fail(cause))
	if (Result.isFailure(read)) {
		if (codeOf(read.failure) === "ENOENT") {
			return { tag: "absent" }
		}
		return { tag: "fault", io: read.failure }
	}
	const parsed = Result.try(function parse() {
		return parseSidecar(codec, read.success)
	})
	if (Result.isFailure(parsed)) {
		return { tag: "corrupt", parse: parsed.failure }
	}
	return { tag: "read", chain: parsed.success }
}
async function writeSidecar(codec: LogCodecHandle, file: string, chain: Chain): Promise<void> {
	const dir = path.dirname(file)
	await fs.mkdir(dir, { recursive: true })
	const temp = path.join(dir, `.chain-${process.pid}-${crypto.randomBytes(4).toString("hex")}`)
	const handle = await fs.open(temp, "wx")
	const written = await Promise.resolve(
		(async function writeAll() {
			await handle.writeFile(renderSidecar(codec, chain))
			await handle.sync()
		})()
	).then(Result.succeed, (cause: unknown) => Result.fail(cause))
	await handle.close()
	if (Result.isFailure(written)) {
		await fs.rm(temp, { force: true })
		throw new LogOperationError({ message: `write sidecar ${file}`, cause: written.failure })
	}
	await fs.rename(temp, file)
	const dirHandle = await fs.open(dir, "r")
	const synced = await Promise.resolve(dirHandle.sync()).then(Result.succeed, (cause: unknown) => Result.fail(cause))
	await dirHandle.close()
	if (Result.isFailure(synced)) {
		throw new LogOperationError({ message: `fsync sidecar directory ${dir}`, cause: synced.failure })
	}
}

export type { Chain, ChainEntry, Pending }
export { CHAIN_FILE, chainGeneration, chainSum, parseSidecar, readSidecar, renderSidecar, writeSidecar }
