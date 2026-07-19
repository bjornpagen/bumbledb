/**
 * PRD-H2 probes: the marshal bijection over closed rosters, against a REAL
 * durable store. Handle NAMES cross the marshal boundary as values; u64
 * row ids stay the engine's truth:
 *
 * - the round trip: insert with `"Savings"`, final-state tx reads and
 *   snapshot `scan`/`get` all return `"Savings"` (strict equality), and
 *   `delete`/`contains` lower the NAME through the same one seam;
 * - the raw cell: the store holds `1n` — asserted through exhume, which
 *   stays RAW by design (it is the recovery surface: rows carry the
 *   engine's ids, and the roster crosses SEPARATELY on the descriptor);
 * - the write throw: an unknown handle name is a pointed marshal refusal
 *   naming the vocabulary and its roster (the 0.4.0 UPGRADE over 0.3.0's
 *   any-bigint-compiles), and a bare bigint is a shape refusal — the old
 *   spelling is dead;
 * - the read throw: an out-of-roster id — constructed via a LAWLESS twin
 *   store whose closed-typed column no containment law pins (raw bigints
 *   written under a plain-u64 twin schema with the identical fingerprint)
 *   — is a pointed error naming the missing law, never a silent fallback;
 * - violations: an offending fact's closed cell arrives as the NAME and
 *   agrees with the `canonical` string's engine rendering of the same
 *   handle;
 * - the marshal module itself stays literally cast-free (its own law).
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import type { Db as DbValue, RelationFields, Selected } from "#index.ts"
import { closed, contained, Db, key, mirrors, on, relation, renderStatement, schema, u64 } from "#index.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-marshal-"))
const storeDir = path.join(tmpRoot, "store")
const exhumeDir = path.join(tmpRoot, "store-exhume")
const lawlessDir = path.join(tmpRoot, "lawless")
const lawlessCopyDir = path.join(tmpRoot, "lawless-copy")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Kind = closed("Kind", ["Checking", "Savings"])
const Account = relation("Account", { id: u64.fresh, kind: Kind.id })
const SavingsTerms = relation("SavingsTerms", { account: u64 })

const savingsKey = key(SavingsTerms, ["account"])
const kindContainment = contained(on(Account, "kind"), on(Kind, "id"))

/**
 * The σ-selected mirror source, HAND-LOWERED (fixture-style): the binding
 * carries the already-lowered wire literal `{ kind: "handle" }`, so this
 * suite pins the MARSHAL bijection independent of the `where()` literal
 * machinery (H3's files own that seam). Structurally identical to what
 * `Account.where({ kind: "Savings" })` resolves to.
 */
const savingsSelected: Selected<"Account", RelationFields<typeof Account>> = {
	relation: Account,
	selection: [{ field: "kind", set: { kind: "one", literal: { kind: "handle", handle: "Savings" } } }]
}
const savingsMirror = mirrors(on(savingsSelected, "id"), on(SavingsTerms, "account"))

const Ledger = schema("MarshalLedger", { Kind, Account, SavingsTerms }, [savingsKey, kindContainment, savingsMirror])

/**
 * The lawless twins: the SAME wire shape (the closed linkage is SDK-side
 * only — a `Kind.id` column lowers as a plain u64 field, and class labels
 * are never fingerprinted), so a store CREATED under the raw-u64 writer
 * schema OPENS under the closed-typed reader schema. Neither declares the
 * containment law, which is exactly the state the read throw names: raw
 * bigints reach a column the reader types by `Kind`.
 */
const RawLawlessAccount = relation("Account", { id: u64.fresh, kind: u64 })
const LawlessAccount = relation("Account", { id: u64.fresh, kind: Kind.id })
const LawlessWriter = schema("Lawless", { Kind, Account: RawLawlessAccount }, [])
const LawlessReader = schema("Lawless", { Kind, Account: LawlessAccount }, [])

/**
 * Copies a store directory to a fresh path, stripping the per-open lock
 * artifacts (LMDB's `lock.mdb` reader table and the `bumbledb.lock`
 * advisory file — both recreated by the engine at open), so the copy is
 * openable while the source stays cached and locked in this process.
 */
function copyStore(from: string, to: string): void {
	fs.cpSync(from, to, { recursive: true })
	fs.rmSync(path.join(to, "lock.mdb"), { force: true })
	fs.rmSync(path.join(to, "bumbledb.lock"), { force: true })
}

/** Unwraps a value the surrounding test just proved present. */
function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

