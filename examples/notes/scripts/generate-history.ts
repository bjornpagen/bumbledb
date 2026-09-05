/**
 * Reproduces the checked-in `bumbledb/migrations/` chain from the staged
 * evolution history — the Drizzle-like edit → generate → review flow, one
 * stage at a time, through the REAL generator (native canonical codec and
 * digests; nothing here hashes or renders plan bytes in JS).
 *
 *   node --experimental-strip-types scripts/generate-history.ts
 *
 * Deterministic: rerunning over an up-to-date repo writes nothing
 * (`status: "unchanged"`); regenerating from scratch produces
 * byte-identical files. The generated artifacts are REVIEWED AND
 * COMMITTED; deployment consumes them as inert data and never runs this
 * script (chapter 33: generation never happens on a server against
 * whatever files exist there).
 */
import { NativeRuntime } from "@bjornpagen/bumbledb"
import type { GenerationReport } from "@bjornpagen/bumbledb-log/migrations"
import { generateMigrations } from "@bjornpagen/bumbledb-log/migrations"
import { Effect } from "effect"
import { App0, App1, App2, App3, App4, evolution1, evolution2, evolution3 } from "../src/db/evolution-stages.ts"
import { adminWork, runtimePolicy } from "../src/db/runtime-policy.ts"

const repository = { directory: "bumbledb/migrations" }

function announce(label: string) {
	return (report: GenerationReport) =>
		Effect.sync(() => {
			console.log(`${label}: ${report.status}${report.planId === null ? "" : ` → ${report.planId}`}`)
		})
}

const program = Effect.gen(function* () {
	yield* generateMigrations({ schema: App0, label: "initialize", repository, work: adminWork }).pipe(
		Effect.flatMap(announce("initialize"))
	)
	yield* generateMigrations({
		schema: App1,
		intent: evolution1,
		label: "note-pinned",
		repository,
		work: adminWork
	}).pipe(Effect.flatMap(announce("note-pinned")))
	yield* generateMigrations({
		schema: App2,
		intent: evolution2,
		label: "create-tag-seed-tag",
		repository,
		work: adminWork
	}).pipe(Effect.flatMap(announce("create-tag-seed-tag")))
	yield* generateMigrations({
		schema: App3,
		intent: evolution3,
		label: "note-text",
		repository,
		work: adminWork
	}).pipe(Effect.flatMap(announce("note-text")))
	yield* generateMigrations({ schema: App4, label: "outbox-attachment", repository, work: adminWork }).pipe(
		Effect.flatMap(announce("outbox-attachment"))
	)
})

await Effect.runPromise(program.pipe(Effect.provide(NativeRuntime.layer(runtimePolicy.native))))
