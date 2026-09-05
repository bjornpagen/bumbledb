import assert from "node:assert/strict"
import * as fs from "node:fs"
import { createRequire } from "node:module"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import type { DbHandle, FactValue, LogBatchHeader } from "#native.ts"
import { dbClose, native } from "#native.ts"
import { parseQueryIr } from "#query/parse-ir.ts"
import type { SchemaSpec } from "#spec.ts"
import { renderLiteral } from "#spec.ts"

// Qualification can select the freshly built addon without overwriting an
// installed platform package. Ordinary source tests exercise their real loader.
const artifact = process.env.BUMBLEDB_TEST_NATIVE
const addon: typeof native & { dbClose(db: DbHandle): void } =
	artifact === undefined ? { ...native, dbClose } : createRequire(import.meta.url)(artifact)

const cases = [
	["0000000000000000", "0000000000000000"],
	["8000000000000000", "0000000000000000"],
	["0000000000000001", "0000000000000001"],
	["8000000000000001", "8000000000000001"],
	["000fffffffffffff", "000fffffffffffff"],
	["0010000000000000", "0010000000000000"],
	["8010000000000000", "8010000000000000"],
	["3ff0000000000000", "3ff0000000000000"],
	["bff0000000000000", "bff0000000000000"],
	["7fefffffffffffff", "7fefffffffffffff"],
	["ffefffffffffffff", "ffefffffffffffff"],
	["7ff0000000000000", "7ff0000000000000"],
	["fff0000000000000", "fff0000000000000"],
	["7ff8000000000000", "7ff8000000000000"],
	["7ff0000000000001", "7ff8000000000000"],
	["fff0000000000001", "7ff8000000000000"],
	["7ff8000000000001", "7ff8000000000000"],
	["ffffffffffffffff", "7ff8000000000000"]
] as const

function fromBits(hex: string): number {
	return Buffer.from(hex, "hex").readDoubleBE()
}

function bits(value: FactValue | undefined): string {
	assert.equal(typeof value, "number")
	const image = Buffer.alloc(8)
	image.writeDoubleBE(value as number)
	return image.toString("hex")
}

const wireSpec: SchemaSpec = {
	relations: [
		{
			name: "Float",
			fields: [{ name: "value", valueType: { kind: "f64" }, newtype: undefined, fresh: false }],
			closed: undefined
		}
	],
	statements: []
}

const header: LogBatchHeader = {
	braid: 0,
	braidGen: 1n,
	prev: new Uint8Array(32),
	writer: 2n,
	timestamp: 3n
}

// Independent v3 byte fixture: explicit field offsets and raw IEEE BE bits.
// This does not call either codec to construct the decode/encode oracle.
function batch(fingerprint: string, hex: string): Buffer {
	const bytes = Buffer.alloc(122)
	bytes.write("BDBL", 0, "ascii")
	bytes.writeUInt16LE(3, 4)
	bytes.set(Buffer.from(fingerprint, "hex"), 8)
	bytes.writeBigUInt64LE(1n, 44)
	bytes.writeBigUInt64LE(2n, 84)
	bytes.writeBigUInt64LE(3n, 92)
	bytes.writeUInt32LE(1, 100)
	bytes[104] = 1
	bytes.writeUInt32LE(1, 109)
	bytes[113] = 7
	bytes.set(Buffer.from(hex, "hex"), 114)
	return bytes
}

test("F64 crosses JS, descriptor, tagged values and the independent raw IEEE log fixture", function floatWire() {
	const descriptor = addon.descriptor(wireSpec)
	assert.deepEqual(descriptor.relations[0]?.fields[0]?.valueType, { kind: "f64" })
	const codec = addon.logCodec(descriptor)
	for (const [input, expected] of cases) {
		const encoded = addon.logEncodeBatch(codec, header, [
			{
				kind: "insert",
				relation: 0,
				rows: [[{ kind: "f64", value: fromBits(input) }]]
			}
		])
		if (!encoded.ok) assert.fail(encoded.message)
		assert.deepEqual(Buffer.from(encoded.value), batch(descriptor.fingerprint, expected), input)
		const decoded = addon.logDecodeBatch(codec, batch(descriptor.fingerprint, expected))
		if (!decoded.ok) assert.fail(decoded.message)
		assert.equal(bits(decoded.value.ops[0]?.rows[0]?.[0]), expected)
	}
	assert.throws(() =>
		addon.logEncodeBatch(codec, header, [
			{
				kind: "insert",
				relation: 0,
				// @ts-expect-error Deliberately hostile JS shape; f64 never coerces bigint.
				rows: [[{ kind: "f64", value: 1n }]]
			}
		])
	)
})

