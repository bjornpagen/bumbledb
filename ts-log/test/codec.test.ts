import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { bool, bytes as bytesField, i64, interval, relation, schema, str, u64 } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import { digest32, toHex } from "#bytes.ts"
import type { BatchHeader, Op } from "#codec.ts"
import { decodeBatch, encodeBatch, verifyChain } from "#codec.ts"
import { braid, descriptorOf } from "#descriptor.ts"
import { chainMismatchOf, ErrChainMismatch, ErrRefused, refusalOf } from "#errors.ts"
import { generation } from "#keys.ts"
import { Ledger } from "#test/fixtures.ts"

const ZERO_DIGEST = digest32(new Uint8Array(32))
const ONES_DIGEST = digest32(new Uint8Array(32).fill(1))

function headerOf(): BatchHeader {
	return {
		fingerprint: digest32(descriptorOf(Ledger).fingerprintBytes),
		braid: braid("c00000000"),
		braidGen: generation(1n),
		prev: ZERO_DIGEST,
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

describe("the command codec seat", function suite() {
	test("decode(encode) roundtrips header and ops; re-encode is byte-identical", function roundtrip() {
		const bytes = encodeBatch(Ledger, headerOf(), opsOf())
		const decoded = decodeBatch(Ledger, bytes)
		assert.deepEqual(decoded.header, headerOf())
		assert.deepEqual(decoded.ops, opsOf())
		const again = encodeBatch(Ledger, decoded.header, decoded.ops)
		assert.equal(toHex(again), toHex(bytes))
	})

	test("every value tag rides the bridge: bool, u64, i64, string, fixedBytes, interval, fixedInterval", function allTags() {
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
				braid: braid("c00000000"),
				braidGen: generation(1n),
				prev: ZERO_DIGEST,
				writer: 1n,
				timestamp: 0n
			},
			[{ op: "insert", relation: "Wide", rows: [row] }]
		)
		const decoded = decodeBatch(WideTheory, encoded)
		assert.deepEqual(decoded.ops[0]?.rows[0], row)
	})

	test("a short prev cannot encode", function shortPrev() {
		assert.equal(
			refusalKindOf(function encodeIt() {
				return encodeBatch(Ledger, { ...headerOf(), prev: new Uint8Array([0xaa, 0xbb]) }, opsOf())
			}),
			"DigestWidth"
		)
	})

	test("a lone surrogate cannot encode", function loneSurrogate() {
		assert.throws(function encodeIt() {
			return encodeBatch(Ledger, headerOf(), [{ op: "insert", relation: "Holder", rows: [[1n, "\uD800"]] }])
		})
	})

	test("an op citing a relation name outside the theory cannot encode", function unknownName() {
		assert.throws(function encodeIt() {
			return encodeBatch(Ledger, headerOf(), [{ op: "insert", relation: "Ghost", rows: [] }])
		})
	})

	test("a fixed interval whose end is the domain ceiling cannot encode", function ray() {
		const Wide = relation("Wide", { lease: interval(u64, 1n) })
		const WideTheory = schema("WideTheory", { Wide }, [])
		assert.throws(function encodeIt() {
			return encodeBatch(
				WideTheory,
				{
					braid: braid("c00000000"),
					braidGen: generation(1n),
					prev: ZERO_DIGEST,
					writer: 1n,
					timestamp: 0n
				},
				[{ op: "insert", relation: "Wide", rows: [[{ start: (1n << 64n) - 1n, end: 1n << 64n }]] }]
			)
		})
	})

	test("a fixed interval of the wrong width refuses with the core's Value identity", function wrongWidth() {
		const Wide = relation("Wide", { lease: interval(u64, 1n) })
		const WideTheory = schema("WideTheory", { Wide }, [])
		assert.equal(
			refusalKindOf(function encodeIt() {
				return encodeBatch(
					WideTheory,
					{
						braid: braid("c00000000"),
						braidGen: generation(1n),
						prev: ZERO_DIGEST,
						writer: 1n,
						timestamp: 0n
					},
					[{ op: "insert", relation: "Wide", rows: [[{ start: 1n, end: 3n }]] }]
				)
			}),
			"Value"
		)
	})

	test("a spanning batch refuses with the core's OpRelationOutsideBraid identity", function spanning() {
		const ops: Op[] = [
			{ op: "insert", relation: "Holder", rows: [[1n, "ada"]] },
			{ op: "insert", relation: "Note", rows: [[1n, "memo"]] }
		]
		assert.equal(
			refusalKindOf(function encodeIt() {
				return encodeBatch(Ledger, headerOf(), ops)
			}),
			"OpRelationOutsideBraid"
		)
	})

	test("a cell past the layout's width refuses Arity at the seat", function wideRow() {
		assert.equal(
			refusalKindOf(function encodeIt() {
				return encodeBatch(Ledger, headerOf(), [{ op: "insert", relation: "Holder", rows: [[1n, "ada", 2n]] }])
			}),
			"Arity"
		)
	})

	test("a decode refusal crosses with the core's identity kind", function trailing() {
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

	test("the chain discipline: slot, prev, and timestamp causes", function chain() {
		const header = headerOf()
		const good = { g: generation(1n), prev: ZERO_DIGEST, ts: 0n }
		verifyChain(header, header.braid, generation(1n), good)
		for (const [probe, cause] of [
			[{ braid: header.braid, slot: generation(2n), chain: good }, "slot"],
			[{ braid: braid(`${header.braid.slice(0, -1)}9`), slot: generation(1n), chain: good }, "slot"],
			[{ braid: header.braid, slot: generation(1n), chain: { ...good, prev: ONES_DIGEST } }, "prev"],
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
