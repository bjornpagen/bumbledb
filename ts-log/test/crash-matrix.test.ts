/**
 * Finding 56: the TS crash matrix runs the same named table Rust
 * executes in f4_crash.rs. Each WriterStep / ReplicaStep prefix is a
 * constructible on-disk state — Settled or Pending, slot present or
 * absent, row applied or not — and recovery asserts the named
 * outcomes, never a loosened boolean.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { internalBlake3 } from "@bjornpagen/bumbledb"
import { digest32 } from "#bytes.ts"
import { readSidecar } from "#chain.ts"
import type { Op } from "#codec.ts"
import { decodeBatch, encodeBatch } from "#codec.ts"
import { braid, descriptorOf } from "#descriptor.ts"
import { generation, logKey } from "#keys.ts"
import { applyOps, clearPending, coreOf, entriesOf, holdPending, openReplica, ZERO_HASH } from "#replica.ts"
import { fsStore } from "#store.ts"
import { Booking, Ledger, Note } from "#test/fixtures.ts"
import { openWriter } from "#writer.ts"

const NOTES = braid("c00000002")
const HOME = braid("c00000000")
const WRITER_STEPS = [
	"encode",
	"pending-write",
	"apply-local",
	"ack-local",
	"put-log",
	"chain-advance",
	"pending-clear"
] as const
type WriterStep = (typeof WRITER_STEPS)[number]
type Mode = "published" | "local"
const REPLICA_STEPS = ["apply-local", "chain-advance"] as const
type ReplicaStep = (typeof REPLICA_STEPS)[number]

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-log-crash-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

let laneCounter = 0
function lane(): { store: ReturnType<typeof fsStore>; prefix: string; dir: (name: string) => string } {
	laneCounter += 1
	const base = path.join(tmpRoot, `lane-${laneCounter}`)
	fs.mkdirSync(base, { recursive: true })
	return {
		store: fsStore(path.join(base, "bucket")),
		prefix: "prod/main",
		dir(name: string) {
			return path.join(base, name)
		}
	}
}

function pendingSurvives(mode: Mode, step: WriterStep): boolean {
	if (step === "encode" || step === "pending-clear") {
		return false
	}
	if (step === "ack-local") {
		return mode === "local"
	}
	return true
}

function lands(step: WriterStep): boolean {
	return step !== "encode"
}

function matrixOps(): Op[] {
	return [{ op: "insert", relation: "Note", rows: [[7n, "matrix"]] }]
}

function digestOf(bytes: Uint8Array) {
	return digest32(new Uint8Array(internalBlake3(bytes)))
}

async function plantWriterPrefix(
	mode: Mode,
	step: WriterStep
): Promise<{
	store: ReturnType<typeof fsStore>
	prefix: string
	dir: string
	bytes: Uint8Array
}> {
	const { store, prefix, dir } = lane()
	const writerDir = dir("w")
	const writer = await openWriter({ store, prefix, dir: writerDir, theory: Ledger })
	const core = coreOf(writer.replica)
	const descriptor = descriptorOf(Ledger)
	const ops = matrixOps()
	const bytes = encodeBatch(
		Ledger,
		{
			fingerprint: digest32(descriptor.fingerprintBytes),
			braid: NOTES,
			braidGen: generation(1n),
			prev: ZERO_HASH,
			writer: 41n,
			timestamp: 1n
		},
		ops
	)

	if (step === "encode") {
		await writer.replica[Symbol.asyncDispose]()
		return { store, prefix, dir: writerDir, bytes }
	}

	holdPending(core, { braid: NOTES, gen: generation(1n), bytes }, ops)

	const publishedAck = step === "ack-local" && mode === "published"
	if (step !== "pending-write") {
		const applied = applyOps(core, ops)
		assert.equal(applied.tag, "accepted", `${mode}/${step}: planted apply is Accepted`)
	}

	if (step === "put-log" || step === "chain-advance" || step === "pending-clear" || publishedAck) {
		const created = await store.putCreate(logKey(prefix, NOTES, generation(1n)), bytes)
		assert.equal(created.tag, "created", `${mode}/${step}: planted slot is Created`)
	}

	if (step === "chain-advance" || step === "pending-clear" || publishedAck) {
		entriesOf(core).set(NOTES, { g: generation(1n), prev: digestOf(bytes), ts: 1n })
	}

	if (step === "pending-clear" || publishedAck) {
		await clearPending(core)
	}

	await writer.replica[Symbol.asyncDispose]()
	return { store, prefix, dir: writerDir, bytes }
}

async function recoverWriter(store: ReturnType<typeof fsStore>, prefix: string, writerDir: string) {
	const writer = await openWriter({ store, prefix, dir: writerDir, theory: Ledger })
	const sidecar = await readSidecar(path.join(writerDir, "chain.json"))
	assert.equal(sidecar.tag, "read")
	return { writer, sidecar }
}

describe("the writer crash matrix (56)", function suite() {
	for (const mode of ["published", "local"] as const) {
		for (const step of WRITER_STEPS) {
			test(`${mode}/${step}: recovery names Settled and the slot arm`, async function cell() {
				const planted = await plantWriterPrefix(mode, step)
				const { writer, sidecar } = await recoverWriter(planted.store, planted.prefix, planted.dir)
				assert.equal(sidecar.chain.tag, "settled", `${mode}/${step}: recovery leaves Settled`)
				assert.equal(writer.replica.vector.get(NOTES) ?? 0n, lands(step) ? 1n : 0n, `${mode}/${step}`)

				const slot = await planted.store.get(logKey(planted.prefix, NOTES, generation(1n)))
				assert.equal(slot !== null, lands(step), `${mode}/${step}: the batch reaches the log iff it lands`)
				if (slot !== null) {
					const decoded = decodeBatch(descriptorOf(Ledger), slot.bytes)
					assert.equal(decoded.header.writer, 41n, `${mode}/${step}: our own slot`)
				}
				const beyond = await planted.store.get(logKey(planted.prefix, NOTES, generation(2n)))
				assert.equal(beyond, null, `${mode}/${step}: recovery never double-publishes`)

				const present = writer.replica.db.read(function has(instance) {
					return instance.scan(Note).some(function row(fact) {
						return fact.id === 7n && fact.body === "matrix"
					})
				})
				assert.equal(present, lands(step), `${mode}/${step}`)
				if (pendingSurvives(mode, step)) {
					assert.equal(present && slot !== null, true, `${mode}/${step}: a surviving pending publishes`)
				}

				const sum = [...writer.replica.vector.values()].reduce(function add(acc, g) {
					return acc + g
				}, 0n)
				assert.equal(
					writer.replica.db.read(function gen(instance) {
						return instance.generation
					}),
					sum,
					`${mode}/${step}: the wholeness identity is exact after recovery`
				)
				await writer.replica[Symbol.asyncDispose]()
			})
		}
	}

	test("a resurrected unjudged batch rejects at recovery and never reaches the log", async function resurrected() {
		for (const mode of ["published", "local"] as const) {
			const { store, prefix, dir } = lane()
			const writerDir = dir("w")
			const writer = await openWriter({ store, prefix, dir: writerDir, theory: Ledger })
			const core = coreOf(writer.replica)
			const descriptor = descriptorOf(Ledger)
			const ops: Op[] = [{ op: "insert", relation: "Booking", rows: [[9n, 1n, "orphan", { start: 1n, end: 2n }]] }]
			const bytes = encodeBatch(
				Ledger,
				{
					fingerprint: digest32(descriptor.fingerprintBytes),
					braid: HOME,
					braidGen: generation(1n),
					prev: ZERO_HASH,
					writer: 41n,
					timestamp: 1n
				},
				ops
			)
			holdPending(core, { braid: HOME, gen: generation(1n), bytes }, ops)
			await writer.replica[Symbol.asyncDispose]()

			const recovered = await openWriter({ store, prefix, dir: writerDir, theory: Ledger })
			const sidecar = await readSidecar(path.join(writerDir, "chain.json"))
			assert.equal(sidecar.tag, "read")
			assert.equal(sidecar.chain.tag, "settled", `${mode}: rejection cleared to Settled`)
			assert.equal(recovered.replica.vector.get(HOME) ?? 0n, 0n, mode)
			const slot = await store.get(logKey(prefix, HOME, generation(1n)))
			assert.equal(slot, null, `${mode}: a born-rejected batch never reaches the log`)
			assert.equal(
				recovered.replica.db.read(function count(instance) {
					return instance.count(Booking)
				}),
				0n,
				mode
			)
			await recovered.replica[Symbol.asyncDispose]()
		}
	})

	test("a born-noop batch clears at the exact vector sum and never reaches the log", async function bornNoop() {
		for (const step of ["pending-write", "apply-local"] as const) {
			const { store, prefix, dir } = lane()
			const writerDir = dir("w")
			const writer = await openWriter({ store, prefix, dir: writerDir, theory: Ledger })
			const seeded = await writer.commit(function seed(batch) {
				batch.insert(Note, [{ id: 1n, body: "first" }])
				return 0
			})
			assert.equal(seeded.tag, "accepted")
			const core = coreOf(writer.replica)
			const entry = entriesOf(core).get(NOTES)
			assert.ok(entry !== undefined)
			const ops: Op[] = [{ op: "insert", relation: "Note", rows: [[1n, "first"]] }]
			const bytes = encodeBatch(
				Ledger,
				{
					fingerprint: digest32(descriptorOf(Ledger).fingerprintBytes),
					braid: NOTES,
					braidGen: generation(2n),
					prev: entry.prev,
					writer: 41n,
					timestamp: entry.ts + 1n
				},
				ops
			)
			holdPending(core, { braid: NOTES, gen: generation(2n), bytes }, ops)
			if (step === "apply-local") {
				const applied = applyOps(core, ops)
				assert.equal(applied.tag, "accepted")
			}
			await writer.replica[Symbol.asyncDispose]()

			const recovered = await openWriter({ store, prefix, dir: writerDir, theory: Ledger })
			const sidecar = await readSidecar(path.join(writerDir, "chain.json"))
			assert.equal(sidecar.tag, "read")
			assert.equal(sidecar.chain.tag, "settled", `${step}: born-noop settles`)
			assert.equal(recovered.replica.vector.get(NOTES), 1n, step)
			const second = await store.get(logKey(prefix, NOTES, generation(2n)))
			assert.equal(second, null, `${step}: a born no-op publishes nothing`)
			assert.equal(
				recovered.replica.db.read(function gen(instance) {
					return instance.generation
				}),
				1n,
				`${step}: cleared at the exact vector sum`
			)
			await recovered.replica[Symbol.asyncDispose]()
		}
	})
})

describe("the replica crash matrix (56)", function suite() {
	test("every prefix recovers through catch-up alone", async function prefixes() {
		for (let len = 0; len <= REPLICA_STEPS.length; len++) {
			const { store, prefix, dir } = lane()
			const writer = await openWriter({ store, prefix, dir: dir("w"), theory: Ledger })
			assert.equal(
				(
					await writer.commit(function first(batch) {
						batch.insert(Note, [{ id: 1n, body: "one" }])
						return 0
					})
				).tag,
				"accepted"
			)
			const replicaDir = dir("replica")
			const replica = await openReplica({ store, prefix, dir: replicaDir, theory: Ledger })
			await replica.waitFor(new Map([[NOTES, generation(1n)]]))
			assert.equal(replica.vector.get(NOTES), 1n)

			assert.equal(
				(
					await writer.commit(function second(batch) {
						batch.insert(Note, [{ id: 2n, body: "two" }])
						return 0
					})
				).tag,
				"accepted"
			)
			const slot = await store.get(logKey(prefix, NOTES, generation(2n)))
			assert.ok(slot !== null)
			const decoded = decodeBatch(descriptorOf(Ledger), slot.bytes)
			const core = coreOf(replica)
			for (const step of REPLICA_STEPS.slice(0, len) as ReplicaStep[]) {
				if (step === "apply-local") {
					const applied = applyOps(core, decoded.ops)
					assert.equal(applied.tag, "accepted", `prefix ${len}: planted apply is Accepted`)
				} else {
					entriesOf(core).set(NOTES, { g: generation(2n), prev: digestOf(slot.bytes), ts: decoded.header.timestamp })
				}
			}
			await replica[Symbol.asyncDispose]()
			await writer.replica[Symbol.asyncDispose]()

			const recovered = await openReplica({ store, prefix, dir: replicaDir, theory: Ledger })
			assert.equal(recovered.vector.get(NOTES), 2n, `prefix ${len}`)
			const sidecar = await readSidecar(path.join(replicaDir, "chain.json"))
			assert.equal(sidecar.tag, "read")
			assert.equal(sidecar.chain.tag, "settled", `prefix ${len}: catch-up leaves Settled`)
			const present = recovered.db.read(function has(instance) {
				return instance.scan(Note).some(function row(fact) {
					return fact.id === 2n && fact.body === "two"
				})
			})
			assert.equal(present, true, `prefix ${len}`)
			const sum = [...recovered.vector.values()].reduce(function add(acc, g) {
				return acc + g
			}, 0n)
			assert.equal(
				recovered.db.read(function gen(instance) {
					return instance.generation
				}),
				sum,
				`prefix ${len}: the wholeness identity is exact`
			)
			await recovered[Symbol.asyncDispose]()
		}
	})
})
