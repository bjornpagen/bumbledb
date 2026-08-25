/**
 * The protocol's one mutable object (10): canonical single-line UTF-8
 * JSON, a template walk, field order fixed. Numbers are exact bigint
 * u64; every digest is 32 bytes. A non-canonical document is a typed
 * refusal. The checkpoint json beside it is immutable and digest-keyed.
 */

import * as errors from "@superbuilders/errors"
import { hex32, saturatingAddU64, utf8Encoder } from "#bytes.ts"
import type { Digest32 } from "#bytes.ts"
import type { Braid } from "#descriptor.ts"
import { braid } from "#descriptor.ts"
import { DOC_VERSION, Text } from "#document.ts"
import { refuse } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"

interface Manifest {
	readonly fingerprint: string
	readonly checkpoint: string | null
}

function renderManifest(manifest: Manifest): Uint8Array {
	const checkpoint = manifest.checkpoint === null ? "null" : `"${manifest.checkpoint}"`
	return utf8Encoder.encode(`{"v":${DOC_VERSION},"fingerprint":"${manifest.fingerprint}","checkpoint":${checkpoint}}`)
}

function renderCheckpoint(facts: CheckpointFacts): Uint8Array {
	const braids = [...facts.braids.keys()].sort()
	const body = braids
		.map(function renderHead(id) {
			const head = facts.braids.get(id)
			if (head === undefined) {
				throw errors.new(`checkpoint lost braid ${id}`)
			}
			return `"${id}":{"g":"${head.g}","hash":"${head.hash}","ts":"${head.ts}"}`
		})
		.join(",")
	const prev = facts.prev === null ? "null" : `"${facts.prev}"`
	return utf8Encoder.encode(
		`{"v":${DOC_VERSION},"braids":{${body}},"catalog":"${facts.catalog}","writer":"${facts.writer}","prev":${prev}}`
	)
}

function parseManifest(bytes: Uint8Array): Manifest {
	const text = new Text(bytes)
	if (!text.lit('{"v":')) {
		refuse({ kind: "ManifestShape" }, "manifest is not the canonical template")
	}
	const version = text.u64()
	if (version === undefined) {
		refuse({ kind: "ManifestShape" }, "manifest version is not a canonical u64")
	}
	if (version !== DOC_VERSION) {
		refuse({ kind: "Version", version: Number(version) }, `manifest version ${version}, consumers refuse ≠ ${DOC_VERSION}`)
	}
	if (!text.lit(',"fingerprint":"')) {
		refuse({ kind: "ManifestShape" }, "manifest fingerprint field is absent")
	}
	const fingerprint = text.hex32()
	if (fingerprint === undefined || !text.lit('","checkpoint":')) {
		refuse({ kind: "ManifestShape" }, "manifest fingerprint is not 32 bytes")
	}
	let checkpoint: Digest32 | null
	if (text.peek("null")) {
		if (!text.lit("null")) {
			refuse({ kind: "ManifestShape" }, "manifest checkpoint null arm failed")
		}
		checkpoint = null
	} else {
		if (!text.lit('"')) {
			refuse({ kind: "ManifestShape" }, "manifest checkpoint is neither null nor a digest")
		}
		const digest = text.hex32()
		if (digest === undefined || !text.lit('"')) {
			refuse({ kind: "ManifestShape" }, "manifest checkpoint is not 32 bytes")
		}
		checkpoint = digest
	}
	if (!text.lit("}") || !text.finished()) {
		refuse({ kind: "ManifestShape" }, "manifest is not the canonical single-line rendering")
	}
	return {
		fingerprint: hex32(fingerprint),
		checkpoint: checkpoint === null ? null : hex32(checkpoint)
	}
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
	readonly sum: bigint
}

