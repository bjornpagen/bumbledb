/**
 * Subprocess half of the consumer-patterns suite: replicates the
 * graph-builder run-store process model (PRD-16 resume = reopen; PRD-07 one
 * process, one exclusive-lock handle) from a REAL second process.
 *
 * Modes (argv[2] = mode, argv[3] = store dir):
 * - `create`: create the store with the run-store fixture theory, seed a
 *   driver-shaped slice (enrich-commit shape: sheet + unit + objectives +
 *   staging grp + memberships, then a task/attempt/verdict ledger chain),
 *   commit a revert-shaped delete of the max-id grp, print one JSON line,
 *   exit cleanly (the exit hook closes the environment).
 * - `hold`: same create + seed, then print the JSON line and HOLD the
 *   exclusive lock alive until killed — the parent asserts a second
 *   process's open is refused (the peekRunStore premise), then SIGKILLs
 *   this process and asserts the committed facts survived (per-commit
 *   fsync) and the fresh high-water mark did too.
 */

import * as errors from "@superbuilders/errors"

import type { Fact } from "#index.ts"
import { Db } from "#index.ts"
import {
	attempt,
	attemptText,
	grp,
	grpMember,
	Outcome,
	objective,
	Pin,
	runStoreSchema,
	sheet,
	TaskKind,
	task,
	unit,
	verdict
} from "#test/fixtures/run-store-schema.ts"

const mode = process.argv[2]
const dir = process.argv[3]
if ((mode !== "create" && mode !== "hold") || dir === undefined) {
	process.stderr.write("usage: reopen-child.ts <create|hold> <store-dir>\n")
	process.exit(2)
}

const db = await Db.create(dir, runStoreSchema)

const seeded: {
	sheet?: Fact<typeof sheet>["id"]
	objectives: bigint[]
	stagingGrp?: bigint
	deletedGrp?: bigint
	task?: bigint
	attempt?: bigint
} = { objectives: [] }

const written = db.write(function seed(tx) {
	const sheetRow = tx.insert(sheet, {
		name: "child-sheet",
		grade: "G7",
		contentHash: new Uint8Array(32)
	})
	seeded.sheet = sheetRow.id
	const unitRow = tx.insert(unit, {
		sheet: sheetRow.id,
		sourceUnitId: "u1",
		title: "unit one",
		description: "d",
		scope: "s"
	})
	const staging = tx.insert(grp, {
		sheet: sheetRow.id,
		label: "STAGING",
		context: "partition pending"
	})
	seeded.stagingGrp = staging.id
	for (const ref of ["G7_a", "G7_b"]) {
		const minted = tx.insert(objective, {
			sheet: sheetRow.id,
			unit: unitRow.id,
			ref,
			goal: `goal ${ref}`
		})
		seeded.objectives.push(minted.id)
		tx.insert(grpMember, { grp: staging.id, objective: minted.id })
	}
	const taskRow = tx.insert(task, {
		kind: TaskKind.Enrich,
		sheet: sheetRow.id,
		subject: 1n
	})
	seeded.task = taskRow.id
	const attemptRow = tx.insert(attempt, {
		task: taskRow.id,
		n: 1n,
		pin: Pin.Gpt56Max,
		promptHash: new Uint8Array(32)
	})
	seeded.attempt = attemptRow.id
	tx.insert(attemptText, { attempt: attemptRow.id, prompt: "p", output: "o" })
	tx.insert(verdict, { attempt: attemptRow.id, outcome: Outcome.Rejected })
})
if (!written.ok) {
	process.stderr.write(`child seed rejected: ${JSON.stringify(written.violations.length)}\n`)
	process.exit(3)
}

const seededSheet = seeded.sheet
if (seededSheet === undefined) {
	process.stderr.write("child seed minted no sheet id\n")
	process.exit(3)
}

/** The revert shape: mint a second grp (now the max grp id), then delete it in a committed write. */
const doomed = db.write(function mintDoomed(tx) {
	const minted = tx.insert(grp, {
		sheet: seededSheet,
		label: "doomed",
		context: "to be reverted"
	})
	seeded.deletedGrp = minted.id
})
if (!doomed.ok) {
	process.stderr.write("child doomed-grp insert rejected\n")
	process.exit(3)
}
const reverted = db.write(function revertDoomed(tx) {
	const row = db.read(function capture(snap) {
		return snap.scan(grp).find(function byLabel(candidate) {
			return candidate.label === "doomed"
		})
	})
	if (row === undefined) {
		throw errors.new("doomed grp vanished")
	}
	tx.delete(grp, row)
})
if (!reverted.ok) {
	process.stderr.write("child revert rejected\n")
	process.exit(3)
}

const report = JSON.stringify({
	sheet: String(seeded.sheet),
	objectives: seeded.objectives.map(String),
	stagingGrp: String(seeded.stagingGrp),
	deletedGrp: String(seeded.deletedGrp),
	task: String(seeded.task),
	attempt: String(seeded.attempt),
	grpRows: db.scan(grp).length,
	grpMemberRows: db.scan(grpMember).length
})
process.stdout.write(`${report}\n`)

if (mode === "hold") {
	/** Hold the exclusive lock until the parent kills this process. */
	setInterval(function keepAlive() {}, 1000)
}
