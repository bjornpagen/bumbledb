/**
 * The protocol's one mutable object (10): canonical single-line UTF-8
 * JSON, a template walk, field order fixed. Document version is 3;
 * a well-formed v:2 document is Version. Every numeric field other
 * than the discriminator is a quoted decimal-string bigint u64; every
 * digest is 32 bytes. A non-canonical document is a typed refusal.
 * The checkpoint json beside it is immutable and digest-keyed. Seed
 * audits the catalog claim when a checkpoint is present (40).
 */

import * as errors from "@superbuilders/errors"
import { checkedAddU64, hex32, utf8Encoder } from "#bytes.ts"
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

function malformed(text: Text, detail: string): never {
	refuse({ kind: "Malformed", at: text.offset() }, detail)
}

function parseManifest(bytes: Uint8Array): Manifest {
	const text = new Text(bytes)
	if (!text.lit('{"v":')) {
		malformed(text, "manifest is not the canonical template")
	}
	const version = text.u64()
	if (version === undefined) {
		malformed(text, "manifest version is not a canonical u64")
	}
	if (version !== DOC_VERSION) {
		refuse(
			{ kind: "Version", version: Number(version) },
			`manifest version ${version}, consumers refuse v:2 and every version other than ${DOC_VERSION}`
		)
	}
	if (!text.lit(',"fingerprint":"')) {
		malformed(text, "manifest fingerprint field is absent")
	}
	const fingerprint = text.hex32()
	if (fingerprint === undefined || !text.lit('","checkpoint":')) {
		malformed(text, "manifest fingerprint is not 32 bytes")
	}
	let checkpoint: Digest32 | null
	if (text.peek("null")) {
		if (!text.lit("null")) {
			malformed(text, "manifest checkpoint null arm failed")
		}
		checkpoint = null
	} else {
		if (!text.lit('"')) {
			malformed(text, "manifest checkpoint is neither null nor a digest")
		}
		const digest = text.hex32()
		if (digest === undefined || !text.lit('"')) {
			malformed(text, "manifest checkpoint is not 32 bytes")
		}
		checkpoint = digest
	}
	if (!text.lit("}") || !text.finished()) {
		malformed(text, "manifest is not the canonical single-line rendering")
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
		malformed(text, "checkpoint json is not the canonical template")
	}
	const version = text.u64()
	if (version === undefined) {
		malformed(text, "checkpoint version is not a canonical u64")
	}
	if (version !== DOC_VERSION) {
		refuse(
			{ kind: "Version", version: Number(version) },
			`checkpoint version ${version}, consumers refuse v:2 and every version other than ${DOC_VERSION}`
		)
	}
	if (!text.lit(',"braids":{')) {
		malformed(text, "checkpoint braids field is absent")
	}
	const braids = new Map<Braid, CheckpointHead>()
	let first = true
	let sum = 0n
	while (!text.peek("}")) {
		if (!first && !text.lit(",")) {
			malformed(text, "checkpoint braid map is not comma-separated")
		}
		first = false
		if (!text.lit('"c')) {
			malformed(text, "checkpoint braid id is not a c-prefixed hex")
		}
		const raw = text.hexU32()
		if (raw === undefined) {
			malformed(text, "checkpoint braid id is not 8 hex")
		}
		const name = braid(`c${raw.toString(16).padStart(8, "0")}`)
		if (known !== undefined && !known.has(name)) {
			refuse({ kind: "UnknownBraid", braid: raw }, `checkpoint cites unknown braid ${name}`)
		}
		if (!text.lit('":{"g":')) {
			malformed(text, `checkpoint braid ${name} head is malformed`)
		}
		const g = text.quotedU64()
		if (g === undefined || !text.lit(',"hash":"')) {
			malformed(text, `checkpoint braid ${name} generation is not a quoted decimal u64`)
		}
		const hash = text.hex32()
		if (hash === undefined || !text.lit('","ts":')) {
			malformed(text, `checkpoint braid ${name} hash is not 32 bytes`)
		}
		const ts = text.quotedU64()
		if (ts === undefined || !text.lit("}")) {
			malformed(text, `checkpoint braid ${name} timestamp is not a quoted decimal u64`)
		}
		const last = [...braids.keys()].at(-1)
		if (last !== undefined && last >= name) {
			malformed(text, "checkpoint braid map is not strictly ascending")
		}
		braids.set(name, { g: generation(g), hash: hex32(hash), ts })
		const next = checkedAddU64(sum, g)
		if (next === undefined) {
			refuse({ kind: "Overflow" }, "checkpoint vector sum overflows u64")
		}
		sum = next
	}
	if (!text.lit('},"catalog":"')) {
		malformed(text, "checkpoint catalog field is absent")
	}
	const catalog = text.hex32()
	if (catalog === undefined || !text.lit('","writer":')) {
		malformed(text, "checkpoint catalog is not 32 bytes")
	}
	const writer = text.quotedU64()
	if (writer === undefined || !text.lit(',"prev":')) {
		malformed(text, "checkpoint writer is not a quoted decimal u64")
	}
	let prev: Digest32 | null
	if (text.peek("null")) {
		if (!text.lit("null")) {
			malformed(text, "checkpoint prev null arm failed")
		}
		prev = null
	} else {
		if (!text.lit('"')) {
			malformed(text, "checkpoint prev is neither null nor a digest")
		}
		const digest = text.hex32()
		if (digest === undefined || !text.lit('"')) {
			malformed(text, "checkpoint prev is not 32 bytes")
		}
		prev = digest
	}
	if (!text.lit("}") || !text.finished()) {
		malformed(text, "checkpoint json is not the canonical single-line rendering")
	}
	return {
		braids,
		catalog: hex32(catalog),
		writer,
		prev: prev === null ? null : hex32(prev),
		sum
	}
}

/** The seed transition's catalog claim (40): a present checkpoint
 *  compares the opened store's `catalog_digest` against the document.
 *  Genesis (no checkpoint) has no claim. A mismatch names the publisher. */
function auditCatalog(facts: CheckpointFacts | null, computed: string): void {
	if (facts === null) {
		return
	}
	if (facts.catalog === computed) {
		return
	}
	refuse(
		{ kind: "CheckpointDigest", expected: facts.catalog, computed },
		`checkpoint catalog ${facts.catalog} disagrees with the opened store ${computed}; publisher ${facts.writer}`
	)
}

export type { CheckpointFacts, CheckpointHead, Manifest }
export { auditCatalog, parseCheckpoint, parseManifest, renderCheckpoint, renderManifest }
