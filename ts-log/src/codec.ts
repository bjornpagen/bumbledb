/**
 * The command codec (20): one binary batch format, implemented twice
 * (Rust in `bumbledb-log`, TS here), pinned equal by cross-goldens. A
 * batch is header + ops; commands carry raw values, never intern ids.
 * Decode is a full parse before any apply: every illegal byte is a
 * typed refusal, and bytes after the last op refuse as trailing
 * garbage.
 */

import * as errors from "@superbuilders/errors"
import { ByteReader, ByteWriter, bytesEqual, fromHex, toHex, utf8Encoder } from "#bytes.ts"
import type { Theory } from "#descriptor.ts"
import { braidHex, descriptorOf } from "#descriptor.ts"
import { refuse, refuseChain } from "#errors.ts"
import type { TaggedRefusal, Value } from "#value.ts"
import { readTagged, writeTagged } from "#value.ts"

const MAGIC = utf8Encoder.encode("BDBL")
const VERSION = 2
const OP_KIND = { insert: 1, delete: 2 } as const

interface Op {
	readonly op: "insert" | "delete"
	readonly relation: string
	readonly rows: ReadonlyArray<readonly Value[]>
}

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
	readonly ops: readonly Op[]
}

function braidIdOf(braid: string): number {
	const match = /^c([0-9a-f]{8})$/.exec(braid)
	if (match === null || match[1] === undefined) {
		throw errors.new(`not a braid id: ${braid}`)
	}
	return Number.parseInt(match[1], 16)
}

/**
 * Encodes one batch. The header's `braid_gen` must equal the slot number
 * the object is published under; every op relation must belong to the
 * header's braid — a spanning batch is unencodable.
 */
function encodeBatch(theory: Theory, header: BatchHeader, ops: readonly Op[]): Uint8Array {
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
	return out.finish()
}

/** Full parse of a batch object; refusals are typed, never partial reads. */
function decodeBatch(theory: Theory, bytes: Uint8Array): DecodedBatch {
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
	const ops: Op[] = []
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
		const rows: Value[][] = []
		for (let rowIndex = 0; rowIndex < rowCount; rowIndex++) {
			const row: Value[] = []
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

	if (reader.remaining() !== 0) {
		refuse(
			{ kind: "TrailingBytes", bytes: reader.remaining() },
			`${reader.remaining()} trailing bytes after the last op`
		)
	}

	return {
		header: { fingerprint, braid, braidGen, prev, writer, timestamp },
		ops
	}
}

interface ChainEntry {
	readonly g: bigint
	readonly prev: string
	readonly ts: bigint
}

/**
 * The chain discipline (20 apply, step 1): one identity, three proved
 * causes — the header's slot identity (braid and generation, both
 * halves of the key the object was fetched from), its `prev` vs the
 * chain head, its timestamp vs the head's. Corruption-class; the header
 * names the misbehaving writer, and the refusal data names the fetched
 * braid.
 */
function verifyChain(header: BatchHeader, braid: string, slot: bigint, chain: ChainEntry): void {
	if (header.braid !== braid || header.braidGen !== slot) {
		refuseChain(
			{ cause: "slot", braid, slot, writer: header.writer },
			`braid ${braid}: header slot identity ${header.braid}/${header.braidGen} ≠ the fetched key's ${braid}/${slot}`
		)
	}
	if (header.prev !== chain.prev) {
		refuseChain(
			{ cause: "prev", braid, slot, writer: header.writer },
			`braid ${braid} slot ${slot}: prev does not cite the predecessor`
		)
	}
	if (header.timestamp < chain.ts) {
		refuseChain(
			{ cause: "timestamp", braid, slot, writer: header.writer },
			`braid ${braid} slot ${slot}: timestamp regresses below the predecessor`
		)
	}
}

export type { BatchHeader, ChainEntry, DecodedBatch, Op }
export { decodeBatch, encodeBatch, verifyChain }
