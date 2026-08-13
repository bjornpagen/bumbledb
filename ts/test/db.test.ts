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

import type { Db as DbValue, InsertFact, ReadScope, Tx } from "#index.ts"
import {
	abandon,
	bool,
	bytes,
	capacity,
	closed,
	contained,
	Db,
	ErrGenerationMoved,
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
		db = await Db.create(storeDir, Ledger)
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
		await assert.rejects(async function badCreate() {
			await Db.create(path.join(tmpRoot, "broken"), Broken)
		}, /schemaError/)
	})

	test("a second open of a live path is EnvironmentLocked", async function secondOpenLocked() {
		await assert.rejects(async function reopen() {
			await Db.open(storeDir, Ledger)
		}, /another live handle holds this environment's lock/)
		await assert.rejects(async function recreate() {
			await Db.create(storeDir, Ledger)
		}, /another live handle holds this environment's lock|alreadyInitialized/)
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
		db.read(function probeScope(snap) {
			assert.equal("close" in snap, false, "release is Symbol.dispose, never a close verb to remember")
			assert.equal(Symbol.dispose in snap, true, "a read scope is a disposable lifetime (R12)")
		})
	})

	test("fresh-mint insert returns bare usable ids; final-state point reads see the delta", function freshMint() {
		const result = db.write(function seed(tx) {
			const holder = tx.insert(Holder, { name: "ada" })
			ids.ada = holder.id
			const account = tx.insert(Account, {
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
				kind: "Savings",
				active: span(0n, 5n)
			})
			ids.graceAccount = account.id
			tx.insert(SavingsTerms, { account: account.id, rate: 3n })
			const kurt = tx.insert(Holder, { name: "kurt" })
			ids.kurt = kurt.id
			tx.insert(Account, { holder: kurt.id, kind: "Checking", active: span(0n, 5n) })
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

	test("containment + capacity violations arrive together as ===-matched statement values", function statementViolations() {
		const ada = must(ids.ada)
		const kurt = must(ids.kurt)
		const rejected = db.write(function violate(tx) {
			tx.insert(Account, { holder: ada, kind: "Checking", active: span(1n, 2n) })
			tx.insert(Account, { holder: ada, kind: "Checking", active: span(2n, 3n) })
			tx.insert(Account, { holder: ada, kind: "Checking", active: span(3n, 4n) })
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

	test("writeFrom lands a clean witnessed commit", function witnessedCommit() {
		const outcome = db.read(function seed(snap) {
			const holders = snap.scan(Holder)
			assert.ok(holders.length > 0)
			return db.writeFrom(snap, function insert(tx) {
				tx.insert(Holder, { name: "witnessed" })
			})
		})
		assert.ok(outcome.ok, "the witnessed commit lands")
		assert.equal(typeof outcome.generation, "bigint")
	})

	test("writeFrom throws GenerationMoved on self-inflicted contention — retry is host policy", function witnessedMoved() {
		const spun = errors.trySync(function contend() {
			return db.read(function compute(snap) {
				const holders = snap.scan(Holder)
				const mover = db.write(function race(inner) {
					inner.insert(Holder, { name: "wit-mover" })
				})
				assert.ok(mover.ok, "the interleaved write lands and moves the generation")
				return db.writeFrom(snap, function insert(tx) {
					tx.insert(Holder, { name: `wit-count-${holders.length}` })
				})
			})
		})
		assert.ok(spun.error, "the one-shot writeFrom throws instead of retrying")
		assert.ok(errors.is(spun.error, ErrGenerationMoved), "the throw is the typed generationMoved error")
		const landed = db.scan(Holder).filter(function witnessedRows(holder) {
			return holder.name.startsWith("wit-count-")
		})
		assert.equal(landed.length, 0, "the stale-premise attempt never committed")
	})

	test("writeFrom surfaces engine rejection as data", function witnessedRejection() {
		const rejected = db.read(function violate(snap) {
			assert.equal(typeof snap.generation, "bigint")
			return db.writeFrom(snap, function insert(tx) {
				tx.insert(SavingsTerms, { account: must(ids.graceAccount), rate: 11n })
			})
		})
		assert.ok(!rejected.ok)
		assert.ok("violations" in rejected, "the rejection is the WriteResult false arm")
		const violation = must(rejected.violations[0])
		assert.strictEqual(violation.statement, savingsKey)
	})

	test("writeFrom abandon aborts without committing — not even an empty commit", function witnessedAbandon() {
		const before = db.read(function generationOf(snap) {
			return snap.generation
		})
		const outcome = db.read(function bail(snap) {
			assert.equal(snap.generation, before)
			return db.writeFrom(snap, function decline(tx) {
				tx.insert(Holder, { name: "never-lands" })
				return abandon({ reason: "stale premise" })
			})
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

	test("writeFrom abandon works with no delta verbs (the begun transaction aborts)", function witnessedAbandonEarly() {
		const before = db.read(function generationOf(snap) {
			return snap.generation
		})
		const outcome = db.read(function bailEarly(snap) {
			return db.writeFrom(snap, function decline() {
				return abandon(snap.scan(Holder).length)
			})
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

	test("db.write honors abandon — the transaction rolls back, the payload is the outcome (R10)", function writeAbandon() {
		const before = db.read(function generationOf(snap) {
			return snap.generation
		})
		const outcome = db.write(function bail(tx) {
			tx.insert(Holder, { name: "write-abandon-never-lands" })
			return abandon({ reason: "declined" })
		})
		assert.ok(!outcome.ok)
		assert.ok("abandoned" in outcome, "the abandon payload is the outcome — never a silent commit")
		assert.deepEqual(outcome.abandoned, { reason: "declined" })
		assert.equal(
			db.read(function generationOf(snap) {
				return snap.generation
			}),
			before,
			"no commit was issued, not even an empty one"
		)
		const ghosts = db.scan(Holder).filter(function abandonedRows(holder) {
			return holder.name === "write-abandon-never-lands"
		})
		assert.equal(ghosts.length, 0, "the recorded delta was aborted")
	})

	test("tx.insert returns { changed, ...fresh } — the Rust-surface bijection (R11)", function insertChanged() {
		const committed = db.write(function replay(tx) {
			const first = tx.insert(Holder, { name: "changed-bit" })
			assert.equal(first.changed, true, "a fresh insert changes the final state")
			assert.equal(typeof first.id, "bigint", "the minted cell rides beside the bit")
			const replayed = tx.insert(Holder, { id: first.id, name: "changed-bit" })
			assert.equal(replayed.changed, false, "the resupplied replay reports no state change — no contains round trip")
			assert.equal(replayed.id, first.id, "resupply preserves identity")
			const noFresh = tx.insert(SavingsTerms, { account: must(ids.graceAccount), rate: 777n })
			assert.equal(noFresh.changed, true, "a fresh-field-less relation's insert carries the bit too")
			assert.equal(tx.insert(SavingsTerms, { account: must(ids.graceAccount), rate: 777n }).changed, false)
			return abandon("probe only")
		})
		assert.ok(!committed.ok, "the probe delta abandons — the store is untouched")
	})

	test("a fresh field named `changed` is an admission refusal — the flattened return never shadows the report (R11)", async function freshChangedRefused() {
		const Shadow = relation("Shadow", { changed: u64.fresh, note: str })
		const Shadowed = schema("Shadowed", { Shadow }, [])
		await assert.rejects(async function shadowCreate() {
			await Db.create(path.join(tmpRoot, "shadow"), Shadowed)
		}, /fresh field named "changed" would shadow/)
		assert.ok(
			!fs.existsSync(path.join(tmpRoot, "shadow")),
			"the refusal precedes the bridge — no store is ever created"
		)
		// A SUPPLIED field named `changed` never rides the return record — the name stays legal.
		const Legal = relation("Legal", { id: u64.fresh, changed: bool })
		const Kept = schema("Kept", { Legal }, [])
		const legalDb = await Db.create(path.join(tmpRoot, "legal-changed"), Kept)
		const outcome = legalDb.write(function insertLegal(tx) {
			const first = tx.insert(Legal, { changed: true })
			assert.equal(first.changed, true, "a fresh insert changes the final state")
			assert.equal(typeof first.id, "bigint", "the minted cell rides beside the bit")
			const replay = tx.insert(Legal, { id: first.id, changed: true })
			assert.equal(
				replay.changed,
				false,
				"the report is the engine's boolean — the supplied `changed` cell (true) never shadows it"
			)
			return abandon("probe only")
		})
		assert.ok(!outcome.ok)
	})

	test("using snap = db.read() — the R12 acquisition: dispose releases the snapshot deterministically", function usingRead() {
		let leaked: ReadScope<(typeof Ledger)["relations"]> | undefined
		{
			using snap = db.read()
			assert.equal(typeof snap.generation, "bigint")
			assert.ok(snap.scan(Holder).length > 0, "the scope reads while its using block is live")
			leaked = snap
		}
		assert.ok(leaked)
		assert.throws(function usedAfterScope() {
			leaked.scan(Holder)
		}, /read scope is invalidated/)
		// An early in-callback disposal is idempotent with the owner's close:
		// the snapshot closes exactly once, and the write path stays healthy.
		db.read(function earlyDispose(snap) {
			snap[Symbol.dispose]()
			assert.throws(function afterDispose() {
				snap.generation < 0n || snap.scan(Holder)
			}, /read scope is invalidated/)
		})
		const landed = db.write(function probe(tx) {
			tx.insert(Holder, { name: "post-dispose-write" })
		})
		assert.ok(landed.ok, "no reader slot leaked — the write begins cleanly")
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
	tx.insert(Audit, {
		flag: true,
		note: "well-shaped",
		tag: new Uint8Array([1, 2, 3, 4]),
		score: -1n,
		at: span(0n, 1n)
	})
	tx.insert(Audit, {
		// @ts-expect-error — a bool field takes boolean, never a string
		flag: "yes",
		note: "ill-shaped bool",
		tag: new Uint8Array([1, 2, 3, 4]),
		score: -1n,
		at: span(0n, 1n)
	})
	tx.insert(Audit, {
		flag: true,
		note: "ill-shaped i64",
		tag: new Uint8Array([1, 2, 3, 4]),
		// @ts-expect-error — an i64 field takes bigint, never number
		score: -1,
		at: span(0n, 1n)
	})
	tx.insert(Audit, {
		flag: true,
		note: "ill-shaped bytes",
		// @ts-expect-error — a bytes<N> field takes Uint8Array, never a number array
		tag: [1, 2, 3, 4],
		score: -1n,
		at: span(0n, 1n)
	})
	tx.insert(Audit, {
		flag: true,
		note: "ill-shaped interval",
		tag: new Uint8Array([1, 2, 3, 4]),
		score: -1n,
		// @ts-expect-error — an interval field takes { start, end } bigints, never a bare point
		at: 5n
	})
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
	account: InsertFact<typeof Account>
): void {
	ledgerDb.write(function accepts(tx) {
		tx.insert(Account, account)
	})
	calendarDb.write(function rejects(tx) {
		// @ts-expect-error — a Ledger fact belongs to Db<Ledger>, never Db<Calendar>
		tx.insert(Account, account)
	})
}

export { dbTypestateHoldsTheWall, marshalShapesAreTyped }
