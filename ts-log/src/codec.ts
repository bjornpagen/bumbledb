/**
 * The command codec (20): one binary batch format, implemented twice
 * (Rust in `bumbledb-log`, TS here), pinned equal by cross-goldens. A
 * batch is header + ops; commands carry raw values, never intern ids.
 * Decode is a full parse before any apply: every illegal byte is a
 * typed refusal, and bytes after the last op refuse as trailing
 * garbage.
 */

import * as errors from "@superbuilders/errors"
import type { Digest32 } from "#bytes.ts"
import { ByteReader, ByteWriter, bytesEqual, digest32, hex32, utf8Encoder } from "#bytes.ts"
import type { Braid, Theory } from "#descriptor.ts"
import { braidHex, descriptorOf } from "#descriptor.ts"
import { refuse, refuseChain } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"
import type { TaggedRefusal, Value } from "#value.ts"
import { checkAgainst, readTagged, writeTagged } from "#value.ts"

const MAGIC = utf8Encoder.encode("BDBL")
const VERSION = 3
const OP_KIND = { insert: 1, delete: 2 } as const
const U32_MAX = 0xffffffffn

interface Op {
	readonly op: "insert" | "delete"
	readonly relation: string
	readonly rows: ReadonlyArray<readonly Value[]>
}

interface BatchHeader {
	readonly fingerprint: Digest32
	readonly braid: Braid
	readonly braidGen: Generation
	readonly prev: Digest32
	readonly writer: bigint
	readonly timestamp: bigint
}

/**
 * Encode input. Digest fields are raw bytes so a short `prev` reaches
 * the named `DigestWidth` refuse at this gate instead of dying at
 * `Digest32` construction. Decode still produces a branded
 * `BatchHeader`.
 */
interface EncodeHeader {
	readonly fingerprint: Uint8Array
	readonly braid: Braid
	readonly braidGen: Generation
	readonly prev: Uint8Array
	readonly writer: bigint
	readonly timestamp: bigint
}

interface DecodedBatch {
	readonly header: BatchHeader
	readonly ops: readonly Op[]
}

function braidIdOf(id: Braid): number {
	return Number.parseInt(id.slice(1), 16)
}

/**
 * Encodes one batch. Digests are branded here: a `prev` or fingerprint
 * that is not 32 bytes is `DigestWidth`. The header's `braid_gen` must
 * equal the slot number the object is published under; every op
 * relation must belong to the header's braid — a spanning batch is
 * unencodable.
 */
