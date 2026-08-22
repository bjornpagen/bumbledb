/**
 * Consumer-pattern pins: the suspicious usages the graph-builder driver
 * makes of this SDK, replicated against the REAL run-store theory (the
 * 17-newtype fixture, `fixtures/run-store-schema.ts`) on real durable
 * stores:
 *
 * - the fixture theory lowers, admits, and fingerprints deterministically
 *   ACROSS PROCESSES (a second process's evaluation of the same theory
 *   reopens the store — resume = reopen, PRD-16 / frozen ruling 3);
 * - the exclusive-lock law the view lane's `peekRunStore` rides (a store
 *   held by a live worker in another process refuses a second open), and
 *   kill-durability (SIGKILL loses nothing committed, including the fresh
 *   high-water mark — the crash-window resume premise);
 * - the repair loop's violations round-trip: rejected commits return
 *   `===`-matchable statement values (what `store/diag-map.ts` resolves),
 *   a rejection leaves the store untouched, and the rebuilt delta commits;
 * - the store's revision idioms (sheet resupply update, verdict revise,
 *   judged-commit revert with its nested read-inside-write capture);
 * - keyed point reads through the PRIMARY-KEY rule exactly as the driver
 *   can and cannot spell them;
 * - snapshot pinning under interleaved writes;
 * - fresh-mint identity across rejected commits (what ids the repair
 *   loop's persisted diagnostics can safely cite).
 */

import assert from "node:assert/strict"
import type { ChildProcess } from "node:child_process"
import { spawn } from "node:child_process"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, before, describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import type { Db as DbValue, Fact, Violation, WriteOutcome } from "#index.ts"
import { Db } from "#index.ts"
import { accepted } from "#test/accepted.ts"
import type { RunStoreSchema } from "#test/fixtures/run-store-schema.ts"
import {
	attempt,
	attemptText,
	capsule,
	entryFormBans,
	grp,
	grpMember,
	laws,
	member,
	objective,
	program,
	runStoreSchema,
	sheet,
	steer,
	task,
	unit,
	verdict
} from "#test/fixtures/run-store-schema.ts"
import { put } from "#test/put.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-consumer-"))
const packageRoot = path.resolve(import.meta.dirname, "..")
const childScript = path.join(import.meta.dirname, "fixtures", "reopen-child.ts")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

/** Unwraps a value the surrounding test just proved present. */
function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

/** The run-store schema's relations record — what the violation values type against. */
type RunRels = RunStoreSchema["relations"]

/** Narrows a write result to its rejection's violations, failing the test on an accept. */
function rejected(result: WriteOutcome<RunRels, unknown>): readonly Violation<RunRels>[] {
	if (result.tag !== "rejected") {
		assert.fail("expected the commit to be rejected")
	}
	return result.violations
}

/** The child's one-line JSON report (`fixtures/reopen-child.ts`). */
interface ChildReport {
	readonly sheet: string
	readonly objectives: readonly string[]
	readonly stagingGrp: string
	readonly deletedGrp: string
	readonly task: string
	readonly attempt: string
	readonly grpRows: number
	readonly grpMemberRows: number
}

/** Spawns the reopen child and resolves its report line plus the process handle. */
function spawnChild(mode: "create" | "hold", dir: string): Promise<{ report: ChildReport; child: ChildProcess }> {
	return new Promise(function run(resolve, reject) {
		const child = spawn(process.execPath, [childScript, mode, dir], {
			cwd: packageRoot,
			stdio: ["ignore", "pipe", "pipe"]
		})
		let out = ""
		let err = ""
		const timer = setTimeout(function timeout() {
			child.kill("SIGKILL")
			reject(errors.new(`reopen child (${mode}) timed out; stderr: ${err}`))
		}, 30000)
		child.stdout.on("data", function collect(chunk: Buffer) {
			out += chunk.toString()
			const line = out.indexOf("\n")
			if (line >= 0) {
				clearTimeout(timer)
				resolve({ report: JSON.parse(out.slice(0, line)), child })
			}
		})
		child.stderr.on("data", function collectErr(chunk: Buffer) {
			err += chunk.toString()
		})
		child.on("exit", function exited(code) {
			if (out.indexOf("\n") < 0) {
				clearTimeout(timer)
				reject(errors.new(`reopen child (${mode}) exited ${code} without a report; stderr: ${err}`))
			}
		})
	})
}

/** Waits for a spawned child's exit. */
function waitExit(child: ChildProcess): Promise<void> {
	return new Promise(function run(resolve) {
		if (child.exitCode !== null || child.signalCode !== null) {
			resolve()
			return
		}
		child.on("exit", function exited() {
			resolve()
		})
	})
}

