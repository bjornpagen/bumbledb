/**
 * The SDK cookbook's compile-and-run pin (PRD-S5). `ts/COOKBOOK.md` carries
 * the engine cookbook's 29 recipes (`bumbledb/docs/cookbook.md`) translated
 * to the structural API, and THIS file is what keeps them true: every
 * recipe's schema is constructed here through the public surface (so it
 * compiles, cast-free), admitted by the REAL engine (`dbCreate` — schema
 * validation is the engine's judgment, never re-implemented), its
 * fingerprint asserted stable across a reopen (the theory's identity is a
 * pure function of the schema), and every query snippet lowered through
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
import { after, describe, test } from "node:test"

import type { Schema, SchemaRelations } from "#index.ts"
import {
	ALLEN,
	abandon,
	bool,
	bytes,
	closed,
	contained,
	Db,
	i64,
	interval,
	key,
	lower,
	mirrors,
	none,
	on,
	oneOf,
	program,
	query,
	relation,
	schema,
	str,
	u64,
	window
} from "#index.ts"
import { native } from "#native.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-cookbook-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

/** One admitted recipe store: the open `Db` and the theory's engine-computed fingerprint. */
interface Admitted<Rels extends SchemaRelations> {
	readonly db: Db<Rels>
	readonly fingerprint: string
}

/**
 * Admits one recipe's theory against the real engine and pins its identity:
 * create (the engine's schema validation is the acceptance judgment), read
 * the fingerprint, close, reopen under the identical theory (the
 * fingerprint gate passes and reads the same hex back — stability), then
 * open through the public `Db` surface for the recipe's query lowering.
 */