function encodeBatch(theory: Theory, header: EncodeHeader, ops: readonly Op[]): Uint8Array {
	const descriptor = descriptorOf(theory)
	const fingerprint = asDigest(header.fingerprint, "fingerprint")
	const prev = asDigest(header.prev, "prev")
	if (!bytesEqual(fingerprint, descriptor.fingerprintBytes)) {
		throw errors.new(`encode fingerprint ${hex32(fingerprint)} is not the descriptor's ${descriptor.fingerprint}`)
	}
	const braidId = braidIdOf(header.braid)
	const members = descriptor.braidMembers.get(header.braid)
	if (members === undefined) {
		throw errors.new(`braid ${header.braid} is not derived from this descriptor`)
	}
	for (const [opIndex, op] of ops.entries()) {
		const relation = descriptor.relationByName.get(op.relation)
		if (relation === undefined) {
			throw errors.new(`op cites unknown relation ${op.relation}`)
		}
		if (relation.closed) {
			refuse(
				{ kind: "ClosedRelation", op: opIndex, relation: relation.id },
				`op ${opIndex} writes closed relation ${relation.name}`
			)
		}
		if (!members.includes(relation.id)) {
			throw errors.new(`op relation ${op.relation} is outside braid ${header.braid} — a spanning batch is unencodable`)
		}
		for (const [rowIndex, row] of op.rows.entries()) {
			if (row.length !== relation.fields.length) {
				refuse(
					{ kind: "Arity", op: opIndex, relation: relation.name, row: rowIndex },
					`op ${opIndex} relation ${relation.name} row ${rowIndex} arity ${row.length} ≠ ${relation.fields.length}`
				)
			}
			relation.fields.forEach(function gateCell(field, ordinal) {
				const value = row[ordinal]
				if (value === undefined) {
					refuse(
						{ kind: "Arity", op: opIndex, relation: relation.name, row: rowIndex },
						`op ${opIndex} relation ${relation.name} row ${rowIndex} cell ${ordinal} absent`
					)
				}
				checkAgainst(`relation ${relation.name} field ${field.name}`, field.type, value)
			})
		}
	}

	const opCount = BigInt(ops.length)
	if (opCount > U32_MAX) {
		throw errors.new(`encode op count ${opCount} exceeds u32`)
	}

	const out = new ByteWriter(4096)
	out.bytes(MAGIC)
	out.u16le(VERSION)
	out.u16le(0)
	out.bytes(fingerprint)
	out.u32le(braidId)
	out.u64le(header.braidGen)
	out.bytes(prev)
	out.u64le(header.writer)
	out.u64le(header.timestamp)
	out.u32le(Number(opCount))
	for (const op of ops) {
		const relation = descriptor.relationByName.get(op.relation)
		if (relation === undefined) {
			throw errors.new(`op cites unknown relation ${op.relation}`)
		}
		const rowCount = BigInt(op.rows.length)
		if (rowCount > U32_MAX) {
			throw errors.new(`encode row count ${rowCount} exceeds u32`)
		}
		out.u8(OP_KIND[op.op])
		out.u32le(relation.id)
		out.u32le(Number(rowCount))
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

function asDigest(bytes: Uint8Array, at: string): Digest32 {
	if (bytes.length !== 32) {
		refuse({ kind: "DigestWidth" }, `${at} is not 32 bytes`)
	}
	return digest32(bytes)
}

function readU32(reader: ByteReader, what: string): bigint {
	return BigInt(reader.u32le(what))
}

/** Kind + relation id + row count: the shortest op the grammar admits. */
const MIN_OP_BYTES = 9n

/** A declared count the remaining bytes cannot open is Truncated
 *  before the loop. Counts are exact bigint so a u32::MAX row vector
 *  cannot wrap a JavaScript number. A zero-field relation has no row
 *  bytes. A nonempty layout uses one tag byte so a first-cell typed
 *  refusal is not swallowed. */
function refuseUnbacked(count: bigint, remaining: number, minItem: bigint, at: string): void {
	if (count === 0n) {
		return
	}
	if (minItem === 0n || BigInt(remaining) / minItem < count) {
		refuse({ kind: "Truncated", at }, `declared ${at} ${count} outruns the remaining ${remaining} bytes`)
	}
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
	const fingerprint = digest32(reader.bytes(32, "fingerprint"))
	if (!bytesEqual(fingerprint, descriptor.fingerprintBytes)) {
		refuse(
			{ kind: "FingerprintMismatch", carried: hex32(fingerprint), expected: descriptor.fingerprint },
			"batch fingerprint does not match the descriptor"
		)
	}
	const braidId = reader.u32le("braid")
	const braid = braidHex(braidId)
	const members = descriptor.braidMembers.get(braid)
	if (members === undefined) {
		refuse({ kind: "UnknownBraid", braid: braidId }, `batch braid ${braid} is not derived from this descriptor`)
	}
	const braidGen = generation(reader.u64le("braid generation"))
	const prev = digest32(reader.bytes(32, "prev"))
	const writer = reader.u64le("writer")
	const timestamp = reader.u64le("timestamp")

	const opCount = readU32(reader, "op count")
	refuseUnbacked(opCount, reader.remaining(), MIN_OP_BYTES, "op count")
	const ops: Op[] = []
	for (let opIndex = 0n; opIndex < opCount; opIndex++) {
		const op = Number(opIndex)
		const kind = reader.u8("op kind")
		if (kind !== OP_KIND.insert && kind !== OP_KIND.delete) {
			refuse(
				{ kind: "UnknownOpKind", op, opKind: kind },
				`op ${op} kind ${kind} is unknown (3 was deleted with floor bumps)`
			)
		}
		const relationId = reader.u32le("op relation")
		const relation = descriptor.relations[relationId]
		if (relation === undefined) {
			refuse({ kind: "UnknownRelation", op, relation: relationId }, `op ${op} cites unknown relation ${relationId}`)
		}
		if (relation.closed) {
			refuse({ kind: "ClosedRelation", op, relation: relationId }, `op ${op} writes closed relation ${relation.name}`)
		}
		if (!members.includes(relationId)) {
			refuse(
				{ kind: "OpRelationOutsideBraid", op, relation: relationId, braid },
				`op ${op} relation ${relation.name} is outside braid ${braid}`
			)
		}
		const rowCount = readU32(reader, "row count")
		const minRow = relation.fields.length === 0 ? 0n : 1n
		refuseUnbacked(rowCount, reader.remaining(), minRow, "row count")
		const rows: Value[][] = []
		for (let rowIndex = 0n; rowIndex < rowCount; rowIndex++) {
			const rowAt = Number(rowIndex)
			const row: Value[] = []
			relation.fields.forEach(function readCell(field) {
				const at = { relation: relation.name, row: rowAt, field: field.name }
				const where = `relation ${relation.name} row ${rowAt} field ${field.name}`
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
	readonly g: Generation
	readonly prev: Digest32
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
function verifyChain(header: BatchHeader, braid: Braid, slot: Generation, chain: ChainEntry): void {
	if (header.braid !== braid || header.braidGen !== slot) {
		refuseChain(
			{ cause: "slot", braid, slot, writer: header.writer },
			`braid ${braid}: header slot identity ${header.braid}/${header.braidGen} ≠ the fetched key's ${braid}/${slot}`
		)
	}
	if (!bytesEqual(header.prev, chain.prev)) {
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

export type { BatchHeader, ChainEntry, DecodedBatch, Digest32, EncodeHeader, Op }
export { decodeBatch, encodeBatch, verifyChain }
