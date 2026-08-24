/**
 * The protocol's one mutable object (10): canonical single-line UTF-8
 * JSON, strict parse, field order fixed — strictness is proved by
 * re-rendering the parse and demanding byte equality, so a
 * non-canonical manifest is a typed refusal, never a tolerated variant.
 * The checkpoint json beside it is immutable and digest-keyed.
 */

import * as errors from "@superbuilders/errors"
import { utf8Encoder, utf8StrictDecoder } from "#bytes.ts"
import type { Braid } from "#descriptor.ts"
import { braid } from "#descriptor.ts"
import { refuse } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"

interface Manifest {
	readonly fingerprint: string
	readonly checkpoint: string | null
}

const HEX64 = /^[0-9a-f]{64}$/

function renderManifest(manifest: Manifest): Uint8Array {
	const checkpoint = manifest.checkpoint === null ? "null" : `"${manifest.checkpoint}"`
	return utf8Encoder.encode(`{"v":2,"fingerprint":"${manifest.fingerprint}","checkpoint":${checkpoint}}`)
}

function parseManifest(bytes: Uint8Array): Manifest {
	const decoded = errors.trySync(function decodeText() {
		return utf8StrictDecoder.decode(bytes)
	})
	if (decoded.error) {
		refuse({ kind: "ManifestShape" }, "manifest is not UTF-8")
	}
	const parsed = errors.trySync(function parseJson() {
		return JSON.parse(decoded.data) as unknown
	})
	if (parsed.error || typeof parsed.data !== "object" || parsed.data === null) {
		refuse({ kind: "ManifestShape" }, "manifest is not a JSON object")
	}
	const record = parsed.data as Record<string, unknown>
	if (typeof record.v !== "number") {
		refuse({ kind: "ManifestShape" }, "manifest carries no version")
	}
	if (record.v !== 2) {
		refuse({ kind: "ManifestVersion", version: record.v }, `manifest version ${record.v}, consumers refuse ≠ 2`)
	}
	const fingerprint = record.fingerprint
	const checkpoint = record.checkpoint
	if (typeof fingerprint !== "string" || !HEX64.test(fingerprint)) {
		refuse({ kind: "ManifestShape" }, "manifest fingerprint is not 64 hex")
	}
	if (checkpoint !== null && (typeof checkpoint !== "string" || !HEX64.test(checkpoint))) {
		refuse({ kind: "ManifestShape" }, "manifest checkpoint is neither null nor 64 hex")
	}
	const manifest: Manifest = { fingerprint, checkpoint: checkpoint === null ? null : checkpoint }
	const canonical = renderManifest(manifest)
	if (canonical.length !== bytes.length || utf8StrictDecoder.decode(canonical) !== decoded.data) {
		refuse({ kind: "ManifestShape" }, "manifest is not the canonical single-line rendering")
	}
	return manifest
}

interface CheckpointHead {
	readonly g: Generation
	readonly hash: string
	readonly ts: bigint
}

interface CheckpointFacts {
	readonly braids: ReadonlyMap<Braid, CheckpointHead>
	readonly catalog: string
	readonly writer: bigint
	readonly prev: string | null
}

function parseCheckpoint(bytes: Uint8Array): CheckpointFacts {
	const parsed = errors.trySync(function parseJson() {
		return JSON.parse(utf8StrictDecoder.decode(bytes)) as unknown
	})
	if (parsed.error || typeof parsed.data !== "object" || parsed.data === null) {
		refuse({ kind: "CheckpointShape" }, "checkpoint json is not an object")
	}
	const record = parsed.data as Record<string, unknown>
	const rawBraids = record.braids
	if (typeof rawBraids !== "object" || rawBraids === null) {
		refuse({ kind: "CheckpointShape" }, "checkpoint json carries no braids map")
	}
	const braids = new Map<Braid, CheckpointHead>()
	for (const [name, head] of Object.entries(rawBraids as Record<string, unknown>)) {
		if (typeof head !== "object" || head === null) {
			refuse({ kind: "CheckpointShape" }, `checkpoint braid ${name} head is not an object`)
		}
		const headRecord = head as Record<string, unknown>
		const g = headRecord.g
		const hash = headRecord.hash
		const ts = headRecord.ts
		if (typeof g !== "number" || typeof ts !== "number" || typeof hash !== "string" || !HEX64.test(hash)) {
			refuse({ kind: "CheckpointShape" }, `checkpoint braid ${name} head is malformed`)
		}
		braids.set(braid(name), { g: generation(BigInt(g)), hash, ts: BigInt(ts) })
	}
	const catalog = record.catalog
	const writer = record.writer
	const prev = record.prev
	if (typeof catalog !== "string" || !HEX64.test(catalog) || typeof writer !== "number") {
		refuse({ kind: "CheckpointShape" }, "checkpoint json catalog or writer is malformed")
	}
	if (prev !== null && (typeof prev !== "string" || !HEX64.test(prev))) {
		refuse({ kind: "CheckpointShape" }, "checkpoint json prev is neither null nor a digest")
	}
	return { braids, catalog, writer: BigInt(writer), prev: prev === null ? null : prev }
}

export type { CheckpointFacts, CheckpointHead, Manifest }
export { parseCheckpoint, parseManifest, renderManifest }