describe("cross-process reopen of the real run-store theory", function crossProcess() {
	test("resume = reopen: a store created by another process admits this process's evaluation of the theory", async function reopen() {
		const dir = path.join(tmpRoot, "reopen")
		const { report, child } = await spawnChild("create", dir)
		await waitExit(child)
		const db = await Db.open(dir, runStoreSchema)
		assert.equal(db.read((i) => i.scan(grp)).length, report.grpRows)
		assert.equal(db.read((i) => i.scan(grpMember)).length, report.grpMemberRows)
		const sheetRow = must(
			db
				.read((i) => i.scan(sheet))
				.find(function byName(row) {
					return row.name === "child-sheet"
				})
		)
		assert.equal(String(sheetRow.id), report.sheet)
		const attemptRow = must(db.read((i) => i.scan(attempt))[0])
		const verdictRow = must(db.read((i) => i.get(verdict, { attempt: attemptRow.id })))
		assert.equal(verdictRow.outcome, "Rejected")
		/**
		 * The fresh high-water survives a clean exit: the child minted grp
		 * `deletedGrp` and committed its delete (the revert idiom), so a
		 * resumed run's next mint must be strictly greater — never a re-issue
		 * of an id that was observable in a committed state.
		 */
		const state: { minted?: bigint } = {}
		const written = db.write(function mintAfterResume(tx) {
			const row = put(tx, grp, { sheet: sheetRow.id, label: "post-resume", context: "c" })
			state.minted = row.id
		})
		assert.equal(written.tag, "accepted", "the post-resume grp insert commits")
		assert.ok(
			must(state.minted) > BigInt(report.deletedGrp),
			`post-resume mint ${state.minted} must exceed the committed-then-deleted grp id ${report.deletedGrp}`
		)
	})

	test("the exclusive lock refuses a second process's open while held, and SIGKILL loses nothing committed", async function lockAndKill() {
		const dir = path.join(tmpRoot, "held")
		const { report, child } = await spawnChild("hold", dir)
		try {
			/** The peekRunStore premise: a live worker in another process refuses this open. */
			await assert.rejects(async function openHeld() {
				await Db.open(dir, runStoreSchema)
			}, /another live handle holds this environment's lock/)
		} finally {
			child.kill("SIGKILL")
			await waitExit(child)
		}
		/** Resume after a kill: everything committed survives, per-commit fsync. */
		const db = await Db.open(dir, runStoreSchema)
		assert.equal(db.read((i) => i.scan(grp)).length, report.grpRows)
		assert.equal(db.read((i) => i.scan(grpMember)).length, report.grpMemberRows)
		assert.equal(db.read((i) => i.scan(verdict)).length, 1)
		assert.equal(db.read((i) => i.scan(attemptText)).length, 1)
		/** The fresh high-water also survives the kill — no re-issue of the reverted grp id. */
		const state: { minted?: bigint } = {}
		const sheetRow = must(db.read((i) => i.scan(sheet))[0])
		const written = db.write(function mintAfterKill(tx) {
			const row = put(tx, grp, { sheet: sheetRow.id, label: "post-kill", context: "c" })
			state.minted = row.id
		})
		assert.equal(written.tag, "accepted", "the post-kill grp insert commits")
		assert.ok(
			must(state.minted) > BigInt(report.deletedGrp),
			`post-kill mint ${state.minted} must exceed the committed-then-deleted grp id ${report.deletedGrp}`
		)
	})
})

describe("the repair loop against the real theory", function repairLoop() {
	let db: DbValue<RunStoreSchema["relations"]>

	/** The branded ids the sequential tests hand forward. */
	const ids: {
		sheet?: Fact<typeof sheet>["id"]
		unit?: Fact<typeof unit>["id"]
		objectives: Fact<typeof objective>["id"][]
		staging?: Fact<typeof grp>["id"]
		planGrps: Fact<typeof grp>["id"][]
		task?: Fact<typeof task>["id"]
		attempt?: Fact<typeof attempt>["id"]
		program?: Fact<typeof program>["id"]
	} = { objectives: [], planGrps: [] }

	before(async function create() {
		db = accepted(await Db.create(path.join(tmpRoot, "main"), runStoreSchema))
	})

	test("the enrich-commit shape lands atomically (sheet + units + objectives + staging partition + task)", function enrichShape() {
		const written = db.write(function build(tx) {
			const sheetRow = put(tx, sheet, {
				name: "sheet-7",
				grade: "G7",
				contentHash: new Uint8Array(32)
			})
			ids.sheet = sheetRow.id
			const unitRow = put(tx, unit, {
				sheet: sheetRow.id,
				sourceUnitId: "u1",
				title: "unit one",
				description: "d",
				scope: "s"
			})
			ids.unit = unitRow.id
			const staging = put(tx, grp, {
				sheet: sheetRow.id,
				label: "STAGING",
				context: "partition pending"
			})
			ids.staging = staging.id
			for (const ref of ["G7_a", "G7_b"]) {
				const minted = put(tx, objective, {
					sheet: sheetRow.id,
					unit: unitRow.id,
					ref,
					goal: `goal ${ref}`
				})
				ids.objectives.push(minted.id)
				put(tx, grpMember, { grp: staging.id, objective: minted.id })
			}
			const taskRow = put(tx, task, {
				kind: "Cartograph",
				sheet: sheetRow.id,
				subject: 1n
			})
			ids.task = taskRow.id
		})
		assert.equal(written.tag, "accepted", "the enrich shape satisfies the theory")
	})

	test("a cartograph swap that uncovers an objective is rejected citing partitionTotality by identity, and the store is untouched", function rejectedSwap() {
		const grpsBefore = db.read((i) => i.scan(grp))
		const membersBefore = db.read((i) => i.scan(grpMember))
		const written = db.write(function badSwap(tx) {
			for (const row of membersBefore) {
				tx.delete(grpMember, [row])
			}
			for (const row of grpsBefore) {
				tx.delete(grp, [row])
			}
			const only = put(tx, grp, { sheet: must(ids.sheet), label: "g-one", context: "c" })
			/** Only one of the two objectives is re-covered: the other's within(1n) capacity law drops to 0. */
			put(tx, grpMember, { grp: only.id, objective: must(ids.objectives[0]) })
		})
		const violations = rejected(written)
		assert.equal(violations.length, 1, "exactly one statement is violated")
		const violation = must(violations[0])
		assert.strictEqual(
			violation.statement,
			laws.partitionTotality,
			"the violation carries the IDENTICAL statement value the diag-map ===-matches"
		)
		assert.equal(violation.kind, "capacity")
		assert.ok(violation.facts.length > 0, "the uncovered parent is cited")
		/** Rejection is data and the store is untouched — the repair loop's premise. */
		assert.deepEqual(
			db.read((i) => i.scan(grp)),
			grpsBefore
		)
		assert.deepEqual(
			db.read((i) => i.scan(grpMember)),
			membersBefore
		)
	})

	test("the rebuilt delta commits (delta rebuild after rejection)", function rebuiltSwap() {
		const grpsBefore = db.read((i) => i.scan(grp))
		const membersBefore = db.read((i) => i.scan(grpMember))
		const written = db.write(function goodSwap(tx) {
			for (const row of membersBefore) {
				tx.delete(grpMember, [row])
			}
			for (const row of grpsBefore) {
				tx.delete(grp, [row])
			}
			for (const [index, objectiveId] of ids.objectives.entries()) {
				const minted = put(tx, grp, {
					sheet: must(ids.sheet),
					label: `plan-${index}`,
					context: "c"
				})
				ids.planGrps.push(minted.id)
				put(tx, grpMember, { grp: minted.id, objective: objectiveId })
			}
		})
		assert.equal(written.tag, "accepted", "the corrected swap satisfies partition totality")
		assert.equal(db.read((i) => i.scan(grp)).length, 2)
		assert.equal(db.read((i) => i.scan(grpMember)).length, 2)
	})

	test("a misauthored hierarchy program is rejected citing BOTH the parent-count capacity law and the entry-form ban by identity", function rejectedAuthor() {
		const written = db.write(function badAuthor(tx) {
			const programRow = put(tx, program, {
				grp: must(ids.planGrps[0]),
				kind: "hierarchy_program"
			})
			const capsuleRow = put(tx, capsule, {
				program: programRow.id,
				ref: "fin_entry",
				toi: "RegularNoun",
				taughtClaim: "t",
				priorAssumption: "p",
				exitCondition: "e",
				transferRange: "r"
			})
			put(tx, member, {
				program: programRow.id,
				capsule: capsuleRow.id,
				pos: 1n,
				kind: "Taught",
				toi: "RegularNoun"
			})
		})
		const violations = rejected(written)
		const statements = new Set(
			violations.map(function statementOf(violation: Violation<RunStoreSchema["relations"]>) {
				return violation.statement
			})
		)
		assert.ok(
			statements.has(laws.hierarchyParentCount),
			"the exactly-one HigherOrderNoun capacity law is cited by identity"
		)
		const regularNounEntryBan = must(
			entryFormBans.find(function byToi(ban) {
				return ban.toi === "RegularNoun"
			})
		)
		assert.ok(
			statements.has(regularNounEntryBan.statement),
			"the generated entry-form ban is cited by identity (the diag-map's generated-family lane)"
		)
		assert.equal(db.read((i) => i.scan(program)).length, 0, "the rejected author payload left nothing behind")
	})

	test("the corrected author payload commits", function acceptedAuthor() {
		const written = db.write(function goodAuthor(tx) {
			const programRow = put(tx, program, {
				grp: must(ids.planGrps[0]),
				kind: "hierarchy_program"
			})
			ids.program = programRow.id
			const parent = put(tx, capsule, {
				program: programRow.id,
				ref: "fin_parent",
				toi: "HigherOrderNoun",
				taughtClaim: "t",
				priorAssumption: "p",
				exitCondition: "e",
				transferRange: "r"
			})
			const intro = put(tx, capsule, {
				program: programRow.id,
				ref: "fin_intro",
				toi: "RegularNoun",
				taughtClaim: "t",
				priorAssumption: "p",
				exitCondition: "e",
				transferRange: "r"
			})
			put(tx, member, {
				program: programRow.id,
				capsule: parent.id,
				pos: 1n,
				kind: "Taught",
				toi: "HigherOrderNoun"
			})
			put(tx, member, {
				program: programRow.id,
				capsule: intro.id,
				pos: 2n,
				kind: "Taught",
				toi: "RegularNoun"
			})
		})
		assert.equal(written.tag, "accepted", "the corrected hierarchy program satisfies the family laws")
	})

	test("a delete that would dangle references is rejected citing the containments by identity, store untouched", function danglingDelete() {
		const target = must(
			db
				.read((i) => i.scan(grp))
				.find(function byId(row) {
					return row.id === must(ids.planGrps[0])
				})
		)
		const before = db.read((i) => i.scan(grp))
		const written = db.write(function badDelete(tx) {
			tx.delete(grp, [target])
		})
		const violations = rejected(written)
		const statements = new Set(
			violations.map(function statementOf(violation: Violation<RunStoreSchema["relations"]>) {
				return violation.statement
			})
		)
		assert.ok(statements.has(laws.grpMemberGrpRef), "the membership containment is cited")
		assert.ok(statements.has(laws.programGrpRef), "the program containment is cited")
		for (const violation of violations) {
			if (violation.kind !== "containment") {
				continue
			}
			assert.ok(violation.direction !== undefined, "containment violations carry a direction")
		}
		assert.deepEqual(
			db.read((i) => i.scan(grp)),
			before
		)
	})

	test("the sheet resupply update (delete + insert with the SAME fresh id) commits in one delta", function sheetResupply() {
		const sheetRow = must(db.read((i) => i.scan(sheet))[0])
		const written = db.write(function resupply(tx) {
			tx.delete(sheet, [sheetRow])
			put(tx, sheet, {
				id: sheetRow.id,
				name: sheetRow.name,
				grade: "G8",
				contentHash: sheetRow.contentHash
			})
		})
		assert.equal(written.tag, "accepted", "identity-preserving revision commits — referencing rows never dangle")
		const rows = db.read((i) => i.scan(sheet))
		assert.equal(rows.length, 1)
		const revised = must(rows[0])
		assert.equal(revised.id, sheetRow.id)
		assert.equal(revised.grade, "G8")
	})

	test("the verdict revise idiom (delete + insert under the one-verdict-per-attempt key) commits in one delta", function verdictRevise() {
		const seeded = db.write(function seedAttempt(tx) {
			const attemptRow = put(tx, attempt, {
				task: must(ids.task),
				n: 1n,
				pin: "Gpt56Max",
				promptHash: new Uint8Array(32)
			})
			ids.attempt = attemptRow.id
			put(tx, verdict, { attempt: attemptRow.id, outcome: "Accepted" })
		})
		assert.equal(seeded.tag, "accepted")
		const attemptId = must(ids.attempt)
		const current = must(db.read((i) => i.get(verdict, { attempt: attemptId })))
		assert.equal(current.outcome, "Accepted")
		const revised = db.write(function revise(tx) {
			tx.delete(verdict, [current])
			put(tx, verdict, { attempt: attemptId, outcome: "Rejected" })
		})
		assert.equal(revised.tag, "accepted", "the settleReviewEdge refutation write commits")
		assert.equal(must(db.read((i) => i.get(verdict, { attempt: attemptId }))).outcome, "Rejected")
	})

	test("the revert-capture idiom: a db.read INSIDE a db.write callback sees the committed pre-delta state", function nestedRead() {
		const written = db.write(function revertShaped(tx) {
			put(tx, steer, { kind: "Observe", task: must(ids.task), note: "diary" })
			/** The nested snapshot must NOT see the pending insert — it reads committed state. */
			const captured = db.read(function capture(snap) {
				return snap.scan(steer)
			})
			assert.equal(captured.length, 0, "the nested read sees the pre-delta committed state")
		})
		assert.equal(written.tag, "accepted")
		assert.equal(db.read((i) => i.scan(steer)).length, 1)
	})

	test("a read scope stays pinned at its snapshot across an interleaved commit", function pinnedScope() {
		db.read(function observe(snap) {
			const before = snap.scan(steer).length
			const written = db.write(function interleaved(tx) {
				put(tx, steer, { kind: "Observe", task: must(ids.task), note: "second" })
			})
			assert.equal(written.tag, "accepted")
			assert.equal(snap.scan(steer).length, before, "the open scope never sees the new commit")
		})
		assert.equal(db.read((i) => i.scan(steer)).length, 2, "a fresh read sees it")
	})

	test("the PRIMARY-KEY rule: get reads through the fresh field or the first declared key, and NOTHING else", function primaryKeyRule() {
		const taskId = must(ids.task)
		assert.ok(db.read((i) => i.get(task, { id: taskId })) !== undefined, "fresh-keyed get hits")
		/** The 2-arg form refuses the declared identity key (kind, subject): its KeyFact is the primary projection alone. The declared-key read is the 3-arg keyed form — get(task, keyStatement, key) (70-api ledger row (b), SHIPPED 2026-07-19; ts/test/keyed-get.test.ts pins it). */
		assert.throws(function declaredKeyGet() {
			// @ts-expect-error — the KeyFact type demands exactly the fresh field; this pins the runtime refusal too
			db.read((i) => i.get(task, { kind: "Cartograph", subject: 1n }))
		}, /missing field id/)
		const objectiveId = must(ids.objectives[0])
		const membership = db.read((i) => i.get(grpMember, { objective: objectiveId }))
		assert.ok(membership !== undefined, "a freshless relation's first declared key is the get lane")
		assert.throws(function offProjectionGet() {
			db.read((i) => i.get(grpMember, { grp: membership?.grp }))
		}, /missing field objective/)
		const attemptId = must(ids.attempt)
		assert.equal(
			db.read((i) => i.get(attemptText, { attempt: attemptId })),
			undefined,
			"keyed miss is undefined"
		)
	})

	test("a resupplied duplicate fresh id violates the engine-materialized auto-key: statement is undefined (the repair loop's identity gap)", function autoKeyGap() {
		const existing = must(db.read((i) => i.scan(grp))[0])
		const written = db.write(function duplicateId(tx) {
			put(tx, grp, {
				id: existing.id,
				sheet: must(ids.sheet),
				label: "impostor",
				context: "c"
			})
		})
		const violations = rejected(written)
		assert.equal(violations.length, 1)
		const violation = must(violations[0])
		assert.equal(
			violation.statement,
			undefined,
			"fresh-implied auto-keys have no declared spelling — dispatch.ts can only render the canonical string"
		)
		assert.equal(violation.kind, "functionality")
		assert.ok(violation.canonical.length > 0, "the canonical rendering is the only identity carried")
	})

	test("fresh mint identity across a REJECTED commit: what ids persisted diagnostics may cite", function rejectedMint() {
		const state: { doomed?: bigint; next?: bigint } = {}
		const written = db.write(function doomedMint(tx) {
			const minted = put(tx, grp, { sheet: must(ids.sheet), label: "doomed-mint", context: "c" })
			state.doomed = minted.id
			/** Force rejection through the objective ref key. */
			put(tx, objective, {
				sheet: must(ids.sheet),
				unit: must(ids.unit),
				ref: "G7_a",
				goal: "duplicate"
			})
		})
		rejected(written)
		const again = db.write(function nextMint(tx) {
			const minted = put(tx, grp, { sheet: must(ids.sheet), label: "next-mint", context: "c" })
			state.next = minted.id
		})
		assert.equal(again.tag, "accepted")
		assert.ok(
			must(state.next) > must(state.doomed),
			`a fresh id handed to the host by a rejected commit (${state.doomed}) must not be re-issued (${state.next}) — the repair loop persists diagnostics citing rejected-payload ids`
		)
	})
})
