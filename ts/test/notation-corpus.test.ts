/**
 * The notation conformance corpus, TS replay (PRD-M4): the checked-in
 * corpus (`crates/bumbledb-query/tests/notation-corpus/`) pins one
 * (notation ⇄ ProgramIr JSON) pair per case, byte-authored by the Rust
 * suite; this suite replays the SAME documents from the other host —
 * every `"builder": true` case is constructed in the query builder and
 * its lowered `ProgramIr` must `JSON.stringify` to exactly the pinned
 * `program` bytes; every `"builder": false` case (the spellings the
 * builder's laws refuse — an idb head position bound only by the idb
 * atom, sparse positions, position selections) is written as hand IR and
 * held to the same byte equality, and the skipped-from-builder count is
 * asserted EXACTLY, so a silent skip is impossible. Every case's program
 * — both flags — must be ACCEPTED by `dbPrepare` against a store of the
 * corpus theory, and that theory is declared here structurally (the laws
 * type the columns) yet pins to the same engine fingerprint as the Rust
 * `schema!` declaration (`schema-fingerprint.txt` — the T5 mechanism,
 * one line), so the corpus schemas cannot drift. A disagreement is a
 * trophy, not a merge conflict (the corpus README states the law).
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, before, describe, test } from "node:test"
import { closed } from "#closed.ts"
import { on } from "#face.ts"
import { i64, interval, str, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import type { DbHandle, ProgramIr } from "#native.ts"
import { native } from "#native.ts"
import { ALLEN } from "#query/atom.ts"
import type { AnyQuery } from "#query/lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { program } from "#query/predicate.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained, key } from "#statements.ts"

const corpusDir = path.join(import.meta.dirname, "..", "..", "crates", "bumbledb-query", "tests", "notation-corpus")

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-notation-corpus-"))
const storeDir = path.join(tmpRoot, "store")

/**
 * The corpus theory: the benchmark ledger, declared structurally — the
 * exact twin of the `schema!` declaration in
 * `crates/bumbledb-query/tests/notation_corpus.rs` (relations, fields,
 * and statements in identical declaration order; Rust spells the domains
 * as newtypes, this side derives them from the same statements). The
 * fingerprint pin below is the referee.
 */
const Currency = closed("Currency", ["Usd", "Eur", "Gbp"])
const Source = closed("Source", ["Manual", "Import", "System"])
const Tag = closed("Tag", ["Fee", "Rebate", "Adjustment"])

const Holder = relation("Holder", { id: u64.fresh, name: str })
const Account = relation("Account", { id: u64.fresh, holder: u64, currency: Currency.id })
const Instrument = relation("Instrument", { id: u64.fresh, symbol: str })
const JournalEntry = relation("JournalEntry", { id: u64.fresh, source: Source.id, created_at: i64 })
const Posting = relation("Posting", {
	id: u64.fresh,
	entry: u64,
	account: u64,
	instrument: u64,
	amount: i64,
	at: i64
})
const PostingTag = relation("PostingTag", { posting: u64, tag: Tag.id })
const Org = relation("Org", { id: u64.fresh, name: str })
const OrgParent = relation("OrgParent", { child: u64, parent: u64 })
const Mandate = relation("Mandate", { account: u64, org: u64, active: interval(i64) })

const Ledger = schema(
	"Ledger",
	{ Currency, Source, Tag, Holder, Account, Instrument, JournalEntry, Posting, PostingTag, Org, OrgParent, Mandate },
	[
		contained(on(Account, "holder"), on(Holder, "id")),
		contained(on(Account, "currency"), on(Currency, "id")),
		contained(on(Posting, "entry"), on(JournalEntry, "id")),
		contained(on(Posting, "account"), on(Account, "id")),
		contained(on(Posting, "instrument"), on(Instrument, "id")),
		contained(on(PostingTag, "posting"), on(Posting, "id")),
		contained(on(PostingTag, "tag"), on(Tag, "id")),
		contained(on(JournalEntry, "source"), on(Source, "id")),
		contained(on(OrgParent, "child"), on(Org, "id")),
		contained(on(OrgParent, "parent"), on(Org, "id")),
		contained(on(Mandate, "account"), on(Account, "id")),
		contained(on(Mandate, "org"), on(Org, "id")),
		key(Mandate, ["account", "active"])
	]
)

