/**
 * PRD-07 runtime pins against a REAL durable store in a temp dir, on the
 * zero-closable surface: create with the Ledger schema; the per-process
 * store cache (same path + identical theory = the SAME `Db` value, a
 * different theory = a typed fingerprint error, create on a cached path
 * refused); fresh-mint insert with the bare bigint id returned and usable;
 * delete + resupplied reinsert preserving identity (scan proves); scoped
 * snapshot reads through `read(fn)` with the scope invalidated after `fn`
 * returns; the `db.X` sugar obeying the symmetry rule; violations arriving
 * as typed VALUES `===`-matched to their SDK statement constants with
 * canonical spellings equal to `renderStatement` output (containment +
 * capacity together in one commit; the FD alone in another — the engine's
 * key phase preempts the statement phase, so no single commit can cite all
 * three forms); `writeFrom` one-shot witnessed writes (retry is host
 * policy), surfacing rejections as data, and aborting without any commit on
 * `abandon`; and a second in-process open of a live path is
 * `EnvironmentLocked` (the host holds the `Db`).
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import type { Db as DbValue, Fact, ReadInstance, Tx } from "#index.ts"
import {
	abandon,
	bool,
	bytes,
	capacity,
	closed,
	contained,
	Db,
	ErrSchemaError,
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
	within
} from "#index.ts"
import { accepted } from "#test/accepted.ts"
import { put } from "#test/put.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-db-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Kind = closed("Kind", ["Checking", "Savings"])
const Holder = relation("Holder", { id: u64.fresh, name: str })
const Account = relation("Account", {
	id: u64.fresh,
	holder: u64,
	kind: Kind.id,
	active: interval(i64)
})
const SavingsTerms = relation("SavingsTerms", { account: u64, rate: i64 })
const Audit = relation("Audit", {
	id: u64.fresh,
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
const savingsMirror = mirrors(on(Account.where({ kind: "Savings" }), "id"), on(SavingsTerms, "account"))
const holderCapacity = capacity(on(Holder, "id"), within(0n, 3n), on(Account, "holder"))

const Ledger = schema("Ledger", { Kind, Holder, Account, SavingsTerms, Audit }, [
	savingsKey,
	holderContainment,
	kindContainment,
	savingsMirror,
	holderCapacity
])

/** Unwraps a value the surrounding test just proved present. */
function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

/** The minted ids the sequential tests hand forward — bare bigints (structural values). */
const ids: {
	ada?: bigint
	adaAccount?: bigint
	grace?: bigint
	graceAccount?: bigint
	kurt?: bigint
	audit?: bigint
} = {}

