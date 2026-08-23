/**
 * The command codec (20): one binary batch format, implemented twice
 * (Rust in `bumbledb-log`, TS here), pinned equal by cross-goldens.
 * Commands carry raw values, never intern ids; the footprint section is
 * derivable and carried — encoders call `footprintOf`, replicas
 * recompute during replay and refuse a mismatch. Decode is a full parse
 * before any apply: every illegal byte is a typed refusal, and the
 * illegal footprint combinations (a delta on a non-W entry, a statement
 * on an F entry) are unparseable rather than refused.
 */

import * as errors from "@superbuilders/errors"
import { ByteReader, ByteWriter, bytesEqual, fromHex, toHex, utf8Encoder } from "#bytes.ts"
import type { LogTheory } from "#descriptor.ts"
import { braidHex, descriptorOf } from "#descriptor.ts"
import { refuse, refuseChain } from "#errors.ts"
import type { BatchOp, FootprintEntry } from "#footprint.ts"
import {
	CAPACITY_MODE_BYTE,
	CLASS_BYTE,
	CONTAINMENT_MODE_BYTE,
	compareEntries,
	computeFootprint,
	FACT_MODE_BYTE
} from "#footprint.ts"
import type { LogValue, TaggedRefusal } from "#value.ts"
import { readTagged, writeTagged } from "#value.ts"

const MAGIC = utf8Encoder.encode("BDBL")
const VERSION = 2
const OP_KIND = { insert: 1, delete: 2 } as const

interface BatchHeader {
	readonly fingerprint: string
	readonly braid: string
	readonly braidGen: bigint
	readonly prev: string
	readonly writer: bigint
	readonly timestamp: bigint
}

interface DecodedBatch {
	readonly header: BatchHeader
	readonly ops: readonly BatchOp[]
	readonly footprint: readonly FootprintEntry[]
}

function braidIdOf(braid: string): number {
	const match = /^c([0-9a-f]{8})$/.exec(braid)
	if (match === null || match[1] === undefined) {
		throw errors.new(`not a braid id: ${braid}`)
	}
	return Number.parseInt(match[1], 16)
}

function writeEntry(out: ByteWriter, entry: FootprintEntry): void {
	out.u8(CLASS_BYTE[entry.class])
	switch (entry.class) {
		case "F": {
			out.bytes(entry.key)
			out.u8(FACT_MODE_BYTE[entry.mode])
			return
		}
		case "K": {
			out.u16le(entry.statement)
			out.bytes(entry.key)
			return
		}
		case "C": {
			out.u16le(entry.statement)
			out.bytes(entry.key)
			out.u8(CONTAINMENT_MODE_BYTE[entry.mode])
			return
		}
		case "W": {
			out.u16le(entry.statement)
			out.bytes(entry.key)
			out.u8(CAPACITY_MODE_BYTE[entry.mode])
			if (entry.mode === "child") {
				out.i64le(entry.delta)
			}
			return
		}
	}
}

function footprintSectionBytes(entries: readonly FootprintEntry[]): Uint8Array {
	const out = new ByteWriter(64 + entries.length * 40)
	out.u32le(entries.length)
	for (const entry of entries) {
		writeEntry(out, entry)
	}
	return out.finish()
}

/**
 * Encodes one batch. The header's `braid_gen` must equal the slot number
 * the object is published under; every op relation must belong to the
 * header's braid — a spanning batch is unencodable.
 */
function encodeBatch(theory: LogTheory, header: BatchHeader, ops: readonly BatchOp[]): Uint8Array {
	const descriptor = descriptorOf(theory)
	if (header.fingerprint !== descriptor.fingerprint) {
		throw errors.new(`encode fingerprint ${header.fingerprint} is not the descriptor's ${descriptor.fingerprint}`)
	}
	const braidId = braidIdOf(header.braid)
	const members = descriptor.braidMembers.get(header.braid)
	if (members === undefined) {
		throw errors.new(`braid ${header.braid} is not derived from this descriptor`)
	}
	for (const op of ops) {
		const relation = descriptor.relationByName.get(op.relation)
		if (relation === undefined) {
			throw errors.new(`op cites unknown relation ${op.relation}`)
		}
		if (!members.includes(relation.id)) {
			throw errors.new(`op relation ${op.relation} is outside braid ${header.braid} — a spanning batch is unencodable`)
		}
	}
	const footprint = computeFootprint(descriptor, ops).entries

	const out = new ByteWriter(4096)
	out.bytes(MAGIC)
	out.u16le(VERSION)
	out.u16le(0)
	out.bytes(fromHex(header.fingerprint))
	out.u32le(braidId)
	out.u64le(header.braidGen)
	out.bytes(fromHex(header.prev))
	out.u64le(header.writer)
	out.u64le(header.timestamp)
	out.u32le(ops.length)
	for (const op of ops) {
		const relation = descriptor.relationByName.get(op.relation)
		if (relation === undefined) {
			throw errors.new(`op cites unknown relation ${op.relation}`)
		}
		out.u8(OP_KIND[op.op])
		out.u32le(relation.id)
		out.u32le(op.rows.length)
		for (const row of op.rows) {
			relation.fields.forEach(function writeCell(field, ordinal) {
				const value = row[ordinal]
				if (value === undefined) {
					throw errors.new(`relation ${relation.name}: row cell ${ordinal} absent`)
				}
				writeTagged(out, field.type, value)
			})
		}
	}
	out.bytes(footprintSectionBytes(footprint))
	return out.finish()
}

