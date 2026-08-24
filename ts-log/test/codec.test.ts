import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { bool, bytes as bytesField, i64, interval, relation, schema, str, u64 } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { toHex } from "#bytes.ts"
import type { BatchHeader, Op } from "#codec.ts"
import { decodeBatch, encodeBatch, verifyChain } from "#codec.ts"
import { braid, descriptorOf } from "#descriptor.ts"
import { chainMismatchOf, ErrChainMismatch, ErrRefused, refusalOf } from "#errors.ts"
import { generation } from "#keys.ts"
import { Ledger } from "#test/fixtures.ts"

const ZERO = "0".repeat(64)

/** Header layout offsets (20): magic 0, version 4, flags 6, fingerprint 8, braid 40, gen 44, prev 52, writer 84, ts 92, ops 100. */
const OFFSET = {
	magic: 0,
	version: 4,
	flags: 6,
	fingerprint: 8,
	braid: 40,
	opCount: 100,
	firstOpKind: 104,
	firstCellTag: 113
}

function headerOf(): BatchHeader {
	return {
		fingerprint: descriptorOf(Ledger).fingerprint,
		braid: braid("c00000000"),
		braidGen: generation(1n),
		prev: ZERO,
		writer: 12345n,
		timestamp: 1755801600000n
	}
}

function opsOf(): Op[] {
	return [
		{ op: "insert", relation: "Holder", rows: [[1n, "ada"]] },
		{
			op: "insert",
			relation: "Booking",
			rows: [
				[7n, 1n, "s1", { start: 1n, end: 2n }],
				[8n, 1n, "s2", { start: 2n, end: 4n }]
			]
		},
		{ op: "delete", relation: "Booking", rows: [[9n, 1n, "s3", { start: 4n, end: 8n }]] }
	]
}

function refusalKindOf(run: () => unknown): string {
	const caught = errors.trySync(run)
	assert.ok(caught.error, "expected a refusal")
	assert.ok(errors.is(caught.error, ErrRefused), `expected ErrRefused, got: ${caught.error.message}`)
	const cause = refusalOf(caught.error)
	assert.ok(cause !== undefined, "refusal carries its cause")
	return cause.kind
}

