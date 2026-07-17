/**
 * PRD-07 runtime pins against a REAL durable store in a temp dir, on the
 * zero-closable surface: create with the Ledger schema; the per-process
 * store cache (same path + identical theory = the SAME `Db` value, a
 * different theory = a typed fingerprint error, create on a cached path
 * refused); fresh-mint insert with the branded id returned and usable;
 * delete + resupplied reinsert preserving identity (scan proves); scoped
 * snapshot reads through `read(fn)` with the scope invalidated after `fn`
 * returns; the `db.X` sugar obeying the symmetry rule; violations arriving
 * as typed VALUES `===`-matched to their SDK statement constants with
 * canonical spellings equal to `renderStatement` output (containment +
 * window together in one commit; the FD alone in another — the engine's
 * key phase preempts the statement phase, so no single commit can cite all
 * three forms); `writeWitnessed` retrying self-inflicted contention,
 * surfacing rejections as data, and aborting without any commit on
 * `abandon`; and resume = reopen, which in-process means the cached value
 * (cross-process durability is the engine's per-commit fsync, pinned at
 * the FFI layer).
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import type { Brand, Db as DbValue, ReadScope, Tx } from "#index.ts"
import {
	abandon,
	atMost,
	bool,
	bytes,
	closed,
	contained,
	Db,
	i64,
	interval,
	key,
	mirrors,
	on,
	relation,
	renderStatement,
	schema,
	span,
	str,
	u64,
	window
} from "#index.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-db-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const HolderId = u64.newtype("HolderId")
const AccountId = u64.newtype("AccountId")
const AuditId = u64.newtype("AuditId")
const ActiveDuring = interval(i64).newtype("ActiveDuring")

const Kind = closed("Kind", ["Checking", "Savings"])
const Holder = relation("Holder", { id: HolderId.fresh, name: str })
const Account = relation("Account", {
	id: AccountId.fresh,
	holder: HolderId,
	kind: Kind.id,
	active: ActiveDuring
})
const SavingsTerms = relation("SavingsTerms", { account: AccountId, rate: i64 })
const Audit = relation("Audit", {
	id: AuditId.fresh,
	flag: bool,
	note: str,
	tag: bytes(4),
	score: i64,
	at: interval(u64)
})

const savingsKey = key(SavingsTerms, ["account"])
const holderContainment = contained(on(Account, "holder"), on(Holder, "id"))
/** The closed-reference companion the `kind == Savings` handle spelling resolves through. */
const kindContainment = contained(on(Account, "kind"), on(Kind, "id"))
const savingsMirror = mirrors(on(Account.where({ kind: Kind.Savings }), "id"), on(SavingsTerms, "account"))
const holderWindow = window(on(Holder, "id"), atMost(3n), on(Account, "holder"))

const Ledger = schema("Ledger", { Kind, Holder, Account, SavingsTerms, Audit }, [
	savingsKey,
	holderContainment,
	kindContainment,
	savingsMirror,
	holderWindow
])

/** Unwraps a value the surrounding test just proved present. */
function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

/** The branded ids the sequential tests hand forward. */
const ids: {
	ada?: Brand<bigint, "HolderId">
	adaAccount?: Brand<bigint, "AccountId">
	grace?: Brand<bigint, "HolderId">
	graceAccount?: Brand<bigint, "AccountId">
	kurt?: Brand<bigint, "HolderId">
	audit?: Brand<bigint, "AuditId">
} = {}