/** Relation ids = record declaration order (the law `lowerQuery` rides). */
const ACCOUNT_ID = 4
const POSTING_ID = 7
const ORG_PARENT_ID = 10

/**
 * The corpus normalization (documented in the corpus README): every id
 * is a JSON number; every `bigint` payload renders as its decimal
 * string. `JSON.stringify` with this replacer over a `ProgramIr` value
 * produces exactly the bytes the Rust encoder pins.
 */
function bigintAsDecimalString(_key: string, value: unknown): unknown {
	return typeof value === "bigint" ? value.toString() : value
}

/**
 * Every `"builder": true` case, constructed in the query builder — one
 * entry per corpus case, the construction the pinned `program` bytes
 * must equal after lowering.
 */
const constructions: Readonly<Record<string, AnyQuery>> = {
	"holder-names": query(Ledger).rule((r) => {
		const { id: h, name } = v(Holder)
		return r.match(Holder, { id: h, name }).find({ name })
	}),
	"amount-selection": query(Ledger).rule((r) => {
		const { id } = v(Posting)
		return r.match(Posting, { id, amount: -100n }).find({ id })
	}),
	"usd-accounts": query(Ledger).rule((r) => {
		const { id } = v(Account)
		return r.match(Account, { id, currency: "Usd" }).find({ id })
	}),
	"account-selection-param": query(Ledger).rule((r) => {
		const { id } = v(Posting)
		return r.match(Posting, { id, account: r.param("acct") }).find({ id })
	}),
	"scalar-comparisons": query(Ledger).rule((r) => {
		const { id, entry, account, instrument, amount, at } = v(Posting)
		return r
			.match(Posting, { id, entry, account, instrument, amount, at })
			.where(r.eq(id, r.param("wanted")))
			.where(r.ne(entry, 0n))
			.where(r.lt(account, 10n))
			.where(r.le(instrument, 10n))
			.where(r.gt(amount, -10n))
			.where(r.ge(at, -10n))
			.find({ id })
	}),
	"currency-in-set": query(Ledger).rule((r) => {
		const { id } = v(Account)
		return r.match(Account, { id, currency: r.inSet("currencies") }).find({ id })
	}),
	"mandate-point-membership": query(Ledger).rule((r) => {
		const { org, active } = v(Mandate)
		return r
			.match(Mandate, { org, active })
			.where(r.pointIn(r.param("today"), active))
			.find({ org })
	}),
	"mandate-window": query(Ledger).rule((r) => {
		const { org, active } = v(Mandate)
		return r
			.match(Mandate, { org, active })
			.where(r.allen(active, ALLEN.intersects, r.param("window")))
			.find({ org })
	}),
	"mandate-adjacent": query(Ledger).rule((r) => {
		const { org, active } = v(Mandate)
		return r
			.match(Mandate, { org, active })
			.where(r.allen(active, ALLEN.before | ALLEN.meets, r.param("window")))
			.find({ org })
	}),
	"mandate-mask-param": query(Ledger).rule((r) => {
		const { account: a, active: s } = v(Mandate)
		const { account: b, active: t } = v(Mandate)
		return r
			.match(Mandate, { account: a, active: s })
			.match(Mandate, { account: b, active: t })
			.where(r.lt(a, b))
			.where(r.allen(s, r.maskParam("rel"), t))
			.find({ a, b })
	}),
	"dormant-holders": query(Ledger).rule((r) => {
		const { id: a, holder } = v(Account)
		return r
			.match(Account, { id: a, holder })
			.where(r.not(Posting, { account: a }))
			.find({ holder })
	}),
	balances: query(Ledger).rule((r) => {
		const { account, amount } = v(Posting)
		return r.match(Posting, { account, amount }).find({ account, amount: r.sum(amount), count: r.count() })
	}),
	"entry-fanout": query(Ledger).rule((r) => {
		const { entry, account } = v(Posting)
		return r.match(Posting, { entry, account }).find({ account, entry: r.countDistinct(entry) })
	}),
	"amount-floor": query(Ledger).rule((r) => {
		const { account, amount } = v(Posting)
		return r.match(Posting, { account, amount }).find({ account, amount: r.min(amount) })
	}),
	"amount-ceiling": query(Ledger).rule((r) => {
		const { account, amount } = v(Posting)
		return r.match(Posting, { account, amount }).find({ account, amount: r.max(amount) })
	}),
	"latest-posting": query(Ledger).rule((r) => {
		const { id, at } = v(Posting)
		return r.match(Posting, { id, at }).find({ id: r.argMax(id, at) })
	}),
	"earliest-posting": query(Ledger).rule((r) => {
		const { id, at } = v(Posting)
		return r.match(Posting, { id, at }).find({ id: r.argMin(id, at) })
	}),
	"mandate-pack": query(Ledger).rule((r) => {
		const { org, active } = v(Mandate)
		return r.match(Mandate, { org, active }).find({ org, active: r.pack(active) })
	}),
	"mandate-durations": query(Ledger).rule((r) => {
		const { org, active } = v(Mandate)
		return r.match(Mandate, { org, active }).find({ org, active: r.duration(active) })
	}),
	"long-mandates": query(Ledger).rule((r) => {
		const { org, active } = v(Mandate)
		return r
			.match(Mandate, { org, active })
			.where(r.ge(r.duration(active), 3600n))
			.find({ org, active: r.sum(r.duration(active)) })
	}),
	"usd-or-eur-accounts": query(Ledger)
		.rule((r) => {
			const { id } = v(Account)
			return r.match(Account, { id, currency: "Usd" }).find({ id })
		})
		.rule((r) => {
			const { id } = v(Account)
			return r.match(Account, { id, currency: "Eur" }).find({ id })
		}),
	"org-reach-rooted": program(Ledger, (p) => {
		const declared = p.rec("reach")
		// One head name across both rules (the TS alignment law names
		// columns; the notation's `o`/`p` are macro-local and erased —
		// the lowered IR is identical either way). Head keys name the idb
		// join positions: rooted's rule-0 head column is `n`, so both idb
		// records bind `{ n: <var> }`.
		const rooted = declared.rule((r) => {
			const { id: n } = v(Org)
			return r
				.match(Org, { id: n })
				.where(r.eq(n, r.param("root")))
				.find({ n })
		})
		const reach = rooted.rule((r) => {
			const { child: c, parent: n } = v(OrgParent)
			return r.match(OrgParent, { child: c, parent: n }).idb(rooted, { n: c }).find({ n })
		})
		return p.output((r) => {
			const { id: p2 } = v(Org)
			return r.match(Org, { id: p2 }).idb(reach, { n: p2 }).find({ p: p2 })
		})
	})
}

