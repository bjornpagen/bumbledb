/**
 * The TS-side primerlane harness
 * (proposals/one-representation/10-measurement.md): generates N facts
 * for a 3-relation Primer-shaped schema and times the persist path
 * through the PUBLIC SDK — `InstanceBuilder.create` + `load` + `admit`
 * + `Db.fromInstance` — printing per-phase wall times. The JS-side
 * component of the report derives from these walls minus the native
 * span tree; nothing here instruments product code.
 *
 * Needs the built native module (`node scripts/build.ts` first) — this
 * harness runs at integration, not in the typecheck-only lanes.
 *
 *     node scripts/bench-primerlane.ts [--facts N]
 */

import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import * as process from "node:process"
import * as errors from "@superbuilders/errors"
import { contained, Db, InstanceBuilder, i64, on, relation, schema, str, u64 } from "#index.ts"

const R0 = relation("R0", { id: u64.fresh, label: str })
const R1 = relation("R1", { id: u64.fresh, parent: u64, label: str, score: i64 })
const R2 = relation("R2", { id: u64.fresh, parent: u64, label: str })
const Primer = schema("PrimerlaneTs", { R0, R1, R2 }, [
	contained(on(R1, "parent"), on(R0, "id")),
	contained(on(R2, "parent"), on(R1, "id"))
])

/** Row-count skew across the three relations (a Primer-ish shape). */
const WEIGHTS = [2n, 5n, 3n]

/** Zipf vocabulary size for the label column. */
const VOCABULARY = 1024n

/** One novel long-tail label per this many draws. */
const NOVEL_DEN = 8n

const MASK = (1n << 64n) - 1n

/** splitmix64 over BigInt — deterministic, dependency-free. */
function splitmix(seed: bigint): () => bigint {
	let state = seed & MASK
	return function next(): bigint {
		state = (state + 0x9e3779b97f4a7c15n) & MASK
		let word = state
		word = ((word ^ (word >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK
		word = ((word ^ (word >> 27n)) * 0x94d049bb133111ebn) & MASK
		return (word ^ (word >> 31n)) & MASK
	}
}

/** A Zipf-ish vocabulary word or a row-unique novel string. */
function label(next: () => bigint, rel: number, index: bigint): string {
	if (next() % NOVEL_DEN === 0n) {
		return `novel-${rel}-${index}`
	}
	// Geometric bucket then uniform within: density falls ~1/rank.
	let bucket = 0n
	let word = next()
	while (bucket < 9n && (word & 1n) === 0n) {
		bucket += 1n
		word >>= 1n
	}
	const lo = (1n << bucket) - 1n
	const hi = bucket === 9n ? VOCABULARY : (1n << (bucket + 1n)) - 1n
	const rank = lo + (next() % (hi - lo))
	return `w${rank.toString().padStart(4, "0")}`
}

function parseFacts(argv: string[]): bigint {
	let facts = 100_000n
	for (let i = 0; i < argv.length; i += 1) {
		const flag = argv[i]
		if (flag !== "--facts") {
			throw errors.new(`unknown flag ${flag} (the harness takes --facts N)`)
		}
		const raw = argv[i + 1]
		if (raw === undefined) {
			throw errors.new("--facts needs a value")
		}
		facts = BigInt(raw)
		if (facts <= 0n) {
			throw errors.new("--facts rejects 0 — the harness measures facts")
		}
		i += 1
	}
	return facts
}

interface Phase {
	name: string
	wallNs: bigint
	rows: bigint
}

function phase<R>(phases: Phase[], name: string, rows: bigint, f: () => R): R {
	const start = process.hrtime.bigint()
	const value = f()
	phases.push({ name, wallNs: process.hrtime.bigint() - start, rows })
	return value
}

async function phaseAsync<R>(phases: Phase[], name: string, rows: bigint, f: () => Promise<R>): Promise<R> {
	const start = process.hrtime.bigint()
	const value = await f()
	phases.push({ name, wallNs: process.hrtime.bigint() - start, rows })
	return value
}

function render(phases: Phase[]): string {
	const lines = ["| phase | wall ms | rows | ns/row |", "| --- | ---: | ---: | ---: |"]
	for (const row of phases) {
		const ms = (Number(row.wallNs) / 1e6).toFixed(1)
		const perRow = row.rows === 0n ? "0" : (Number(row.wallNs) / Number(row.rows)).toFixed(0)
		lines.push(`| ${row.name} | ${ms} | ${row.rows} | ${perRow} |`)
	}
	return lines.join("\n")
}

async function main(): Promise<void> {
	const facts = parseFacts(process.argv.slice(2))
	const totalWeight = WEIGHTS.reduce(function add(a, b) {
		return a + b
	}, 0n)
	const counts = WEIGHTS.map(function share(w) {
		const n = (facts * w) / totalWeight
		return n < 2n ? 2n : n
	})
	const [n0, n1, n2] = counts
	if (n0 === undefined || n1 === undefined || n2 === undefined) {
		throw errors.new("three relations, three counts")
	}
	const phases: Phase[] = []
	const total = n0 + n1 + n2

	const gen = splitmix(1n)
	const rows0 = phase(phases, "generate", total, function generate0() {
		const out: { id: bigint; label: string }[] = []
		for (let i = 0n; i < n0; i += 1n) {
			out.push({ id: i, label: label(gen, 0, i) })
		}
		return out
	})
	const rows1: { id: bigint; parent: bigint; label: string; score: bigint }[] = []
	const rows2: { id: bigint; parent: bigint; label: string }[] = []
	phase(phases, "generate_children", n1 + n2, function generateChildren() {
		for (let i = 0n; i < n1; i += 1n) {
			rows1.push({
				id: i,
				parent: gen() % n0,
				label: label(gen, 1, i),
				score: (gen() % 2_000_000n) - 1_000_000n
			})
		}
		for (let i = 0n; i < n2; i += 1n) {
			rows2.push({ id: i, parent: gen() % n1, label: label(gen, 2, i) })
		}
	})

	const builder = phase(phases, "builder_create", 0n, function create() {
		return InstanceBuilder.create(Primer)
	})
	phase(phases, "builder_load", total, function load() {
		builder.reserve(R0, "id", n0)
		builder.load(R0, rows0)
		builder.reserve(R1, "id", n1)
		builder.load(R1, rows1)
		builder.reserve(R2, "id", n2)
		builder.load(R2, rows2)
	})
	const admission = await phaseAsync(phases, "builder_admit", total, function admit() {
		return builder.admit()
	})
	if (admission.tag !== "accepted") {
		throw errors.new(`primerlane admission rejected: ${admission.violations.length} violations`)
	}
	const instance = admission.value
	const storeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-primerlane-ts-"))
	const db = await phaseAsync(phases, "publish", total, function publish() {
		return Db.fromInstance(path.join(storeRoot, "store"), instance)
	})
	if (db === undefined) {
		throw errors.new("fromInstance returned no handle")
	}

	console.log(`primerlane (ts): facts ${total} (${n0}/${n1}/${n2}), seed 1\n`)
	console.log(render(phases))
	fs.rmSync(storeRoot, { recursive: true, force: true })
}

await main()