describe("the Db runtime against a real store", function suite() {
	let db: DbValue<(typeof Ledger)["relations"]>

	test("create admits the Ledger theory", async function create() {
		db = await Db.create(storeDir, Ledger)
		assert.equal(db.schema, Ledger)
	})

	test("create surfaces the engine's schemaError with the message intact", async function schemaError() {
		const Broken = schema("Broken", { Holder, Account }, [contained(on(Account, "holder"), on(Holder, "name"))])
		await assert.rejects(async function badCreate() {
			await Db.create(path.join(tmpRoot, "broken"), Broken)
		}, /schemaError/)
	})

	test("the store cache: same path + identical theory is the SAME Db value", async function cacheIdentity() {
		const again = await Db.open(storeDir, Ledger)
		assert.strictEqual(again, db, "the cache returns the one value, never a second handle")
		const viaAliasedSpelling = await Db.open(`${storeDir}${path.sep}.`, Ledger)
		assert.strictEqual(viaAliasedSpelling, again, "the cache key is the canonical path")
	})

	test("a different theory on a cached path is a typed fingerprint error", async function cacheFingerprint() {
		const Other = schema("Ledger", { Kind, Holder, Account, SavingsTerms, Audit }, [savingsKey])
		await assert.rejects(async function mismatched() {
			await Db.open(storeDir, Other)
		}, /fingerprintMismatch/)
	})

	test("create refuses a cached path (the entry proves the directory initialized)", async function cacheCreateRefusal() {
		await assert.rejects(async function recreate() {
			await Db.create(storeDir, Ledger)
		}, /already open in this process/)
	})

	test("no closable spelling exists anywhere on the surface", function zeroClosables() {
		assert.equal("close" in db, false)
		assert.equal(Symbol.dispose in db, false)
		assert.equal(Symbol.asyncDispose in db, false)
		assert.equal("snapshot" in db, false)
		assert.deepEqual(
			Reflect.ownKeys(db).toSorted(),
			["contains", "execute", "get", "prepare", "read", "scan", "schema", "write", "writeWitnessed"],
			"the surface is exactly the pinned verbs — no retired write form survives"
		)
		db.read(function probeScope(snap) {
			assert.equal("close" in snap, false)
			assert.equal(Symbol.dispose in snap, false)
		})
	})

	test("fresh-mint insert returns branded usable ids; final-state point reads see the delta", function freshMint() {
		const result = db.write(function seed(tx) {
			const holder = tx.insert(Holder, { name: "ada" })
			ids.ada = holder.id
			const account = tx.insert(Account, {
				holder: holder.id,
				kind: Kind.Checking,
				active: span(0n, 10n)
			})
			ids.adaAccount = account.id
			assert.equal(typeof holder.id, "bigint")
			assert.equal(tx.contains(Holder, { id: holder.id, name: "ada" }), true)
			const read = tx.get(Account, { id: account.id })
			assert.ok(read)
			assert.equal(read.holder, holder.id)
			assert.deepEqual(read.active, { start: 0n, end: 10n })
		})
		assert.ok(result.ok, "the clean commit lands")
		assert.equal(typeof result.generation, "bigint")
	})

	test("delete + reinsert with the resupplied id preserves identity (scan proves)", function resupply() {
		const ada = must(ids.ada)
		const result = db.write(function rename(tx) {
			assert.equal(tx.delete(Holder, { id: ada, name: "ada" }), true)
			const reinserted = tx.insert(Holder, { id: ada, name: "ada lovelace" })
			assert.equal(reinserted.id, ada)
		})
		assert.ok(result.ok)
		const holders = db.scan(Holder)
		assert.equal(holders.length, 1)
		assert.deepStrictEqual(holders[0], { id: ada, name: "ada lovelace" })
	})

	test("scoped reads round-trip every field type", function roundTrip() {
		const written = db.write(function seedAudit(tx) {
			const audit = tx.insert(Audit, {
				flag: true,
				note: "π ≤ 4",
				tag: new Uint8Array([1, 2, 3, 4]),
				score: -7n,
				at: span(5n, 9n)
			})
			ids.audit = audit.id
		})
		assert.ok(written.ok)
		db.read(function readBack(snap) {
			assert.equal(typeof snap.generation, "bigint")
			const rows = snap.scan(Audit)
			assert.deepStrictEqual(rows, [
				{
					id: ids.audit,
					flag: true,
					note: "π ≤ 4",
					tag: new Uint8Array([1, 2, 3, 4]),
					score: -7n,
					at: { start: 5n, end: 9n }
				}
			])
			assert.equal(snap.contains(Audit, must(rows[0])), true)
			assert.deepStrictEqual(snap.get(Audit, { id: must(ids.audit) }), rows[0])
		})
	})

	test("the db.X sugar obeys the symmetry rule db.X(...) === db.read(snap => snap.X(...))", function symmetry() {
		const audit = must(ids.audit)
		assert.deepStrictEqual(
			db.get(Audit, { id: audit }),
			db.read(function getInScope(snap) {
				return snap.get(Audit, { id: audit })
			})
		)
		assert.deepStrictEqual(
			db.scan(Audit),
			db.read(function scanInScope(snap) {
				return snap.scan(Audit)
			})
		)
		const fact = must(db.get(Audit, { id: audit }))
		assert.equal(
			db.contains(Audit, fact),
			db.read(function containsInScope(snap) {
				return snap.contains(Audit, fact)
			})
		)
	})

	test("keyed get reads through a declared (non-fresh) primary key", function declaredKey() {
		const setup = db.write(function seedSavings(tx) {
			const grace = tx.insert(Holder, { name: "grace" })
			ids.grace = grace.id
			const account = tx.insert(Account, {
				holder: grace.id,
				kind: Kind.Savings,
				active: span(0n, 5n)
			})
			ids.graceAccount = account.id
			tx.insert(SavingsTerms, { account: account.id, rate: 3n })
			const kurt = tx.insert(Holder, { name: "kurt" })
			ids.kurt = kurt.id
			tx.insert(Account, { holder: kurt.id, kind: Kind.Checking, active: span(0n, 5n) })
		})
		assert.ok(setup.ok)
		assert.deepStrictEqual(db.get(SavingsTerms, { account: must(ids.graceAccount) }), {
			account: ids.graceAccount,
			rate: 3n
		})
		assert.equal(db.get(SavingsTerms, { account: must(ids.adaAccount) }), undefined)
		assert.throws(function missingKeyField() {
			db.get(SavingsTerms, {})
		}, /missing field account/)
	})

	test("containment + window violations arrive together as ===-matched statement values", function statementViolations() {
		const ada = must(ids.ada)
		const kurt = must(ids.kurt)
		const rejected = db.write(function violate(tx) {
			tx.insert(Account, { holder: ada, kind: Kind.Checking, active: span(1n, 2n) })
			tx.insert(Account, { holder: ada, kind: Kind.Checking, active: span(2n, 3n) })
			tx.insert(Account, { holder: ada, kind: Kind.Checking, active: span(3n, 4n) })
			tx.delete(Holder, { id: kurt, name: "kurt" })
		})
		assert.ok(!rejected.ok, "the statement judgment rejects")
		assert.equal(rejected.violations.length, 2, "the statement phase is scan-complete")

		const containmentViolation = must(
			rejected.violations.find(function byKind(violation) {
				return violation.kind === "containment"
			})
		)
		assert.strictEqual(containmentViolation.statement, holderContainment)
		assert.equal(containmentViolation.canonical, renderStatement(holderContainment))
		assert.equal(containmentViolation.direction, "targetRequired")
		const orphan = must(containmentViolation.facts[0])
		assert.equal(orphan.relation, "Account")
		assert.equal(orphan.fact.holder, kurt)

		const windowViolation = must(
			rejected.violations.find(function byKind(violation) {
				return violation.kind === "cardinality"
			})
		)
		assert.strictEqual(windowViolation.statement, holderWindow)
		assert.equal(windowViolation.canonical, renderStatement(holderWindow))
		assert.equal(windowViolation.count, 4n)
		const parent = must(windowViolation.facts[0])
		assert.equal(parent.relation, "Holder")
		assert.equal(parent.fact.id, ada)
	})

	test("an FD violation cites its declared key statement (key phase preempts)", function fdViolation() {
		const rejected = db.write(function duplicateTerms(tx) {
			tx.insert(SavingsTerms, { account: must(ids.graceAccount), rate: 9n })
		})
		assert.ok(!rejected.ok, "the key judgment rejects")
		assert.equal(rejected.violations.length, 1, "key violations preempt the statement phase")
		const violation = must(rejected.violations[0])
		assert.equal(violation.kind, "functionality")
		assert.strictEqual(violation.statement, savingsKey)
		assert.equal(violation.canonical, renderStatement(savingsKey))
		assert.equal(violation.canonical, "SavingsTerms(account) -> SavingsTerms")
		const cited = must(violation.facts[0])
		assert.equal(cited.relation, "SavingsTerms")
		assert.equal(cited.fact.account, ids.graceAccount)
	})

	test("a fresh-implied key violation carries statement: undefined", function impliedKey() {
		const rejected = db.write(function forkAda(tx) {
			tx.insert(Holder, { id: must(ids.ada), name: "imposter" })
		})
		assert.ok(!rejected.ok)
		const violation = must(rejected.violations[0])
		assert.equal(violation.kind, "functionality")
		assert.equal(violation.statement, undefined)
		assert.equal(violation.canonical, "Holder(id) -> Holder")
	})

	test("a leaked read scope is invalidated the moment read(fn) returns", function usedAfterScope() {
		let escaped: ReadScope<(typeof Ledger)["relations"]> | undefined
		const generation = db.read(function capture(snap) {
			escaped = snap
			return snap.generation
		})
		assert.equal(typeof generation, "bigint")
		const leaked = must(escaped)
		assert.throws(function scanAfterScope() {
			leaked.scan(Holder)
		}, /invalidated/)
		assert.throws(function getAfterScope() {
			leaked.get(Holder, { id: must(ids.ada) })
		}, /invalidated/)
		assert.throws(function containsAfterScope() {
			leaked.contains(Holder, { id: must(ids.ada), name: "ada lovelace" })
		}, /invalidated/)
	})

	test("a spent transaction refuses use", function spentTx() {
		let escaped: Tx<(typeof Ledger)["relations"]> | undefined
		const captured = db.write(function capture(tx) {
			escaped = tx
		})
		assert.ok(captured.ok)
		assert.throws(function useAfterSpend() {
			must(escaped).insert(Holder, { name: "late" })
		}, /spent/)
	})

	test("writeWitnessed lands a clean witnessed commit", function witnessedCommit() {
		const outcome = db.writeWitnessed(function seed(snap, tx) {
			const holders = snap.scan(Holder)
			assert.ok(holders.length > 0)
			tx.insert(Holder, { name: "witnessed" })
		})
		assert.ok(outcome.ok, "the witnessed commit lands")
		assert.equal(typeof outcome.generation, "bigint")
	})

	test("writeWitnessed retries the whole fn on self-inflicted contention", function witnessedRetry() {
		let attempts = 0
		const outcome = db.writeWitnessed(function compute(snap, tx) {
			attempts += 1
			const holders = snap.scan(Holder)
			if (attempts === 1) {
				const mover = db.write(function race(inner) {
					inner.insert(Holder, { name: "wit-mover" })
				})
				assert.ok(mover.ok, "the interleaved write lands and moves the generation")
			}
			tx.insert(Holder, { name: `wit-count-${holders.length}` })
		})
		assert.ok(outcome.ok, "the retried witness lands")
		assert.equal(attempts, 2, "one generation move, one convergence")
		const landed = db.scan(Holder).filter(function witnessedRows(holder) {
			return holder.name.startsWith("wit-count-")
		})
		assert.equal(landed.length, 1, "only the fresh-premise attempt committed")
	})

	test("writeWitnessed surfaces engine rejection as data", function witnessedRejection() {
		const rejected = db.writeWitnessed(function violate(snap, tx) {
			assert.equal(typeof snap.generation, "bigint")
			tx.insert(SavingsTerms, { account: must(ids.graceAccount), rate: 11n })
		})
		assert.ok(!rejected.ok)
		assert.ok("violations" in rejected, "the rejection is the WriteResult false arm")
		const violation = must(rejected.violations[0])
		assert.strictEqual(violation.statement, savingsKey)
	})

	test("writeWitnessed abandon aborts without committing — not even an empty commit", function witnessedAbandon() {
		const before = db.read(function generationOf(snap) {
			return snap.generation
		})
		const outcome = db.writeWitnessed(function bail(snap, tx) {
			assert.equal(snap.generation, before)
			tx.insert(Holder, { name: "never-lands" })
			return abandon({ reason: "stale premise" })
		})
		assert.ok(!outcome.ok)
		assert.ok("abandoned" in outcome, "the abandon payload is the outcome")
		assert.deepEqual(outcome.abandoned, { reason: "stale premise" })
		const after = db.read(function generationOf(snap) {
			return snap.generation
		})
		assert.equal(after, before, "no commit was issued on the abandon path")
		const ghosts = db.scan(Holder).filter(function abandonedRows(holder) {
			return holder.name === "never-lands"
		})
		assert.equal(ghosts.length, 0, "the recorded delta was aborted")
	})

	test("writeWitnessed abandon works before any delta verb (no transaction ever begins)", function witnessedAbandonEarly() {
		const before = db.read(function generationOf(snap) {
			return snap.generation
		})
		const outcome = db.writeWitnessed(function bailEarly(snap) {
			return abandon(snap.scan(Holder).length)
		})
		assert.ok(!outcome.ok)
		assert.ok("abandoned" in outcome)
		assert.equal(typeof outcome.abandoned, "number")
		assert.equal(
			db.read(function generationOf(snap) {
				return snap.generation
			}),
			before
		)
	})

	test("resume = reopen: the cached open reads every committed fact back", async function reopen() {
		const again = await Db.open(storeDir, Ledger)
		assert.strictEqual(again, db, "in-process resume is the cached value itself")
		const ada = again.get(Holder, { id: must(ids.ada) })
		assert.ok(ada, "the committed data reads back")
		assert.equal(ada.name, "ada lovelace")
	})
})
