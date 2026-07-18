/**
 * The exhume surface's semantic pins (course-serialization PRD-02), against
 * REAL temp stores:
 *
 * - a store declaring every field type (domain-labeled u64 fresh, i64, str,
 *   bytes<8>, bool, u64/i64 intervals, closed id with a payload roster) is
 *   created and seeded under its true schema, its directory copied to a
 *   process-fresh path (never opened, never cached, never locked in this
 *   process), and `Db.exhume` reads the copy with NO theory in scope: the
 *   descriptor lists every relation and sealed field, the closed roster
 *   arrives with handles and payload values, and `scan` returns every row
 *   with values IDENTICAL (bigint/string/bytes equality) to what typed
 *   `snap.scan` returns under the true schema;
 * - the adoption loop at SDK level: the committed legacy fixture
 *   (`fixtures/legacy-store`, created by the pre-descriptor engine — see
 *   `fixtures/legacy-schema.ts` for provenance) refuses exhume with the
 *   typed `ErrExhumeNoDescriptor`, ONE fingerprint-matching `Db.open` under
 *   the creating schema (run in a child process, since an open in this
 *   process would hold the exclusive lock forever) back-fills the
 *   descriptor, and the same path then exhumes successfully;
 * - zero closables: no close/dispose spelling exists on the exhumed value,
 *   and an unknown relation name is a typed refusal.
 */

import assert from "node:assert/strict"
import type { ChildProcess } from "node:child_process"
import { spawn } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import * as errors from "@superbuilders/errors"