/** One positional variable term (assignable at both term and find positions). */
function posVar(id: number): { readonly kind: "var"; readonly var: number } {
	return { kind: "var", var: id }
}

/**
 * The `"builder": false` cases as HAND-WRITTEN `ProgramIr` — the spellings
 * the builder's laws refuse (an idb head position bound only by the idb
 * atom; sparse idb positions; idb position selections). A host writing IR
 * by hand is exactly the story these pins tell: the bytes must still
 * equal the corpus, and the engine must still prepare them.
 */
const handWritten: Readonly<Record<string, ProgramIr>> = {
	"org-reach": {
		predicates: [
			{
				head: [{ kind: "var" }, { kind: "var" }],
				rules: [
					{
						finds: [posVar(0), posVar(1)],
						atoms: [
							{
								source: { kind: "edb", relation: ORG_PARENT_ID },
								bindings: [
									[0, posVar(0)],
									[1, posVar(1)]
								]
							}
						],
						negated: [],
						conditions: []
					},
					{
						finds: [posVar(0), posVar(2)],
						atoms: [
							{
								source: { kind: "edb", relation: ORG_PARENT_ID },
								bindings: [
									[0, posVar(0)],
									[1, posVar(1)]
								]
							},
							{
								source: { kind: "idb", pred: 0 },
								bindings: [
									[0, posVar(1)],
									[1, posVar(2)]
								]
							}
						],
						negated: [],
						conditions: []
					}
				]
			},
			{
				head: [{ kind: "var" }, { kind: "var" }],
				rules: [
					{
						finds: [posVar(0), posVar(1)],
						atoms: [
							{
								source: { kind: "idb", pred: 0 },
								bindings: [
									[0, posVar(0)],
									[1, posVar(1)]
								]
							}
						],
						negated: [],
						conditions: []
					}
				]
			}
		],
		output: 1
	},
	"posted-sparse": {
		predicates: [
			{
				head: [{ kind: "var" }, { kind: "var" }, { kind: "var" }],
				rules: [
					{
						finds: [posVar(0), posVar(1), posVar(2)],
						atoms: [
							{
								source: { kind: "edb", relation: POSTING_ID },
								bindings: [
									[0, posVar(0)],
									[2, posVar(1)],
									[4, posVar(2)]
								]
							}
						],
						negated: [],
						conditions: []
					}
				]
			},
			{
				head: [{ kind: "var" }],
				rules: [
					{
						finds: [posVar(0)],
						atoms: [
							{
								source: { kind: "idb", pred: 0 },
								bindings: [
									[2, posVar(0)],
									[0, { kind: "paramSet", param: 0 }]
								]
							}
						],
						negated: [],
						conditions: []
					}
				]
			}
		],
		output: 1
	},
	"usd-selected": {
		predicates: [
			{
				head: [{ kind: "var" }, { kind: "var" }],
				rules: [
					{
						finds: [posVar(0), posVar(1)],
						atoms: [
							{
								source: { kind: "edb", relation: ACCOUNT_ID },
								bindings: [
									[0, posVar(0)],
									[2, posVar(1)]
								]
							}
						],
						negated: [],
						conditions: []
					}
				]
			},
			{
				head: [{ kind: "var" }],
				rules: [
					{
						finds: [posVar(0)],
						atoms: [
							{
								source: { kind: "idb", pred: 0 },
								bindings: [
									[0, posVar(0)],
									// The WIRE is raw: "Usd" lowers to its declaration-order
									// row id (Currency: Usd 0, Eur 1, Gbp 2) — the name↔id
									// bijection is the SDK's, above this seam.
									[1, { kind: "literal", value: { kind: "u64", value: 0n } }]
								]
							}
						],
						negated: [],
						conditions: []
					}
				]
			}
		],
		output: 1
	}
}

