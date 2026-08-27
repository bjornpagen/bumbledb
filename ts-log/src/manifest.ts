/**
 * The protocol's one mutable object and its checkpoint, as typed
 * payloads. The byte grammar has one implementation —
 * `crates/bumbledb-log` behind the napi bridge — so parse and render
 * here are marshal walks: branded `Digest32` and `Braid` values in,
 * bridge-tagged payloads across, never a second reader of the bytes.
 * The vector algebra over checkpoint heads and the seed transition's
 * catalog audit are machine logic and stay here.
 */

import type { LogCheckpointDoc, LogCheckpointKind, LogCodecHandle, LogManifestKind } from "@bjornpagen/bumbledb"
import {
	internalLogParseCheckpoint,
	internalLogParseManifest,
	internalLogRenderCheckpoint,
	internalLogRenderManifest
} from "@bjornpagen/bumbledb"
import type { Digest32 } from "#bytes.ts"
import { bytesEqual, digest32, hex32 } from "#bytes.ts"
import type { Braid } from "#descriptor.ts"
import { braidHex } from "#descriptor.ts"
import { refuse } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"
import { Vector } from "#vector.ts"

interface Manifest {
	readonly fingerprint: Digest32
	readonly checkpoint: Digest32 | null
}

/**
 * Remints a bridge refusal row as the driver's typed refusal. The
 * boundary carries `{ kind, message }` only: the kind is the log
 * core's own identity string, so the cause payload holds the data this
 * side owns — the document's length, its version byte (byte 0 of every
 * v:3 document) — and nothing invented. The raw braid id of an
 * `UnknownBraid` rides the message; the drifted set of a `BraidSet`
 * rides the message the same way.
 */
function refuseBridged(kind: LogManifestKind | LogCheckpointKind, message: string, bytes: Uint8Array): never {
	switch (kind) {
		case "Version":
			return refuse({ kind: "Version", version: bytes[0] ?? 0 }, message)
		case "Overflow":
			return refuse({ kind: "Overflow" }, message)
		case "UnknownBraid":
			return refuse({ kind: "UnknownBraid" }, message)
		case "BraidSet":
			return refuse({ kind: "BraidSet" }, message)
		case "Malformed":
			return refuse({ kind: "Malformed", at: bytes.length }, message)
	}
}

function braidIdOf(id: Braid): number {
	return Number.parseInt(id.slice(1), 16)
}

function renderManifest(manifest: Manifest): Uint8Array {
	return internalLogRenderManifest(
		manifest.checkpoint === null
			? { fingerprint: manifest.fingerprint }
			: { fingerprint: manifest.fingerprint, checkpoint: manifest.checkpoint }
	)
}

function parseManifest(bytes: Uint8Array): Manifest {
	const parsed = internalLogParseManifest(bytes)
	if (!parsed.ok) {
		refuseBridged(parsed.kind, parsed.message, bytes)
	}
	return {
		fingerprint: digest32(parsed.value.fingerprint),
		checkpoint: parsed.value.checkpoint === undefined ? null : digest32(parsed.value.checkpoint)
	}
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

function renderCheckpoint(codec: LogCodecHandle, facts: CheckpointFacts): Uint8Array {
	const braids = [...facts.braids.entries()]
		.sort(function ascending(a, b) {
			return braidIdOf(a[0]) - braidIdOf(b[0])
		})
		.map(function headOf([id, head]) {
			return { braid: braidIdOf(id), g: head.g, hash: head.hash, ts: head.ts }
		})
	const doc: LogCheckpointDoc =
		facts.prev === null
			? { braids, catalog: facts.catalog, writer: facts.writer }
			: { braids, catalog: facts.catalog, writer: facts.writer, prev: facts.prev }
	return internalLogRenderCheckpoint(codec, doc)
}

function parseCheckpoint(codec: LogCodecHandle, bytes: Uint8Array): CheckpointFacts {
	const parsed = internalLogParseCheckpoint(codec, bytes)
	if (!parsed.ok) {
		refuseBridged(parsed.kind, parsed.message, bytes)
	}
	const braids = new Map<Braid, CheckpointHead>()
	for (const head of parsed.value.braids) {
		braids.set(braidHex(head.braid), { g: generation(head.g), hash: digest32(head.hash), ts: head.ts })
	}
	/** The core refuses `Overflow` at parse; the narrowing here is the
	 *  derived sum's own type gate over the same heads. */
	const summed = vectorOfHeads(braids).sum()
	if (typeof summed !== "bigint") {
		refuse({ kind: "Overflow" }, "checkpoint vector sum overflows u64")
	}
	return {
		braids,
		catalog: digest32(parsed.value.catalog),
		writer: parsed.value.writer,
		prev: parsed.value.prev === undefined ? null : digest32(parsed.value.prev),
		sum: summed
	}
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

export type { CheckpointFacts, CheckpointHead }
export { auditCatalog, checkpointVector, parseCheckpoint, parseManifest, renderCheckpoint, renderManifest }