describe("the command codec", function suite() {
	test("decode(encode) roundtrips header and ops; re-encode is byte-identical", function roundtrip() {
		const bytes = encodeBatch(Ledger, headerOf(), opsOf())
		const decoded = decodeBatch(Ledger, bytes)
		assert.deepEqual(decoded.header, headerOf())
		assert.deepEqual(decoded.ops, opsOf())
		const again = encodeBatch(Ledger, decoded.header, decoded.ops)
		assert.equal(toHex(again), toHex(bytes))
	})

	test("every value tag rides the wire: bool, u64, i64, string, fixedBytes, interval, fixedInterval", function allTags() {
		const Wide = relation("Wide", {
			flag: bool,
			count: u64,
			delta: i64,
			name: str,
			digest: bytesField(3),
			at: interval(u64),
			lease: interval(i64, 5n)
		})
		const WideTheory = schema("WideTheory", { Wide }, [])
		const row = [true, 9n, -9n, "naïve", new Uint8Array([1, 2, 3]), { start: 1n, end: 9n }, { start: -2n, end: 3n }]
		const encoded = encodeBatch(
			WideTheory,
			{
				fingerprint: descriptorOf(WideTheory).fingerprint,
				braid: braid("c00000000"),
				braidGen: generation(1n),
				prev: ZERO,
				writer: 1n,
				timestamp: 0n
			},
			[{ op: "insert", relation: "Wide", rows: [row] }]
		)
		const decoded = decodeBatch(WideTheory, encoded)
		assert.deepEqual(decoded.ops[0]?.rows[0], row)
	})

	test("bad magic refuses", function magic() {
		const bytes = encodeBatch(Ledger, headerOf(), opsOf())
		bytes[OFFSET.magic] = 0x58
		assert.equal(
			refusalKindOf(function decodeIt() {
				return decodeBatch(Ledger, bytes)
			}),
			"BadMagic"
		)
	})

	test("version 1 refuses", function version() {
		const bytes = encodeBatch(Ledger, headerOf(), opsOf())
		bytes[OFFSET.version] = 1
		assert.equal(
			refusalKindOf(function decodeIt() {
				return decodeBatch(Ledger, bytes)
			}),
			"Version"
		)
	})

	test("nonzero flags refuse", function flags() {
		const bytes = encodeBatch(Ledger, headerOf(), opsOf())
		bytes[OFFSET.flags] = 1
		assert.equal(
			refusalKindOf(function decodeIt() {
				return decodeBatch(Ledger, bytes)
			}),
			"Flags"
		)
	})

	test("a wrong fingerprint refuses", function fingerprint() {
		const bytes = encodeBatch(Ledger, headerOf(), opsOf())
		bytes[OFFSET.fingerprint] = (bytes[OFFSET.fingerprint] ?? 0) ^ 0xff
		assert.equal(
			refusalKindOf(function decodeIt() {
				return decodeBatch(Ledger, bytes)
			}),
			"FingerprintMismatch"
		)
	})

	test("an op relation outside the header braid refuses", function braidMembership() {
		const bytes = encodeBatch(Ledger, headerOf(), opsOf())
		bytes[OFFSET.braid] = 2
		assert.equal(
			refusalKindOf(function decodeIt() {
				return decodeBatch(Ledger, bytes)
			}),
			"OpRelationOutsideBraid"
		)
	})

	test("op kind 3 refuses like any unknown kind — FloorBump stays deleted", function floorBumpDeleted() {
		const bytes = encodeBatch(Ledger, headerOf(), [{ op: "insert", relation: "Holder", rows: [[1n, "ada"]] }])
		bytes[OFFSET.firstOpKind] = 3
		assert.equal(
			refusalKindOf(function decodeIt() {
				return decodeBatch(Ledger, bytes)
			}),
			"UnknownOpKind"
		)
	})

	test("a row tag that disagrees with the layout refuses naming relation, row, and field", function rowShape() {
		const bytes = encodeBatch(Ledger, headerOf(), [{ op: "insert", relation: "Holder", rows: [[1n, "ada"]] }])
		bytes[OFFSET.firstCellTag] = 3
		const caught = errors.trySync(function decodeIt() {
			return decodeBatch(Ledger, bytes)
		})
		assert.ok(caught.error && errors.is(caught.error, ErrRefused))
		const cause = refusalOf(caught.error)
		assert.ok(cause !== undefined && cause.kind === "TagMismatch")
		assert.equal(cause.relation, "Holder")
		assert.equal(cause.row, 0)
		assert.equal(cause.field, "id")
	})

	test("trailing bytes refuse", function trailing() {
		const bytes = encodeBatch(Ledger, headerOf(), opsOf())
		const padded = new Uint8Array(bytes.length + 1)
		padded.set(bytes)
		assert.equal(
			refusalKindOf(function decodeIt() {
				return decodeBatch(Ledger, padded)
			}),
			"TrailingBytes"
		)
	})

	test("truncation refuses", function truncated() {
		const bytes = encodeBatch(Ledger, headerOf(), opsOf())
		assert.equal(
			refusalKindOf(function decodeIt() {
				return decodeBatch(Ledger, bytes.slice(0, 50))
			}),
			"Truncated"
		)
	})

	test("a spanning batch is unencodable", function spanning() {
		const ops: Op[] = [
			{ op: "insert", relation: "Holder", rows: [[1n, "ada"]] },
			{ op: "insert", relation: "Note", rows: [[1n, "memo"]] }
		]
		assert.throws(function encodeIt() {
			return encodeBatch(Ledger, headerOf(), ops)
		})
	})

	test("the chain discipline: slot, prev, and timestamp causes", function chain() {
		const header = headerOf()
		const good = { g: generation(1n), prev: ZERO, ts: 0n }
		verifyChain(header, header.braid, generation(1n), good)
		for (const [probe, cause] of [
			[{ braid: header.braid, slot: generation(2n), chain: good }, "slot"],
			[{ braid: braid(`${header.braid.slice(0, -1)}9`), slot: generation(1n), chain: good }, "slot"],
			[{ braid: header.braid, slot: generation(1n), chain: { ...good, prev: "1".repeat(64) } }, "prev"],
			[{ braid: header.braid, slot: generation(1n), chain: { ...good, ts: header.timestamp + 1n } }, "timestamp"]
		] as const) {
			const caught = errors.trySync(function checkIt() {
				verifyChain(header, probe.braid, probe.slot, probe.chain)
			})
			assert.ok(caught.error && errors.is(caught.error, ErrChainMismatch))
			const data = chainMismatchOf(caught.error)
			assert.equal(data?.cause, cause)
			assert.equal(data?.braid, probe.braid)
			assert.equal(data?.writer, header.writer)
		}
	})
})
