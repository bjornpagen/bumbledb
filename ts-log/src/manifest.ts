/**
 * The protocol's one mutable object: a v:3 binary record — version
 * byte 3, a branded Digest32 fingerprint, an optional branded Digest32
 * checkpoint. The checkpoint record is digest-keyed and immutable:
 * version byte 3, a count-bounded braid roster (id, g, hash, ts) in
 * ascending id order, catalog digest, writer, optional prev. Every
 * digest field is Digest32. The content address is blake3 of the
 * record bytes. Seed audits the catalog claim when a checkpoint is
 * present.
 */

import * as errors from "@superbuilders/errors"
import type { Digest32 } from "#bytes.ts"
import { ByteReader, ByteWriter, bytesEqual, hex32 } from "#bytes.ts"
import type { Braid } from "#descriptor.ts"
import { braidHex } from "#descriptor.ts"
import { refuse } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"
import { Vector } from "#vector.ts"

const DOC_VERSION = 3
const U32_MAX = 0xffffffffn
/** braid u32 + g u64 + hash 32 + ts u64 */
const MIN_HEAD_BYTES = 52n

interface Manifest {
	readonly fingerprint: Digest32
	readonly checkpoint: Digest32 | null
}

function readerOf(bytes: Uint8Array): ByteReader {
	return new ByteReader(bytes, {
		fail(what: string): never {
			refuse({ kind: "Malformed", at: bytes.length }, `document truncated at ${what}`)
		}
	})
}

function finish(reader: ByteReader, at: string): void {
	if (reader.remaining() !== 0) {
		refuse({ kind: "Malformed", at: reader.remaining() }, `${reader.remaining()} trailing bytes after ${at}`)
	}
}

function refuseUnbacked(count: bigint, remaining: number, minItem: bigint, at: string): void {
	if (count === 0n) {
		return
	}
	if (minItem === 0n || BigInt(remaining) / minItem < count) {
		refuse({ kind: "Malformed", at: remaining }, `declared ${at} ${count} outruns the remaining ${remaining} bytes`)
	}
}

function writeOptionalDigest(out: ByteWriter, digest: Digest32 | null): void {
	if (digest === null) {
		out.u8(0)
		return
	}
	out.u8(1)
	out.array32(digest)
}

function readOptionalDigest(reader: ByteReader, at: string): Digest32 | null {
	const tag = reader.u8(at)
	if (tag === 0) {
		return null
	}
	if (tag === 1) {
		return reader.array32(at)
	}
	refuse({ kind: "Flags", flags: tag }, `${at} presence is not 0 or 1`)
}

function readVersion(reader: ByteReader, at: string): void {
	const version = reader.u8("version")
	if (version !== DOC_VERSION) {
		refuse(
			{ kind: "Version", version },
			`${at} version ${version}, consumers refuse every version other than ${DOC_VERSION}`
		)
	}
}

function braidIdOf(id: Braid): number {
	return Number.parseInt(id.slice(1), 16)
}

function renderManifest(manifest: Manifest): Uint8Array {
	const out = new ByteWriter(66)
	out.u8(DOC_VERSION)
	out.array32(manifest.fingerprint)
	writeOptionalDigest(out, manifest.checkpoint)
	return out.finish()
}

function parseManifest(bytes: Uint8Array): Manifest {
	const reader = readerOf(bytes)
	readVersion(reader, "manifest")
	const fingerprint = reader.array32("fingerprint")
	const checkpoint = readOptionalDigest(reader, "checkpoint")
	finish(reader, "the manifest")
	return { fingerprint, checkpoint }
}

interface CheckpointHead {
	readonly g: Generation
	readonly hash: Digest32
	readonly ts: bigint
}

interface CheckpointFacts {
	readonly braids: ReadonlyMap<Braid, CheckpointHead>
	readonly catalog: Digest32
	readonly writer: bigint
	readonly prev: Digest32 | null
	readonly sum: bigint
}

function vectorOfHeads(braids: ReadonlyMap<Braid, CheckpointHead>): Vector {
	const counts = new Map<Braid, bigint>()
	for (const [id, head] of braids) {
		counts.set(id, head.g)
	}
	return Vector.from(counts)
}

function checkpointVector(facts: CheckpointFacts): Vector {
	return vectorOfHeads(facts.braids)
}

function renderCheckpoint(facts: CheckpointFacts): Uint8Array {
	const braids = [...facts.braids.keys()].sort()
	const count = BigInt(braids.length)
	if (count > U32_MAX) {
		throw errors.new(`checkpoint braid count ${count} exceeds u32`)
	}
	const out = new ByteWriter(1 + 4 + braids.length * 52 + 32 + 8 + 1 + 32)
	out.u8(DOC_VERSION)
	out.u32le(Number(count))
	for (const id of braids) {
		const head = facts.braids.get(id)
		if (head === undefined) {
			throw errors.new(`checkpoint lost braid ${id}`)
		}
		out.u32le(braidIdOf(id))
		out.u64le(head.g)
		out.array32(head.hash)
		out.u64le(head.ts)
	}
	out.array32(facts.catalog)
	out.u64le(facts.writer)
	writeOptionalDigest(out, facts.prev)
	return out.finish()
}

function parseCheckpoint(bytes: Uint8Array, known?: ReadonlySet<Braid>): CheckpointFacts {
	const reader = readerOf(bytes)
	readVersion(reader, "checkpoint")
	const count = BigInt(reader.u32le("braid count"))
	refuseUnbacked(count, reader.remaining(), MIN_HEAD_BYTES, "braid count")
	const braids = new Map<Braid, CheckpointHead>()
	for (let i = 0n; i < count; i++) {
		const raw = reader.u32le("braid")
		const name = braidHex(raw)
		if (known !== undefined && !known.has(name)) {
			refuse({ kind: "UnknownBraid", braid: raw }, `checkpoint cites unknown braid ${name}`)
		}
		const g = reader.u64le("generation")
		const hash = reader.array32("hash")
		const ts = reader.u64le("timestamp")
		const last = [...braids.keys()].at(-1)
		if (last !== undefined && last >= name) {
			refuse(
				{ kind: "Malformed", at: bytes.length - reader.remaining() },
				"checkpoint braid roster is not strictly ascending"
			)
		}
		braids.set(name, { g: generation(g), hash, ts })
	}
	const catalog = reader.array32("catalog")
	const writer = reader.u64le("writer")
	const prev = readOptionalDigest(reader, "prev")
	finish(reader, "the checkpoint")
	const summed = vectorOfHeads(braids).sum()
	if (typeof summed !== "bigint") {
		refuse({ kind: "Overflow" }, "checkpoint vector sum overflows u64")
	}
	return { braids, catalog, writer, prev, sum: summed }
}

/** The seed transition's catalog claim: a present checkpoint
 *  compares the opened store's `catalog_digest` against the document.
 *  Genesis (no checkpoint) has no claim. A mismatch names the publisher. */
function auditCatalog(facts: CheckpointFacts | null, computed: Digest32): void {
	if (facts === null) {
		return
	}
	if (bytesEqual(facts.catalog, computed)) {
		return
	}
	refuse(
		{ kind: "CheckpointDigest", expected: hex32(facts.catalog), computed: hex32(computed) },
		`checkpoint catalog ${hex32(facts.catalog)} disagrees with the opened store ${hex32(computed)}; publisher ${facts.writer}`
	)
}

export type { CheckpointFacts, CheckpointHead, Manifest }
export { auditCatalog, checkpointVector, DOC_VERSION, parseCheckpoint, parseManifest, renderCheckpoint, renderManifest }
