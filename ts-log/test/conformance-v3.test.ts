/**
 * The v:3 conformance inventory: every ok golden decodes and
 * re-encodes byte-identically, every `r_*` golden refuses under the
 * sidecar's typed identity, and every materialised fuzz prefix is a
 * named refusal. Documents are `v:3`. Pending is lowercase hex. u64s
 * are bigint / decimal strings, never JSON number.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as path from "node:path"
import { describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import { digest32FromHex, fromHex, toHex } from "#bytes.ts"
import { parseSidecar, renderSidecar } from "#chain.ts"
import type { BatchHeader, DecodedBatch, Op } from "#codec.ts"
import { decodeBatch, encodeBatch, verifyChain } from "#codec.ts"
import type { Descriptor } from "#descriptor.ts"
import { braid } from "#descriptor.ts"
import { DOC_VERSION } from "#document.ts"
import { chainMismatchOf, ErrChainMismatch, ErrRefused, refusalOf } from "#errors.ts"
import { generation } from "#keys.ts"
import { parseCheckpoint, parseManifest, renderCheckpoint, renderManifest } from "#manifest.ts"
import { corpusRoot, pinned, schemaNamed } from "#test/conformance-v3-support.ts"
import type { Interval, Value } from "#value.ts"

interface Inventory {
	readonly version: number
	readonly batch_ok: readonly string[]
	readonly batch_refusal: readonly string[]
	readonly chain: readonly string[]
	readonly documents: readonly string[]
	readonly fuzz_materialised: readonly string[]
	readonly fuzz_storm: string
}

interface BatchSidecar {
	readonly expect: "ok" | "refusal" | "encode-refusal"
	readonly schema: string
	readonly fingerprint: string
	readonly refusal?: string
	readonly header?: {
		readonly braid: string
		readonly braidGen: unknown
		readonly prev: unknown
		readonly writer: unknown
		readonly timestamp: unknown
	}
	readonly ops?: unknown
}

interface DocumentSidecar {
	readonly kind: "manifest" | "checkpoint" | "sidecar"
	readonly expect: "ok" | "refusal"
	readonly schema?: string
	readonly refusal?: string
	readonly value?: unknown
}

interface ChainSidecar {
	readonly schema: string
	readonly fingerprint: string
	readonly braid: string
	readonly slot: unknown
	readonly chain: { readonly g: unknown; readonly prev: unknown; readonly ts: unknown }
	readonly expect: "ok" | "chainMismatch"
	readonly cause?: "prev" | "slot" | "timestamp"
	readonly writer?: unknown
}

interface FuzzSidecar {
	readonly expect: "ok" | "refusal"
	readonly refusal?: string
	readonly schema?: string
	readonly fingerprint?: string
	readonly kind?: "manifest" | "checkpoint" | "sidecar"
}

const present = fs.existsSync(path.join(corpusRoot, "inventory.json"))
const MAGIC = new TextEncoder().encode("BDBL")
const utf8 = new TextDecoder()

function inventoryOf(): Inventory {
	return JSON.parse(fs.readFileSync(path.join(corpusRoot, "inventory.json"), "utf8")) as Inventory
}

function readJson(rel: string): unknown {
	return JSON.parse(fs.readFileSync(path.join(corpusRoot, `${rel}.json`), "utf8"))
}

function readBin(rel: string): Uint8Array {
	return new Uint8Array(fs.readFileSync(path.join(corpusRoot, `${rel}.bin`)))
}

function jsonStemsUnder(rel: string): Set<string> {
	const root = path.join(corpusRoot, rel)
	const stems = new Set<string>()
	const stack = [root]
	while (stack.length > 0) {
		const dir = stack.pop()
		if (dir === undefined) {
			break
		}
		for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
			const full = path.join(dir, entry.name)
			if (entry.isDirectory()) {
				stack.push(full)
				continue
			}
			if (!entry.name.endsWith(".json")) {
				continue
			}
			const relPath = path
				.relative(corpusRoot, full)
				.replaceAll("\\", "/")
				.replace(/\.json$/, "")
			stems.add(relPath)
		}
	}
	return stems
}

function assertDecimalString(label: string, value: unknown): string {
	assert.equal(typeof value, "string", `${label}: target API carries a decimal string, never a JSON number`)
	const text = value as string
	assert.ok(/^-?\d+$/.test(text), `${label}: decimal digits`)
	return text
}

function assertLowercaseHex(label: string, value: unknown): string {
	assert.equal(typeof value, "string", `${label}: hex is a string`)
	const text = value as string
	assert.ok(text.length > 0 && text.length % 2 === 0, `${label}: even-length hex`)
	assert.ok(/^[0-9a-f]+$/.test(text), `${label}: lowercase hex`)
	return text
}

function refusalKind(error: Error): string {
	assert.ok(errors.is(error, ErrRefused), `expected ErrRefused, got: ${error.message}`)
	const cause = refusalOf(error)
	assert.ok(cause !== undefined, "refusal carries its cause")
	return cause.kind
}

function descriptorOf(sidecar: { schema: string; fingerprint?: string }): Descriptor {
	if (sidecar.fingerprint !== undefined) {
		return pinned(sidecar.schema, sidecar.fingerprint)
	}
	return schemaNamed(sidecar.schema)
}

function knownOf(descriptor: Descriptor): Set<ReturnType<typeof braid>> {
	return new Set(descriptor.braidMembers.keys())
}

function digestHex(value: Uint8Array | string): string {
	return typeof value === "string" ? value : toHex(value)
}

function renderHeader(header: BatchHeader): Record<string, string> {
	return {
		braid: header.braid,
		braidGen: header.braidGen.toString(),
		prev: digestHex(header.prev),
		writer: header.writer.toString(),
		timestamp: header.timestamp.toString()
	}
}

function renderCell(
	type: Descriptor["relations"][number]["fields"][number]["type"],
	value: Value
): Record<string, unknown> {
	if (type.kind === "bool") {
		assert.equal(typeof value, "boolean")
		return { bool: value }
	}
	if (type.kind === "u64") {
		assert.equal(typeof value, "bigint", "u64 cell is bigint, never number")
		return { u64: (value as bigint).toString() }
	}
	if (type.kind === "i64") {
		assert.equal(typeof value, "bigint", "i64 cell is bigint, never number")
		return { i64: (value as bigint).toString() }
	}
	if (type.kind === "string") {
		assert.equal(typeof value, "string")
		return { string: value }
	}
	if (type.kind === "fixedBytes") {
		assert.ok(value instanceof Uint8Array)
		return { fixedBytes: toHex(value) }
	}
	assert.ok(typeof value === "object" && value !== null && !(value instanceof Uint8Array))
	const interval = value as Interval
	assert.equal(typeof interval.start, "bigint", "interval start is bigint, never number")
	assert.equal(typeof interval.end, "bigint", "interval end is bigint, never number")
	const pair = [interval.start.toString(), interval.end.toString()]
	return type.element === "i64" ? { intervalI64: pair } : { intervalU64: pair }
}

function renderOps(descriptor: Descriptor, ops: readonly Op[]): unknown {
	return ops.map(function renderOp(op) {
		const relation = descriptor.relationByName.get(op.relation)
		assert.ok(relation !== undefined, `decoded relation ${op.relation}`)
		return {
			kind: op.op,
			relation: relation.id,
			rows: op.rows.map(function renderRow(row) {
				return row.map(function renderValue(value, ordinal) {
					const field = relation.fields[ordinal]
					assert.ok(field !== undefined, `${op.relation} field ${ordinal}`)
					return renderCell(field.type, value)
				})
			})
		}
	})
}

function assertDecodedU64s(label: string, decoded: DecodedBatch): void {
	assert.equal(typeof decoded.header.braidGen, "bigint", `${label}.header.braidGen is bigint`)
	assert.equal(typeof decoded.header.writer, "bigint", `${label}.header.writer is bigint`)
	assert.equal(typeof decoded.header.timestamp, "bigint", `${label}.header.timestamp is bigint`)
}

function assertSidecarHeaderNumbers(label: string, header: NonNullable<BatchSidecar["header"]>): void {
	assertDecimalString(`${label}.header.braidGen`, header.braidGen)
	assertDecimalString(`${label}.header.writer`, header.writer)
	assertDecimalString(`${label}.header.timestamp`, header.timestamp)
	assertLowercaseHex(`${label}.header.prev`, header.prev)
}

function headerFromSidecar(sidecar: BatchSidecar): BatchHeader {
	assert.ok(sidecar.header !== undefined, "sidecar carries a header")
	return {
		fingerprint: digest32FromHex(sidecar.fingerprint),
		braid: braid(sidecar.header.braid),
		braidGen: generation(BigInt(assertDecimalString("header.braidGen", sidecar.header.braidGen))),
		prev: fromHex(assertLowercaseHex("header.prev", sidecar.header.prev)),
		writer: BigInt(assertDecimalString("header.writer", sidecar.header.writer)),
		timestamp: BigInt(assertDecimalString("header.timestamp", sidecar.header.timestamp))
	}
}

function renderManifestValue(parsed: ReturnType<typeof parseManifest>): unknown {
	return { fingerprint: parsed.fingerprint, checkpoint: parsed.checkpoint }
}

function renderCheckpointValue(parsed: ReturnType<typeof parseCheckpoint>): unknown {
	const braids: Record<string, { g: string; hash: string; ts: string }> = {}
	for (const [id, head] of parsed.braids) {
		assert.equal(typeof head.g, "bigint", `${id}.g is bigint`)
		assert.equal(typeof head.ts, "bigint", `${id}.ts is bigint`)
		braids[id] = { g: head.g.toString(), hash: head.hash, ts: head.ts.toString() }
	}
	assert.equal(typeof parsed.writer, "bigint", "checkpoint.writer is bigint")
	return {
		braids,
		catalog: parsed.catalog,
		writer: parsed.writer.toString(),
		prev: parsed.prev
	}
}

function sidecarEntries(parsed: ReturnType<typeof parseSidecar>): ReturnType<typeof parseSidecar>["entries"] {
	return parsed.entries
}

function sidecarPending(
	parsed: ReturnType<typeof parseSidecar>
): { braid: string; gen: bigint; bytes: Uint8Array } | null {
	if (parsed.tag === "settled") {
		return null
	}
	return parsed.batch
}

function renderSidecarValue(parsed: ReturnType<typeof parseSidecar>): unknown {
	const chain: Record<string, { g: string; prev: string; ts: string }> = {}
	for (const [id, entry] of sidecarEntries(parsed)) {
		assert.equal(typeof entry.g, "bigint", `${id}.g is bigint`)
		assert.equal(typeof entry.ts, "bigint", `${id}.ts is bigint`)
		chain[id] = { g: entry.g.toString(), prev: digestHex(entry.prev), ts: entry.ts.toString() }
	}
	const pending = sidecarPending(parsed)
	if (pending === null) {
		return { chain, pending: null }
	}
	assert.equal(typeof pending.gen, "bigint", "pending.gen is bigint")
	const bytes = toHex(pending.bytes)
	assertLowercaseHex("pending.bytes", bytes)
	return { chain, pending: { braid: pending.braid, gen: pending.gen.toString(), bytes } }
}

function assertDocumentValueNumbers(label: string, kind: DocumentSidecar["kind"], value: unknown): void {
	assert.ok(value !== null && typeof value === "object", `${label}: value object`)
	const record = value as Record<string, unknown>
	if (kind === "checkpoint") {
		const braids = record.braids as Record<string, Record<string, unknown>>
		for (const [id, head] of Object.entries(braids)) {
			assertDecimalString(`${label}.${id}.g`, head.g)
			assertDecimalString(`${label}.${id}.ts`, head.ts)
		}
		assertDecimalString(`${label}.writer`, record.writer)
	}
	if (kind === "sidecar") {
		const chain = record.chain as Record<string, Record<string, unknown>>
		for (const [id, entry] of Object.entries(chain)) {
			assertDecimalString(`${label}.${id}.g`, entry.g)
			assertDecimalString(`${label}.${id}.ts`, entry.ts)
		}
		if (record.pending !== null && record.pending !== undefined) {
			const pending = record.pending as Record<string, unknown>
			assertDecimalString(`${label}.pending.gen`, pending.gen)
			assertLowercaseHex(`${label}.pending.bytes`, pending.bytes)
		}
	}
}

function assertWireVersion3(label: string, bytes: Uint8Array): void {
	assert.ok(bytes.length >= 6, `${label}: header present`)
	assert.equal(utf8.decode(bytes.subarray(0, 4)), utf8.decode(MAGIC), `${label}: magic`)
	const version = (bytes[4] ?? 0) | ((bytes[5] ?? 0) << 8)
	assert.equal(version, 3, `${label}: wire version 3`)
}

function assertDocumentV3(label: string, bytes: Uint8Array): void {
	const prefix = utf8.decode(bytes.subarray(0, '{"v":3'.length))
	assert.equal(prefix, '{"v":3', `${label}: document begins {"v":3`)
}

if (!present) {
	describe("v3 inventory", function suite() {
		test("skipped: conformance/v3/inventory.json is not in the tree", { skip: true }, function absent() {})
	})
} else {
	const roster = inventoryOf()

	describe("v3 inventory roster", function suite() {
		test("inventory is the v:3 case roster", function rosterTest() {
			assert.equal(roster.version, 3)
			assert.equal(DOC_VERSION, 3n)
			assert.ok(roster.batch_ok.length > 0, "ok batch goldens")
			assert.ok(roster.batch_refusal.length > 0, "refusal batch goldens")
			for (const stem of roster.batch_ok) {
				assert.ok(stem.startsWith("ok_"), `${stem}: ok stem`)
			}
			for (const stem of roster.batch_refusal) {
				assert.ok(stem.startsWith("r_"), `${stem}: refusal stem`)
			}

			const listedBatch = new Set(
				[...roster.batch_ok, ...roster.batch_refusal].map(function rel(stem) {
					return `batch/${stem}`
				})
			)
			assert.deepEqual(listedBatch, jsonStemsUnder("batch"), "inventory batch roster matches the goldens")
			assert.ok(
				fs.existsSync(path.join(corpusRoot, "batch", "r_encode_short_prev.json")),
				"encode-only short prev sidecar"
			)
			assert.equal(
				fs.existsSync(path.join(corpusRoot, "batch", "r_encode_short_prev.bin")),
				false,
				"short prev is unconstructible as [u8; 32]"
			)

			assert.deepEqual(
				new Set(roster.documents),
				jsonStemsUnder("documents"),
				"inventory document roster matches the goldens"
			)
			assert.deepEqual(
				new Set(
					roster.chain.map(function rel(stem) {
						return `chain/${stem}`
					})
				),
				jsonStemsUnder("chain"),
				"inventory chain roster matches the goldens"
			)

			const onDiskFuzz = jsonStemsUnder("fuzz")
			onDiskFuzz.delete("fuzz/storm")
			assert.deepEqual(new Set(roster.fuzz_materialised), onDiskFuzz, "inventory fuzz roster matches the prefixes")
			assert.equal(roster.fuzz_storm, "fuzz/storm.json")
		})
	})

	describe("v3 inventory batch ok", function suite() {
		for (const stem of roster.batch_ok) {
			test(`batch/${stem}`, function golden() {
				const sidecar = readJson(`batch/${stem}`) as BatchSidecar
				const bytes = readBin(`batch/${stem}`)
				assert.equal(sidecar.expect, "ok", stem)
				assertWireVersion3(stem, bytes)
				assert.ok(sidecar.header !== undefined, `${stem}: header`)
				assertSidecarHeaderNumbers(stem, sidecar.header)
				const descriptor = descriptorOf(sidecar)
				assert.equal(sidecar.fingerprint, descriptor.fingerprint, `${stem}: fingerprint`)
				const decoded = decodeBatch(descriptor, bytes)
				assertDecodedU64s(stem, decoded)
				assert.deepEqual(renderHeader(decoded.header), sidecar.header, `${stem}: header`)
				assert.deepEqual(renderOps(descriptor, decoded.ops), sidecar.ops, `${stem}: ops`)
				const again = encodeBatch(descriptor, decoded.header, decoded.ops)
				assert.equal(toHex(again), toHex(bytes), `${stem}: byte-exact re-encode`)
			})
		}
	})

	describe("v3 inventory batch refusal", function suite() {
		for (const stem of roster.batch_refusal) {
			test(`batch/${stem}`, function golden() {
				const sidecar = readJson(`batch/${stem}`) as BatchSidecar
				if (sidecar.expect === "encode-refusal") {
					assert.equal(sidecar.refusal, "DigestWidth", stem)
					assert.ok(sidecar.header !== undefined, `${stem}: header`)
					const prev = assertLowercaseHex(`${stem}.header.prev`, sidecar.header.prev)
					assert.notEqual(prev.length, 64, `${stem}: short prev is not 32 bytes`)
					assert.equal(
						fs.existsSync(path.join(corpusRoot, "batch", `${stem}.bin`)),
						false,
						`${stem}: encode-only has no wire bytes`
					)
					const descriptor = descriptorOf(sidecar)
					const ran = errors.trySync(function encodeIt() {
						return encodeBatch(descriptor, headerFromSidecar(sidecar), [])
					})
					assert.ok(ran.error, `${stem}: expected an encode refusal`)
					assert.equal(refusalKind(ran.error), sidecar.refusal, `${stem}: encode refusal identity`)
					return
				}
				assert.equal(sidecar.expect, "refusal", stem)
				assert.ok(sidecar.refusal !== undefined, `${stem}: named refusal`)
				const bytes = readBin(`batch/${stem}`)
				const descriptor = descriptorOf(sidecar)
				const ran = errors.trySync(function decodeIt() {
					return decodeBatch(descriptor, bytes)
				})
				assert.ok(ran.error, `${stem}: refusal golden must refuse`)
				assert.equal(refusalKind(ran.error), sidecar.refusal, `${stem}: refusal identity`)
			})
		}
	})

	describe("v3 inventory documents", function suite() {
		for (const rel of roster.documents) {
			test(rel, function golden() {
				const sidecar = readJson(rel) as DocumentSidecar
				const bytes = readBin(rel)
				if (sidecar.expect === "ok") {
					assertDocumentV3(rel, bytes)
					assert.ok(sidecar.value !== undefined, `${rel}: value`)
					assertDocumentValueNumbers(rel, sidecar.kind, sidecar.value)
					if (sidecar.kind === "manifest") {
						const parsed = parseManifest(bytes)
						assert.deepEqual(renderManifestValue(parsed), sidecar.value, `${rel}: manifest value`)
						assert.equal(toHex(renderManifest(parsed)), toHex(bytes), `${rel}: manifest fixpoint`)
						return
					}
					assert.ok(sidecar.schema !== undefined, `${rel}: schema`)
					const descriptor = descriptorOf({ schema: sidecar.schema })
					const known = knownOf(descriptor)
					if (sidecar.kind === "checkpoint") {
						const parsed = parseCheckpoint(bytes, known)
						assert.deepEqual(renderCheckpointValue(parsed), sidecar.value, `${rel}: checkpoint value`)
						assert.equal(toHex(renderCheckpoint(parsed)), toHex(bytes), `${rel}: checkpoint fixpoint`)
						return
					}
					const parsed = parseSidecar(bytes, known)
					assert.deepEqual(renderSidecarValue(parsed), sidecar.value, `${rel}: sidecar value`)
					assert.equal(toHex(new TextEncoder().encode(renderSidecar(parsed))), toHex(bytes), `${rel}: sidecar fixpoint`)
					return
				}
				assert.equal(sidecar.expect, "refusal", rel)
				assert.ok(sidecar.refusal !== undefined, `${rel}: named refusal`)
				const descriptor = sidecar.schema === undefined ? undefined : descriptorOf({ schema: sidecar.schema })
				const known = descriptor === undefined ? undefined : knownOf(descriptor)
				const ran = errors.trySync(function parseIt() {
					if (sidecar.kind === "manifest") {
						return parseManifest(bytes)
					}
					if (sidecar.kind === "checkpoint") {
						return parseCheckpoint(bytes, known)
					}
					return parseSidecar(bytes, known)
				})
				assert.ok(ran.error, `${rel}: refusal golden must refuse`)
				assert.equal(refusalKind(ran.error), sidecar.refusal, `${rel}: document refusal`)
			})
		}
	})

	describe("v3 inventory fuzz prefixes", function suite() {
		for (const rel of roster.fuzz_materialised) {
			test(rel, function golden() {
				const sidecar = readJson(rel) as FuzzSidecar
				const bytes = readBin(rel)
				if (sidecar.expect === "ok") {
					if (rel.includes("/batch/")) {
						assert.ok(sidecar.schema !== undefined, `${rel}: schema`)
						const descriptor = descriptorOf(sidecar)
						const decoded = decodeBatch(descriptor, bytes)
						const again = encodeBatch(descriptor, decoded.header, decoded.ops)
						assert.equal(toHex(again), toHex(bytes), `${rel}: accepted mutant is a fixpoint`)
						return
					}
					if (rel.includes("manifest_")) {
						const parsed = parseManifest(bytes)
						assert.equal(toHex(renderManifest(parsed)), toHex(bytes), `${rel}: manifest fixpoint`)
						return
					}
					const kitchen = descriptorOf({ schema: "kitchen" })
					const known = knownOf(kitchen)
					if (rel.includes("checkpoint_")) {
						const parsed = parseCheckpoint(bytes, known)
						assert.equal(toHex(renderCheckpoint(parsed)), toHex(bytes), `${rel}: checkpoint fixpoint`)
						return
					}
					const parsed = parseSidecar(bytes, known)
					assert.equal(toHex(new TextEncoder().encode(renderSidecar(parsed))), toHex(bytes), `${rel}: sidecar fixpoint`)
					return
				}
				assert.equal(sidecar.expect, "refusal", rel)
				assert.ok(sidecar.refusal !== undefined, `${rel}: named refusal`)
				const ran = errors.trySync(function parseIt() {
					if (rel.includes("/batch/")) {
						const schema = sidecar.schema ?? "kitchen"
						return decodeBatch(descriptorOf({ schema, fingerprint: sidecar.fingerprint }), bytes)
					}
					if (rel.includes("manifest_") || sidecar.kind === "manifest") {
						return parseManifest(bytes)
					}
					const kitchen = descriptorOf({ schema: sidecar.schema ?? "kitchen" })
					const known = knownOf(kitchen)
					if (rel.includes("checkpoint_") || sidecar.kind === "checkpoint") {
						return parseCheckpoint(bytes, known)
					}
					return parseSidecar(bytes, known)
				})
				assert.ok(ran.error, `${rel}: prefix refuses`)
				assert.equal(refusalKind(ran.error), sidecar.refusal, `${rel}: refusal identity`)
			})
		}
	})

	describe("v3 inventory chain", function suite() {
		for (const stem of roster.chain) {
			test(`chain/${stem}`, function golden() {
				const sidecar = readJson(`chain/${stem}`) as ChainSidecar
				const bytes = readBin(`chain/${stem}`)
				const descriptor = descriptorOf(sidecar)
				assert.equal(sidecar.fingerprint, descriptor.fingerprint, `${stem}: fingerprint`)
				const slot = assertDecimalString(`${stem}.slot`, sidecar.slot)
				const g = assertDecimalString(`${stem}.chain.g`, sidecar.chain.g)
				const ts = assertDecimalString(`${stem}.chain.ts`, sidecar.chain.ts)
				assertLowercaseHex(`${stem}.chain.prev`, sidecar.chain.prev)
				const decoded = decodeBatch(descriptor, bytes)
				const again = encodeBatch(descriptor, decoded.header, decoded.ops)
				assert.equal(toHex(again), toHex(bytes), `${stem}: byte-exact re-encode`)
				const position = {
					g: generation(BigInt(g)),
					prev: digest32FromHex(sidecar.chain.prev as string),
					ts: BigInt(ts)
				}
				const ran = errors.trySync(function checkIt() {
					verifyChain(decoded.header, braid(sidecar.braid), generation(BigInt(slot)), position)
				})
				if (sidecar.expect === "ok") {
					assert.equal(ran.error, undefined, `${stem}: ${ran.error?.message}`)
					return
				}
				assert.equal(sidecar.expect, "chainMismatch", stem)
				assert.ok(ran.error, `${stem}: expected a chain mismatch`)
				assert.ok(errors.is(ran.error, ErrChainMismatch), `${stem}: expected ErrChainMismatch`)
				const data = chainMismatchOf(ran.error)
				assert.equal(data?.cause, sidecar.cause, `${stem}: cause`)
				assert.equal(data?.braid, sidecar.braid, `${stem}: fetched braid`)
				assert.equal(data?.slot, BigInt(slot), `${stem}: slot`)
				const writer = assertDecimalString(`${stem}.writer`, sidecar.writer)
				assert.equal(data?.writer, BigInt(writer), `${stem}: writer`)
			})
		}
	})

	describe("v3 inventory storm recipe", function suite() {
		test("storm.json names the XorShift64 mutation lane", function recipe() {
			const storm = readJson("fuzz/storm") as {
				prng: { name: string; batch_storm_iters: number }
				goldens: { batch: readonly string[] }
				operators: readonly unknown[]
			}
			assert.equal(storm.prng.name, "XorShift64")
			assert.ok(storm.goldens.batch.length > 0)
			assert.ok(storm.operators.length > 0)
			assert.ok(storm.prng.batch_storm_iters >= 2000)
		})
	})
}
