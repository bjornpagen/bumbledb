/**
 * The SDK cookbook's compile-and-run pin (PRD-S5; rewritten to the 0.3.0
 * idioms by PRD-K7, swept to the 0.4.0 host idiom — handle names as string
 * literals, native `switch` dispatch, the record-table idiom — by PRD-H6;
 * the fingerprints never moved: spellings are not the theory).
 * `ts/COOKBOOK.md` carries the engine cookbook's 30
 * recipes (`bumbledb/docs/cookbook.md`) translated to the structural API,
 * and THIS file is what keeps them true: every recipe's schema is
 * constructed here through the public surface (so it compiles, cast-free),
 * admitted by the REAL engine (`dbCreate` — schema validation is the
 * engine's judgment, never re-implemented), its fingerprint asserted stable
 * across a reopen (the theory's identity is a pure function of the schema)
 * AND pinned to the per-recipe cross-host goldens
 * (`fixtures/cookbook-fingerprints.txt` — the file the Rust cookbook suite
 * also asserts against, PRD-T5), and every query snippet lowered through
 * `db.prepare` (the engine's own IR validation accepts the lowering). A
 * recipe that stops compiling against the SDK fails the build — the
 * cookbook can never drift from the surface. Recipes whose GUARANTEES the
 * engine's own cookbook test already proves (pointwise disjointness, keyed
 * `==`, exact partition, …) are construct-and-lower only here — the pin is
 * that the SURFACE expresses the recipe, not a re-test of engine
 * semantics. The two host-code recipes (20's witnessed update-where, 24's
 * frontier loop) run for real: their loops ARE the recipe, so they execute
 * against the store they were written for.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import process from "node:process"
import { after, describe, test } from "node:test"

import type { Infer, Schema, SchemaRelations } from "#index.ts"
import {
	ALLEN,
	abandon,
	allen,
	bool,
	bytes,
	closed,
	contained,
	Db,
	eq,
	i64,
	interval,
	key,
	lower,
	lt,
	mirrors,
	not,
	on,
	pointIn,
	program,
	query,
	relation,
	renderStatement,
	schema,
	str,
	u64,
	v
} from "#index.ts"
import { native } from "#native.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-cookbook-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

/**
 * The per-recipe cross-host goldens (PRD-T5):
 * `fixtures/cookbook-fingerprints.txt` pins every recipe's engine-computed
 * fingerprint, and the Rust cookbook suite
 * (`crates/bumbledb-query/tests/cookbook.rs`) reads the SAME file — so drift
 * anywhere upstream of the hasher (a recipe schema edited on one side, the
 * lowering, the duplicated name→id resolution) fails the suite on the side
 * that moved. `REGEN_FINGERPRINTS=1 node --test test/cookbook.test.ts`
 * rewrites the fixture from this suite's fingerprints; the Rust side never
 * writes it.
 */
const goldensPath = path.join(import.meta.dirname, "fixtures", "cookbook-fingerprints.txt")
const regenerating = process.env.REGEN_FINGERPRINTS === "1"

/** The pinned goldens — absent while regenerating (the values are in flux). */
const goldens = regenerating ? undefined : readGoldens()

/** Every admitted pinned fingerprint, recipe id → hex — the regen source. */
const witnessed = new Map<string, string>()

/** The header the regeneration writes back verbatim, above the sorted lines. */
const GOLDENS_HEADER = `# ts/test/fixtures/cookbook-fingerprints.txt — the per-recipe cross-host
# fingerprint goldens: one line per engine-cookbook recipe, \`rNN <64-hex>\`,
# sorted by recipe number.
#
# Each value is the recipe theory's engine-computed schema fingerprint —
# blake3 over the canonical descriptor bytes (label \`bumbledb-schema-v4\`),
# never syntax, never spellings. BOTH cookbook suites read this ONE file:
#   ts/test/cookbook.test.ts                 (SDK constructions, hashed across the FFI)
#   crates/bumbledb-query/tests/cookbook.rs  (schema! constructions, hashed in-process)
# so the same theory drifting on either side fails the suite that moved.
#
# Regeneration (the TS side ONLY — the Rust side never writes this file):
#   cd ts && REGEN_FINGERPRINTS=1 node --test test/cookbook.test.ts
# Two consecutive regenerations are byte-identical. K7 (the cookbook
# rewrite) owns regenerating the values for the recipes it rewrites.`

after(function writeGoldens() {
	if (!regenerating) {
		return
	}
	const entries = [...witnessed.entries()].sort(function byRecipe(a, b) {
		if (a[0] < b[0]) {
			return -1
		}
		if (a[0] > b[0]) {
			return 1
		}
		return 0
	})
	const lines = entries.map(function line([recipe, hex]) {
		return `${recipe} ${hex}`
	})
	fs.writeFileSync(goldensPath, `${GOLDENS_HEADER}\n${lines.join("\n")}\n`)
})

/** Reads the goldens fixture: `rNN <64-hex>` lines; `#` comments and blanks skipped. */
function readGoldens(): ReadonlyMap<string, string> {
	const pinned = new Map<string, string>()
	for (const raw of fs.readFileSync(goldensPath, "utf8").split("\n")) {
		const line = raw.trim()
		if (line === "" || line.startsWith("#")) {
			continue
		}
		const [recipe, hex, ...rest] = line.split(" ")
		assert.ok(
			recipe !== undefined && hex !== undefined && rest.length === 0,
			`a goldens line is \`rNN <64-hex>\`: ${line}`
		)
		assert.match(recipe, /^r\d\d$/, `a goldens recipe id is rNN: ${line}`)
		assert.match(hex, /^[0-9a-f]{64}$/, `a golden is 64 lowercase hex chars: ${line}`)
		assert.ok(!pinned.has(recipe), `one goldens line per recipe: ${recipe}`)
		pinned.set(recipe, hex)
	}
	return pinned
}

/**
 * Recipe 28's v1 theory is the migration SOURCE the recipe exports from —
 * prose in the engine cookbook, not the recipe's pinned schema block — so
 * its store admits unpinned: the test body asserts its fingerprint differs
 * from the pinned v2 target, and the Rust roster carries only the target.
 */
const unpinnedStores: ReadonlySet<string> = new Set(["r28-payroll-v1"])

