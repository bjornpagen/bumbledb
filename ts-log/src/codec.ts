/**
 * The command codec seat (20): one implementation reads and writes the
 * batch wire — `crates/bumbledb-log`, reached through the sealed
 * per-theory `LogCodec` handle the descriptor parse mints
 * (`descriptor.codec`). This module is typed payload construction only:
 * raw rows tag by the descriptor's layout on the way in, decoded rows
 * cross exactly as the engine's `ValueOut` walk, and every grammar
 * refusal carries the log core's own identity kind, minted through the
 * bridge's `log-identities.json` table. The chain discipline
 * (`verifyChain`) is pure slot algebra over decoded headers and stays
 * host-side; no byte grammar lives here.
 */

import type {
	FactValue,
	LogBatchDecodeKind,
	LogBatchEncodeKind,
	LogOpIn,
	ValueSpec,
	ValueTypeSpec
} from "@bjornpagen/bumbledb"
import { internalLogDecodeBatch, internalLogEncodeBatch } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import type { Digest32 } from "#bytes.ts"
import { bytesEqual, digest32 } from "#bytes.ts"
import type { ChainEntry } from "#chain.ts"
import type { Braid, Descriptor, Theory } from "#descriptor.ts"
import { braidHex, descriptorOf } from "#descriptor.ts"
import { refuse, refuseChain } from "#errors.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"

interface Op {
	readonly op: "insert" | "delete"
	readonly relation: string
	readonly rows: ReadonlyArray<readonly FactValue[]>
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
 * Encode input. The handle is the fingerprint authority, so no
 * fingerprint field exists here — encode fills the wire's from the
 * sealed codec. `prev` is raw bytes so a short digest reaches the named
 * `DigestWidth` refuse at this gate instead of dying at `Digest32`
 * construction. Decode still produces a branded `BatchHeader`.
 */
interface EncodeHeader {
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

function asDigest(bytes: Uint8Array, at: string): Digest32 {
	if (bytes.length !== 32) {
		refuse({ kind: "DigestWidth" }, `${at} is not 32 bytes`)
	}
	return digest32(bytes)
}

/** A bridge refusal row surfaces as `ErrRefused` carrying the core's identity kind. */
function refuseBridge(kind: LogBatchDecodeKind | LogBatchEncodeKind, message: string): never {
	refuse({ kind }, message)
}

/**
 * Tags one raw cell by the field's declared layout — the bridge's
 * inbound spelling. Only the JS shape is judged here (the tag must be
 * constructible); range, width, and interval emptiness are the core's
 * own `Value` refusal.
 */
function taggedCell(where: string, type: ValueTypeSpec, value: FactValue): ValueSpec {
	switch (type.kind) {
		case "bool": {
			if (typeof value !== "boolean") {
				throw errors.new(`${where}: expected boolean`)
			}
			return { kind: "bool", value }
		}
		case "u64": {
			if (typeof value !== "bigint") {
				throw errors.new(`${where}: expected u64 bigint`)
			}
			return { kind: "u64", value }
		}
		case "i64": {
			if (typeof value !== "bigint") {
				throw errors.new(`${where}: expected i64 bigint`)
			}
			return { kind: "i64", value }
		}
		case "string": {
			if (typeof value !== "string") {
				throw errors.new(`${where}: expected well-formed string`)
			}
			if (!value.isWellFormed()) {
				throw errors.new(`${where}: string cell is not well-formed UTF-8`)
			}
			return { kind: "string", value }
		}
		case "fixedBytes": {
			if (!(value instanceof Uint8Array)) {
				throw errors.new(`${where}: expected ${type.len}-byte Uint8Array`)
			}
			return { kind: "fixedBytes", value }
		}
		case "interval": {
			if (typeof value !== "object" || value instanceof Uint8Array) {
				throw errors.new(`${where}: expected interval value`)
			}
			return type.element === "u64"
				? { kind: "intervalU64", start: value.start, end: value.end }
				: { kind: "intervalI64", start: value.start, end: value.end }
		}
	}
}

/**
 * Ops to the bridge's spelling: relation names resolve through the
 * descriptor's vocabulary (the core never sees a name), cells tag by
 * the layout. A cell past the layout's width has no type to tag by and
 * refuses `Arity` here; every other judgment is the core's.
 */
function opsIn(descriptor: Descriptor, ops: readonly Op[]): LogOpIn[] {
	return ops.map(function opIn(op, opIndex) {
		const relation = descriptor.relationByName.get(op.relation)
		if (relation === undefined) {
			throw errors.new(`op cites unknown relation ${op.relation}`)
		}
		const rows = op.rows.map(function rowIn(row, rowIndex) {
			return row.map(function cellIn(value, ordinal) {
				const field = relation.fields[ordinal]
				if (field === undefined) {
					refuse(
						{ kind: "Arity", op: opIndex, relation: relation.name, row: rowIndex },
						`op ${opIndex} relation ${relation.name} row ${rowIndex} cell ${ordinal} is outside the ${relation.fields.length}-field layout`
					)
				}
				return taggedCell(`relation ${relation.name} field ${field.name}`, field.type, value)
			})
		})
		return { kind: op.op, relation: relation.id, rows }
	})
}

/**
 * Encodes one batch through the sealed codec. The handle is the
 * fingerprint authority: encode fills the wire's fingerprint from the
 * sealed codec, so none rides the bridge (a short `prev` is
 * `DigestWidth` at this gate). Braid membership, closedness, arity, and
 * value validity are the core's refusals, crossing with their identity
 * kinds.
 */
function encodeBatch(theory: Theory, header: EncodeHeader, ops: readonly Op[]): Uint8Array {
	const descriptor = descriptorOf(theory)
	const prev = asDigest(header.prev, "prev")
	const outcome = internalLogEncodeBatch(
		descriptor.codec,
		{
			braid: braidIdOf(header.braid),
			braidGen: header.braidGen,
			prev,
			writer: header.writer,
			timestamp: header.timestamp
		},
		opsIn(descriptor, ops)
	)
	if (!outcome.ok) {
		refuseBridge(outcome.kind, outcome.message)
	}
	return outcome.value
}

/** Full parse of a batch object by the one grammar; refusals cross typed, never partial reads. */
function decodeBatch(theory: Theory, bytes: Uint8Array): DecodedBatch {
	const descriptor = descriptorOf(theory)
	const outcome = internalLogDecodeBatch(descriptor.codec, bytes)
	if (!outcome.ok) {
		refuseBridge(outcome.kind, outcome.message)
	}
	const batch = outcome.value
	const ops = batch.ops.map(function opOut(op) {
		const relation = descriptor.relations[op.relation]
		if (relation === undefined) {
			throw errors.new(`decoded op cites relation ${op.relation} outside the descriptor`)
		}
		return { op: op.kind, relation: relation.name, rows: op.rows }
	})
	return {
		header: {
			// Decode already refused any batch whose fingerprint is not
			// the handle's own, so the descriptor's is the batch's.
			fingerprint: digest32(descriptor.fingerprintBytes),
			braid: braidHex(batch.header.braid),
			braidGen: generation(batch.header.braidGen),
			prev: digest32(batch.header.prev),
			writer: batch.header.writer,
			timestamp: batch.header.timestamp
		},
		ops
	}
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

export type { BatchHeader, ChainEntry, DecodedBatch, EncodeHeader, Op }
export { decodeBatch, encodeBatch, verifyChain }
