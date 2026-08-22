import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import type { Db as DbValue, RelationFields, Selected } from "#index.ts"
import { closed, contained, Db, key, mirrors, on, relation, renderStatement, schema, u64 } from "#index.ts"
import { accepted } from "#test/accepted.ts"
import { put } from "#test/put.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-marshal-"))
const storeDir = path.join(tmpRoot, "store")
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

const savingsSelected: Selected<"Account", RelationFields<typeof Account>> = {
	relation: Account,
	selection: [{ field: "kind", set: { kind: "one", literal: { kind: "handle", handle: "Savings" } } }]
}
const savingsMirror = mirrors(on(savingsSelected, "id"), on(SavingsTerms, "account"))

const Ledger = schema("MarshalLedger", { Kind, Account, SavingsTerms }, [savingsKey, kindContainment, savingsMirror])

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

function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

describe("the marshal bijection over closed rosters", function suite() {
	let db: DbValue<(typeof Ledger)["relations"]>
	const ids: { savings?: bigint } = {}

	test("insert with the handle NAME round-trips through tx reads, scan, and get", async function roundTrip() {
		db = accepted(await Db.create(storeDir, Ledger))
		const written = db.write(function seed(tx) {
			const minted = put(tx, Account, { kind: "Savings" })
			ids.savings = minted.id
			put(tx, SavingsTerms, { account: minted.id })
			assert.equal(
				tx.contains(Account, { id: minted.id, kind: "Savings" }),
				true,
				"contains lowers the NAME through the one cellOf seam"
			)
			const read = tx.get(Account, { id: minted.id })
			assert.ok(read, "the final-state point read sees the pending insert")
			assert.strictEqual(read.kind, "Savings", "the tx point read decodes the id back to the NAME")
		})
		assert.equal(written.tag, "accepted", "the seed commit lands")
		const rows = db.read((i) => i.scan(Account))
		assert.equal(rows.length, 1)
		assert.strictEqual(must(rows[0]).kind, "Savings", "scan decodes the id back to the NAME")
		const got = db.read((i) => i.get(Account, { id: must(ids.savings) }))
		assert.strictEqual(must(got).kind, "Savings", "get decodes the id back to the NAME")
	})

	test("delete lowers the NAME through the same seam", function deletePath() {
		const cycle = db.write(function insertAndDelete(tx) {
			const minted = put(tx, Account, { kind: "Checking" })
			assert.equal(
				tx.delete(Account, [{ id: minted.id, kind: "Checking" }]).changed,
				1n,
				"delete reaches the closed arm through rowOf"
			)
		})
		assert.equal(cycle.tag, "accepted", "the net-zero delta commits")
		assert.equal(db.read((i) => i.scan(Account)).length, 1, "the checking row died in its own delta")
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
				tx.insert(Account, [{ id: 1n, kind: "DirectPas" }])
			})
		}, /"DirectPas" is not a handle of Kind — the roster is Checking, Savings/)
		assert.throws(function bigintShape() {
			db.write(function tryInsert(tx) {
				// @ts-expect-error — a bigint is not a handle name
				tx.insert(Account, [{ id: 1n, kind: 1n }])
			})
		}, /expected a Kind handle name \(string\), got bigint/)
		assert.equal(db.read((i) => i.scan(Account)).length, 1, "both refusals aborted before any commit")
	})

	test("a violation's offending fact speaks the NAME and agrees with canonical", function violationNames() {
		const rejected = db.write(function orphanSavings(tx) {
			put(tx, Account, { kind: "Savings" })
		})
		assert.equal(rejected.tag, "rejected", "a savings account without terms violates the mirror")
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
		const writer = accepted(await Db.create(lawlessDir, LawlessWriter))
		const seeded = writer.write(function seedRaw(tx) {
			put(tx, RawLawlessAccount, { kind: 7n })
		})
		assert.equal(seeded.tag, "accepted", "the lawless writer commits a raw out-of-roster id")
		copyStore(lawlessDir, lawlessCopyDir)
		const reader = await Db.open(lawlessCopyDir, LawlessReader)
		assert.throws(function scanLawless() {
			reader.read((i) => i.scan(LawlessAccount))
		}, /id 7 is outside the Kind roster \(Checking, Savings\) — the column types Kind but no law pins it — a containment statement is the missing piece/)
	})

	test("the marshal module stays literally cast-free (its own law)", function castFree() {
		const marshalPath = path.resolve(import.meta.dirname, "..", "src", "marshal.ts")
		const source = fs.readFileSync(marshalPath, "utf8")
		const code = source
			.split("\n")
			.filter(function codeLine(line) {
				const trimmed = line.trim()

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