describe("the Db runtime against a real store", function suite() {
	let db: DbValue<(typeof Ledger)["relations"]>

	test("create admits the Ledger theory", async function create() {
		db = accepted(await Db.create(storeDir, Ledger))
		assert.equal(db.schema, Ledger)
	})

	test("create surfaces the engine's schemaError with the message intact", async function schemaError() {
		/**
		 * The two-boundary split: domains pair structurally (rate and score are
		 * both unlabeled i64, so this containment COMPILES), but whether the
		 * target face resolves a declared key of its relation is a property of
		 * the whole statement set no face type can see — Audit(score) keys
		 * nothing, and the ENGINE's schema judgment refuses it at create.
		 */
		const Broken = schema("Broken", { SavingsTerms, Audit }, [contained(on(SavingsTerms, "rate"), on(Audit, "score"))])
		await assert.rejects(
			async function badCreate() {
				await Db.create(path.join(tmpRoot, "broken"), Broken)
			},
			function isSchemaError(error: unknown) {
				return error instanceof Error && errors.is(error, ErrSchemaError)
			}
		)
	})

	test("a second open of a live path is EnvironmentLocked", async function secondOpenLocked() {
		await assert.rejects(async function reopen() {
			await Db.open(storeDir, Ledger)
		}, /another live handle holds this environment's lock/)
		await assert.rejects(async function recreate() {
			await Db.create(storeDir, Ledger)
		}, /already exists/)
	})

	test("no close verb exists anywhere — lifetimes are disposables (R12)", function zeroClosables() {
		assert.equal("close" in db, false)
		assert.equal(Symbol.dispose in db, false, "the Db value is process-cached — it has no lifetime to dispose")
		assert.equal(Symbol.asyncDispose in db, false)
		assert.equal("snapshot" in db, false)
		assert.deepEqual(
			Reflect.ownKeys(db).toSorted(),
			["contains", "execute", "get", "prepare", "read", "scan", "schema", "write", "writeFrom"],
			"the surface is exactly the pinned verbs — no retired write form survives"
		)
		db.read(function probeScope(instance) {
			assert.equal("close" in instance, false, "a read instance is a lexical lease, not a closeable handle")
			assert.equal(
				Symbol.dispose in instance,
				false,
				"InstanceBuilder, OwnedInstance, and Witness dispose; the read callback's instance does not"
			)
		})
	})

	test("fresh-mint insert returns bare usable ids; final-state point reads see the delta", function freshMint() {
		const result = db.write(function seed(tx) {
			const holder = put(tx, Holder, { name: "ada" })
			ids.ada = holder.id
			const account = put(tx, Account, {
				holder: holder.id,
				kind: "Checking",
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
		assert.equal(result.tag, "accepted", "the clean commit lands")
		assert.equal(typeof result.value.generation, "bigint")
	})

	test("delete + reinsert with the resupplied id preserves identity (scan proves)", function resupply() {
		const ada = must(ids.ada)
		const result = db.write(function rename(tx) {
			assert.equal(tx.delete(Holder, [{ id: ada, name: "ada" }]).changed, 1n)
			const reinserted = put(tx, Holder, { id: ada, name: "ada lovelace" })
			assert.equal(reinserted.id, ada)
		})
		assert.equal(result.tag, "accepted")
		const holders = db.scan(Holder)
		assert.equal(holders.length, 1)
		assert.deepStrictEqual(holders[0], { id: ada, name: "ada lovelace" })
	})

	test("scoped reads round-trip every field type", function roundTrip() {
		const written = db.write(function seedAudit(tx) {
			const audit = put(tx, Audit, {
				flag: true,
				note: "π ≤ 4",
				tag: new Uint8Array([1, 2, 3, 4]),
				score: -7n,
				at: span(5n, 9n)
			})
			ids.audit = audit.id
		})
		assert.equal(written.tag, "accepted")
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
			const grace = put(tx, Holder, { name: "grace" })
			ids.grace = grace.id
			const account = put(tx, Account, {
				holder: grace.id,
				kind: "Savings",
				active: span(0n, 5n)
			})
			ids.graceAccount = account.id
			put(tx, SavingsTerms, { account: account.id, rate: 3n })
			const kurt = put(tx, Holder, { name: "kurt" })
			ids.kurt = kurt.id
			put(tx, Account, { holder: kurt.id, kind: "Checking", active: span(0n, 5n) })
		})
		assert.equal(setup.tag, "accepted")
		assert.deepStrictEqual(db.get(SavingsTerms, { account: must(ids.graceAccount) }), {
			account: ids.graceAccount,
			rate: 3n
		})
		assert.equal(db.get(SavingsTerms, { account: must(ids.adaAccount) }), undefined)
		assert.throws(function missingKeyField() {
			db.get(SavingsTerms, {})
		}, /missing field account/)
	})

	test("containment + capacity violations arrive together as ===-matched statement values", function statementViolations() {
		const ada = must(ids.ada)
		const kurt = must(ids.kurt)
		const rejected = db.write(function violate(tx) {
			put(tx, Account, { holder: ada, kind: "Checking", active: span(1n, 2n) })
			put(tx, Account, { holder: ada, kind: "Checking", active: span(2n, 3n) })
			put(tx, Account, { holder: ada, kind: "Checking", active: span(3n, 4n) })
			tx.delete(Holder, [{ id: kurt, name: "kurt" }])
		})
		assert.equal(rejected.tag, "rejected", "the statement judgment rejects")
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

		const capacityViolation = must(
			rejected.violations.find(function byKind(violation) {
				return violation.kind === "capacity"
			})
		)
		assert.strictEqual(capacityViolation.statement, holderCapacity)
		assert.equal(capacityViolation.canonical, renderStatement(holderCapacity))
		assert.equal(capacityViolation.measure, 4n)
		const parent = must(capacityViolation.facts[0])
		assert.equal(parent.relation, "Holder")
		assert.equal(parent.fact.id, ada)
	})

	test("an FD violation cites its declared key statement (key phase preempts)", function fdViolation() {
		const rejected = db.write(function duplicateTerms(tx) {
			put(tx, SavingsTerms, { account: must(ids.graceAccount), rate: 9n })
		})
		assert.equal(rejected.tag, "rejected", "the key judgment rejects")
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
			put(tx, Holder, { id: must(ids.ada), name: "imposter" })
		})
		assert.equal(rejected.tag, "rejected")
		const violation = must(rejected.violations[0])
		assert.equal(violation.kind, "functionality")
		assert.equal(violation.statement, undefined)
		assert.equal(violation.canonical, "Holder(id) -> Holder")
	})

	test("a leaked read scope is invalidated the moment read(fn) returns", function usedAfterScope() {
		let escaped: ReadInstance<(typeof Ledger)["relations"]> | undefined
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
		assert.equal(captured.tag, "accepted")
		assert.throws(function useAfterSpend() {
			put(must(escaped), Holder, { name: "late" })
		}, /spent/)
	})

	test("writeFrom lands a clean witnessed commit", function witnessedCommit() {
		const outcome = db.read(function seed(_instance, witness) {
			const holders = db.scan(Holder)
			assert.ok(holders.length > 0)
			return db.writeFrom(witness, function insert(tx) {
				put(tx, Holder, { name: "witnessed" })
			})
		})
		assert.equal(outcome.tag, "accepted", "the witnessed commit lands")
		assert.equal(typeof outcome.value.generation, "bigint")
	})

	test("writeFrom reports moved on self-inflicted contention — retry is host policy", function witnessedMoved() {
		const outcome = db.read(function compute(_instance, witness) {
			const holders = db.scan(Holder)
			const mover = db.write(function race(inner) {
				put(inner, Holder, { name: "wit-mover" })
			})
			assert.equal(mover.tag, "accepted", "the interleaved write lands and moves the generation")
			return db.writeFrom(witness, function insert(tx) {
				put(tx, Holder, { name: `wit-count-${holders.length}` })
			})
		})
		assert.equal(outcome.tag, "moved", "the one-shot writeFrom reports moved instead of retrying")
		const landed = db.scan(Holder).filter(function witnessedRows(holder) {
			return holder.name.startsWith("wit-count-")
		})
		assert.equal(landed.length, 0, "the stale-premise attempt never committed")
	})

	test("writeFrom surfaces engine rejection as data", function witnessedRejection() {
		const rejected = db.read(function violate(instance, witness) {
			assert.equal(typeof instance.generation, "bigint")
			return db.writeFrom(witness, function insert(tx) {
				put(tx, SavingsTerms, { account: must(ids.graceAccount), rate: 11n })
			})
		})
		assert.equal(rejected.tag, "rejected")
		assert.ok("violations" in rejected, "the rejection is the rejected arm")
		const violation = must(rejected.violations[0])
		assert.strictEqual(violation.statement, savingsKey)
	})

	test("writeFrom abandon aborts without committing — not even an empty commit", function witnessedAbandon() {
		const before = db.read(function generationOf(instance) {
			return instance.generation
		})
		const outcome = db.read(function bail(instance, witness) {
			assert.equal(instance.generation, before)
			return db.writeFrom(witness, function decline(tx) {
				put(tx, Holder, { name: "never-lands" })
				return abandon({ reason: "stale premise" })
			})
		})
		assert.equal(outcome.tag, "abandoned")
		assert.deepEqual(outcome.abandoned, { reason: "stale premise" })
		const after = db.read(function generationOf(instance) {
			return instance.generation
		})
		assert.equal(after, before, "no commit was issued on the abandon path")
		const ghosts = db.scan(Holder).filter(function abandonedRows(holder) {
			return holder.name === "never-lands"
		})
		assert.equal(ghosts.length, 0, "the recorded delta was aborted")
	})

	test("writeFrom abandon works with no delta verbs (the begun transaction aborts)", function witnessedAbandonEarly() {
		const before = db.read(function generationOf(instance) {
			return instance.generation
		})
		const outcome = db.read(function bailEarly(instance, witness) {
			const count = instance.scan(Holder).length
			return db.writeFrom(witness, function decline() {
				return abandon(count)
			})
		})
		assert.equal(outcome.tag, "abandoned")
		assert.equal(typeof outcome.abandoned, "number")
		assert.equal(
			db.read(function generationOf(instance) {
				return instance.generation
			}),
			before
		)
	})

	test("db.write honors abandon — the transaction rolls back, the payload is the outcome (R10)", function writeAbandon() {
		const before = db.read(function generationOf(instance) {
			return instance.generation
		})
		const outcome = db.write(function bail(tx) {
			put(tx, Holder, { name: "write-abandon-never-lands" })
			return abandon({ reason: "declined" })
		})
		assert.equal(outcome.tag, "abandoned")
		assert.deepEqual(outcome.abandoned, { reason: "declined" })
		assert.equal(
			db.read(function generationOf(instance) {
				return instance.generation
			}),
			before,
			"no commit was issued, not even an empty one"
		)
		const ghosts = db.scan(Holder).filter(function abandonedRows(holder) {
			return holder.name === "write-abandon-never-lands"
		})
		assert.equal(ghosts.length, 0, "the recorded delta was aborted")
	})

	test("tx.insert returns MutationReport { submitted, changed }", function insertChanged() {
		const committed = db.write(function replay(tx) {
			const id = tx.reserve(Holder, "id", 1n).at(0n)
			assert.ok(id !== undefined)
			const first = tx.insert(Holder, [{ id, name: "changed-bit" }])
			assert.equal(first.submitted, 1n)
			assert.equal(first.changed, 1n, "a fresh insert changes the final state")
			const replayed = tx.insert(Holder, [{ id, name: "changed-bit" }])
			assert.equal(replayed.submitted, 1n)
			assert.equal(replayed.changed, 0n, "the resupplied replay reports no state change — no contains round trip")
			const created = tx.insert(SavingsTerms, [{ account: must(ids.graceAccount), rate: 777n }])
			assert.equal(created.changed, 1n, "a fresh-field-less relation's insert carries the bit too")
			assert.equal(tx.insert(SavingsTerms, [{ account: must(ids.graceAccount), rate: 777n }]).changed, 0n)
			return abandon("probe only")
		})
		assert.equal(committed.tag, "abandoned", "the probe delta abandons — the store is untouched")
	})

	test("empty insert/delete/reserve still enter the transaction", function emptyIsAMutation() {
		const Other = relation("Holder", { id: u64.fresh, name: str })
		const committed = db.write(function emptyOps(tx) {
			const inserted = tx.insert(Holder, [])
			assert.equal(inserted.submitted, 0n)
			assert.equal(inserted.changed, 0n)
			const deleted = tx.delete(Holder, [])
			assert.equal(deleted.submitted, 0n)
			assert.equal(deleted.changed, 0n)
			const empty = tx.reserve(Holder, "id", 0n)
			assert.equal(empty.empty, true)
			assert.equal(empty.count, 0n)
			assert.equal(empty.at(0n), undefined)
			const next = tx.reserve(Holder, "id", 1n)
			if (next.empty) {
				throw new Error("reserve(1) must be nonempty")
			}
			assert.equal(typeof next.start, "bigint")
			assert.throws(function foreignEmptyInsert() {
				tx.insert(Other, [])
			}, /not a member of schema/)
			return abandon("probe only")
		})
		assert.equal(committed.tag, "abandoned")
	})

	test("a field named `changed` is a legal cell — MutationReport is a separate value", async function changedFieldIsLegal() {
		const Shadow = relation("Shadow", { changed: u64.fresh, note: str })
		const Shadowed = schema("Shadowed", { Shadow }, [])
		const shadowDb = accepted(await Db.create(path.join(tmpRoot, "shadow"), Shadowed))
		const shadowed = shadowDb.write(function insertShadow(tx) {
			const id = tx.reserve(Shadow, "changed", 1n).at(0n)
			assert.ok(id !== undefined)
			const report = tx.insert(Shadow, [{ changed: id, note: "ok" }])
			assert.equal(report.changed, 1n)
			return abandon("probe only")
		})
		assert.equal(shadowed.tag, "abandoned")
		const Legal = relation("Legal", { id: u64.fresh, changed: bool })
		const Kept = schema("Kept", { Legal }, [])
		const legalDb = accepted(await Db.create(path.join(tmpRoot, "legal-changed"), Kept))
		const outcome = legalDb.write(function insertLegal(tx) {
			const id = tx.reserve(Legal, "id", 1n).at(0n)
			assert.ok(id !== undefined)
			const first = tx.insert(Legal, [{ id, changed: true }])
			assert.equal(first.changed, 1n, "a fresh insert changes the final state")
			const replay = tx.insert(Legal, [{ id, changed: true }])
			assert.equal(
				replay.changed,
				0n,
				"the report is the engine's count — the supplied `changed` cell (true) never shadows it"
			)
			return abandon("probe only")
		})
		assert.equal(outcome.tag, "abandoned")
	})

	test("a stashed read instance throws after the callback returns", function usedAfterScope() {
		let leaked: ReadInstance<(typeof Ledger)["relations"]> | undefined
		db.read(function capture(instance) {
			leaked = instance
			assert.ok(instance.scan(Holder).length > 0, "the instance reads while the callback is live")
		})
		assert.ok(leaked)
		assert.throws(function usedAfterScope() {
			must(leaked).scan(Holder)
		}, /invalidated|useAfterScope|closed instance/)
		const landed = db.write(function probe(tx) {
			put(tx, Holder, { name: "post-stash-write" })
		})
		assert.equal(landed.tag, "accepted", "no reader slot leaked — the write begins cleanly")
	})

	test("the live handle still reads every committed fact", function liveReads() {
		const ada = db.get(Holder, { id: must(ids.ada) })
		assert.ok(ada, "the committed data reads back")
		assert.equal(ada.name, "ada lovelace")
	})
})

/**
 * The marshal boundary is typed on the way IN (each `@ts-expect-error`
 * real): a fact cell at the wrong STRUCTURAL shape is a compile error —
 * bare values at exact structural types, the wall the runtime shape
 * refusals at the seam back up. The well-shaped insert compiles as the
 * control.
 */
function marshalShapesAreTyped(tx: Tx<(typeof Ledger)["relations"]>): void {
	tx.insert(Audit, [
		{
			id: 1n,
			flag: true,
			note: "well-shaped",
			tag: new Uint8Array([1, 2, 3, 4]),
			score: -1n,
			at: span(0n, 1n)
		}
	])
	// @ts-expect-error — a bool field takes boolean, never a string
	tx.insert(Audit, [
		{
			id: 1n,
			flag: "yes",
			note: "ill-shaped bool",
			tag: new Uint8Array([1, 2, 3, 4]),
			score: -1n,
			at: span(0n, 1n)
		}
	])
	// @ts-expect-error — an i64 field takes bigint, never number
	tx.insert(Audit, [
		{
			id: 1n,
			flag: true,
			note: "ill-shaped i64",
			tag: new Uint8Array([1, 2, 3, 4]),
			score: -1,
			at: span(0n, 1n)
		}
	])
	// @ts-expect-error — a bytes<N> field takes Uint8Array, never a number array
	tx.insert(Audit, [
		{
			id: 1n,
			flag: true,
			note: "ill-shaped bytes",
			tag: [1, 2, 3, 4],
			score: -1n,
			at: span(0n, 1n)
		}
	])
	// @ts-expect-error — an interval field takes { start, end } bigints, never a bare point
	tx.insert(Audit, [
		{
			id: 1n,
			flag: true,
			note: "ill-shaped interval",
			tag: new Uint8Array([1, 2, 3, 4]),
			score: -1n,
			at: 5n
		}
	])
}

/** The Calendar theory the typestate probe holds against the Ledger. */
const Booking = relation("Booking", { room: u64, during: interval(u64) })
const Calendar = schema("Calendar", { Booking }, [key(Booking, ["room", "during"])])

/**
 * `schema()` carries its relation record as typestate: `Db` over one
 * schema's relations accepts exactly those relations — a schema-A fact
 * into a schema-B store is a compile error (relation identity is the
 * membership rule). Moved here from the statement suite (its subject is
 * `Db`, and the statement suite stays kernel-isolated).
 */
function dbTypestateHoldsTheWall(
	ledgerDb: DbValue<(typeof Ledger)["relations"]>,
	calendarDb: DbValue<(typeof Calendar)["relations"]>,
	account: Fact<typeof Account>
): void {
	ledgerDb.write(function accepts(tx) {
		tx.insert(Account, [account])
	})
	calendarDb.write(function rejects(tx) {
		// @ts-expect-error — a Ledger fact belongs to Db<Ledger>, never Db<Calendar>
		tx.insert(Account, [account])
	})
}

export { dbTypestateHoldsTheWall, marshalShapesAreTyped }
