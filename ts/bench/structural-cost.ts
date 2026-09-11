/** Focused host measurements, not release qualification. Run after building the SDK. */

import { execFileSync } from "node:child_process"
import { createHash } from "node:crypto"
import { readFileSync, rmSync } from "node:fs"
import { arch, cpus, platform } from "node:os"
import { performance } from "node:perf_hooks"
import { Effect, ManagedRuntime } from "effect"
import { ChangeSet } from "#changes.ts"
import { Db } from "#db.ts"
import { interval, u64 } from "#fields.ts"
import { Compute } from "#query/compute.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { schema } from "#schema.ts"
import { key } from "#statements.ts"
import { runtimeOptions, storeDir } from "../test/fixtures/learning.ts"

const Row = relation("Row", { id: u64, span: interval(u64), clip: interval(u64) })
const Theory = schema("StructuralCost", { Row }, [key(Row, ["id"])])
const pieces = query(Theory).rule((r) => {
	const row = v(Row)
	return r.match(Row, row).find({ id: row.id, span: r.difference(row.span, row.clip) })
})
const measured = query(Theory).rule((r) => {
	const row = v(pieces)
	return r.match(pieces, row).find({ id: row.id, span: row.span, width: Compute.measure(row.span) })
})
const quotient = query(Theory).rule((r) => {
	const row = v(Row)
	return r
		.match(Row, row)
		.find({ id: row.id, value: Compute.mulDiv(row.id, Compute.u64(2n), Compute.u64(3n), "towardZero") })
})
const checked = query(Theory).rule((r) => {
	const row = v(Row)
	return r
		.match(Row, row)
		.find({ id: row.id, value: Compute.divide(Compute.multiply(row.id, Compute.u64(2n)), Compute.u64(3n)) })
})
const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
const paths: string[] = []
const median = (samples: number[]) => samples.toSorted((a, b) => a - b)[Math.floor(samples.length / 2)]
const sourceHash = createHash("sha256")
const sourceFiles = execFileSync(
	"git",
	[
		"ls-files",
		"--cached",
		"--others",
		"--exclude-standard",
		"--deduplicate",
		"-z",
		"--",
		"crates",
		"ts",
		"ts-log",
		"crates/bumbledb-bench/fixtures/conformance",
		"Cargo.toml",
		"Cargo.lock",
		"scripts/structural-corpus.py"
	],
	{ encoding: "utf8" }
)
	.split("\0")
	.filter(Boolean)
	.sort()
for (const file of sourceFiles) sourceHash.update(file).update("\0").update(readFileSync(file)).update("\0")
console.log(
	JSON.stringify({
		source: execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim(),
		workingTree: "structural extension candidate",
		sourceSha256: sourceHash.digest("hex"),
		platform: platform(),
		arch: arch(),
		cpu: cpus()[0]?.model,
		node: process.version,
		repetitions: 9,
		warmup: 2
	})
)
try {
	await runtime.runPromise(
		Effect.scoped(
			Effect.gen(function* () {
				for (const count of [0, 1000, 10000]) {
					for (const width of [30n, 3_000_000_000_000n]) {
						const path = storeDir("structural-cost")
						paths.push(path)
						yield* Effect.scoped(
							Effect.gen(function* () {
								const db = yield* Db.create(path, Theory)
								const fact = (id: number) => ({
									id: BigInt(id),
									span: { start: 0n, end: width },
									clip: { start: width / 3n, end: (2n * width) / 3n }
								})
								const draft = yield* ChangeSet.builder(Theory)
								yield* draft.insert(
									Row,
									Array.from({ length: count }, (_, i) => fact(i))
								)
								yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
								const snap = yield* db.snapshot()
								const results: Record<string, unknown> = {
									count,
									width: width.toString()
								}
								for (const [name, execution, expectedRows] of [
									[
										"segmentsAndMeasure",
										snap.execute(measured, {}).pipe(
											Effect.flatMap((result) => result.collect()),
											Effect.map((rows) => rows.length)
										),
										count * 2
									],
									[
										"mulDiv",
										snap.execute(quotient, {}).pipe(
											Effect.flatMap((result) => result.collect()),
											Effect.map((rows) => rows.length)
										),
										count
									],
									[
										"multiplyThenDivide",
										snap.execute(checked, {}).pipe(
											Effect.flatMap((result) => result.collect()),
											Effect.map((rows) => rows.length)
										),
										count
									]
								] as const) {
									const samples: number[] = []
									let rows = 0
									for (let i = 0; i < 11; i++) {
										const start = performance.now()
										rows = yield* execution
										if (rows !== expectedRows) throw new Error(`${name}: incomplete output`)
										if (i >= 2) samples.push(performance.now() - start)
									}
									results[name] = { rows, medianMs: median(samples), samplesMs: samples }
								}
								console.log(JSON.stringify(results))
							})
						)
					}
				}
			})
		)
	)
} finally {
	await Effect.runPromise(runtime.disposeEffect)
	for (const path of paths) rmSync(path, { recursive: true, force: true })
}