describe("the marshal bijection over closed rosters", function suite() {
	let db: DbValue<(typeof Ledger)["relations"]>
	const ids: { savings?: bigint } = {}

	test("insert with the handle NAME round-trips through tx reads, scan, and get", async function roundTrip() {
		db = await Db.create(storeDir, Ledger)
		const written = db.write(function seed(tx) {
			const minted = tx.insert(Account, { kind: "Savings" })
			ids.savings = minted.id
			tx.insert(SavingsTerms, { account: minted.id })
			assert.equal(
				tx.contains(Account, { id: minted.id, kind: "Savings" }),
				true,
				"contains lowers the NAME through the one cellOf seam"
			)
			const read = tx.get(Account, { id: minted.id })
			assert.ok(read, "the final-state point read sees the pending insert")
			assert.strictEqual(read.kind, "Savings", "the tx point read decodes the id back to the NAME")
		})
		assert.ok(written.ok, "the seed commit lands")
		const rows = db.scan(Account)
		assert.equal(rows.length, 1)
		assert.strictEqual(must(rows[0]).kind, "Savings", "scan decodes the id back to the NAME")
		const got = db.get(Account, { id: must(ids.savings) })
		assert.strictEqual(must(got).kind, "Savings", "get decodes the id back to the NAME")
	})

	test("delete lowers the NAME through the same seam", function deletePath() {
		const cycle = db.write(function insertAndDelete(tx) {
			const minted = tx.insert(Account, { kind: "Checking" })
			assert.equal(
				tx.delete(Account, { id: minted.id, kind: "Checking" }),
				true,
				"delete reaches the closed arm through rowOf"
			)
		})
		assert.ok(cycle.ok, "the net-zero delta commits")
		assert.equal(db.scan(Account).length, 1, "the checking row died in its own delta")
	})

	test("the store's raw cell is the u64 row id — exhume stays RAW by design", async function rawCell() {
		copyStore(storeDir, exhumeDir)
		const exhumed = await Db.exhume(exhumeDir)
		const raw = exhumed.scan("Account")
		assert.equal(raw.length, 1)
		const cell = must(raw[0]).kind
		/**
		 * Exhume is the recovery surface and DELIBERATELY not a consumer of
		 * the bijection: rows stay raw (the engine's cell, never the decoded
		 * name) and the roster crosses SEPARATELY on the descriptor — so a
		 * store outliving its schema is still fully recoverable by name.
		 */
		assert.equal(typeof cell, "bigint", "the exhumed cell is the raw engine cell, never the decoded name")
		assert.strictEqual(cell, 1n, 'the raw cell of "Savings" is its declaration-order row id')
		const kindRelation = must(
			exhumed.descriptor.relations.find(function byName(candidate) {
				return candidate.name === "Kind"
			})
		)
		assert.deepEqual(
			must(kindRelation.roster).map(function pair(axiom) {
				return { handle: axiom.handle, id: axiom.id }
			}),
			[
				{ handle: "Checking", id: 0n },
				{ handle: "Savings", id: 1n }
			],
			"the roster travels separately on the descriptor — the bijection is recoverable without the theory"
		)
	})

	test("an unknown handle name is a pointed write refusal (the 0.4.0 upgrade)", function unknownName() {
		assert.throws(function misspelled() {
			db.write(function tryInsert(tx) {
				/**
				 * Ruling 5: a wrong string is a compile error AND a marshal
				 * refusal — the expect-error pins the compile half, the throw
				 * (before the engine ever sees a row) pins the runtime half.
				 */
				// @ts-expect-error — "DirectPas" is not in Kind's handle union
				tx.insert(Account, { kind: "DirectPas" })
			})
		}, /"DirectPas" is not a handle of Kind — the roster is Checking, Savings/)
		assert.throws(function bigintShape() {
			db.write(function tryInsert(tx) {
				/** The 0.3.0 spelling is dead: a bare bigint no longer crosses the closed seam. */
				// @ts-expect-error — a bigint is not a handle name
				tx.insert(Account, { kind: 1n })
			})
		}, /expected a Kind handle name \(string\), got bigint/)
		assert.equal(db.scan(Account).length, 1, "both refusals aborted before any commit")
	})

	test("a violation's offending fact speaks the NAME and agrees with canonical", function violationNames() {
		const rejected = db.write(function orphanSavings(tx) {
			tx.insert(Account, { kind: "Savings" })
		})
		assert.ok(!rejected.ok, "a savings account without terms violates the mirror")
		const violation = must(
			rejected.violations.find(function byKind(candidate) {
				return candidate.kind === "containment"
			})
		)
		assert.strictEqual(violation.statement, savingsMirror)
		assert.equal(violation.canonical, renderStatement(savingsMirror))
		assert.equal(violation.canonical, "Account(id | kind == Savings) == SavingsTerms(account)")
		const offending = must(violation.facts[0])
		assert.equal(offending.relation, "Account")
		assert.strictEqual(offending.fact.kind, "Savings", "the offending fact's closed cell is the NAME")
		assert.ok(
			violation.canonical.includes(`kind == ${String(offending.fact.kind)}`),
			"the record and the canonical string agree on the one spelling"
		)
	})

	test("an out-of-roster id in a LAWLESS store is a pointed read throw, never a fallback", async function lawlessRead() {
		const writer = await Db.create(lawlessDir, LawlessWriter)
		const seeded = writer.write(function seedRaw(tx) {
			/** The writer twin types the column as plain u64 — no law pins it, so any bigint commits. */
			tx.insert(RawLawlessAccount, { kind: 7n })
		})
		assert.ok(seeded.ok, "the lawless writer commits a raw out-of-roster id")
		copyStore(lawlessDir, lawlessCopyDir)
		const reader = await Db.open(lawlessCopyDir, LawlessReader)
		assert.throws(function scanLawless() {
			reader.scan(LawlessAccount)
		}, /id 7 is outside the Kind roster \(Checking, Savings\) — the column types Kind but no law pins it — a containment statement is the missing piece/)
	})

	test("the marshal module stays literally cast-free (its own law)", function castFree() {
		const marshalPath = path.resolve(import.meta.dirname, "..", "src", "marshal.ts")
		const source = fs.readFileSync(marshalPath, "utf8")
		const code = source
			.split("\n")
			.filter(function codeLine(line) {
				const trimmed = line.trim()
				/** Comment prose and namespace imports may spell "as"; a CAST may not. */
				return !(
					trimmed.startsWith("*") ||
					trimmed.startsWith("/*") ||
					trimmed.startsWith("//") ||
					trimmed.startsWith("import ")
				)
			})
			.join("\n")
		assert.equal(/ as /.test(code), false, "no cast spelling exists in the module's code")
		assert.equal(/\bany\b/.test(code), false, "no any exists in the module's code")
	})
})