/** The recipe id a store name carries (`rNN-…`), or undefined when unpinned. */
function recipeIdOf(name: string): string | undefined {
	if (unpinnedStores.has(name)) {
		return undefined
	}
	const id = name.slice(0, 3)
	assert.match(id, /^r\d\d$/, `a recipe store name leads with its recipe id: ${name}`)
	assert.equal(name.charAt(3), "-", `a recipe store name leads with its recipe id: ${name}`)
	return id
}

/** One admitted recipe store: the open `Db` and the theory's engine-computed fingerprint. */
interface Admitted<Rels extends SchemaRelations> {
	readonly db: Db<Rels>
	readonly fingerprint: string
}

/**
 * Admits one recipe's theory against the real engine and pins its identity:
 * create (the engine's schema validation is the acceptance judgment), read
 * the fingerprint, assert it equals the recipe's cross-host golden (the
 * same line the Rust cookbook suite asserts), close, reopen under the
 * identical theory (the fingerprint gate passes and reads the same hex back
 * — stability), then open through the public `Db` surface for the recipe's
 * query lowering.
 */
async function admit<Rels extends SchemaRelations>(name: string, theory: Schema<Rels>): Promise<Admitted<Rels>> {
	const dir = path.join(tmpRoot, name)
	const spec = lower(theory)
	const created = native.dbCreate(dir, spec)
	assert.ok(created.ok, `${name}: the engine admits the theory`)
	const fingerprint = native.dbFingerprint(created.db)
	native.dbClose(created.db)
	const recipe = recipeIdOf(name)
	if (recipe !== undefined) {
		witnessed.set(recipe, fingerprint)
		if (goldens !== undefined) {
			const pinned = goldens.get(recipe)
			assert.ok(pinned !== undefined, `${name}: the goldens fixture pins ${recipe}`)
			assert.equal(fingerprint, pinned, `${name}: the fingerprint matches the cross-host golden (${recipe})`)
		}
	}
	const reopened = native.dbOpen(dir, spec)
	assert.ok(reopened.ok, `${name}: the identical theory reopens the store`)
	assert.equal(native.dbFingerprint(reopened.db), fingerprint, `${name}: the fingerprint is stable across reopen`)
	native.dbClose(reopened.db)
	const db = await Db.open(dir, theory)
	return { db, fingerprint }
}

/** Unwraps a value the surrounding test just proved present. */
function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

/** Sorts a bigint array ascending (answers are sets; the host sorts). */
function sorted(values: readonly bigint[]): bigint[] {
	return [...values].sort(function compare(a, b) {
		if (a < b) {
			return -1
		}
		if (a > b) {
			return 1
		}
		return 0
	})
}

