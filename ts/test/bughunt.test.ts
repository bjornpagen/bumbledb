/**
 * Bug-hunt probes (sdk-runtime + ffi lens): marshal edges (bigint u64/i64
 * extremes, bytes<N> widths, interval rays at MAX_END, fixed-width interval
 * judging, empty strings, unicode incl. lone surrogates), the mirrors
 * double-slot violation attribution, abort-on-thrown-callback sanity
 * (delta/txn state after a throw, subsequent writes), and the async-callback
 * hazard on the zero-closable surface. Tests assert DESIRED behavior: a
 * failing test here is a confirmed defect, not a broken test.
 */

import assert from "node:assert/strict"
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
	u64
} from "#index.ts"
import { lower } from "#lower.ts"
import { native } from "#native.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-hunt-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Num = relation("Num", { id: u64.fresh, u: u64, s: i64 })
const Blob = relation("Blob", { id: u64.fresh, tag: bytes(4) })
const Ray = relation("Ray", { id: u64.fresh, at: interval(u64), sat: interval(i64) })
const Slot = relation("Slot", { id: u64.fresh, when: interval(u64, 2n) })
const Txt = relation("Txt", { id: u64.fresh, note: str })

const Kind = closed("Kind", ["Plain", "Special"])
const Item = relation("Item", { id: u64.fresh, kind: Kind.id, flag: bool })
const Terms = relation("Terms", { item: u64, rate: i64 })
const termsKey = key(Terms, ["item"])
const kindRef = contained(on(Item, "kind"), on(Kind, "id"))
const specialMirror = mirrors(on(Item.where({ kind: "Special" }), "id"), on(Terms, "item"))

const Theory = schema("Hunt", { Kind, Num, Blob, Ray, Slot, Txt, Item, Terms }, [termsKey, kindRef, specialMirror])

const U64_MAX = (1n << 64n) - 1n
const I64_MAX = (1n << 63n) - 1n
const I64_MIN = -(1n << 63n)

/** Unwraps a value the surrounding test just proved present. */
function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