import {
	bool,
	bytes,
	closed,
	contained,
	Db,
	ErrExhumeNoDescriptor,
	i64,
	interval,
	key,
	on,
	relation,
	schema,
	span,
	str,
	u64
} from "#index.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-exhume-"))
const packageRoot = path.resolve(import.meta.dirname, "..")
const adoptScript = path.join(import.meta.dirname, "fixtures", "adopt-child.ts")
const legacyFixture = path.join(import.meta.dirname, "fixtures", "legacy-store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Grade = closed("Grade", { points: u64 }, { Pass: { points: 10n }, Fail: { points: 0n } })
const Specimen = relation("Specimen", {
	id: u64.fresh,
	label: str,
	grade: Grade.id,
	flag: bool,
	score: i64,
	digest: bytes(8),
	window: interval(u64)
})
const Reading = relation("Reading", {
	specimen: u64,
	note: str,
	at: interval(i64)
})

const Exhumable = schema("Exhumable", { Grade, Specimen, Reading }, [
	key(Reading, ["specimen"]),
	contained(on(Reading, "specimen"), on(Specimen, "id"))
])

/**
 * Copies a store directory to a fresh path, stripping the per-open lock
 * artifacts (LMDB's `lock.mdb` reader table and the `bumbledb.lock`
 * advisory file — both recreated by the engine at open; the copied
 * `lock.mdb` would carry the SOURCE store's live reader slots).
 */
function copyStore(from: string, to: string): void {
	fs.cpSync(from, to, { recursive: true })
	fs.rmSync(path.join(to, "lock.mdb"), { force: true })
	fs.rmSync(path.join(to, "bumbledb.lock"), { force: true })
}

/** The adoption child's one-line JSON report (`fixtures/adopt-child.ts`). */
interface AdoptReport {
	readonly docRows: number
	readonly taggedRows: number
}

/**
 * Runs the adoption child to completion and parses its report. No timeout
 * exists here (the house no-limits law): a hung child hangs the test
 * loudly, which is the correct failure mode.
 */
function adoptInChild(dir: string): Promise<AdoptReport> {
	return new Promise(function run(resolve, reject) {
		const child: ChildProcess = spawn(process.execPath, [adoptScript, dir], {
			cwd: packageRoot,
			stdio: ["ignore", "pipe", "pipe"]
		})
		let out = ""
		let err = ""
		child.stdout?.on("data", function collect(chunk: Buffer) {
			out += chunk.toString()
		})
		child.stderr?.on("data", function collectErr(chunk: Buffer) {
			err += chunk.toString()
		})
		child.on("exit", function exited(code) {
			const line = out.indexOf("\n")
			if (code === 0 && line >= 0) {
				resolve(JSON.parse(out.slice(0, line)))
				return
			}
			reject(errors.new(`adopt child exited ${code} without a report; stderr: ${err}`))
		})
	})
}

describe("the exhume surface against real stores", function suite() {
	const storeDir = path.join(tmpRoot, "store")
	const copyDir = path.join(tmpRoot, "store-copy")

	test("every field type survives the theory-less read of a process-fresh copy", async function everyFieldType() {
		const db = await Db.create(storeDir, Exhumable)
		const written = db.write(function seed(tx) {
			const alpha = tx.insert(Specimen, {
				label: "alpha",
				grade: Grade.Pass,
				flag: true,
				score: -7n,
				digest: new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]),
				window: span(1n, 4n)
			})
			const beta = tx.insert(Specimen, {
				label: "βeta — π ≤ 4",
				grade: Grade.Fail,
				flag: false,
				score: 42n,
				digest: new Uint8Array([255, 0, 254, 1, 253, 2, 252, 3]),
				window: span(5n, 9n)
			})
			tx.insert(Reading, { specimen: alpha.id, note: "first contact", at: span(-3n, 3n) })
			tx.insert(Reading, { specimen: beta.id, note: "second contact", at: span(-9n, -1n) })
		})
		assert.ok(written.ok, "the seed commit lands")

		copyStore(storeDir, copyDir)
		const exhumed = await Db.exhume(copyDir)

		assert.deepEqual(
			exhumed.descriptor.relations.map(function name(rel) {
				return rel.name
			}),
			["Grade", "Specimen", "Reading"],
			"relations arrive in engine-id (declaration) order"
		)

		const grade = exhumed.descriptor.relations[0]
		assert.ok(grade)
		assert.deepEqual(
			grade.fields.map(function tag(field) {
				return `${field.name}:${field.valueType.kind}`
			}),
			["id:u64", "points:u64"],
			"the closed relation's sealed field list opens with the synthetic id"
		)
		assert.ok(grade.roster, "a closed relation carries its roster")
		assert.deepStrictEqual(
			grade.roster.map(function axiom(row) {
				return { handle: row.handle, id: row.id, values: { ...row.values } }
			}),
			[
				{ handle: "Pass", id: 0n, values: { points: 10n } },
				{ handle: "Fail", id: 1n, values: { points: 0n } }
			],
			"the roster carries every ground axiom's handle, row id, and payload"
		)

		const specimen = exhumed.descriptor.relations[1]
		assert.ok(specimen)
		assert.deepEqual(
			specimen.fields.map(function tag(field) {
				return `${field.name}:${field.valueType.kind}`
			}),
			["id:u64", "label:string", "grade:u64", "flag:bool", "score:i64", "digest:fixedBytes", "window:interval"],
			"every declared field arrives with its structural type tag"
		)
		const digestType = specimen.fields[5]?.valueType
		assert.ok(digestType && digestType.kind === "fixedBytes")
		assert.equal(digestType.len, 8, "byte width crosses where applicable")
		const windowType = specimen.fields[6]?.valueType
		assert.ok(windowType && windowType.kind === "interval")
		assert.equal(windowType.element, "u64")
		assert.equal(specimen.roster, undefined, "an ordinary relation has no roster")

		const reading = exhumed.descriptor.relations[2]
		assert.ok(reading)
		const atType = reading.fields[2]?.valueType
		assert.ok(atType && atType.kind === "interval")
		assert.equal(atType.element, "i64")

		db.read(function compare(snap) {
			assert.deepStrictEqual(
				exhumed.scan("Specimen"),
				snap.scan(Specimen),
				"exhumed Specimen rows equal the typed snap.scan rows, value for value"
			)
			assert.deepStrictEqual(
				exhumed.scan("Reading"),
				snap.scan(Reading),
				"exhumed Reading rows equal the typed snap.scan rows, value for value"
			)
		})
		assert.deepStrictEqual(
			exhumed.scan("Grade"),
			[
				{ id: 0n, points: 10n },
				{ id: 1n, points: 0n }
			],
			"a closed relation scans its sealed roster (the typed surface refuses closed scans)"
		)
	})

	test("zero closables and the typed unknown-relation refusal", async function surfaceShape() {
		/**
		 * A fresh copy: the previous test's exhumed value may not be
		 * collected yet, and a live handle holds its path's exclusive lock
		 * (reclamation is GC's, never a close verb's).
		 */
		const surfaceCopy = path.join(tmpRoot, "store-copy-surface")
		copyStore(storeDir, surfaceCopy)
		const exhumed = await Db.exhume(surfaceCopy)
		assert.equal("close" in exhumed, false)
		assert.equal(Symbol.dispose in exhumed, false)
		assert.equal(Symbol.asyncDispose in exhumed, false)
		assert.deepEqual(
			Reflect.ownKeys(exhumed).toSorted(),
			["descriptor", "scan"],
			"the surface is exactly descriptor + scan"
		)
		assert.throws(function ghost() {
			exhumed.scan("Ghost")
		}, /declares no relation Ghost/)
	})

	test("the adoption loop: a legacy store refuses exhume, one open under the creating schema adopts it", async function adoption() {
		const legacyDir = path.join(tmpRoot, "legacy")
		copyStore(legacyFixture, legacyDir)

		await assert.rejects(
			async function beforeAdoption() {
				await Db.exhume(legacyDir)
			},
			function refusal(error: Error) {
				assert.ok(errors.is(error, ErrExhumeNoDescriptor), `expected ErrExhumeNoDescriptor, got: ${error.message}`)
				assert.match(error.message, /open it once under its creating schema/)
				return true
			}
		)

		const report = await adoptInChild(legacyDir)
		assert.equal(report.docRows, 1)
		assert.equal(report.taggedRows, 1)

		const exhumed = await Db.exhume(legacyDir)
		assert.deepEqual(
			exhumed.descriptor.relations.map(function name(rel) {
				return rel.name
			}),
			["Doc", "Tagged"],
			"the back-filled descriptor is the creating declaration"
		)
		assert.deepStrictEqual(exhumed.scan("Doc"), [{ id: 0n, title: "the record outlives the schema" }])
		assert.deepStrictEqual(exhumed.scan("Tagged"), [{ doc: 0n, tag: "legacy" }])
	})
})