function decodeEntry(reader: ByteReader, index: number): FootprintEntry {
	function badMode(): never {
		refuse({ kind: "UnknownFootprintMode", index }, `footprint entry ${index} carries an unknown mode byte`)
	}
	const classByte = reader.u8("footprint class")
	if (classByte === CLASS_BYTE.F) {
		const key = reader.bytes(32, "footprint key")
		const mode = reader.u8("footprint mode")
		if (mode !== FACT_MODE_BYTE.insert && mode !== FACT_MODE_BYTE.delete) {
			badMode()
		}
		return { class: "F", key, mode: mode === FACT_MODE_BYTE.insert ? "insert" : "delete" }
	}
	if (classByte !== CLASS_BYTE.K && classByte !== CLASS_BYTE.C && classByte !== CLASS_BYTE.W) {
		refuse({ kind: "UnknownFootprintClass", index }, `footprint entry ${index} carries class byte ${classByte}`)
	}
	const statement = reader.u16le("footprint statement")
	const key = reader.bytes(32, "footprint key")
	if (classByte === CLASS_BYTE.K) {
		return { class: "K", statement, key }
	}
	if (classByte === CLASS_BYTE.C) {
		const mode = reader.u8("footprint mode")
		if (mode === CONTAINMENT_MODE_BYTE.need) {
			return { class: "C", statement, key, mode: "need" }
		}
		if (mode === CONTAINMENT_MODE_BYTE["support+"]) {
			return { class: "C", statement, key, mode: "support+" }
		}
		if (mode === CONTAINMENT_MODE_BYTE["support-"]) {
			return { class: "C", statement, key, mode: "support-" }
		}
		badMode()
	}
	const mode = reader.u8("footprint mode")
	if (mode === CAPACITY_MODE_BYTE.child) {
		const delta = reader.i64le("footprint delta")
		return { class: "W", statement, key, mode: "child", delta }
	}
	if (mode === CAPACITY_MODE_BYTE["parent+"]) {
		return { class: "W", statement, key, mode: "parent+" }
	}
	if (mode === CAPACITY_MODE_BYTE["parent-"]) {
		return { class: "W", statement, key, mode: "parent-" }
	}
	badMode()
}