test("NonCanonicalF64 refusal keeps exact payload bits as text, including above 2^53", function badFloatWire() {
	const descriptor = addon.descriptor(wireSpec)
	const codec = addon.logCodec(descriptor)
	for (const [input, expected] of cases) {
		if (input === expected) continue
		const refused = addon.logDecodeBatch(codec, batch(descriptor.fingerprint, input))
		assert.equal(refused.ok, false, input)
		if (refused.ok) assert.fail("noncanonical float accepted")
		assert.equal(refused.kind, "NonCanonicalF64")
		assert.ok(refused.message.includes(`bits: 0x${input}`), refused.message)
	}
	for (let size = 114; size < 122; size += 1) {
		const refused = addon.logDecodeBatch(codec, batch(descriptor.fingerprint, "0000000000000000").subarray(0, size))
		assert.equal(refused.ok, false)
		if (refused.ok) assert.fail("truncated float accepted")
		assert.equal(refused.kind, "Truncated")
	}
})

test("real LMDB F64 ingestion, membership, keyed reads, scalar/set binds and answers are canonical", async function floatStore() {
	const directory = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-float-bridge-"))
	const spec: SchemaSpec = {
		relations: [
			{
				name: "Sample",
				fields: [
					{ name: "value", valueType: { kind: "f64" }, fresh: false, newtype: undefined },
					{ name: "label", valueType: { kind: "string" }, fresh: false, newtype: undefined }
				],
				closed: undefined
			}
		],
		statements: [{ kind: "fd", relation: "Sample", projection: ["value"] }]
	}
	const created = await addon.dbCreate(path.join(directory, "store"), spec)
	assert.equal(created.tag, "accepted")
	if (created.tag !== "accepted") assert.fail("create refused")
	const db = created.db
	try {
		const outcome = addon.dbWrite(db, (tx) => {
			const cells = cases.flatMap(([input, expected]) => [fromBits(input), expected])
			addon.txInsert(tx, 0, BigInt(cases.length), cells)
			assert.equal(addon.txContains(tx, 0, [-0, "0000000000000000"]), true)
			return true
		})
		assert.equal(outcome.tag, "accepted")
		const key = addon.dbManifest(db).statements.find((statement) => statement.kind === "functionality")
		assert.ok(key)
		addon.dbRead(db, (snapshot, witness) => {
			try {
				const actual = addon
					.instanceScan(snapshot, 0)
					.map((row) => [bits(row[0]), row[1]])
					.sort()
				const expected = [...new Set(cases.map(([, image]) => image))].map((image) => [image, image]).sort()
				assert.deepEqual(actual, expected)
				for (const [input, image] of cases) {
					const found = addon.instanceGet(snapshot, 0, key.id, [fromBits(input)])
					assert.equal(bits(found?.[0]), image)
					assert.equal(found?.[1], image)
				}
				assert.throws(() => addon.instanceContains(snapshot, 0, [1n, "wrong"]), /number \(f64\)/)
				for (const set of [false, true]) {
					const prepared = addon.instancePrepare(
						snapshot,
						parseQueryIr({
							kind: "cq",
							interiors: [],
							head: [{ kind: "var" }, { kind: "var" }],
							rules: [
								{
									finds: [
										{ kind: "var", var: 0 },
										{ kind: "var", var: 1 }
									],
									atoms: [
										{
											source: { kind: "edb", relation: 0 },
											bindings: [
												[0, { kind: "var", var: 0 }],
												[1, { kind: "var", var: 1 }]
											]
										}
									],
									negated: [],
									conditions: [
										{
											kind: "leaf",
											cmp: {
												op: { kind: "eq" },
												lhs: { kind: "var", var: 0 },
												rhs: { kind: set ? "paramSet" : "param", param: 0 }
											}
										}
									]
								}
							]
						})
					)
					if (!prepared.ok) assert.fail(prepared.message)
					try {
						const answer = addon.preparedExecute(prepared.prepared, snapshot, [
							set
								? {
										kind: "set",
										values: [
											{ kind: "f64", value: -0 },
											{ kind: "f64", value: Number.NaN }
										]
									}
								: { kind: "f64", value: Number.NaN }
						])
						assert.deepEqual(
							answer.map((row) => bits(row[0])).sort(),
							set ? ["0000000000000000", "7ff8000000000000"] : ["7ff8000000000000"]
						)
					} finally {
						addon.preparedClose(prepared.prepared)
					}
				}
			} finally {
				addon.witnessClose(witness)
			}
		})
	} finally {
		addon.dbClose(db)
		fs.rmSync(directory, { recursive: true, force: true })
	}
})

test("F64 literal diagnostics use canonical hexadecimal IEEE images", function floatDiagnostics() {
	for (const [input, expected] of cases) {
		assert.equal(renderLiteral({ kind: "value", value: { kind: "f64", value: fromBits(input) } }), `f64:0x${expected}`)
	}
})