async function admit<Rels extends SchemaRelations>(name: string, theory: Schema<Rels>): Promise<Admitted<Rels>> {
	const dir = path.join(tmpRoot, name)
	const spec = lower(theory)
	const created = native.dbCreate(dir, spec)
	assert.ok(created.ok, `${name}: the engine admits the theory`)
	const fingerprint = native.dbFingerprint(created.db)
	native.dbClose(created.db)
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
		const ServiceId = u64.as("ServiceId")

		const Service = relation("Service", { id: ServiceId.fresh, name: str })
		const Outage = relation("Outage", { service: ServiceId, window: interval(i64) })

		const Uptime = schema("Uptime", { Service, Outage }, [
			contained(on(Outage, "service"), on(Service, "id")),
			key(Outage, ["service", "window"])
		])

		const downAt = query(Uptime).rule((r) =>
			r
				.match(Outage, { service: r.var("service"), window: r.var("w") })
				.where(r.pointIn(r.param("t"), r.var("w")))
				.select("service")
		)
		const overlapping = query(Uptime).rule((r) =>
			r
				.match(Outage, { service: r.var("service"), window: r.var("w") })
				.where(r.allen(r.var("w"), ALLEN.intersects, r.param("incident")))
				.select("service", "w")
		)
		const downtime = query(Uptime).rule((r) =>
			r.match(Outage, { service: r.var("service"), window: r.var("w") }).select("service", r.sum(r.duration("w")))
		)

		const { db } = await admit("r01-uptime", Uptime)
		assert.ok(db.prepare(downAt))
		assert.ok(db.prepare(overlapping))
		assert.ok(db.prepare(downtime))
	})

	test("2. discriminated unions", async function r02() {
		const TaskId = u64.as("TaskId")

		const Kind = closed("Kind", ["Deterministic", "CustomOperator"])
		const Task = relation("Task", { id: TaskId.fresh, kind: Kind.id })
		const DeterministicGrading = relation("DeterministicGrading", { task: TaskId, tolerance: i64 })
		const CustomOperatorGrading = relation("CustomOperatorGrading", { task: TaskId, operator: str })

		const Grading = schema("Grading", { Kind, Task, DeterministicGrading, CustomOperatorGrading }, [
			contained(on(Task, "kind"), on(Kind, "id")),
			key(DeterministicGrading, ["task"]),
			key(CustomOperatorGrading, ["task"]),
			mirrors(on(Task.where({ kind: Kind.Deterministic }), "id"), on(DeterministicGrading, "task")),
			mirrors(on(Task.where({ kind: Kind.CustomOperator }), "id"), on(CustomOperatorGrading, "task"))
		])

		await admit("r02-grading", Grading)
	})

	test("3. 0..1 optional attributes", async function r03() {
		const BusinessId = u64.as("BusinessId")

		const Business = relation("Business", { id: BusinessId.fresh, name: str })
		const MailingAddress = relation("MailingAddress", { business: BusinessId, line: str, city: str })

		const Optionality = schema("Optionality", { Business, MailingAddress }, [
			key(MailingAddress, ["business"]),
			contained(on(MailingAddress, "business"), on(Business, "id"))
		])

		const unaddressed = query(Optionality).rule((r) =>
			r
				.match(Business, { id: r.var("b") })
				.where(r.not(MailingAddress, { business: r.var("b") }))
				.select("b")
		)

		const { db } = await admit("r03-optionality", Optionality)
		assert.ok(db.prepare(unaddressed))
	})

	test("4. money", async function r04() {
		const AccountId = u64.as("AccountId")
		const PostingId = u64.as("PostingId")
		const Minor = i64.as("Minor")

		const Currency = closed("Currency", ["Usd", "Eur", "Gbp"])
		const Account = relation("Account", { id: AccountId.fresh, name: str })
		const Posting = relation("Posting", {
			id: PostingId.fresh,
			account: AccountId,
			currency: Currency.id,
			minor: Minor
		})

		const Money = schema("Money", { Currency, Account, Posting }, [
			contained(on(Posting, "account"), on(Account, "id")),
			contained(on(Posting, "currency"), on(Currency, "id"))
		])

		const totals = query(Money).rule((r) =>
			r
				.match(Posting, {
					id: r.var("id"),
					account: r.var("account"),
					currency: r.var("currency"),
					minor: r.var("minor")
				})
				.select("account", "currency", r.sum("minor"))
		)

		const { db } = await admit("r04-money", Money)
		assert.ok(db.prepare(totals))
	})

	test("5. content addressing", async function r05() {
		const DocumentId = u64.as("DocumentId")
		const PayloadHash = bytes(32).as("PayloadHash")

		const Region = closed("Region", ["Us", "Eu"])
		const Document = relation("Document", { id: DocumentId.fresh, name: str, payload: PayloadHash })
		const Replica = relation("Replica", { payload: PayloadHash, region: Region.id })

		const Content = schema("Content", { Region, Document, Replica }, [
			key(Document, ["payload"]),
			contained(on(Replica, "payload"), on(Document, "payload")),
			contained(on(Replica, "region"), on(Region, "id"))
		])

		const byDigest = query(Content).rule((r) =>
			r.match(Document, { id: r.var("id"), payload: r.param("digest") }).select("id")
		)

		const { db } = await admit("r05-content", Content)
		assert.ok(db.prepare(byDigest))
	})

	test("6. the vocabulary", async function r06() {
		const TicketId = u64.as("TicketId")

		const Priority = closed("Priority", ["Low", "Normal", "Urgent"])
		const Ticket = relation("Ticket", { id: TicketId.fresh, priority: Priority.id, opened_at: i64 })

		const Tickets = schema("Tickets", { Priority, Ticket }, [contained(on(Ticket, "priority"), on(Priority, "id"))])

		const urgent = query(Tickets).rule((r) =>
			r.match(Ticket, { id: r.var("t"), priority: Priority.Urgent }).select("t")
		)

		const { db } = await admit("r06-tickets", Tickets)
		assert.ok(db.prepare(urgent))
	})

	test("7. the classification", async function r07() {
		const AttemptId = u64.as("AttemptId")

		const Kind = closed("Kind", { mastered: bool, rank: u64 })({
			DirectPass: { mastered: true, rank: 30n },
			JudgedPass: { mastered: true, rank: 20n },
			Failed: { mastered: false, rank: 10n }
		})
		const Attempt = relation("Attempt", { id: AttemptId.fresh, kind: Kind.id })
		const Certificate = relation("Certificate", { attempt: AttemptId, kind: Kind.id })

		const Review = schema("Review", { Kind, Attempt, Certificate }, [
			contained(on(Attempt, "kind"), on(Kind, "id")),
			key(Certificate, ["attempt"]),
			contained(on(Certificate, "attempt"), on(Attempt, "id")),
			contained(on(Certificate, "kind"), on(Kind, "id")),
			window(on(Attempt, "id"), none, on(Certificate.where({ kind: Kind.Failed }), "attempt"))
		])

		const masteredAttempts = query(Review)
			.rule((r) => r.match(Attempt, { id: r.var("a"), kind: Kind.DirectPass }).select("a"))
			.rule((r) => r.match(Attempt, { id: r.var("a"), kind: Kind.JudgedPass }).select("a"))

		const { db } = await admit("r07-review", Review)
		assert.ok(db.prepare(masteredAttempts))
	})

	test("8. the sub-vocabulary", async function r08() {
		const IncidentId = u64.as("IncidentId")

		const Severity = closed("Severity", { pages: bool })({
			Info: { pages: false },
			Warning: { pages: false },
			Critical: { pages: true },
			Fatal: { pages: true }
		})
		const Incident = relation("Incident", { id: IncidentId.fresh, severity: Severity.id })
		const Escalation = relation("Escalation", { incident: IncidentId, severity: Severity.id, at: i64 })

		const Oncall = schema("Oncall", { Severity, Incident, Escalation }, [
			contained(on(Incident, "severity"), on(Severity, "id")),
			contained(on(Escalation, "incident"), on(Incident, "id")),
			contained(on(Escalation, "severity"), on(Severity, "id")),
			window(
				on(Incident, "id"),
				none,
				on(Escalation.where({ severity: oneOf(Severity.Info, Severity.Warning) }), "incident")
			)
		])

		const paged = query(Oncall)
			.rule((r) => r.match(Escalation, { incident: r.var("i"), severity: Severity.Critical }).select("i"))
			.rule((r) => r.match(Escalation, { incident: r.var("i"), severity: Severity.Fatal }).select("i"))

		const { db } = await admit("r08-oncall", Oncall)
		assert.ok(db.prepare(paged))
	})

	test("9. ordered collections", async function r09() {
		const PlaylistId = u64.as("PlaylistId")

		const Playlist = relation("Playlist", { id: PlaylistId.fresh, name: str })
		const Extent = relation("Extent", { playlist: PlaylistId, span: interval(u64) })
		const Slot = relation("Slot", { playlist: PlaylistId, slot: interval(u64, 1n), track: str })

		const Playlists = schema("Playlists", { Playlist, Extent, Slot }, [
			contained(on(Extent, "playlist"), on(Playlist, "id")),
			contained(on(Slot, "playlist"), on(Playlist, "id")),
			key(Extent, ["playlist"]),
			key(Extent, ["playlist", "span"]),
			key(Slot, ["playlist", "slot"]),
			mirrors(on(Extent, ["playlist", "span"]), on(Slot, ["playlist", "slot"]))
		])

		const playingAt = query(Playlists).rule((r) =>
			r
				.match(Slot, { playlist: r.param("list"), slot: r.var("s"), track: r.var("track") })
				.where(r.pointIn(r.param("pos"), r.var("s")))
				.select("track")
		)

		const { db } = await admit("r09-playlists", Playlists)
		assert.ok(db.prepare(playingAt))
	})

	test("10. trees and ASTs", async function r10() {
		const NodeId = u64.as("NodeId")

		const Kind = closed("Kind", ["Lit", "Add"])
		const Node = relation("Node", { id: NodeId.fresh, kind: Kind.id })
		const Lit = relation("Lit", { node: NodeId, value: i64 })
		const Add = relation("Add", { node: NodeId, lhs: NodeId, rhs: NodeId })
		const Parent = relation("Parent", { child: NodeId, parent: NodeId })

		const Ast = schema("Ast", { Kind, Node, Lit, Add, Parent }, [
			contained(on(Node, "kind"), on(Kind, "id")),
			key(Lit, ["node"]),
			key(Add, ["node"]),
			mirrors(on(Node.where({ kind: Kind.Lit }), "id"), on(Lit, "node")),
			mirrors(on(Node.where({ kind: Kind.Add }), "id"), on(Add, "node")),
			contained(on(Add, "lhs"), on(Node, "id")),
			contained(on(Add, "rhs"), on(Node, "id")),
			key(Parent, ["child"]),
			contained(on(Parent, "child"), on(Node, "id")),
			contained(on(Parent, "parent"), on(Node, "id"))
		])

		const lhsLiteral = query(Ast).rule((r) =>
			r
				.match(Add, { node: r.param("n"), lhs: r.var("l") })
				.match(Lit, { node: r.var("l"), value: r.var("v") })
				.select("v")
		)

		const { db } = await admit("r10-ast", Ast)
		assert.ok(db.prepare(lhsLiteral))
	})

	test("11. typed graphs", async function r11() {
		const PersonId = u64.as("PersonId")
		const RepoId = u64.as("RepoId")

		const Person = relation("Person", { id: PersonId.fresh, name: str })
		const Repo = relation("Repo", { id: RepoId.fresh, name: str })
		const Follows = relation("Follows", { follower: PersonId, followee: PersonId })
		const Maintains = relation("Maintains", { person: PersonId, repo: RepoId })

		const Graph = schema("Graph", { Person, Repo, Follows, Maintains }, [
			contained(on(Follows, "follower"), on(Person, "id")),
			contained(on(Follows, "followee"), on(Person, "id")),
			key(Follows, ["follower", "followee"]),
			contained(on(Maintains, "person"), on(Person, "id")),
			contained(on(Maintains, "repo"), on(Repo, "id")),
			key(Maintains, ["person", "repo"])
		])

		const mutual = query(Graph).rule((r) =>
			r
				.match(Follows, { follower: r.var("a"), followee: r.var("b") })
				.match(Follows, { follower: r.var("b"), followee: r.var("a") })
				.where(r.lt(r.var("a"), r.var("b")))
				.select("a", "b")
		)

		const { db } = await admit("r11-graph", Graph)
		assert.ok(db.prepare(mutual))
	})

	test("12. entity-component", async function r12() {
		const EntityId = u64.as("EntityId")

		const Entity = relation("Entity", { id: EntityId.fresh, name: str })
		const Transform = relation("Transform", { entity: EntityId, x: i64, y: i64 })
		const Velocity = relation("Velocity", { entity: EntityId, dx: i64, dy: i64 })
		const Renderable = relation("Renderable", { entity: EntityId, mesh: str })

		const Ecs = schema("Ecs", { Entity, Transform, Velocity, Renderable }, [
			key(Transform, ["entity"]),
			contained(on(Transform, "entity"), on(Entity, "id")),
			key(Velocity, ["entity"]),
			contained(on(Velocity, "entity"), on(Entity, "id")),
			key(Renderable, ["entity"]),
			contained(on(Renderable, "entity"), on(Transform, "entity"))
		])

		const physics = query(Ecs).rule((r) =>
			r
				.match(Transform, { entity: r.var("e"), x: r.var("x"), y: r.var("y") })
				.match(Velocity, { entity: r.var("e"), dx: r.var("dx"), dy: r.var("dy") })
				.select("e", "x", "y", "dx", "dy")
		)

		const { db } = await admit("r12-ecs", Ecs)
		assert.ok(db.prepare(physics))
	})

	test("13. state machines", async function r13() {
		const OrderId = u64.as("OrderId")

		const State = closed("State", ["Cart", "Placed", "Shipped"])
		const Order = relation("Order", { id: OrderId.fresh, state: State.id })
		const Placement = relation("Placement", { order: OrderId, at: i64 })
		const Shipment = relation("Shipment", { order: OrderId, carrier: str, at: i64 })

		const Orders = schema("Orders", { State, Order, Placement, Shipment }, [
			contained(on(Order, "state"), on(State, "id")),
			key(Placement, ["order"]),
			key(Shipment, ["order"]),
			contained(on(Placement, "order"), on(Order, "id")),
			mirrors(on(Shipment, "order"), on(Order.where({ state: State.Shipped }), "id"))
		])

		const shipped = query(Orders).rule((r) =>
			r
				.match(Order, { id: r.var("id"), state: State.Shipped })
				.match(Shipment, { order: r.var("id"), carrier: r.var("carrier") })
				.select("id", "carrier")
		)

		const { db } = await admit("r13-orders", Orders)
		assert.ok(db.prepare(shipped))
	})

	test("14. the calendar core", async function r14() {
		const PersonId = u64.as("PersonId")
		const RoomId = u64.as("RoomId")
		const EventId = u64.as("EventId")
		const AttendanceId = u64.as("AttendanceId")

		const Rsvp = closed("Rsvp", ["Accepted", "Tentative", "Declined"])
		const Arm = closed("Arm", ["Busy", "Ooo"])

		const Person = relation("Person", { id: PersonId.fresh, name: str })
		const Room = relation("Room", { id: RoomId.fresh, name: str })
		const Event = relation("Event", { id: EventId.fresh, span: interval(i64) })
		const Attendance = relation("Attendance", {
			id: AttendanceId.fresh,
			event: EventId,
			person: PersonId,
			rsvp: Rsvp.id
		})
		const Claim = relation("Claim", {
			source: u64.as("AttendanceId"),
			person: PersonId,
			arm: Arm.id,
			span: interval(i64)
		})
		const Booking = relation("Booking", { room: RoomId, event: EventId, span: interval(i64) })
		const WorkHours = relation("WorkHours", { person: PersonId, hours: interval(i64) })

		const Calendar = schema("Calendar", { Rsvp, Arm, Person, Room, Event, Attendance, Claim, Booking, WorkHours }, [
			contained(on(Attendance, "event"), on(Event, "id")),
			contained(on(Attendance, "person"), on(Person, "id")),
			contained(on(Attendance, "rsvp"), on(Rsvp, "id")),
			key(Attendance, ["event", "person"]),
			key(Claim, ["source"]),
			contained(on(Claim, "person"), on(Person, "id")),
			contained(on(Claim, "arm"), on(Arm, "id")),
			key(Booking, ["room", "span"]),
			mirrors(on(Attendance.where({ rsvp: Rsvp.Accepted }), "id"), on(Claim.where({ arm: Arm.Busy }), "source")),
			key(WorkHours, ["person", "hours"]),
			contained(on(Claim.where({ arm: Arm.Busy }), ["person", "span"]), on(WorkHours, ["person", "hours"])),
			contained(on(Booking, "room"), on(Room, "id")),
			contained(on(Booking, "event"), on(Event, "id"))
		])

		const roomConflicts = query(Calendar).rule((r) =>
			r
				.match(Booking, { room: r.var("room"), span: r.var("s") })
				.where(r.allen(r.var("s"), ALLEN.intersects, r.param("want")))
				.select("room", "s")
		)
		const personLoad = query(Calendar).rule((r) =>
			r
				.match(Claim, { person: r.var("person"), span: r.var("s") })
				.where(r.allen(r.var("s"), ALLEN.intersects, r.param("window")))
				.select("person", "s")
		)

		const { db } = await admit("r14-calendar", Calendar)
		assert.ok(db.prepare(roomConflicts))
		assert.ok(db.prepare(personLoad))
	})

	test("15. effective-dated configuration", async function r15() {
		const PolicyId = u64.as("PolicyId")

		const Policy = relation("Policy", { id: PolicyId.fresh, live: interval(i64) })
		const Version = relation("Version", { policy: PolicyId, rate_bps: i64, valid: interval(i64) })

		const Pricing = schema("Pricing", { Policy, Version }, [
			contained(on(Version, "policy"), on(Policy, "id")),
			key(Version, ["policy", "valid"]),
			contained(on(Policy, ["id", "live"]), on(Version, ["policy", "valid"]))
		])

		const inForce = query(Pricing).rule((r) =>
			r
				.match(Version, { policy: r.param("p"), rate_bps: r.var("rate_bps"), valid: r.var("v") })
				.where(r.pointIn(r.param("t"), r.var("v")))
				.select("rate_bps")
		)
		const successions = query(Pricing).rule((r) =>
			r
				.match(Version, { policy: r.var("p"), valid: r.var("a") })
				.match(Version, { policy: r.var("p"), valid: r.var("b") })
				.where(r.allen(r.var("a"), ALLEN.meets, r.var("b")))
				.select("a", "b")
		)

		const { db } = await admit("r15-pricing", Pricing)
		assert.ok(db.prepare(inForce))
		assert.ok(db.prepare(successions))
	})

	test("16. disjoint covers", async function r16() {
		const FiscalYearId = u64.as("FiscalYearId")

		const FiscalYear = relation("FiscalYear", { id: FiscalYearId.fresh, span: interval(i64) })
		const PayPeriod = relation("PayPeriod", { year: FiscalYearId, seq: u64, span: interval(i64) })

		const Payroll = schema("Payroll", { FiscalYear, PayPeriod }, [
			contained(on(PayPeriod, "year"), on(FiscalYear, "id")),
			key(PayPeriod, ["year", "seq"]),
			key(PayPeriod, ["year", "span"]),
			contained(on(FiscalYear, ["id", "span"]), on(PayPeriod, ["year", "span"]))
		])

		const holding = query(Payroll).rule((r) =>
			r
				.match(PayPeriod, { year: r.param("y"), seq: r.var("seq"), span: r.var("s") })
				.where(r.pointIn(r.param("t"), r.var("s")))
				.select("seq")
		)

		const { db } = await admit("r16-payroll", Payroll)
		assert.ok(db.prepare(holding))
	})

	test("17. federal income tax", async function r17() {
		const RegimeId = u64.as("RegimeId")

		const Status = closed("Status", ["Single", "MarriedJoint", "HeadOfHousehold"])
		const Regime = relation("Regime", { id: RegimeId.fresh, year: i64, status: Status.id })
		const Bracket = relation("Bracket", { regime: RegimeId, income: interval(i64), rate_bps: i64 })
		const Residency = relation("Residency", { person: u64, span: interval(i64) })
		const Earned = relation("Earned", { person: u64, regime: RegimeId, span: interval(i64), minor: i64 })

		const Tax = schema("Tax", { Status, Regime, Bracket, Residency, Earned }, [
			contained(on(Regime, "status"), on(Status, "id")),
			key(Regime, ["year", "status"]),
			contained(on(Bracket, "regime"), on(Regime, "id")),
			key(Bracket, ["regime", "income"]),
			contained(on(Earned, "regime"), on(Regime, "id")),
			key(Residency, ["person", "span"]),
			contained(on(Earned, ["person", "span"]), on(Residency, ["person", "span"]))
		])

		const marginal = query(Tax).rule((r) =>
			r
				.match(Regime, { id: r.var("reg"), year: r.param("y"), status: r.param("s") })
				.match(Bracket, { regime: r.var("reg"), income: r.var("b"), rate_bps: r.var("rate_bps") })
				.where(r.pointIn(r.param("taxable"), r.var("b")))
				.select("rate_bps")
		)

		const { db } = await admit("r17-tax", Tax)
		assert.ok(db.prepare(marginal))
	})

	test("18. free time and coalescing", async function r18() {
		const PersonId = u64.as("PersonId")

		const Person = relation("Person", { id: PersonId.fresh, name: str })
		const Claim = relation("Claim", { person: PersonId, span: interval(i64) })

		const FreeTime = schema("FreeTime", { Person, Claim }, [contained(on(Claim, "person"), on(Person, "id"))])

		const busy = query(FreeTime).rule((r) =>
			r.match(Claim, { person: r.var("person"), span: r.var("span") }).select("person", r.pack("span"))
		)
		const claimed = query(FreeTime).rule((r) =>
			r.match(Claim, { person: r.var("person"), span: r.var("span") }).select("person", r.sum(r.duration("span")))
		)

		const { db } = await admit("r18-freetime", FreeTime)
		assert.ok(db.prepare(busy))
		assert.ok(db.prepare(claimed))
	})

	test("19. the ledger", async function r19() {
		const AccountId = u64.as("AccountId")
		const JournalEntryId = u64.as("JournalEntryId")
		const PostingId = u64.as("PostingId")

		const Account = relation("Account", { id: AccountId.fresh, name: str })
		const JournalEntry = relation("JournalEntry", { id: JournalEntryId.fresh, at: i64, memo: str })
		const Posting = relation("Posting", {
			id: PostingId.fresh,
			entry: JournalEntryId,
			account: AccountId,
			minor: i64
		})

		const Ledger = schema("Ledger", { Account, JournalEntry, Posting }, [
			contained(on(Posting, "entry"), on(JournalEntry, "id")),
			contained(on(Posting, "account"), on(Account, "id"))
		])

		const balances = query(Ledger).rule((r) =>
			r
				.match(Posting, { id: r.var("id"), account: r.var("account"), minor: r.var("minor") })
				.select("account", r.sum("minor"))
		)
		const doubleEntry = query(Ledger).rule((r) =>
			r
				.match(Posting, { id: r.var("id"), entry: r.var("entry"), minor: r.var("minor") })
				.select("entry", r.sum("minor"))
		)

		const { db } = await admit("r19-ledger", Ledger)
		assert.ok(db.prepare(balances))
		assert.ok(db.prepare(doubleEntry))
	})

	test("20. conditional writes — the witnessed update-where runs", async function r20() {
		const JobId = u64.as("JobId")

		const State = closed("State", ["Queued", "Running", "Done"])
		const Job = relation("Job", { id: JobId.fresh, state: State.id, payload: str })
		const Lease = relation("Lease", { job: JobId, worker: u64, until: i64 })

		const Jobs = schema("Jobs", { State, Job, Lease }, [
			contained(on(Job, "state"), on(State, "id")),
			key(Lease, ["job"]),
			mirrors(on(Lease, "job"), on(Job.where({ state: State.Running }), "id"))
		])

		const stillQueued = query(Jobs).rule((r) =>
			r.match(Job, { id: r.var("id"), state: State.Queued, payload: r.var("payload") }).select("id", "payload")
		)

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
				tx.delete(Job, { id: row.id, state: State.Queued, payload: row.payload })
				tx.insert(Job, { id: row.id, state: State.Running, payload: row.payload })
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
			contained(on(BusySpan, ["person", "span"]), on(Claim.where({ arm: Arm.Busy }), ["person", "span"]))
		])

		const deriving = query(Rollup).rule((r) =>
			r.match(Claim, { person: r.var("person"), span: r.var("span"), arm: Arm.Busy }).select("person", r.pack("span"))
		)

		const { db } = await admit("r21-rollup", Rollup)
		assert.ok(db.prepare(deriving))
	})

	test("22. union reads", async function r22() {
		const PaymentId = u64.as("PaymentId")

		const Kind = closed("Kind", ["Card", "Ach"])
		const Payment = relation("Payment", { id: PaymentId.fresh, kind: Kind.id })
		const Card = relation("Card", { payment: PaymentId, last4: u64 })
		const Ach = relation("Ach", { payment: PaymentId, routing: u64 })

		const Payments = schema("Payments", { Kind, Payment, Card, Ach }, [
			contained(on(Payment, "kind"), on(Kind, "id")),
			key(Card, ["payment"]),
			key(Ach, ["payment"]),
			mirrors(on(Payment.where({ kind: Kind.Card }), "id"), on(Card, "payment")),
			mirrors(on(Payment.where({ kind: Kind.Ach }), "id"), on(Ach, "payment"))
		])

		const wholeDu = query(Payments)
			.rule((r) =>
				r
					.match(Payment, { id: r.var("id"), kind: Kind.Card })
					.match(Card, { payment: r.var("id"), last4: r.var("n") })
					.select("id", "n")
			)
			.rule((r) =>
				r
					.match(Payment, { id: r.var("id"), kind: Kind.Ach })
					.match(Ach, { payment: r.var("id"), routing: r.var("n") })
					.select("id", "n")
			)

		const { db } = await admit("r22-payments", Payments)
		assert.ok(db.prepare(wholeDu))
	})

	test("23. the anti-recipes: five gravestones", async function r23() {
		const GravestoneEventId = u64.as("GravestoneEventId")

		const Step = relation("Step", { flow: u64, pos: u64, action: str })
		const Score = relation("Score", { subject: u64, bps: i64 })
		const ActiveRun = relation("ActiveRun", { student: u64, run: u64 })
		const Usage = relation("Usage", { meter: u64, period: u64, used: interval(i64) })
		const Event = relation("Event", { id: GravestoneEventId.fresh, at: i64 })

		const Gravestones = schema("Gravestones", { Step, Score, ActiveRun, Usage, Event }, [
			key(Step, ["flow", "pos"]),
			key(Score, ["subject"]),
			key(ActiveRun, ["student"]),
			key(Usage, ["meter", "used"])
		])

		await admit("r23-gravestones", Gravestones)
	})

	test("24. the closure idiom — both dialects agree, root for root", async function r24() {
		const NodeId = u64.as("NodeId")

		const Node = relation("Node", { id: NodeId.fresh, name: str })
		const Parent = relation("Parent", { child: NodeId, parent: NodeId })

		const Closure = schema("Closure", { Node, Parent }, [
			key(Parent, ["child"]),
			contained(on(Parent, "child"), on(Node, "id")),
			contained(on(Parent, "parent"), on(Node, "id"))
		])

		// The loop's one query — the frontier's children, one ∈-set probe:
		const step = query(Closure).rule((r) =>
			r.match(Parent, { child: r.var("c"), parent: r.inSet("frontier") }).select("c")
		)
		// The same closure, one stratified program under the fixpoint driver
		// (?root seeds the predicate; the output joins the finished set back
		// through the theory's own domain relation — an idb atom is a join
		// position, so the head rides the Node atom):
		const reach = program(Closure, (p) => {
			const rec = p.rec("reach")
			const seeded = rec
				.rule((r) =>
					r
						.match(Node, { id: r.var("c") })
						.where(r.eq(r.var("c"), r.param("root")))
						.select("c")
				)
				.rule((r) =>
					r
						.match(Parent, { child: r.var("c"), parent: r.var("m") })
						.idb(rec, r.var("m"))
						.select("c")
				)
			return p.output((r) =>
				r
					.match(Node, { id: r.var("c") })
					.idb(seeded, r.var("c"))
					.select("c")
			)
		})

		const { db } = await admit("r24-closure", Closure)
		const stepPrepared = db.prepare(step)
		const reachPrepared = db.prepare(reach)

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
	})

	test("25. the chart of accounts", async function r25() {
		const AccountId = u64.as("AccountId")
		const PostingId = u64.as("PostingId")

		const Account = relation("Account", { id: AccountId.fresh, name: str })
		const AccountParent = relation("AccountParent", { child: AccountId, parent: AccountId })
		const Posting = relation("Posting", { id: PostingId.fresh, account: AccountId, minor: i64 })

		const Accounts = schema("Accounts", { Account, AccountParent, Posting }, [
			key(AccountParent, ["child"]),
			contained(on(AccountParent, "child"), on(Account, "id")),
			contained(on(AccountParent, "parent"), on(Account, "id")),
			contained(on(Posting, "account"), on(Account, "id"))
		])

		// The host composition's two queries (recipe 24's loop runs between them):
		const frontierStep = query(Accounts).rule((r) =>
			r.match(AccountParent, { child: r.var("c"), parent: r.inSet("frontier") }).select("c")
		)
		const subtreeRollup = query(Accounts).rule((r) =>
			r.match(Posting, { id: r.var("id"), account: r.inSet("subtree"), minor: r.var("minor") }).select(r.sum("minor"))
		)
		// The engine-native form: the closure stratum converges first, then the
		// output's fold runs once over the finished subtree.
		const nativeRollup = program(Accounts, (p) => {
			const sub = p.rec("sub")
			const seeded = sub
				.rule((r) =>
					r
						.match(Account, { id: r.var("a") })
						.where(r.eq(r.var("a"), r.param("root")))
						.select("a")
				)
				.rule((r) =>
					r
						.match(AccountParent, { child: r.var("a"), parent: r.var("p") })
						.idb(sub, r.var("p"))
						.select("a")
				)
			return p.output((r) =>
				r
					.match(Posting, { id: r.var("id"), account: r.var("a"), minor: r.var("minor") })
					.idb(seeded, r.var("a"))
					.select(r.sum("minor"))
			)
		})

		const { db } = await admit("r25-accounts", Accounts)
		assert.ok(db.prepare(frontierStep))
		assert.ok(db.prepare(subtreeRollup))
		assert.ok(db.prepare(nativeRollup))
	})

	test("26. exact partition", async function r26() {
		const PolicyId = u64.as("PolicyId")

		const Policy = relation("Policy", { id: PolicyId.fresh, live: interval(i64) })
		const Version = relation("Version", { policy: PolicyId, valid: interval(i64) })

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
			contained(on(BusySpan, ["person", "span"]), on(Claim.where({ arm: Arm.Busy }), ["person", "span"]))
		])

		const deriving = query(MaintainedRollup).rule((r) =>
			r
				.match(Claim, { source: r.var("source"), person: r.var("person"), arm: Arm.Busy, span: r.var("span") })
				.select("person", r.pack("span"))
		)

		const { db } = await admit("r27-maintained-rollup", MaintainedRollup)
		assert.ok(db.prepare(deriving))
	})

	test("28. migration is ETL — two theories, two fingerprints", async function r28() {
		const EmployeeId = u64.as("EmployeeId")

		// The old theory, judged and fingerprinted:
		const EmployeeV1 = relation("Employee", { id: EmployeeId.fresh, name: str })
		const SalaryV1 = relation("Salary", { employee: EmployeeId, amount: i64 })
		const PayrollV1 = schema("PayrollV1", { Employee: EmployeeV1, Salary: SalaryV1 }, [
			contained(on(SalaryV1, "employee"), on(EmployeeV1, "id"))
		])

		// The new theory adds what v1 never recorded — WHEN a salary applied:
		const Employee = relation("Employee", { id: EmployeeId.fresh, name: str })
		const Salary = relation("Salary", { employee: EmployeeId, amount: i64, applies: interval(i64) })
		const Payroll = schema("Payroll", { Employee, Salary }, [
			contained(on(Salary, "employee"), on(Employee, "id")),
			key(Salary, ["employee", "applies"])
		])

		const inForceAt = query(Payroll).rule((r) =>
			r
				.match(Employee, { id: r.var("e"), name: r.var("name") })
				.match(Salary, { employee: r.var("e"), amount: r.var("amount"), applies: r.var("w") })
				.where(r.pointIn(r.param("at"), r.var("w")))
				.select("name", "amount")
		)

		const v1 = await admit("r28-payroll-v1", PayrollV1)
		const v2 = await admit("r28-payroll", Payroll)
		assert.notEqual(v1.fingerprint, v2.fingerprint, "a schema is a theory — the new dimension is a new fingerprint")
		assert.ok(v2.db.prepare(inForceAt))
	})

	test("29. the zone ledger", async function r29() {
		const LedgerId = u64.as("LedgerId")

		const Kind = closed("Kind", ["Unit", "Pair"])
		const Ledger = relation("Ledger", { id: LedgerId.fresh, name: str })
		const Zone = relation("Zone", { ledger: LedgerId, kind: Kind.id, at: interval(u64) })
		const UnitSlot = relation("UnitSlot", { ledger: LedgerId, at: interval(u64, 1n), entry: u64 })
		const PairSlot = relation("PairSlot", { ledger: LedgerId, at: interval(u64, 2n), entry: u64 })

		const ZoneLedger = schema("ZoneLedger", { Kind, Ledger, Zone, UnitSlot, PairSlot }, [
			contained(on(Zone, "ledger"), on(Ledger, "id")),
			contained(on(Zone, "kind"), on(Kind, "id")),
			key(Zone, ["ledger", "at"]),
			key(UnitSlot, ["ledger", "at"]),
			key(PairSlot, ["ledger", "at"]),
			mirrors(on(Zone.where({ kind: Kind.Unit }), ["ledger", "at"]), on(UnitSlot, ["ledger", "at"])),
			mirrors(on(Zone.where({ kind: Kind.Pair }), ["ledger", "at"]), on(PairSlot, ["ledger", "at"]))
		])

		await admit("r29-zone-ledger", ZoneLedger)
	})
})