/** One parsed corpus document (the fields this replayer reads). */
interface CorpusDoc {
	readonly name: string
	readonly builder: boolean
	readonly program: unknown
}

/** Parses and shape-checks one corpus document. */
function docOf(file: string): CorpusDoc {
	const text = fs.readFileSync(path.join(corpusDir, file), "utf8")
	const parsed: unknown = JSON.parse(text)
	if (typeof parsed !== "object" || parsed === null) {
		assert.fail(`${file}: a corpus document is a JSON object`)
	}
	if (!("name" in parsed) || typeof parsed.name !== "string") {
		assert.fail(`${file}: a corpus document names itself`)
	}
	if (!("builder" in parsed) || typeof parsed.builder !== "boolean") {
		assert.fail(`${file}: a corpus document carries the builder flag`)
	}
	if (!("program" in parsed)) {
		assert.fail(`${file}: a corpus document pins a program`)
	}
	assert.equal(`${parsed.name}.json`, file, `${file}: the document name is the file name`)
	return { name: parsed.name, builder: parsed.builder, program: parsed.program }
}

describe("the notation conformance corpus (TS replay)", () => {
	let db: DbHandle

	before(function openTheCorpusStore() {
		const created = native.dbCreate(storeDir, lower(Ledger))
		if (!created.ok) {
			assert.fail(`create the corpus store: ${created.message}`)
		}
		db = created.db
	})

	after(function cleanup() {
		native.dbClose(db)
		fs.rmSync(tmpRoot, { recursive: true, force: true })
	})

	/** Prepares one corpus program; refusal fails the case by name. */
	function prepared(name: string, ir: ProgramIr): void {
		const result = native.dbPrepare(db, ir)
		if (!result.ok) {
			assert.fail(`case ${name}: dbPrepare refused the corpus program: ${result.message}`)
		}
		native.preparedClose(result.prepared)
	}

	test("the corpus theory pins to the cross-host schema fingerprint", () => {
		const pinned = fs.readFileSync(path.join(corpusDir, "schema-fingerprint.txt"), "utf8").trim()
		assert.equal(
			native.dbFingerprint(db),
			pinned,
			"the structural declaration and the Rust schema! declaration are one theory"
		)
	})

	test("every case replays byte-identical and prepares", () => {
		const files = fs
			.readdirSync(corpusDir)
			.filter(function isCase(file) {
				return file.endsWith(".json")
			})
			.sort()
		assert.ok(files.length >= 20, `the corpus holds at least 20 cases (got ${files.length})`)

		let skipped = 0
		for (const file of files) {
			const doc = docOf(file)
			const pinned = JSON.stringify(doc.program)
			if (doc.builder) {
				const construction = constructions[doc.name]
				if (construction === undefined) {
					assert.fail(`case ${doc.name}: builder-expressible but no construction replays it`)
				}
				const ir = lowerQuery(construction)
				assert.equal(
					JSON.stringify(ir, bigintAsDecimalString),
					pinned,
					`case ${doc.name}: the builder lowering equals the pinned ProgramIr bytes`
				)
				prepared(doc.name, ir)
			} else {
				skipped += 1
				const ir = handWritten[doc.name]
				if (ir === undefined) {
					assert.fail(`case ${doc.name}: not builder-expressible and no hand-written IR replays it`)
				}
				assert.equal(
					JSON.stringify(ir, bigintAsDecimalString),
					pinned,
					`case ${doc.name}: the hand-written ProgramIr equals the pinned bytes`
				)
				prepared(doc.name, ir)
			}
		}

		// The skip census, exact: the corpus's `"builder": false` count is
		// pinned here, so a case silently falling out of the builder lane
		// (or a new unbuildable case arriving) fails until this number is
		// consciously moved.
		assert.equal(skipped, 3, "exactly the three unbuildable spellings skip the builder lane")
		assert.equal(Object.keys(handWritten).length, skipped, "every hand-written IR belongs to a skipped corpus case")

		// No orphan constructions: every entry replays a real corpus case.
		const names = new Set(
			files.map(function stem(file) {
				return file.slice(0, -".json".length)
			})
		)
		for (const name of Object.keys(constructions)) {
			assert.ok(names.has(name), `construction ${name} replays no corpus case`)
		}
		for (const name of Object.keys(handWritten)) {
			assert.ok(names.has(name), `hand-written IR ${name} replays no corpus case`)
		}
		assert.equal(
			Object.keys(constructions).length + Object.keys(handWritten).length,
			files.length,
			"every corpus case is replayed exactly once"
		)
	})
})
