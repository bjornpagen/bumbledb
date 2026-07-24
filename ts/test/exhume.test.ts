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
 *   the creating schema (run in a child process: an in-process `Db.open`
 *   would hold the environment forever, and heed's single-open rule
 *   refuses a second same-path open) back-fills the descriptor, and the
 *   same path then exhumes successfully;
 * - lifetimes are disposables (R12): `Symbol.dispose` releases the engine
 *   handle and its environment deterministically (the same path
 *   re-exhumes in-process after disposal), no `close` verb exists, a
 *   disposed value's verbs are typed refusals, disposal is idempotent, and
 *   an unknown relation name is a typed refusal;
 * - the lock law is a writer law (R17): exhume never creates the advisory
 *   `bumbledb.lock`, and reads a store whose data file and directory
 *   carry no write bits — the archival lane on read-only media.
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
				grade: "Pass",
				flag: true,
				score: -7n,
				digest: new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]),
				window: span(1n, 4n)
			})
			const beta = tx.insert(Specimen, {
				label: "βeta — π ≤ 4",
				grade: "Fail",
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
			/**
			 * The exhume surface is theory-less — BENEATH the marshal's
			 * name↔id bijection — so its closed cells are raw declaration-
			 * order row ids where the typed surface speaks handle names
			 * (H2). The comparison translates through the roster: Grade's
			 * Pass is row 0, Fail row 1.
			 */
			assert.deepStrictEqual(
				exhumed.scan("Specimen"),
				snap.scan(Specimen).map(function rawGrade(row) {
					return { ...row, grade: BigInt(Grade.data.handles.indexOf(row.grade)) }
				}),
				"exhumed Specimen rows equal the typed snap.scan rows with grade lowered to its row id"
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

	test("lifetimes are disposables (R12): Symbol.dispose releases the environment, and no close verb exists", async function surfaceShape() {
		const surfaceCopy = path.join(tmpRoot, "store-copy-surface")
		copyStore(storeDir, surfaceCopy)
		{
			/**
			 * The `using` idiom: disposal at scope exit releases the engine
			 * handle AND its environment deterministically — the exact
			 * lifetime GC reclamation could never promise (066: retry after
			 * a half-failed migration, a second forensic read).
			 */
			using exhumed = await Db.exhume(surfaceCopy)
			assert.equal("close" in exhumed, false, "no close verb exists — lifetimes are disposables, never close()")
			assert.equal(Symbol.asyncDispose in exhumed, false, "teardown is synchronous: Symbol.dispose is the protocol")
			assert.deepEqual(
				Reflect.ownKeys(exhumed).toSorted(function bySpelling(a, b) {
					return String(a) < String(b) ? -1 : 1
				}),
				[Symbol.dispose, "descriptor", "scan"].toSorted(function bySpelling(a, b) {
					return String(a) < String(b) ? -1 : 1
				}),
				"the surface is exactly descriptor + scan + the dispose protocol"
			)
			assert.throws(function ghost() {
				exhumed.scan("Ghost")
			}, /declares no relation Ghost/)
		}
		// The environment released at scope exit: the SAME path exhumes
		// again in-process (heed's single-open rule refuses a still-live
		// handle) — deterministic, never a GC race.
		using again = await Db.exhume(surfaceCopy)
		assert.ok(again.descriptor.relations.length > 0, "the same-path re-exhume lands after disposal")
	})

	test("a disposed exhumed value's verbs are typed refusals, and disposal is idempotent", async function disposedRefusal() {
		const disposedCopy = path.join(tmpRoot, "store-copy-disposed")
		copyStore(storeDir, disposedCopy)
		const exhumed = await Db.exhume(disposedCopy)
		assert.ok(exhumed.scan("Specimen").length > 0)
		exhumed[Symbol.dispose]()
		exhumed[Symbol.dispose]()
		assert.throws(function scanAfterDispose() {
			exhumed.scan("Specimen")
		}, /disposed — its using scope already exited/)
	})

	test("the lock law is a writer law (R17): exhume takes no advisory lock and reads on read-only media", async function locklessArchival() {
		const roCopy = path.join(tmpRoot, "store-copy-readonly")
		copyStore(storeDir, roCopy)
		{
			// A first exhume while the directory is writable: LMDB re-mints
			// its reader table (`lock.mdb`, stripped by the copy), and the
			// advisory `bumbledb.lock` never appears — no lock is taken even
			// where one could be.
			using minted = await Db.exhume(roCopy)
			assert.ok(minted.scan("Specimen").length > 0)
		}
		assert.equal(
			fs.existsSync(path.join(roCopy, "bumbledb.lock")),
			false,
			"no advisory lock was created — the lock law is a writer law"
		)
		// Read-only media, as far as a chmod fixture can spell it: the data
		// file and directory lose their write bits (LMDB's reader table
		// stays writable — on a genuinely read-only FILESYSTEM mdb.c omits
		// the lockfile under MDB_RDONLY).
		fs.chmodSync(path.join(roCopy, "data.mdb"), 0o444)
		fs.chmodSync(roCopy, 0o555)
		try {
			using exhumed = await Db.exhume(roCopy)
			assert.deepEqual(
				exhumed.descriptor.relations.map(function name(rel) {
					return rel.name
				}),
				["Grade", "Specimen", "Reading"],
				"the descriptor reads back from read-only media"
			)
			assert.ok(exhumed.scan("Specimen").length > 0, "rows read back from read-only media")
		} finally {
			fs.chmodSync(roCopy, 0o755)
			fs.chmodSync(path.join(roCopy, "data.mdb"), 0o644)
		}
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
