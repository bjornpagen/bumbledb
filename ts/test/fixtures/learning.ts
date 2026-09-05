/**
 * The chapter 34 `Learning` schema, transcribed for the Effect-core test
 * lanes, plus the shared measured policies. One fixture, every suite: the
 * same declarations the Rust `schema!` example spells, so cross-language
 * schema-identity checks (API-08, F-*) can pin one canonical fingerprint.
 */
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { ref, weigh, within } from "#capacity.ts"
import { on } from "#face.ts"
import { f64, i64, id128, interval, str, u64 } from "#fields.ts"
import { relation } from "#relation.ts"
import type { ExecutionPolicy, NativeRuntimeOptions } from "#runtime.ts"
import { schema } from "#schema.ts"
import { capacity, contained, key } from "#statements.ts"

export const Student = relation("Student", { id: id128, name: str, budget: u64 })
export const Attempt = relation("Attempt", {
	id: id128,
	student: id128,
	score: f64,
	units: u64,
	active: interval(i64)
})

export const Learning = schema("Learning", { Student, Attempt }, [
	key(Student, ["id"]),
	key(Attempt, ["id"]),
	contained(on(Attempt, "student"), on(Student, "id")),
	capacity(on(Student, "id"), {
		from: on(Attempt, "student"),
		weight: weigh("units"),
		within: within(0n, ref("budget"))
	})
])

export const runtimeOptions: NativeRuntimeOptions = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	inputBytes: 16_000_000n,
	workingBytes: 64_000_000n,
	scratchBytes: 64_000_000n,
	resultBytes: 16_000_000n,
	chunkBytes: 1_000_000n,
	cleanupTimeout: "2 seconds"
}

export const work: ExecutionPolicy = {
	inputBytes: 4_000_000n,
	workingBytes: 16_000_000n,
	scratchBytes: 16_000_000n,
	resultBytes: 4_000_000n,
	rows: 100_000n,
	workUnits: 10_000_000n,
	timeout: "10 seconds"
}

/** A fresh store directory per test; the caller's scope owns the database. */
export function storeDir(tag: string): string {
	const dir = path.join(os.tmpdir(), `bumbledb-effect-${tag}-${process.pid}`)
	fs.rmSync(dir, { recursive: true, force: true })
	return dir
}