describe("the SDK cookbook — every recipe compiles, admits, and lowers", function suite() {
	test("1. the minimal interval schema", async function r01() {
		const Service = relation("Service", { id: u64.fresh, name: str })
		const Outage = relation("Outage", { service: u64, window: interval(i64) })

		const Uptime = schema("Uptime", { Service, Outage }, [
			contained(on(Outage, "service"), on(Service, "id")),
			key(Outage, ["service", "window"])
		])

		// The M1 ruling, pinned through the renderer (never by hand): the key
		// is the dependency-theoretic arrow closing over its own relation.
		assert.equal(
			renderStatement(key(Outage, ["service", "window"])),
			"Outage(service, window) -> Outage",
			"the key renders as the FD arrow — the canonical spelling, never respelled"
		)

		const downAt = query(Uptime).rule((r) => {
			const { service, window } = v(Outage)
			return r
				.match(Outage, { service, window })
				.where(pointIn(r.param("t"), window))
				.find({ service })
		})
		const overlapping = query(Uptime).rule((r) => {
			const { service, window } = v(Outage)
			return r
				.match(Outage, { service, window })
				.where(allen(window, ALLEN.intersects, r.param("incident")))
				.find({ service, window })
		})
		const downtime = query(Uptime).rule((r) => {
			const { service, window } = v(Outage)
			return r.match(Outage, { service, window }).find({ service, downtime: r.sum(r.duration(window)) })
		})

		const { db } = await admit("r01-uptime", Uptime)
		assert.ok(db.prepare(downAt))
		assert.ok(db.prepare(overlapping))
		assert.ok(db.prepare(downtime))
	})

	test("2. discriminated unions", async function r02() {
		const Kind = closed("Kind", ["Deterministic", "CustomOperator"])
		const Task = relation("Task", { id: u64.fresh, kind: Kind.id })
		const DeterministicGrading = relation("DeterministicGrading", { task: u64, tolerance: i64 })
		const CustomOperatorGrading = relation("CustomOperatorGrading", { task: u64, operator: str })

		const Grading = schema("Grading", { Kind, Task, DeterministicGrading, CustomOperatorGrading }, [
			contained(on(Task, "kind"), on(Kind, "id")),
			key(DeterministicGrading, ["task"]),
			key(CustomOperatorGrading, ["task"]),
			mirrors(on(Task.where({ kind: "Deterministic" }), "id"), on(DeterministicGrading, "task")),
			mirrors(on(Task.where({ kind: "CustomOperator" }), "id"), on(CustomOperatorGrading, "task"))
		])

		// Host dispatch over the discriminator: native `switch` narrowing over
		// the handle union, exhaustive via `satisfies never` (the cookbook's
		// `gradedBy`).
		const gradedBy = (kind: Infer<typeof Kind.id>) => {
			switch (kind) {
				case "Deterministic":
					return "tolerance"
				case "CustomOperator":
					return "operator"
				default:
					return kind satisfies never
			}
		}
		assert.equal(gradedBy("Deterministic"), "tolerance")
		assert.equal(gradedBy("CustomOperator"), "operator")

		await admit("r02-grading", Grading)
	})

	test("3. 0..1 optional attributes", async function r03() {
		const Business = relation("Business", { id: u64.fresh, name: str })
		const MailingAddress = relation("MailingAddress", { business: u64, line: str, city: str })

		const Optionality = schema("Optionality", { Business, MailingAddress }, [
			key(MailingAddress, ["business"]),
			contained(on(MailingAddress, "business"), on(Business, "id"))
		])

		const unaddressed = query(Optionality).rule((r) => {
			const { id: b } = v(Business)
			return r
				.match(Business, { id: b })
				.where(not(MailingAddress, { business: b }))
				.find({ b })
		})

		const { db } = await admit("r03-optionality", Optionality)
		assert.ok(db.prepare(unaddressed))
	})

	test("4. money", async function r04() {
		const Currency = closed("Currency", ["Usd", "Eur", "Gbp"])
		const Account = relation("Account", { id: u64.fresh, name: str })
		const Posting = relation("Posting", {
			id: u64.fresh,
			account: u64,
			currency: Currency.id,
			minor: i64
		})

		const Money = schema("Money", { Currency, Account, Posting }, [
			contained(on(Posting, "account"), on(Account, "id")),
			contained(on(Posting, "currency"), on(Currency, "id"))
		])

		const totals = query(Money).rule((r) => {
			const { id, account, currency, minor } = v(Posting)
			return r.match(Posting, { id, account, currency, minor }).find({ account, currency, total: r.sum(minor) })
		})

		const { db } = await admit("r04-money", Money)
		assert.ok(db.prepare(totals))
	})

	test("5. content addressing", async function r05() {
		const Region = closed("Region", ["Us", "Eu"])
		const Document = relation("Document", { id: u64.fresh, name: str, payload: bytes(32) })
		const Replica = relation("Replica", { payload: bytes(32), region: Region.id })

		const Content = schema("Content", { Region, Document, Replica }, [
			key(Document, ["payload"]),
			contained(on(Replica, "payload"), on(Document, "payload")),
			contained(on(Replica, "region"), on(Region, "id"))
		])

		const byDigest = query(Content).rule((r) => {
			const { id } = v(Document)
			return r.match(Document, { id, payload: r.param("digest") }).find({ id })
		})

		const { db } = await admit("r05-content", Content)
		assert.ok(db.prepare(byDigest))
	})

	test("6. the vocabulary", async function r06() {
		const Priority = closed("Priority", ["Low", "Normal", "Urgent"])
		const Ticket = relation("Ticket", { id: u64.fresh, priority: Priority.id, opened_at: i64 })

		const Tickets = schema("Tickets", { Priority, Ticket }, [contained(on(Ticket, "priority"), on(Priority, "id"))])

		const urgent = query(Tickets).rule((r) => {
			const { id: t } = v(Ticket)
			return r.match(Ticket, { id: t, priority: "Urgent" }).find({ t })
		})

		// Set membership is a plain array — closed-only in query match records
		// (ordinary-field membership is a bound ∈-set param, r.inSet).
		const actionable = query(Tickets).rule((r) => {
			const { id: t } = v(Ticket)
			return r.match(Ticket, { id: t, priority: ["Normal", "Urgent"] }).find({ t })
		})

		const { db } = await admit("r06-tickets", Tickets)
		assert.ok(db.prepare(urgent))
		assert.ok(db.prepare(actionable))
	})

	test("7. the classification", async function r07() {
		const Kind = closed(
			"Kind",
			{ mastered: bool, rank: u64 },
			{
				DirectPass: { mastered: true, rank: 30n },
				JudgedPass: { mastered: true, rank: 20n },
				Failed: { mastered: false, rank: 10n }
			}
		)
		const Attempt = relation("Attempt", { id: u64.fresh, kind: Kind.id })
		const Certificate = relation("Certificate", { attempt: u64, kind: Kind.id })

		const Review = schema("Review", { Kind, Attempt, Certificate }, [
			contained(on(Attempt, "kind"), on(Kind, "id")),
			key(Certificate, ["attempt"]),
			contained(on(Certificate, "attempt"), on(Attempt, "id")),
			contained(on(Certificate, "kind"), on(Kind.where({ mastered: true }), "id"))
		])

		// ψ on the read side: the closed atom is matchable like any relation.
		const masteredAttempts = query(Review).rule((r) => {
			const { id: a, kind: k } = v(Attempt)
			return r.match(Attempt, { id: a, kind: k }).match(Kind, { id: k, mastered: true }).find({ a })
		})

		// The payload tier's host dispatch: the record-table idiom — total by
		// type over the handle union, each entry reading its sealed axiom row
		// off the typed `Kind.axioms` readback.
		const labels: Record<Infer<typeof Kind.id>, string> = {
			DirectPass: `mastered, rank ${Kind.axioms.DirectPass.rank}`,
			JudgedPass: `mastered, rank ${Kind.axioms.JudgedPass.rank}`,
			Failed: "not mastered"
		}
		const label = (k: Infer<typeof Kind.id>) => labels[k]
		assert.equal(label("DirectPass"), "mastered, rank 30")
		assert.equal(label("JudgedPass"), "mastered, rank 20")
		assert.equal(label("Failed"), "not mastered")

		const { db } = await admit("r07-review", Review)
		assert.ok(db.prepare(masteredAttempts))
	})

	test("8. the sub-vocabulary", async function r08() {
		const Severity = closed(
			"Severity",
			{ pages: bool },
			{
				Info: { pages: false },
				Warning: { pages: false },
				Critical: { pages: true },
				Fatal: { pages: true }
			}
		)
		const Incident = relation("Incident", { id: u64.fresh, severity: Severity.id })
		const Escalation = relation("Escalation", { incident: u64, severity: Severity.id, at: i64 })

		const Oncall = schema("Oncall", { Severity, Incident, Escalation }, [
			contained(on(Incident, "severity"), on(Severity, "id")),
			contained(on(Escalation, "incident"), on(Incident, "id")),
			contained(on(Escalation, "severity"), on(Severity.where({ pages: true }), "id"))
		])

		const paged = query(Oncall).rule((r) => {
			const { incident: i, severity: s } = v(Escalation)
			return r.match(Escalation, { incident: i, severity: s }).match(Severity, { id: s, pages: true }).find({ i })
		})

		const { db } = await admit("r08-oncall", Oncall)
		assert.ok(db.prepare(paged))
	})

	test("9. ordered collections", async function r09() {
		const Playlist = relation("Playlist", { id: u64.fresh, name: str })
		const Extent = relation("Extent", { playlist: u64, span: interval(u64) })
		const Slot = relation("Slot", { playlist: u64, slot: interval(u64, 1n), track: str })

		const Playlists = schema("Playlists", { Playlist, Extent, Slot }, [
			contained(on(Extent, "playlist"), on(Playlist, "id")),
			contained(on(Slot, "playlist"), on(Playlist, "id")),
			key(Extent, ["playlist"]),
			key(Extent, ["playlist", "span"]),
			key(Slot, ["playlist", "slot"]),
			mirrors(on(Extent, ["playlist", "span"]), on(Slot, ["playlist", "slot"]))
		])

		const playingAt = query(Playlists).rule((r) => {
			const { slot, track } = v(Slot)
			return r
				.match(Slot, { playlist: r.param("list"), slot, track })
				.where(pointIn(r.param("pos"), slot))
				.find({ track })
		})

		const { db } = await admit("r09-playlists", Playlists)
		assert.ok(db.prepare(playingAt))
	})

	test("10. trees and ASTs", async function r10() {
		const Kind = closed("Kind", ["Lit", "Add"])
		const Node = relation("Node", { id: u64.fresh, kind: Kind.id })
		const Lit = relation("Lit", { node: u64, value: i64 })
		const Add = relation("Add", { node: u64, lhs: u64, rhs: u64 })
		const Parent = relation("Parent", { child: u64, parent: u64 })

		const Ast = schema("Ast", { Kind, Node, Lit, Add, Parent }, [
			contained(on(Node, "kind"), on(Kind, "id")),
			key(Lit, ["node"]),
			key(Add, ["node"]),
			mirrors(on(Node.where({ kind: "Lit" }), "id"), on(Lit, "node")),
			mirrors(on(Node.where({ kind: "Add" }), "id"), on(Add, "node")),
			contained(on(Add, "lhs"), on(Node, "id")),
			contained(on(Add, "rhs"), on(Node, "id")),
			key(Parent, ["child"]),
			contained(on(Parent, "child"), on(Node, "id")),
			contained(on(Parent, "parent"), on(Node, "id"))
		])

		const lhsLiteral = query(Ast).rule((r) => {
			const { lhs: l } = v(Add)
			const { value } = v(Lit)
			return r
				.match(Add, { node: r.param("n"), lhs: l })
				.match(Lit, { node: l, value })
				.find({ value })
		})

		const { db } = await admit("r10-ast", Ast)
		assert.ok(db.prepare(lhsLiteral))
	})

	test("11. typed graphs", async function r11() {
		const Person = relation("Person", { id: u64.fresh, name: str })
		const Repo = relation("Repo", { id: u64.fresh, name: str })
		const Follows = relation("Follows", { follower: u64, followee: u64 })
		const Maintains = relation("Maintains", { person: u64, repo: u64 })

		const Graph = schema("Graph", { Person, Repo, Follows, Maintains }, [
			contained(on(Follows, "follower"), on(Person, "id")),
			contained(on(Follows, "followee"), on(Person, "id")),
			key(Follows, ["follower", "followee"]),
			contained(on(Maintains, "person"), on(Person, "id")),
			contained(on(Maintains, "repo"), on(Repo, "id")),
			key(Maintains, ["person", "repo"])
		])

		const mutual = query(Graph).rule((r) => {
			const { follower: a, followee: b } = v(Follows)
			return r
				.match(Follows, { follower: a, followee: b })
				.match(Follows, { follower: b, followee: a })
				.where(lt(a, b))
				.find({ a, b })
		})

		const { db } = await admit("r11-graph", Graph)
		assert.ok(db.prepare(mutual))
	})

	test("12. entity-component", async function r12() {
		const Entity = relation("Entity", { id: u64.fresh, name: str })
		const Transform = relation("Transform", { entity: u64, x: i64, y: i64 })
		const Velocity = relation("Velocity", { entity: u64, dx: i64, dy: i64 })
		const Renderable = relation("Renderable", { entity: u64, mesh: str })

		const Ecs = schema("Ecs", { Entity, Transform, Velocity, Renderable }, [
			key(Transform, ["entity"]),
			contained(on(Transform, "entity"), on(Entity, "id")),
			key(Velocity, ["entity"]),
			contained(on(Velocity, "entity"), on(Entity, "id")),
			key(Renderable, ["entity"]),
			contained(on(Renderable, "entity"), on(Transform, "entity"))
		])

		const physics = query(Ecs).rule((r) => {
			const { entity, x, y } = v(Transform)
			const { dx, dy } = v(Velocity)
			return r.match(Transform, { entity, x, y }).match(Velocity, { entity, dx, dy }).find({ entity, x, y, dx, dy })
		})

		const { db } = await admit("r12-ecs", Ecs)
		assert.ok(db.prepare(physics))
	})

	test("13. state machines", async function r13() {
		const State = closed("State", ["Cart", "Placed", "Shipped"])
		const Order = relation("Order", { id: u64.fresh, state: State.id })
		const Placement = relation("Placement", { order: u64, at: i64 })
		const Shipment = relation("Shipment", { order: u64, carrier: str, at: i64 })

		const Orders = schema("Orders", { State, Order, Placement, Shipment }, [
			contained(on(Order, "state"), on(State, "id")),
			key(Placement, ["order"]),
			key(Shipment, ["order"]),
			contained(on(Placement, "order"), on(Order, "id")),
			mirrors(on(Shipment, "order"), on(Order.where({ state: "Shipped" }), "id"))
		])

		const shipped = query(Orders).rule((r) => {
			const { id } = v(Order)
			const { carrier } = v(Shipment)
			return r.match(Order, { id, state: "Shipped" }).match(Shipment, { order: id, carrier }).find({ id, carrier })
		})

		const { db } = await admit("r13-orders", Orders)
		assert.ok(db.prepare(shipped))
	})

	test("14. the calendar core", async function r14() {
		const Rsvp = closed("Rsvp", ["Accepted", "Tentative", "Declined"])
		const Arm = closed("Arm", ["Busy", "Ooo"])

		const Person = relation("Person", { id: u64.fresh, name: str })
		const Room = relation("Room", { id: u64.fresh, name: str })
		const Event = relation("Event", { id: u64.fresh, span: interval(i64) })
		const Attendance = relation("Attendance", {
			id: u64.fresh,
			event: u64,
			person: u64,
			rsvp: Rsvp.id
		})
		const Claim = relation("Claim", {
			source: u64,
			person: u64,
			arm: Arm.id,
			span: interval(i64)
		})
		const Booking = relation("Booking", { room: u64, event: u64, span: interval(i64) })
		const WorkHours = relation("WorkHours", { person: u64, hours: interval(i64) })

		const Calendar = schema("Calendar", { Rsvp, Arm, Person, Room, Event, Attendance, Claim, Booking, WorkHours }, [
			contained(on(Attendance, "event"), on(Event, "id")),
			contained(on(Attendance, "person"), on(Person, "id")),
			contained(on(Attendance, "rsvp"), on(Rsvp, "id")),
			key(Attendance, ["event", "person"]),
			key(Claim, ["source"]),
			contained(on(Claim, "person"), on(Person, "id")),
			contained(on(Claim, "arm"), on(Arm, "id")),
			key(Booking, ["room", "span"]),
			// The statement that TYPES Claim.source into "Attendance.id":
			mirrors(on(Attendance.where({ rsvp: "Accepted" }), "id"), on(Claim.where({ arm: "Busy" }), "source")),
			key(WorkHours, ["person", "hours"]),
			contained(on(Claim.where({ arm: "Busy" }), ["person", "span"]), on(WorkHours, ["person", "hours"])),
			contained(on(Booking, "room"), on(Room, "id")),
			contained(on(Booking, "event"), on(Event, "id"))
		])

		const roomConflicts = query(Calendar).rule((r) => {
			const { room, span } = v(Booking)
			return r
				.match(Booking, { room, span })
				.where(allen(span, ALLEN.intersects, r.param("want")))
				.find({ room, span })
		})
		const personLoad = query(Calendar).rule((r) => {
			const { person, span } = v(Claim)
			return r
				.match(Claim, { person, span })
				.where(allen(span, ALLEN.intersects, r.param("window")))
				.find({ person, span })
		})

		const { db } = await admit("r14-calendar", Calendar)
		assert.ok(db.prepare(roomConflicts))
		assert.ok(db.prepare(personLoad))
	})

	test("15. effective-dated configuration", async function r15() {
		const Policy = relation("Policy", { id: u64.fresh, live: interval(i64) })
		const Version = relation("Version", { policy: u64, rate_bps: i64, valid: interval(i64) })

		const Pricing = schema("Pricing", { Policy, Version }, [
			contained(on(Version, "policy"), on(Policy, "id")),
			key(Version, ["policy", "valid"]),
			contained(on(Policy, ["id", "live"]), on(Version, ["policy", "valid"]))
		])

		const inForce = query(Pricing).rule((r) => {
			const { rate_bps, valid } = v(Version)
			return r
				.match(Version, { policy: r.param("p"), rate_bps, valid })
				.where(pointIn(r.param("t"), valid))
				.find({ rate_bps })
		})
		const successions = query(Pricing).rule((r) => {
			const { policy: p, valid: a } = v(Version)
			const { valid: b } = v(Version)
			return r
				.match(Version, { policy: p, valid: a })
				.match(Version, { policy: p, valid: b })
				.where(allen(a, ALLEN.meets, b))
				.find({ a, b })
		})

		const { db } = await admit("r15-pricing", Pricing)
		assert.ok(db.prepare(inForce))
		assert.ok(db.prepare(successions))
	})

	test("16. disjoint covers", async function r16() {
		const FiscalYear = relation("FiscalYear", { id: u64.fresh, span: interval(i64) })
		const PayPeriod = relation("PayPeriod", { year: u64, seq: u64, span: interval(i64) })

		const Payroll = schema("Payroll", { FiscalYear, PayPeriod }, [
			contained(on(PayPeriod, "year"), on(FiscalYear, "id")),
			key(PayPeriod, ["year", "seq"]),
			key(PayPeriod, ["year", "span"]),
			contained(on(FiscalYear, ["id", "span"]), on(PayPeriod, ["year", "span"]))
		])

		const holding = query(Payroll).rule((r) => {
			const { seq, span } = v(PayPeriod)
			return r
				.match(PayPeriod, { year: r.param("y"), seq, span })
				.where(pointIn(r.param("t"), span))
				.find({ seq })
		})

		const { db } = await admit("r16-payroll", Payroll)
		assert.ok(db.prepare(holding))
	})

	test("17. federal income tax", async function r17() {
		const Status = closed("Status", ["Single", "MarriedJoint", "HeadOfHousehold"])
		const Regime = relation("Regime", { id: u64.fresh, year: i64, status: Status.id })
		const Bracket = relation("Bracket", { regime: u64, income: interval(i64), rate_bps: i64 })
		const Residency = relation("Residency", { person: u64, span: interval(i64) })
		const Earned = relation("Earned", { person: u64, regime: u64, span: interval(i64), minor: i64 })

		const Tax = schema("Tax", { Status, Regime, Bracket, Residency, Earned }, [
			contained(on(Regime, "status"), on(Status, "id")),
			key(Regime, ["year", "status"]),
			contained(on(Bracket, "regime"), on(Regime, "id")),
			key(Bracket, ["regime", "income"]),
			contained(on(Earned, "regime"), on(Regime, "id")),
			key(Residency, ["person", "span"]),
			contained(on(Earned, ["person", "span"]), on(Residency, ["person", "span"]))
		])

		const marginal = query(Tax).rule((r) => {
			const { id: reg } = v(Regime)
			const { income: b, rate_bps } = v(Bracket)
			return r
				.match(Regime, { id: reg, year: r.param("y"), status: r.param("s") })
				.match(Bracket, { regime: reg, income: b, rate_bps })
				.where(pointIn(r.param("taxable"), b))
				.find({ rate_bps })
		})

		const { db } = await admit("r17-tax", Tax)
		assert.ok(db.prepare(marginal))
	})

	test("18. free time and coalescing", async function r18() {
		const Person = relation("Person", { id: u64.fresh, name: str })
		const Claim = relation("Claim", { person: u64, span: interval(i64) })

		const FreeTime = schema("FreeTime", { Person, Claim }, [contained(on(Claim, "person"), on(Person, "id"))])

		const busy = query(FreeTime).rule((r) => {
			const { person, span } = v(Claim)
			return r.match(Claim, { person, span }).find({ person, packed: r.pack(span) })
		})
		const claimed = query(FreeTime).rule((r) => {
			const { person, span } = v(Claim)
			return r.match(Claim, { person, span }).find({ person, claimed: r.sum(r.duration(span)) })
		})

		const { db } = await admit("r18-freetime", FreeTime)
		assert.ok(db.prepare(busy))
		assert.ok(db.prepare(claimed))
	})

	test("19. the ledger", async function r19() {
		const Account = relation("Account", { id: u64.fresh, name: str })
		const JournalEntry = relation("JournalEntry", { id: u64.fresh, at: i64, memo: str })
		const Posting = relation("Posting", {
			id: u64.fresh,
			entry: u64,
			account: u64,
			minor: i64
		})

		const Ledger = schema("Ledger", { Account, JournalEntry, Posting }, [
			contained(on(Posting, "entry"), on(JournalEntry, "id")),
			contained(on(Posting, "account"), on(Account, "id"))
		])

		const balances = query(Ledger).rule((r) => {
			const { id, account, minor } = v(Posting)
			return r.match(Posting, { id, account, minor }).find({ account, balance: r.sum(minor) })
		})
		const doubleEntry = query(Ledger).rule((r) => {
			const { id, entry, minor } = v(Posting)
			return r.match(Posting, { id, entry, minor }).find({ entry, balance: r.sum(minor) })
		})

		const { db } = await admit("r19-ledger", Ledger)
		assert.ok(db.prepare(balances))
		assert.ok(db.prepare(doubleEntry))
	})

	test("20. conditional writes — the witnessed update-where runs", async function r20() {
		const State = closed("State", ["Queued", "Running", "Done"])
		const Job = relation("Job", { id: u64.fresh, state: State.id, payload: str })
		const Lease = relation("Lease", { job: u64, worker: u64, until: i64 })

		const Jobs = schema("Jobs", { State, Job, Lease }, [
			contained(on(Job, "state"), on(State, "id")),
			key(Lease, ["job"]),
			mirrors(on(Lease, "job"), on(Job.where({ state: "Running" }), "id"))
		])

		const stillQueued = query(Jobs).rule((r) => {
			const { id, payload } = v(Job)
			return r.match(Job, { id, state: "Queued", payload }).find({ id, payload })
		})

		const { db } = await admit("r20-jobs", Jobs)
		const prepared = db.prepare(stillQueued)

		// update-where, witnessed: query the premise on the attempt's snapshot,
		// then delete(old) + insert(new) per matched fact — "still Queued" is
		// the witness; the claim and its lease commit together (the mirrors).
		const outcome = db.writeWitnessed(function updateWhere(snap, tx) {
			const queued = snap.execute(prepared, {})
			if (queued.length === 0) {
				return abandon("nothing queued")
			}
			for (const row of queued) {
				tx.delete(Job, { id: row.id, state: "Queued", payload: row.payload })
				tx.insert(Job, { id: row.id, state: "Running", payload: row.payload })
				tx.insert(Lease, { job: row.id, worker: 7n, until: 60n })
			}
			return undefined
		})
		assert.ok(!outcome.ok, "the empty store has nothing queued — the loop abandons")
		assert.ok("abandoned" in outcome && outcome.abandoned === "nothing queued")
	})

	test("21. derived relations", async function r21() {
		const Arm = closed("Arm", ["Busy", "Ooo"])
		const Claim = relation("Claim", { source: u64, person: u64, arm: Arm.id, span: interval(i64) })
		const BusySpan = relation("BusySpan", { person: u64, span: interval(i64) })

		const Rollup = schema("Rollup", { Arm, Claim, BusySpan }, [
			contained(on(Claim, "arm"), on(Arm, "id")),
			key(Claim, ["source"]),
			key(Claim, ["person", "span"]),
			key(BusySpan, ["person", "span"]),
			contained(on(BusySpan, ["person", "span"]), on(Claim.where({ arm: "Busy" }), ["person", "span"]))
		])

		const deriving = query(Rollup).rule((r) => {
			const { person, span } = v(Claim)
			return r.match(Claim, { person, span, arm: "Busy" }).find({ person, packed: r.pack(span) })
		})

		const { db } = await admit("r21-rollup", Rollup)
		assert.ok(db.prepare(deriving))
	})

	test("22. union reads", async function r22() {
		const Kind = closed("Kind", ["Card", "Ach"])
		const Payment = relation("Payment", { id: u64.fresh, kind: Kind.id })
		const Card = relation("Card", { payment: u64, last4: u64 })
		const Ach = relation("Ach", { payment: u64, routing: u64 })

		const Payments = schema("Payments", { Kind, Payment, Card, Ach }, [
			contained(on(Payment, "kind"), on(Kind, "id")),
			key(Card, ["payment"]),
			key(Ach, ["payment"]),
			mirrors(on(Payment.where({ kind: "Card" }), "id"), on(Card, "payment")),
			mirrors(on(Payment.where({ kind: "Ach" }), "id"), on(Ach, "payment"))
		])

		const wholeDu = query(Payments)
			.rule((r) => {
				const { id } = v(Payment)
				const { last4: n } = v(Card)
				return r.match(Payment, { id, kind: "Card" }).match(Card, { payment: id, last4: n }).find({ id, n })
			})
			.rule((r) => {
				const { id } = v(Payment)
				const { routing: n } = v(Ach)
				return r.match(Payment, { id, kind: "Ach" }).match(Ach, { payment: id, routing: n }).find({ id, n })
			})

		const { db } = await admit("r22-payments", Payments)
		assert.ok(db.prepare(wholeDu))
	})

	test("23. the anti-recipes: five gravestones", async function r23() {
		const Step = relation("Step", { flow: u64, pos: u64, action: str })
		const Score = relation("Score", { subject: u64, bps: i64 })
		const ActiveRun = relation("ActiveRun", { student: u64, run: u64 })
		const Usage = relation("Usage", { meter: u64, period: u64, used: interval(i64) })
		const Event = relation("Event", { id: u64.fresh, at: i64 })

		const Gravestones = schema("Gravestones", { Step, Score, ActiveRun, Usage, Event }, [
			key(Step, ["flow", "pos"]),
			key(Score, ["subject"]),
			key(ActiveRun, ["student"]),
			key(Usage, ["meter", "used"])
		])

		await admit("r23-gravestones", Gravestones)
	})

	test("24. the closure idiom — both dialects agree, root for root", async function r24() {
		const Node = relation("Node", { id: u64.fresh, name: str })
		const Parent = relation("Parent", { child: u64, parent: u64 })

		const Closure = schema("Closure", { Node, Parent }, [
			key(Parent, ["child"]),
			contained(on(Parent, "child"), on(Node, "id")),
			contained(on(Parent, "parent"), on(Node, "id"))
		])

		// The loop's one query — the frontier's children, one ∈-set probe:
		const step = query(Closure).rule((r) => {
			const { child: c } = v(Parent)
			return r.match(Parent, { child: c, parent: r.inSet("frontier") }).find({ c })
		})
		// The same closure, one stratified program under the fixpoint driver
		// (?root seeds the predicate; the output is the finished set's own
		// identity projection — an idb atom is a positive occurrence, so it
		// grounds its variables and no re-grounding join exists):
		const reach = program(Closure, (p) => {
			const rec = p.rec("reach")
			const seeded = rec
				.rule((r) => {
					const { id: c } = v(Node)
					return r
						.match(Node, { id: c })
						.where(eq(c, r.param("root")))
						.find({ c })
				})
				.rule((r) => {
					const { child: c, parent } = v(Parent)
					return r.match(Parent, { child: c, parent }).idb(rec, { c: parent }).find({ c })
				})
			return p.output((r) => {
				const { id: c } = v(Node)
				return r.idb(seeded, { c }).find({ c })
			})
		})
		// The complement — negation OF the finished stratum is engine-legal
		// (the strata judge refuses only negation *through* a cycle):
		const unreached = program(Closure, (p) => {
			const rec = p.rec("reach")
			const seeded = rec
				.rule((r) => {
					const { id: c } = v(Node)
					return r
						.match(Node, { id: c })
						.where(eq(c, r.param("root")))
						.find({ c })
				})
				.rule((r) => {
					const { child: c, parent } = v(Parent)
					return r.match(Parent, { child: c, parent }).idb(rec, { c: parent }).find({ c })
				})
			return p.output((r) => {
				const { id: c } = v(Node)
				return r.match(Node, { id: c }).where(r.not(seeded, { c })).find({ c })
			})
		})

		const { db } = await admit("r24-closure", Closure)
		const stepPrepared = db.prepare(step)
		const reachPrepared = db.prepare(reach)
		const unreachedPrepared = db.prepare(unreached)

		const minted: { root?: bigint; mid?: bigint; leaf?: bigint } = {}
		const seededForest = db.write(function seed(tx) {
			minted.root = tx.insert(Node, { name: "root" }).id
			minted.mid = tx.insert(Node, { name: "mid" }).id
			minted.leaf = tx.insert(Node, { name: "leaf" }).id
			tx.insert(Node, { name: "lone" })
			tx.insert(Parent, { child: must(minted.mid), parent: must(minted.root) })
			tx.insert(Parent, { child: must(minted.leaf), parent: must(minted.mid) })
		})
		assert.ok(seededForest.ok, "the three-level forest lands")
		const root = must(minted.root)

		// The host loop — the frontier discipline IS semi-naive evaluation's Δ:
		const seen = new Set<bigint>([root])
		let frontier: readonly bigint[] = [root]
		for (;;) {
			const next = db.execute(stepPrepared, { frontier })
			const fresh = next
				.map((row) => row.c)
				.filter((c) => {
					return !seen.has(c)
				})
			if (fresh.length === 0) {
				break
			}
			for (const c of fresh) {
				seen.add(c)
			}
			frontier = fresh
		}

		const engineNative = db.execute(reachPrepared, { root }).map((row) => row.c)
		assert.deepEqual(sorted([...seen]), sorted(engineNative), "the two dialects agree, root for root")
		assert.deepEqual(sorted([...seen]), sorted([root, must(minted.mid), must(minted.leaf)]))

		// The complement lands in-plan: every node the closure never reached.
		const complement = db.execute(unreachedPrepared, { root }).map((row) => row.c)
		const everyNode = db.scan(Node).map((node) => node.id)
		assert.deepEqual(
			sorted(complement),
			sorted(everyNode.filter((id) => !seen.has(id))),
			"negation of the finished stratum answers the complement"
		)
	})

	test("25. the chart of accounts", async function r25() {
		const Account = relation("Account", { id: u64.fresh, name: str })
		const AccountParent = relation("AccountParent", { child: u64, parent: u64 })
		const Posting = relation("Posting", { id: u64.fresh, account: u64, minor: i64 })

		const Accounts = schema("Accounts", { Account, AccountParent, Posting }, [
			key(AccountParent, ["child"]),
			contained(on(AccountParent, "child"), on(Account, "id")),
			contained(on(AccountParent, "parent"), on(Account, "id")),
			contained(on(Posting, "account"), on(Account, "id"))
		])

		// The host composition's two queries (recipe 24's loop runs between them):
		const frontierStep = query(Accounts).rule((r) => {
			const { child: c } = v(AccountParent)
			return r.match(AccountParent, { child: c, parent: r.inSet("frontier") }).find({ c })
		})
		const subtreeRollup = query(Accounts).rule((r) => {
			const { id, minor } = v(Posting)
			return r.match(Posting, { id, account: r.inSet("subtree"), minor }).find({ total: r.sum(minor) })
		})
		// The engine-native form: the closure stratum converges first, then the
		// output's fold runs once over the finished subtree.
		const nativeRollup = program(Accounts, (p) => {
			const sub = p.rec("sub")
			const seeded = sub
				.rule((r) => {
					const { id: a } = v(Account)
					return r
						.match(Account, { id: a })
						.where(eq(a, r.param("root")))
						.find({ a })
				})
				.rule((r) => {
					const { child: a, parent } = v(AccountParent)
					return r.match(AccountParent, { child: a, parent }).idb(sub, { a: parent }).find({ a })
				})
			return p.output((r) => {
				const { id, account: a, minor } = v(Posting)
				return r
					.match(Posting, { id, account: a, minor })
					.idb(seeded, { a })
					.find({ total: r.sum(minor) })
			})
		})

		const { db } = await admit("r25-accounts", Accounts)
		assert.ok(db.prepare(frontierStep))
		assert.ok(db.prepare(subtreeRollup))
		assert.ok(db.prepare(nativeRollup))
	})

	test("26. exact partition", async function r26() {
		const Policy = relation("Policy", { id: u64.fresh, live: interval(i64) })
		const Version = relation("Version", { policy: u64, valid: interval(i64) })

		const ExactPartition = schema("ExactPartition", { Policy, Version }, [
			contained(on(Version, "policy"), on(Policy, "id")),
			key(Version, ["policy", "valid"]),
			key(Policy, ["id", "live"]),
			contained(on(Policy, ["id", "live"]), on(Version, ["policy", "valid"])),
			contained(on(Version, ["policy", "valid"]), on(Policy, ["id", "live"]))
		])

		await admit("r26-exact-partition", ExactPartition)
	})

	test("27. derived facts, maintained", async function r27() {
		const Arm = closed("Arm", ["Busy", "Ooo"])
		const Claim = relation("Claim", { source: u64, person: u64, arm: Arm.id, span: interval(i64) })
		const BusySpan = relation("BusySpan", { person: u64, span: interval(i64) })

		const MaintainedRollup = schema("MaintainedRollup", { Arm, Claim, BusySpan }, [
			contained(on(Claim, "arm"), on(Arm, "id")),
			key(Claim, ["source"]),
			key(Claim, ["person", "span"]),
			key(BusySpan, ["person", "span"]),
			contained(on(BusySpan, ["person", "span"]), on(Claim.where({ arm: "Busy" }), ["person", "span"]))
		])

		const deriving = query(MaintainedRollup).rule((r) => {
			const { source, person, span } = v(Claim)
			return r.match(Claim, { source, person, arm: "Busy", span }).find({ person, packed: r.pack(span) })
		})

		const { db } = await admit("r27-maintained-rollup", MaintainedRollup)
		assert.ok(db.prepare(deriving))
	})

	test("28. migration is ETL — two theories, two fingerprints", async function r28() {
		// The old theory, judged and fingerprinted:
		const EmployeeV1 = relation("Employee", { id: u64.fresh, name: str })
		const SalaryV1 = relation("Salary", { employee: u64, amount: i64 })
		const PayrollV1 = schema("PayrollV1", { Employee: EmployeeV1, Salary: SalaryV1 }, [
			contained(on(SalaryV1, "employee"), on(EmployeeV1, "id"))
		])

		// The new theory adds what v1 never recorded — WHEN a salary applied:
		const Employee = relation("Employee", { id: u64.fresh, name: str })
		const Salary = relation("Salary", { employee: u64, amount: i64, applies: interval(i64) })
		const Payroll = schema("Payroll", { Employee, Salary }, [
			contained(on(Salary, "employee"), on(Employee, "id")),
			key(Salary, ["employee", "applies"])
		])

		const inForceAt = query(Payroll).rule((r) => {
			const { id: e, name } = v(Employee)
			const { amount, applies: w } = v(Salary)
			return r
				.match(Employee, { id: e, name })
				.match(Salary, { employee: e, amount, applies: w })
				.where(pointIn(r.param("at"), w))
				.find({ name, amount })
		})

		const v1 = await admit("r28-payroll-v1", PayrollV1)
		const v2 = await admit("r28-payroll", Payroll)
		assert.notEqual(v1.fingerprint, v2.fingerprint, "a schema is a theory — the new dimension is a new fingerprint")
		assert.ok(v2.db.prepare(inForceAt))
	})

	test("29. the zone ledger", async function r29() {
		const Kind = closed("Kind", ["Unit", "Pair"])
		const Ledger = relation("Ledger", { id: u64.fresh, name: str })
		const Zone = relation("Zone", { ledger: u64, kind: Kind.id, at: interval(u64) })
		const UnitSlot = relation("UnitSlot", { ledger: u64, at: interval(u64, 1n), entry: u64 })
		const PairSlot = relation("PairSlot", { ledger: u64, at: interval(u64, 2n), entry: u64 })

		const ZoneLedger = schema("ZoneLedger", { Kind, Ledger, Zone, UnitSlot, PairSlot }, [
			contained(on(Zone, "ledger"), on(Ledger, "id")),
			contained(on(Zone, "kind"), on(Kind, "id")),
			key(Zone, ["ledger", "at"]),
			key(UnitSlot, ["ledger", "at"]),
			key(PairSlot, ["ledger", "at"]),
			mirrors(on(Zone.where({ kind: "Unit" }), ["ledger", "at"]), on(UnitSlot, ["ledger", "at"])),
			mirrors(on(Zone.where({ kind: "Pair" }), ["ledger", "at"]), on(PairSlot, ["ledger", "at"]))
		])

		await admit("r29-zone-ledger", ZoneLedger)
	})

	test("30. the keyed read — the key law made callable, on every scope", async function r30() {
		const Grp = relation("Grp", { id: u64.fresh, label: str })
		const Program = relation("Program", { id: u64.fresh, grp: u64, title: str })
		// The law: one program per group — hold the statement VALUE, it is
		// the read's selector (statement identity is the membership rule).
		const programGrpKey = key(Program, ["grp"])

		const KeyedRead = schema("KeyedRead", { Grp, Program }, [
			contained(on(Program, "grp"), on(Grp, "id")),
			programGrpKey
		])

		const { db } = await admit("r30-keyed-read", KeyedRead)

		const minted: { grp?: bigint; program?: bigint } = {}
		const seeded = db.write(function seed(tx) {
			const g = tx.insert(Grp, { label: "algebra" })
			const p = tx.insert(Program, { grp: g.id, title: "linear equations" })
			minted.grp = g.id
			minted.program = p.id
		})
		assert.ok(seeded.ok, "the seed commits")
		const grp = must(minted.grp)
		const program = must(minted.program)

		// db.get, 3-arg — the declared key statement selects the read:
		const byGroup = db.get(Program, programGrpKey, { grp })
		assert.ok(byGroup, "the declared key answers the typed point read")
		assert.equal(byGroup.id, program)
		assert.equal(byGroup.title, "linear equations")

		// The primary 2-arg form — the fresh field IS the primary key:
		const byId = db.get(Program, { id: program })
		assert.ok(byId, "the fresh field answers the primary point read")
		assert.equal(byId.grp, grp)

		// snap.get — the same spelling inside a read scope (the symmetry rule):
		assert.equal(
			db.read(function inScope(snap) {
				return snap.get(Program, programGrpKey, { grp })?.id
			}),
			program,
			"the read scope agrees with the standalone spelling"
		)

		// tx.get — the write transaction answers the FINAL state
		// (read-your-writes), through the key statement and the primary form:
		const mutated = db.write(function mutate(tx) {
			const g = tx.insert(Grp, { label: "geometry" })
			const p = tx.insert(Program, { grp: g.id, title: "proofs" })
			const pending = tx.get(Program, programGrpKey, { grp: g.id })
			assert.ok(pending, "the pending insert answers through the declared key")
			assert.equal(pending.id, p.id)
			assert.equal(tx.get(Program, { id: p.id })?.title, "proofs", "the primary form agrees pre-commit")
		})
		assert.ok(mutated.ok, "the keyed read-modify-write commits")
	})

	test("the goldens fixture pins exactly the 30 recipes, one line each", function goldensShape() {
		const expected: string[] = []
		for (let recipe = 1; recipe <= 30; recipe += 1) {
			expected.push(`r${String(recipe).padStart(2, "0")}`)
		}
		assert.deepEqual([...witnessed.keys()].sort(), expected, "every recipe admitted exactly one pinned theory")
		if (goldens !== undefined) {
			assert.deepEqual([...goldens.keys()].sort(), expected, "the fixture carries exactly one line per recipe")
		}
	})
})