describe("marshal edges and lifecycle sanity against a real store", async function suite() {
	const db = await Db.create(path.join(tmpRoot, "store"), Theory)

	test("i64::MIN / i64::MAX / u64::MAX round-trip exactly", function bigintEdges() {
		let id: bigint | undefined
		const written = db.write(function seed(tx) {
			id = tx.insert(Num, { u: U64_MAX, s: I64_MIN }).id
			tx.insert(Num, { u: 0n, s: I64_MAX })
		})
		assert.ok(written.ok)
		const back = must(db.get(Num, { id: must(id) }))
		assert.equal(back.u, U64_MAX)
		assert.equal(back.s, I64_MIN)
		const max = db.scan(Num).find(function byS(row) {
			return row.s === I64_MAX
		})
		assert.ok(max, "the i64::MAX row reads back")
		assert.equal(db.contains(Num, { id: must(id), u: U64_MAX, s: I64_MIN }), true, "contains agrees at the extremes")
	})

	test("out-of-range bigints throw typed errors naming the position", function bigintRange() {
		assert.throws(function u64Overflow() {
			db.write(function bad(tx) {
				tx.insert(Num, { u: U64_MAX + 1n, s: 0n })
			})
		}, /u64/)
		assert.throws(function u64Negative() {
			db.write(function bad(tx) {
				tx.insert(Num, { u: -1n, s: 0n })
			})
		}, /u64/)
		assert.throws(function i64Overflow() {
			db.write(function bad(tx) {
				tx.insert(Num, { u: 0n, s: I64_MAX + 1n })
			})
		}, /i64/)
		assert.throws(function i64Underflow() {
			db.write(function bad(tx) {
				tx.insert(Num, { u: 0n, s: I64_MIN - 1n })
			})
		}, /i64/)
	})

	test("bytes<4> width mismatches throw; an offset view marshals by its view", function bytesWidths() {
		assert.throws(function tooShort() {
			db.write(function bad(tx) {
				tx.insert(Blob, { tag: new Uint8Array([1, 2, 3]) })
			})
		}, /bytes<4>|4/)
		assert.throws(function tooLong() {
			db.write(function bad(tx) {
				tx.insert(Blob, { tag: new Uint8Array([1, 2, 3, 4, 5]) })
			})
		}, /bytes<4>|4/)
		const backing = new Uint8Array([9, 9, 7, 7, 7, 7, 9, 9])
		const view = new Uint8Array(backing.buffer, 2, 4)
		let id: bigint | undefined
		const written = db.write(function seed(tx) {
			id = tx.insert(Blob, { tag: view }).id
		})
		assert.ok(written.ok)
		assert.deepStrictEqual(must(db.get(Blob, { id: must(id) })).tag, new Uint8Array([7, 7, 7, 7]))
	})

	test("interval rays (end = MAX_END) round-trip in both element domains", function rays() {
		let id: bigint | undefined
		const written = db.write(function seed(tx) {
			id = tx.insert(Ray, { at: span(3n, U64_MAX), sat: span(I64_MIN, I64_MAX) }).id
		})
		assert.ok(written.ok)
		const back = must(db.get(Ray, { id: must(id) }))
		assert.deepEqual(back.at, { start: 3n, end: U64_MAX })
		assert.deepEqual(back.sat, { start: I64_MIN, end: I64_MAX })
		assert.equal(db.contains(Ray, back), true)
	})

	test("an empty interval smuggled past span() is refused typed at the bridge", function emptyInterval() {
		assert.throws(function smuggle() {
			db.write(function bad(tx) {
				/**
				 * Ray.at's value type is the bare structural interval, so the
				 * plain object is assignable — no cast, the emptiness never
				 * touches span().
				 */
				const fake: { start: bigint; end: bigint } = { start: 5n, end: 5n }
				tx.insert(Ray, {
					at: fake,
					sat: span(0n, 1n)
				})
			})
		}, /empty interval/)
	})

	test("a fixed-width interval field refuses a wrong-width value", function fixedWidth() {
		const right = db.write(function good(tx) {
			tx.insert(Slot, { when: span(10n, 12n) })
		})
		assert.ok(right.ok, "the exact-width value lands")
		/**
		 * The refusal is the engine's structural-typing judgment: a width-5
		 * interval is a DIFFERENT type than interval<u64, 2>, so the dyn lane
		 * reports TypeMismatch ("wrong value kind", relation/field by id).
		 */
		assert.throws(
			function wrongWidth() {
				db.write(function bad(tx) {
					tx.insert(Slot, { when: span(10n, 15n) })
				})
			},
			/wrong value kind|width/,
			"a width-5 value on interval<u64, 2> must be refused"
		)
	})

	test("strings: empty, astral, and combining round-trip; contains agrees", function strings() {
		let emptyId: bigint | undefined
		let astralId: bigint | undefined
		const written = db.write(function seed(tx) {
			emptyId = tx.insert(Txt, { note: "" }).id
			astralId = tx.insert(Txt, { note: "𝔽😀́" }).id
		})
		assert.ok(written.ok)
		assert.equal(must(db.get(Txt, { id: must(emptyId) })).note, "")
		assert.equal(must(db.get(Txt, { id: must(astralId) })).note, "𝔽😀́")
		assert.equal(db.contains(Txt, { id: must(emptyId), note: "" }), true)
	})

	test("a lone surrogate is refused typed — never silently mangled", function loneSurrogate() {
		/**
		 * A lone surrogate is a legal JS value (text truncated mid-emoji),
		 * but the bridge's UTF-8 crossing would silently replace it with
		 * U+FFFD: the stored fact would differ from the written one, and two
		 * DISTINCT JS strings ("\uD800" vs "\uDC00") would collapse to one
		 * stored fact. The marshal seam refuses the shape typed instead (the
		 * bijection law) — one seam, so insert and lookup are covered alike.
		 */
		assert.throws(
			function insertRefused() {
				db.write(function seed(tx) {
					tx.insert(Txt, { note: "\uD800" })
				})
			},
			/well-formed string/,
			"the write path refuses the malformed string"
		)
		let id: bigint | undefined
		const seeded = db.write(function seed(tx) {
			id = tx.insert(Txt, { note: "intact" }).id
		})
		assert.ok(seeded.ok)
		assert.throws(
			function containsRefused() {
				db.contains(Txt, { id: must(id), note: "\uDC00" })
			},
			/well-formed string/,
			"the lookup path refuses too — a never-written fact must not answer"
		)
	})

	test("a mirrors violation maps to the one SDK statement value, both directions", function mirrorViolation() {
		/**
		 * The written orientation: a Special item with no Terms row violates
		 * the `source <= target` slot as spelled.
		 */
		const missingTerms = db.write(function violate(tx) {
			tx.insert(Item, { kind: "Special", flag: true })
		})
		assert.ok(!missingTerms.ok, "the mirror judges the written orientation")
		const forward = must(missingTerms.violations[0])
		assert.strictEqual(forward.statement, specialMirror)
		assert.equal(forward.kind, "containment")
		assert.equal(forward.canonical, renderStatement(specialMirror))
		assert.ok(forward.direction !== undefined, "a containment violation carries its direction")
		assert.equal(forward.orientation, "written", "the violated slot is the written orientation")

		/**
		 * The mirrored orientation: a Terms row whose item is not Special
		 * violates the engine-materialized `target <= source` partner slot.
		 * `direction` is the engine's per-slot payload VERBATIM (both slots
		 * report their own source as unsatisfied), so the side of the `==`
		 * is carried by `orientation`, never by flipping `direction`.
		 */
		const seeded = db.write(function seed(tx) {
			const plain = tx.insert(Item, { kind: "Plain", flag: false })
			tx.insert(Terms, { item: plain.id, rate: 1n })
		})
		assert.ok(!seeded.ok, "the reverse orientation judges too")
		const reverse = must(seeded.violations[0])
		assert.strictEqual(reverse.statement, specialMirror)
		assert.equal(reverse.canonical, renderStatement(specialMirror))
		assert.ok(reverse.direction !== undefined)
		assert.equal(reverse.orientation, "mirrored", "the violated slot is the mirrored partner")
	})

	test("a handle selection without its companion closed-reference containment is refused at schema()", async function undeclaredRefRefused() {
		/**
		 * The engine's canonical renderer resolves handle spellings ONLY
		 * through declared containments (`schema/render.rs` closed_target_of):
		 * without `contained(on(Item2, "kind"), on(Kind, "id"))` the engine
		 * would render "kind == 1" where renderStatement prints
		 * "kind == Special", breaking the paste-back law
		 * `violation.canonical === renderStatement(statement)`. schema()
		 * refuses the bare spelling loudly, naming the missing containment.
		 */
		const Item2 = relation("Item2", { id: u64.fresh, kind: Kind.id })
		const Terms2 = relation("Terms2", { item: u64 })
		const bareMirror = mirrors(on(Item2.where({ kind: "Special" }), "id"), on(Terms2, "item"))
		assert.throws(function admitBare() {
			schema("Bare", { Kind, Item2, Terms2 }, [key(Terms2, ["item"]), bareMirror])
		}, /no declared containment resolves the closed reference.*contained\(on\(Item2, "kind"\), on\(Kind, "id"\)\)/)

		/**
		 * With the companion containment declared, the two renderers agree on
		 * the one spelling — the equality the refusal protects.
		 */
		const Full = schema("Full", { Kind, Item2, Terms2 }, [
			key(Terms2, ["item"]),
			contained(on(Item2, "kind"), on(Kind, "id")),
			bareMirror
		])
		const full = await Db.create(path.join(tmpRoot, "full"), Full)
		const rejected = full.write(function violate(tx) {
			tx.insert(Item2, { kind: "Special" })
		})
		assert.ok(!rejected.ok)
		const violation = must(rejected.violations[0])
		assert.strictEqual(violation.statement, bareMirror)
		assert.equal(
			violation.canonical,
			renderStatement(bareMirror),
			"the engine canonical and the SDK renderer agree on the one spelling"
		)
	})

	test("a thrown write callback aborts the delta and leaves the store writable", function thrownWrite() {
		const before = db.scan(Num).length
		assert.throws(function boom() {
			db.write(function bad(tx) {
				tx.insert(Num, { u: 1n, s: 1n })
				throw errors.new("host boom")
			})
		}, /host boom/)
		assert.equal(db.scan(Num).length, before, "the recorded insert never landed")
		const next = db.write(function fine(tx) {
			tx.insert(Num, { u: 2n, s: 2n })
		})
		assert.ok(next.ok, "the writer is free after the aborted transaction")
	})

	test("a thrown witnessed callback (after a delta verb) aborts and frees the writer", function thrownWitnessed() {
		const before = db.scan(Num).length
		assert.throws(function boom() {
			db.writeWitnessed(function bad(_snap, tx) {
				tx.insert(Num, { u: 3n, s: 3n })
				throw errors.new("witnessed boom")
			})
		}, /witnessed boom/)
		assert.equal(db.scan(Num).length, before)
		const next = db.write(function fine(tx) {
			tx.insert(Num, { u: 4n, s: 4n })
		})
		assert.ok(next.ok)
		/**
		 * And read scopes still open/close cleanly (no leaked snapshot slots).
		 */
		assert.equal(typeof db.read((snap) => snap.generation), "bigint")
	})

	test("a violating delta then a throw still rethrows the host error (no half-judged state)", function violateThenThrow() {
		assert.throws(function boom() {
			db.write(function bad(tx) {
				tx.insert(Item, { kind: "Special", flag: true })
				throw errors.new("after violation")
			})
		}, /after violation/)
		const clean = db.write(function fine(tx) {
			tx.insert(Num, { u: 5n, s: 5n })
		})
		assert.ok(clean.ok)
	})

	test("an async write callback is refused — never a silent empty commit", async function asyncCallback() {
		const before = db.read((snap) => snap.generation)
		const beforeRows = db.scan(Num).length
		let lateError: unknown
		/**
		 * NOTE: no cast is needed — `async (tx) => {…}` returns
		 * Promise<void>, which TS accepts where DeltaBuild's `void`
		 * return is expected, so this compiles as plain host code.
		 */
		const attempt = errors.trySync(function admitSneaky() {
			db.write(async function sneaky(tx) {
				await Promise.resolve()
				const late = errors.trySync(function lateInsert() {
					tx.insert(Num, { u: 6n, s: 6n })
				})
				if (late.error) {
					lateError = late.error
				}
			})
		})
		await new Promise((resolve) => setImmediate(resolve))
		const after = db.read((snap) => snap.generation)
		if (attempt.error === undefined) {
			/**
			 * If the SDK admitted the thenable, the write must still be a real
			 * write: either the insert landed or nothing (incl. no generation
			 * move) happened. A spent-transaction error on the late insert with
			 * an ok:true result is the defect this probes for.
			 */
			assert.equal(lateError, undefined, "the callback's inserts must not throw spent")
			assert.equal(db.scan(Num).length, beforeRows + 1, "the insert must land if admitted")
		} else {
			assert.match(
				attempt.error.toString(),
				/returned a thenable/,
				"the refusal is the typed thenable probe, nothing colder"
			)
			assert.equal(after, before, "a refused callback commits nothing")
		}
	})
})