/** Full parse of a batch object; refusals are typed, never partial reads. */
function decodeBatch(theory: LogTheory, bytes: Uint8Array): DecodedBatch {
	const descriptor = descriptorOf(theory)
	const reader = new ByteReader(bytes, {
		fail(what: string): never {
			refuse({ kind: "Truncated", at: what }, `batch truncated at ${what}`)
		}
	})

	const magic = reader.bytes(4, "magic")
	if (!bytesEqual(magic, MAGIC)) {
		refuse({ kind: "BadMagic" }, "batch magic is not BDBL")
	}
	const version = reader.u16le("version")
	if (version !== VERSION) {
		refuse({ kind: "Version", version }, `batch version ${version}, consumers refuse ≠ ${VERSION}`)
	}
	const flags = reader.u16le("flags")
	if (flags !== 0) {
		refuse({ kind: "Flags", flags }, `batch flags ${flags} must be 0`)
	}
	const fingerprint = toHex(reader.bytes(32, "fingerprint"))
	if (fingerprint !== descriptor.fingerprint) {
		refuse(
			{ kind: "FingerprintMismatch", carried: fingerprint, expected: descriptor.fingerprint },
			"batch fingerprint does not match the descriptor"
		)
	}
	const braidId = reader.u32le("braid")
	const braid = braidHex(braidId)
	const members = descriptor.braidMembers.get(braid)
	if (members === undefined) {
		refuse({ kind: "UnknownBraid", braid: braidId }, `batch braid ${braid} is not derived from this descriptor`)
	}
	const braidGen = reader.u64le("braid generation")
	const prev = toHex(reader.bytes(32, "prev"))
	const writer = reader.u64le("writer")
	const timestamp = reader.u64le("timestamp")

	const opCount = reader.u32le("op count")
	const ops: BatchOp[] = []
	for (let opIndex = 0; opIndex < opCount; opIndex++) {
		const kind = reader.u8("op kind")
		if (kind !== OP_KIND.insert && kind !== OP_KIND.delete) {
			refuse(
				{ kind: "UnknownOpKind", op: opIndex, opKind: kind },
				`op ${opIndex} kind ${kind} is unknown (3 was deleted with floor bumps)`
			)
		}
		const relationId = reader.u32le("op relation")
		const relation = descriptor.relations[relationId]
		if (relation === undefined) {
			refuse(
				{ kind: "UnknownRelation", op: opIndex, relation: relationId },
				`op ${opIndex} cites unknown relation ${relationId}`
			)
		}
		if (relation.closed) {
			refuse(
				{ kind: "ClosedRelation", op: opIndex, relation: relationId },
				`op ${opIndex} writes closed relation ${relation.name}`
			)
		}
		if (!members.includes(relationId)) {
			refuse(
				{ kind: "OpRelationOutsideBraid", op: opIndex, relation: relationId, braid },
				`op ${opIndex} relation ${relation.name} is outside braid ${braid}`
			)
		}
		const rowCount = reader.u32le("row count")
		const rows: LogValue[][] = []
		for (let rowIndex = 0; rowIndex < rowCount; rowIndex++) {
			const row: LogValue[] = []
			relation.fields.forEach(function readCell(field) {
				const at = { relation: relation.name, row: rowIndex, field: field.name }
				const where = `relation ${relation.name} row ${rowIndex} field ${field.name}`
				const refusal: TaggedRefusal = {
					badTag(): never {
						refuse({ kind: "TagMismatch", ...at }, `${where}: tag does not match the layout`)
					},
					boolByte(byte: number): never {
						refuse({ kind: "BoolByte", ...at }, `${where}: bool byte ${byte}`)
					},
					invalidUtf8(): never {
						refuse({ kind: "InvalidUtf8", ...at }, `${where}: string payload is not UTF-8`)
					},
					emptyInterval(): never {
						refuse({ kind: "EmptyInterval", ...at }, `${where}: interval start does not precede its end`)
					},
					intervalOverflow(): never {
						refuse({ kind: "IntervalOverflow", ...at }, `${where}: fixed interval end leaves the element domain`)
					}
				}
				row.push(readTagged(reader, field.type, refusal))
			})
			rows.push(row)
		}
		ops.push({ op: kind === OP_KIND.insert ? "insert" : "delete", relation: relation.name, rows })
	}

	const fpCount = reader.u32le("footprint count")
	const footprint: FootprintEntry[] = []
	for (let index = 0; index < fpCount; index++) {
		const entry = decodeEntry(reader, index)
		const last = footprint[footprint.length - 1]
		if (last !== undefined) {
			const order = compareEntries(last, entry)
			if (order === 0) {
				refuse({ kind: "DuplicateFootprintEntry", index }, `footprint entry ${index} duplicates its predecessor`)
			}
			if (order > 0) {
				refuse({ kind: "UnsortedFootprint", index }, `footprint entry ${index} is out of order`)
			}
		}
		footprint.push(entry)
	}
	if (reader.remaining() !== 0) {
		refuse(
			{ kind: "TrailingBytes", bytes: reader.remaining() },
			`${reader.remaining()} trailing bytes after the footprint`
		)
	}

	return {
		header: { fingerprint, braid, braidGen, prev, writer, timestamp },
		ops,
		footprint
	}
}

interface ChainPosition {
	readonly g: bigint
	readonly prev: string
	readonly ts: bigint
}

/**
 * The chain discipline (20 apply, step 1): one identity, three proved
 * causes — the header's slot vs the key it was fetched from, its `prev`
 * vs the chain head, its timestamp vs the head's. Corruption-class; the
 * header names the misbehaving writer.
 */
function verifyChain(header: BatchHeader, slot: bigint, chain: ChainPosition): void {
	if (header.braidGen !== slot) {
		refuseChain(
			{ cause: "slot", braid: header.braid, slot, writer: header.writer },
			`braid ${header.braid}: header generation ${header.braidGen} ≠ slot ${slot}`
		)
	}
	if (header.prev !== chain.prev) {
		refuseChain(
			{ cause: "prev", braid: header.braid, slot, writer: header.writer },
			`braid ${header.braid} slot ${slot}: prev does not cite the predecessor`
		)
	}
	if (header.timestamp < chain.ts) {
		refuseChain(
			{ cause: "timestamp", braid: header.braid, slot, writer: header.writer },
			`braid ${header.braid} slot ${slot}: timestamp regresses below the predecessor`
		)
	}
}

/** Byte equality of two footprint sections — the recompute-and-refuse comparator. */
function footprintSectionsEqual(a: readonly FootprintEntry[], b: readonly FootprintEntry[]): boolean {
	return bytesEqual(footprintSectionBytes(a), footprintSectionBytes(b))
}

export type { BatchHeader, ChainPosition, DecodedBatch }
export { braidIdOf, decodeBatch, encodeBatch, footprintSectionBytes, footprintSectionsEqual, verifyChain }