function parseCheckpoint(bytes: Uint8Array, known?: ReadonlySet<Braid>): CheckpointFacts {
	const text = new Text(bytes)
	if (!text.lit('{"v":')) {
		refuse({ kind: "CheckpointShape" }, "checkpoint json is not the canonical template")
	}
	const version = text.u64()
	if (version === undefined) {
		refuse({ kind: "CheckpointShape" }, "checkpoint version is not a canonical u64")
	}
	if (version !== DOC_VERSION) {
		refuse({ kind: "Version", version: Number(version) }, `checkpoint version ${version}, consumers refuse ≠ ${DOC_VERSION}`)
	}
	if (!text.lit(',"braids":{')) {
		refuse({ kind: "CheckpointShape" }, "checkpoint braids field is absent")
	}
	const braids = new Map<Braid, CheckpointHead>()
	let first = true
	let sum = 0n
	while (!text.peek("}")) {
		if (!first && !text.lit(",")) {
			refuse({ kind: "CheckpointShape" }, "checkpoint braid map is not comma-separated")
		}
		first = false
		if (!text.lit('"c')) {
			refuse({ kind: "CheckpointShape" }, "checkpoint braid id is not a c-prefixed hex")
		}
		const raw = text.hexU32()
		if (raw === undefined) {
			refuse({ kind: "CheckpointShape" }, "checkpoint braid id is not 8 hex")
		}
		const name = braid(`c${raw.toString(16).padStart(8, "0")}`)
		if (known !== undefined && !known.has(name)) {
			refuse({ kind: "UnknownBraid", braid: raw }, `checkpoint cites unknown braid ${name}`)
		}
		if (!text.lit('":{"g":')) {
			refuse({ kind: "CheckpointShape" }, `checkpoint braid ${name} head is malformed`)
		}
		const g = text.quotedU64()
		if (g === undefined || !text.lit(',"hash":"')) {
			refuse({ kind: "CheckpointShape" }, `checkpoint braid ${name} generation is not a quoted decimal u64`)
		}
		const hash = text.hex32()
		if (hash === undefined || !text.lit('","ts":')) {
			refuse({ kind: "CheckpointShape" }, `checkpoint braid ${name} hash is not 32 bytes`)
		}
		const ts = text.quotedU64()
		if (ts === undefined || !text.lit("}")) {
			refuse({ kind: "CheckpointShape" }, `checkpoint braid ${name} timestamp is not a quoted decimal u64`)
		}
		const last = [...braids.keys()].at(-1)
		if (last !== undefined && last >= name) {
			refuse({ kind: "CheckpointShape" }, "checkpoint braid map is not strictly ascending")
		}
		braids.set(name, { g: generation(g), hash: hex32(hash), ts })
		sum = saturatingAddU64(sum, g)
	}
	if (!text.lit('},"catalog":"')) {
		refuse({ kind: "CheckpointShape" }, "checkpoint catalog field is absent")
	}
	const catalog = text.hex32()
	if (catalog === undefined || !text.lit('","writer":')) {
		refuse({ kind: "CheckpointShape" }, "checkpoint catalog is not 32 bytes")
	}
	const writer = text.quotedU64()
	if (writer === undefined || !text.lit(',"prev":')) {
		refuse({ kind: "CheckpointShape" }, "checkpoint writer is not a quoted decimal u64")
	}
	let prev: Digest32 | null
	if (text.peek("null")) {
		if (!text.lit("null")) {
			refuse({ kind: "CheckpointShape" }, "checkpoint prev null arm failed")
		}
		prev = null
	} else {
		if (!text.lit('"')) {
			refuse({ kind: "CheckpointShape" }, "checkpoint prev is neither null nor a digest")
		}
		const digest = text.hex32()
		if (digest === undefined || !text.lit('"')) {
			refuse({ kind: "CheckpointShape" }, "checkpoint prev is not 32 bytes")
		}
		prev = digest
	}
	if (!text.lit("}") || !text.finished()) {
		refuse({ kind: "CheckpointShape" }, "checkpoint json is not the canonical single-line rendering")
	}
	return {
		braids,
		catalog: hex32(catalog),
		writer,
		prev: prev === null ? null : hex32(prev),
		sum
	}
}

export type { CheckpointFacts, CheckpointHead, Manifest }
export { parseCheckpoint, parseManifest, renderCheckpoint, renderManifest }