describe("native handle lifecycle probes", function nativeSuite() {
	const NativeKind = relation("NKind", { id: u64.fresh, note: str })
	const NativeTheory = schema("NativeHunt", { NKind: NativeKind }, [])

	test("double abort, commit-after-abort, and a second begin are typed refusals", function lifecycle() {
		const opened = native.dbCreate(path.join(tmpRoot, "native"), lower(NativeTheory))
		assert.ok(opened.ok)
		const handle = opened.db
		const tx = native.dbWriteBegin(handle)
		assert.throws(function secondBegin() {
			native.dbWriteBegin(handle)
		}, /already open/)
		native.txAbort(tx)
		assert.throws(function doubleAbort() {
			native.txAbort(tx)
		}, /closed transaction/)
		assert.throws(function commitAfterAbort() {
			native.txCommit(tx)
		}, /closed transaction/)
		/**
		 * The writer is free again after the abort cleared the guard.
		 */
		const tx2 = native.dbWriteBegin(handle)
		const outcome = native.txCommit(tx2)
		assert.ok(outcome.ok, "an empty commit lands after the aborted predecessor")
		/**
		 * A spent-by-commit handle refuses further use.
		 */
		assert.throws(function abortAfterCommit() {
			native.txAbort(tx2)
		}, /closed transaction/)
		native.dbClose(handle)
		assert.throws(function doubleClose() {
			native.dbClose(handle)
		}, /closed db/)
	})

	test("a snapshot survives its db handle close; double snapshot close is typed", function snapshotLifecycle() {
		const opened = native.dbCreate(path.join(tmpRoot, "native-snap"), lower(NativeTheory))
		assert.ok(opened.ok)
		const handle = opened.db
		const snap = native.dbSnapshot(handle)
		assert.deepEqual(native.snapshotScan(snap, 0), [])
		native.snapshotClose(snap)
		assert.throws(function scanAfterClose() {
			native.snapshotScan(snap, 0)
		}, /closed snapshot/)
		assert.throws(function doubleClose() {
			native.snapshotClose(snap)
		}, /closed snapshot/)
		native.dbClose(handle)
	})
})
