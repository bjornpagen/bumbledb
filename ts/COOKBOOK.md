# The cookbook — modeling intuition as schemas, in TypeScript

The bumbledb engine's 29 cookbook recipes (`bumbledb/docs/cookbook.md`),
translated to this SDK's structural API. **This document is illustrative,
never normative**: where a recipe and an engine architecture chapter disagree,
the chapter wins (`docs/architecture/README.md` rule 5) — the SDK is the same
theory in another skin, and the engine cookbook's deference chain
(`10-data-model.md`, `30-dependencies.md`, `20-query-ir.md`, `70-api.md`)
applies here unchanged.

Every schema below compiles and validates verbatim against the current SDK
and engine — `ts/test/cookbook.test.ts` constructs each one through the
public surface, admits it on a real store (the engine's schema validation is
the acceptance judgment), asserts its fingerprint stable across a reopen, and
lowers every query snippet through `db.prepare` (the engine's own IR
validation). A recipe that stops compiling against the SDK fails the build.

Guarantee labels that name Lean results cite the checked spec in the engine
repo's `lean/` by theorem name, exactly as the engine cookbook does
(`scripts/spec-census.sh` verifies every citation resolves); the label always
names any additional Rust premise.

Everything below imports from the one package entry:

```ts
import {
	ALLEN,
	Db,
	abandon,
	bool,
	bytes,
	closed,
	contained,
	i64,
	interval,
	key,
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
} from "@bjornpagen/bumbledb"
```

## Foundations

## 1. The minimal interval schema

Guarantee: Lean theorem + validator/runtime premise — the pointwise key
enforces per-service disjointness (`lean/Bumbledb/Dependencies.lean:
pointwise_key_disjoint`); checked intervals supply nonempty values
(`lean/Bumbledb/Values.lean: interval_nonempty`).

One fact per outage window; the pointwise key is the whole temporal design.

```ts
const ServiceId = u64.as("ServiceId")

const Service = relation("Service", { id: ServiceId.fresh, name: str })
// The window is one value, not a (start, end) column pair: the denotation
// (a set of points, half-open) is what the judgments below read through.
const Outage = relation("Outage", { service: ServiceId, window: interval(i64) })

const Uptime = schema("Uptime", { Service, Outage }, [
	contained(on(Outage, "service"), on(Service, "id")),
	// The pointwise key: per service, no two outages share a point — every
	// pair satisfies DISJOINT. SQL:2011's WITHOUT OVERLAPS, as a theorem.
	key(Outage, ["service", "window"])
])

// down at instant t (point membership is a typing rule):
const downAt = query(Uptime).rule((r) =>
	r
		.match(Outage, { service: r.var("service"), window: r.var("w") })
		.where(r.pointIn(r.param("t"), r.var("w")))
		.select("service")
)
// overlapping an incident window (one Allen mask, no operator zoo):
const overlapping = query(Uptime).rule((r) =>
	r
		.match(Outage, { service: r.var("service"), window: r.var("w") })
		.where(r.allen(r.var("w"), ALLEN.intersects, r.param("incident")))
		.select("service", "w")
)
// total downtime per service (the denotation's one arithmetic):
const downtime = query(Uptime).rule((r) =>
	r.match(Outage, { service: r.var("service"), window: r.var("w") }).select("service", r.sum(r.duration("w")))
)
```

## 2. Discriminated unions

Guarantee: Lean theorem + validator/runtime premises — key-backed equality
gives unique source/target correspondence (`lean/Bumbledb/Dependencies.lean:
keyed_eq_unique_correspondence`); both projections must resolve to declared keys.

Sum-typed entities: a closed-relation discriminator plus per-arm child
relations, glued by bidirectional conditional containments.

```ts
const TaskId = u64.as("TaskId")

// The discriminator vocabulary is a closed relation: its ground axioms are
// axioms, and the handle constants (`Kind.Deterministic`, bare bigints) are
// the literals on every surface.
const Kind = closed("Kind", ["Deterministic", "CustomOperator"])
const Task = relation("Task", { id: TaskId.fresh, kind: Kind.id })
const DeterministicGrading = relation("DeterministicGrading", { task: TaskId, tolerance: i64 })
const CustomOperatorGrading = relation("CustomOperatorGrading", { task: TaskId, operator: str })

const Grading = schema("Grading", { Kind, Task, DeterministicGrading, CustomOperatorGrading }, [
	contained(on(Task, "kind"), on(Kind, "id")), // the discriminator resolves
	key(DeterministicGrading, ["task"]), // one arm fact per parent
	key(CustomOperatorGrading, ["task"]),
	// Totality (==, left to right): a Deterministic task HAS its arm fact —
	// same commit, always. Arm validity (right to left): an arm fact's parent
	// exists WITH that kind — composite-FK-plus-CHECK, one statement.
	mirrors(on(Task.where({ kind: Kind.Deterministic }), "id"), on(DeterministicGrading, "task")),
	mirrors(on(Task.where({ kind: Kind.CustomOperator }), "id"), on(CustomOperatorGrading, "task"))
	// Exclusivity is a theorem, not a statement: one id in two arms would
	// force `kind` to equal two handles against the fresh key on id.
	// The executor spends the same theorem again — recipe 22's free lunch.
])
```

## 3. 0..1 optional attributes

Guarantee: Lean theorem + validator/runtime premises — the child key proves at
most one fact (`lean/Bumbledb/Dependencies.lean: functionality_unique_witness`)
and containment requires its parent (`lean/Bumbledb/Dependencies.lean:
contains_iff_view_subset`); absence remains legal.

No nulls, anywhere. Optional data is an absent fact in a child relation; the
child's key plus a one-way containment *is* "nullable column", done honestly.

```ts
const BusinessId = u64.as("BusinessId")

const Business = relation("Business", { id: BusinessId.fresh, name: str })
const MailingAddress = relation("MailingAddress", { business: BusinessId, line: str, city: str })

const Optionality = schema("Optionality", { Business, MailingAddress }, [
	key(MailingAddress, ["business"]), // at most one address...
	contained(on(MailingAddress, "business"), on(Business, "id")) // ...and only for a real business
	// One-way containment on purpose: absence is the fact that isn't. The
	// all-or-nothing column group (line+city together or neither) is
	// unstatable TO VIOLATE — the fact carries both fields or does not exist.
])

// Negation is plain anti-join (no null branch exists in any operator):
const unaddressed = query(Optionality).rule((r) =>
	r
		.match(Business, { id: r.var("b") })
		.where(r.not(MailingAddress, { business: r.var("b") }))
		.select("b")
)
```

## 4. Money

Guarantee: host discipline + validator premises — fixed-point scale and
currency grouping live in host domains; containments only resolve references.

Fixed-point i64 minor units; the domain label owns scale and currency intent.
Floats are permanently refused (the ledger); proration and FX are host
arithmetic.

```ts
const AccountId = u64.as("AccountId")
const PostingId = u64.as("PostingId")
// Minor units in i64 (±92 quadrillion cents); `.as("Minor")` is the domain
// label — the schema type polices cross-domain confusion, not the engine
// (hard structural typing, 10-data-model.md).
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

// Multi-currency totals: currency is a group key, never summed across —
// Sum folds wide with one final range check, so totals cannot wrap
// silently. Bind the fresh id: set semantics would collapse two equal
// (account, currency, minor) postings without it.
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
```

## 5. Content addressing

Guarantee: validator/runtime premises + host discipline — the payload key and
containments enforce identity/reference shape; hashing and blob durability stay external.

The decision rule for byte-shaped data: **intern what repeats (`str`); inline
what identifies (`bytes(n)`)**.

```ts
const DocumentId = u64.as("DocumentId")
const PayloadHash = bytes(32).as("PayloadHash")

const Region = closed("Region", ["Us", "Eu"])
const Document = relation("Document", {
	id: DocumentId.fresh,
	name: str, // repeats: interned, id-equality
	payload: PayloadHash // identifies: the blake3 of the external blob — inline, never interned
})
const Replica = relation("Replica", { payload: PayloadHash, region: Region.id })

const Content = schema("Content", { Region, Document, Replica }, [
	key(Document, ["payload"]), // content-addressed: one doc per digest
	contained(on(Replica, "payload"), on(Document, "payload")),
	contained(on(Replica, "region"), on(Region, "id"))
	// bytes(n) is identity-only (Eq/Ne, membership): a digest's lexicographic
	// order is an encoding artifact, refused as semantics. Large objects:
	// facts stay fixed-width; the payload lives in external storage,
	// referenced by identity (the large-object refusal).
])

// a bytes param self-encodes (Uint8Array by inference):
const byDigest = query(Content).rule((r) => r.match(Document, { id: r.var("id"), payload: r.param("digest") }).select("id"))
```

## Vocabularies

## 6. The vocabulary

Guarantee: Lean theorem + validator/runtime premise — the sealed closed
extension is constant at every instance (`lean/Bumbledb/Schema.lean:
den_closed_constant`) and the compiled member-set containment admits only
declared priority handles.

The enum idiom's replacement, first-class: a vocabulary is a **closed
relation** — its ground axioms are declared in the schema, sealed at
validate, frozen by the fingerprint, virtual in storage. The store holds zero
vocabulary bytes, and handles are the literals on every surface.

```ts
const TicketId = u64.as("TicketId")

// Tier 1: handles only. `closed()` mints one bare-bigint constant per handle
// (ids = declaration order) — an emission, not a type: the engine's
// vocabulary stays relational; the host matches on `Priority.Urgent`.
const Priority = closed("Priority", ["Low", "Normal", "Urgent"])

const Ticket = relation("Ticket", { id: TicketId.fresh, priority: Priority.id, opened_at: i64 })

const Tickets = schema("Tickets", { Priority, Ticket }, [
	// A closed reference is an ordinary u64 under the handle domain plus one
	// containment; the judgment compiles at validate to a member-set test —
	// one AND, one bit test, no probe (30-dependencies.md).
	contained(on(Ticket, "priority"), on(Priority, "id"))
])

// Handles are literals in queries exactly as in statements, and the
// renderer prints them back — the round trip runs on names. The boundary
// law: intrinsic meaning goes here (changing it is a new theory); policy
// that drifts without a rebuild is an ordinary relation — a vocabulary is
// never written, only declared.
const urgent = query(Tickets).rule((r) => r.match(Ticket, { id: r.var("t"), priority: Priority.Urgent }).select("t"))
```

## 7. The classification

Guarantee: validator/runtime premise — closed payload facts and the compiled
member-set restriction confine certificates to mastered handles.

The fused form: the vocabulary carries its intrinsic facts as **payload
columns** — one ground axiom per handle, values sealed with the schema.
Axioms are declared, never written.

> **SDK surface note.** The engine spells the restriction as a ψ-selected
> containment over the vocabulary itself — `Certificate(kind) <= Kind(id |
> mastered == true)` — and its reads walk the payload in the query (`Kind(id:
> k, mastered == true)`). Neither face is writable in this SDK today: a
> `closed()` value carries no `.where`, and query atoms match ordinary
> relations only (a reported surface finding). Both spellings below are
> engine-equivalent **because the roster is sealed**: the host folds the
> payload over the sealed axioms at schema-build time — the same member set
> the engine's ψ would compile, {DirectPass, JudgedPass} against complement
> {Failed} — and the exclusion window enforces the identical commit judgment.

```ts
const AttemptId = u64.as("AttemptId")

// Tier 2: payload columns state what each word MEANS, next to the word.
// A rubric change is a new theory — exactly right for meaning.
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
	// Certificates carry mastered kinds only, spelled over the sealed
	// roster's complement: per attempt, ZERO certificates at a non-mastered
	// kind — O(1) at commit, judged on every touching commit.
	window(on(Attempt, "id"), none, on(Certificate.where({ kind: Kind.Failed }), "attempt"))
])

// The classification read duplicates no flag onto Attempt — the payload is
// read off the sealed axioms (`Kind.axioms.DirectPass.mastered`), and the
// member set is spelled as a rule union (recipe 22's shape; rules selecting
// different handles are provably disjoint — the free lunch):
const masteredAttempts = query(Review)
	.rule((r) => r.match(Attempt, { id: r.var("a"), kind: Kind.DirectPass }).select("a"))
	.rule((r) => r.match(Attempt, { id: r.var("a"), kind: Kind.JudgedPass }).select("a"))
```

## 8. The sub-vocabulary

Guarantee: validator/runtime premise — the member set compiled from the
sealed extension confines escalations to paging severities; a nonmember
write is commit-rejected.

A reference constrained to the facts of a vocabulary that satisfy a payload
selection. Because the target is closed and sealed, the enforcement plan is
not a probe strategy — it is **the answer set itself**, fixed when the schema
is built. (The engine's direct ψ spelling — `Escalation(severity) <=
Severity(id | pages == true)` — is not yet writable here; see recipe 7's
surface note. The complement exclusion below compiles to the same commit
judgment over the same sealed roster.)

```ts
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
	// The sub-vocabulary: an escalation carries a PAGING severity, by
	// statement — per incident, ZERO escalations at a non-paging severity
	// (every escalation resolves its incident, so every escalation is
	// judged). An escalation at Severity.Info aborts the commit.
	window(on(Incident, "id"), none, on(Escalation.where({ severity: oneOf(Severity.Info, Severity.Warning) }), "incident"))
])

// who is being paged (the same member set, on the read side):
const paged = query(Oncall)
	.rule((r) => r.match(Escalation, { incident: r.var("i"), severity: Severity.Critical }).select("i"))
	.rule((r) => r.match(Escalation, { incident: r.var("i"), severity: Severity.Fatal }).select("i"))
```

## Structure

## 9. Ordered collections

Guarantee: Lean theorem + validator/runtime premises — mutual point coverage
plus pointwise keys realizes exact partition
(`lean/Bumbledb/Dependencies.lean: exact_partition_iff`), and the mixed-width
interval positions type by element domain
(`lean/Bumbledb/Schema.lean: Value.points_one_tag_u64`); ordering the result
remains a host presentation step.

The linked-list verdict: successor pointers are control flow smuggled into
data. Order is a value. The idiomatic ordered collection is an interval
partition, spelled as a **triple**: the entity, the extent as a 0..1 child
(empty lists exist, empty intervals do not — presence of the child IS
nonemptiness), and the unit-slot sidecar (`interval(u64, 1n)` — the width is
the type: a wrong-width value is unrepresentable).

```ts
const PlaylistId = u64.as("PlaylistId")

const Playlist = relation("Playlist", { id: PlaylistId.fresh, name: str })
// The extent: a 0..1 child, because empty playlists exist and empty
// intervals do not — presence of the child IS nonemptiness.
const Extent = relation("Extent", { playlist: PlaylistId, span: interval(u64) })
// The unit slot: position p occupies [p, p+1) — the width is the type.
const Slot = relation("Slot", { playlist: PlaylistId, slot: interval(u64, 1n), track: str })

const Playlists = schema("Playlists", { Playlist, Extent, Slot }, [
	contained(on(Extent, "playlist"), on(Playlist, "id")),
	contained(on(Slot, "playlist"), on(Playlist, "id")),
	key(Extent, ["playlist"]), // 0..1 extent per playlist
	key(Extent, ["playlist", "span"]), // exact target key (recipe 26's note)
	key(Slot, ["playlist", "slot"]), // one occupant per position
	mirrors(on(Extent, ["playlist", "span"]), on(Slot, ["playlist", "slot"])) // slots tile the span exactly
])

// Positional access is membership — "what plays at position ?pos":
const playingAt = query(Playlists).rule((r) =>
	r
		.match(Slot, { playlist: r.param("list"), slot: r.var("s"), track: r.var("track") })
		.where(r.pointIn(r.param("pos"), r.var("s")))
		.select("track")
)
```

Middle insert is honest about its cost: making room at position `k` shifts
every later slot and grows the extent — O(k) writes in **one delta**, judged
once at commit. If middle inserts dominate, the demoted escape hatch is the
spread slot: a scalar `pos: u64` written in gapped strides under the same
composite key — bumbledb has no lexicographic fractional indexing, because
string order is refused: there is no "between two strings" to allocate.

## 10. Trees and ASTs

Guarantee: Lean theorem + validator/runtime premises for key-backed arms
(`lean/Bumbledb/Dependencies.lean: keyed_eq_unique_correspondence`); host
discipline for acyclicity — statements prove arm/edge shape, never a tree theorem.

Node header + per-kind arms (recipe 2's pattern); every edge resolves; the
shape theorems come from keys on the edge relations.

```ts
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
	// Every node's arm is total, valid, and exclusive (recipe 2's theorems):
	mirrors(on(Node.where({ kind: Kind.Lit }), "id"), on(Lit, "node")),
	mirrors(on(Node.where({ kind: Kind.Add }), "id"), on(Add, "node")),
	// Every child edge resolves — no dangling subtrees, judged at commit:
	contained(on(Add, "lhs"), on(Node, "id")),
	contained(on(Add, "rhs"), on(Node, "id")),
	// Functional parent (one parent per child) ⇒ the reachable shape is
	// paths-or-cycles; acyclicity itself is outside the ∀∃ vocabulary —
	// host discipline, recorded. Transitive reach is recipe 24's closure.
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
```

## 11. Typed graphs

Guarantee: validator/runtime premises — endpoint containments type each edge
and composite keys deduplicate pairs; no transitive graph property is claimed.

One relation per edge kind: endpoint containments pin which node kinds each
edge may touch.

```ts
const PersonId = u64.as("PersonId")
const RepoId = u64.as("RepoId")

const Person = relation("Person", { id: PersonId.fresh, name: str })
const Repo = relation("Repo", { id: RepoId.fresh, name: str })
const Follows = relation("Follows", { follower: PersonId, followee: PersonId })
const Maintains = relation("Maintains", { person: PersonId, repo: RepoId })

const Graph = schema("Graph", { Person, Repo, Follows, Maintains }, [
	contained(on(Follows, "follower"), on(Person, "id")), // a Person→Person edge, by statement —
	contained(on(Follows, "followee"), on(Person, "id")), // a Follows fact cannot touch a Repo
	key(Follows, ["follower", "followee"]), // at most one edge per pair
	contained(on(Maintains, "person"), on(Person, "id")),
	contained(on(Maintains, "repo"), on(Repo, "id")),
	key(Maintains, ["person", "repo"])
])

// Mutual follows — joins are explicit var reuse on both ends; `lt` keeps
// each pair once:
const mutual = query(Graph).rule((r) =>
	r
		.match(Follows, { follower: r.var("a"), followee: r.var("b") })
		.match(Follows, { follower: r.var("b"), followee: r.var("a") })
		.where(r.lt(r.var("a"), r.var("b")))
		.select("a", "b")
)
```

## 12. Entity-component

Guarantee: definition + validator/runtime premises — component keys give 0..1
and containments require the stated entity/archetype facts.

The 0..1 idiom (recipe 3) at scale: components are sidecar relations; an
entity has a component iff the fact exists; a new component kind is a new
relation, not a wider fact.

```ts
const EntityId = u64.as("EntityId")

const Entity = relation("Entity", { id: EntityId.fresh, name: str })
const Transform = relation("Transform", { entity: EntityId, x: i64, y: i64 })
const Velocity = relation("Velocity", { entity: EntityId, dx: i64, dy: i64 })
const Renderable = relation("Renderable", { entity: EntityId, mesh: str })

const Ecs = schema("Ecs", { Entity, Transform, Velocity, Renderable }, [
	key(Transform, ["entity"]), // each component 0..1 per entity
	contained(on(Transform, "entity"), on(Entity, "id")),
	key(Velocity, ["entity"]),
	contained(on(Velocity, "entity"), on(Entity, "id")),
	key(Renderable, ["entity"]),
	// An archetype rule is one containment: every Renderable has a Transform
	// (and, through it, an Entity — containment composes).
	contained(on(Renderable, "entity"), on(Transform, "entity"))
])

// The physics join is the component intersection:
const physics = query(Ecs).rule((r) =>
	r
		.match(Transform, { entity: r.var("e"), x: r.var("x"), y: r.var("y") })
		.match(Velocity, { entity: r.var("e"), dx: r.var("dx"), dy: r.var("dy") })
		.select("e", "x", "y", "dx", "dy")
)
```

## 13. State machines

Guarantee: Lean theorem + validator/runtime premises for the shipped arm
(`lean/Bumbledb/Dependencies.lean: keyed_eq_unique_correspondence`); host
discipline for allowed transitions — equality pins state evidence, not paths.

States are a discriminated union; per-state data lives in arms; and the
conditional reference target — a reference to "an order *that is shipped*" —
is one selected statement, the statement SQL cannot write.

```ts
const OrderId = u64.as("OrderId")

const State = closed("State", ["Cart", "Placed", "Shipped"])
const Order = relation("Order", { id: OrderId.fresh, state: State.id })
const Placement = relation("Placement", { order: OrderId, at: i64 })
const Shipment = relation("Shipment", { order: OrderId, carrier: str, at: i64 })

const Orders = schema("Orders", { State, Order, Placement, Shipment }, [
	contained(on(Order, "state"), on(State, "id")),
	key(Placement, ["order"]),
	key(Shipment, ["order"]),
	// History accretes: a Shipped order keeps its Placement — one-way
	// containment admits arms from earlier states surviving the transition.
	contained(on(Placement, "order"), on(Order, "id")),
	// The conditional target, both ways: every Shipment references an order
	// THAT IS Shipped (validity), and every Shipped order has its Shipment
	// (totality) — the transition and its evidence commit together.
	mirrors(on(Shipment, "order"), on(Order.where({ state: State.Shipped }), "id"))
	// Transition predicates ("only Placed may ship") are host code under the
	// generation witness — recipe 20; the schema pins the states, not the paths.
])

const shipped = query(Orders).rule((r) =>
	r
		.match(Order, { id: r.var("id"), state: State.Shipped })
		.match(Shipment, { order: r.var("id"), carrier: r.var("carrier") })
		.select("id", "carrier")
)
```

## Time and coverage

## 14. The calendar core

Guarantee: Lean theorem + validator/runtime premises — accepted equality is
key-backed correspondence (`lean/Bumbledb/Dependencies.lean:
keyed_eq_unique_correspondence`), while pointwise keys/coverage enforce only
declared hard policy.

Policy as schema: hard rules are pointwise keys, soft rules are the statements
you decline to write.

> **SDK surface note.** `Claim.source` carries the accepted attendance's id,
> so it is labeled `.as("AttendanceId")` here: the SDK's domain wall pairs
> faces by label, where the engine cookbook's bare `source: u64` relied on
> host discipline. The label lives in the schema type only — it never touches
> the value or the fingerprint.

```ts
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
	key(Attendance, ["event", "person"]), // one RSVP per (event, person)
	key(Claim, ["source"]),
	contained(on(Claim, "person"), on(Person, "id")),
	contained(on(Claim, "arm"), on(Arm, "id")),
	// HARD: rooms cannot double-book — the pointwise key (recipe 1's theorem).
	key(Booking, ["room", "span"]),
	// SOFT: people CAN double-book — key(Claim, ["person", "span"]) is simply
	// not declared. Policy is the presence or absence of one statement.
	// Accepting an invitation IS claiming the time (totality + validity):
	mirrors(on(Attendance.where({ rsvp: Rsvp.Accepted }), "id"), on(Claim.where({ arm: Arm.Busy }), "source")),
	// Busy time lies inside working hours, pointwise — coverage rides the
	// target's own key (disjoint + ordered is a theorem, not a request):
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
```

## 15. Effective-dated configuration

Guarantee: Lean theorem/countermodel + validator/runtime premise — pointwise
keys plus one-way support inclusion form a disjoint cover
(`lean/Bumbledb/Dependencies.lean: pointwise_key_disjoint`,
`coverage_is_support_inclusion`); target overhang is legal
(`lean/Bumbledb/Countermodels.lean: one_way_overhang`).

Versioned rules: no overlaps (pointwise key), no gaps in the policy's source
lifetime (one-way coverage; version overhang remains legal), and "in force on
date t" is one membership probe.

```ts
const PolicyId = u64.as("PolicyId")

const Policy = relation("Policy", { id: PolicyId.fresh, live: interval(i64) })
const Version = relation("Version", { policy: PolicyId, rate_bps: i64, valid: interval(i64) })

const Pricing = schema("Pricing", { Policy, Version }, [
	contained(on(Version, "policy"), on(Policy, "id")),
	// No overlapping versions: at any instant, at most one rate is the law.
	key(Version, ["policy", "valid"]),
	// No gaps in the policy lifetime: every source point is covered by
	// versions. Together with the key above this is a disjoint cover, not an
	// exact partition: Version intervals may overhang (recipe 16).
	contained(on(Policy, ["id", "live"]), on(Version, ["policy", "valid"]))
])

// in force on date t — one membership probe:
const inForce = query(Pricing).rule((r) =>
	r
		.match(Version, { policy: r.param("p"), rate_bps: r.var("rate_bps"), valid: r.var("v") })
		.where(r.pointIn(r.param("t"), r.var("v")))
		.select("rate_bps")
)
// clean successions (half-open makes MEETS exact, no ±1 fudge):
const successions = query(Pricing).rule((r) =>
	r
		.match(Version, { policy: r.var("p"), valid: r.var("a") })
		.match(Version, { policy: r.var("p"), valid: r.var("b") })
		.where(r.allen(r.var("a"), ALLEN.meets, r.var("b")))
		.select("a", "b")
)
```

## 16. Disjoint covers

Guarantee: Lean theorem/countermodel + validator/runtime premise —
`lean/Bumbledb/Dependencies.lean: coverage_is_support_inclusion` proves source
coverage, not exact partition (`lean/Bumbledb/Countermodels.lean:
one_way_overhang`).

Pay periods, shifts, estimated-tax quarters: a pointwise key plus one-way
coverage is a **disjoint cover** — no overlaps among pay periods and no holes
in the fiscal year's source span. Pay periods may extend beyond that span.

```ts
const FiscalYearId = u64.as("FiscalYearId")

const FiscalYear = relation("FiscalYear", { id: FiscalYearId.fresh, span: interval(i64) })
const PayPeriod = relation("PayPeriod", { year: FiscalYearId, seq: u64, span: interval(i64) })

const Payroll = schema("Payroll", { FiscalYear, PayPeriod }, [
	contained(on(PayPeriod, "year"), on(FiscalYear, "id")),
	key(PayPeriod, ["year", "seq"]), // sequence numbers stay unique
	key(PayPeriod, ["year", "span"]), // disjoint: no shared instant
	// Covering: no holes in the fiscal year's span; overhang is legal.
	contained(on(FiscalYear, ["id", "span"]), on(PayPeriod, ["year", "span"]))
])

// the period holding date t:
const holding = query(Payroll).rule((r) =>
	r
		.match(PayPeriod, { year: r.param("y"), seq: r.var("seq"), span: r.var("s") })
		.where(r.pointIn(r.param("t"), r.var("s")))
		.select("seq")
)
```

## 17. Federal income tax

Guarantee: validator/runtime premises + host discipline — keys prove bracket
disjointness and statements prove residency coverage; full bracket coverage and proration are host duties.

Brackets are intervals over money; the top bracket is a ray; regimes key on
(year, status); and proration happens at write time, never at query time.

```ts
const RegimeId = u64.as("RegimeId")

const Status = closed("Status", ["Single", "MarriedJoint", "HeadOfHousehold"])
const Regime = relation("Regime", { id: RegimeId.fresh, year: i64, status: Status.id })
const Bracket = relation("Bracket", { regime: RegimeId, income: interval(i64), rate_bps: i64 })
const Residency = relation("Residency", { person: u64, span: interval(i64) })
// Split at write: an Earned fact never spans a year boundary — writers
// split (prorate) at the boundary, so no reader ever clips. The
// representation move that deletes clip-at-query (gravestone, recipe 23).
const Earned = relation("Earned", { person: u64, regime: RegimeId, span: interval(i64), minor: i64 })

const Tax = schema("Tax", { Status, Regime, Bracket, Residency, Earned }, [
	contained(on(Regime, "status"), on(Status, "id")),
	key(Regime, ["year", "status"]), // one regime per (year, filing status)
	contained(on(Bracket, "regime"), on(Regime, "id")),
	// Brackets are disjoint per regime. Seed data conventionally covers
	// [0, ∞) and the top bracket is a ray, but this key proves disjointness
	// only. end == MAX denotes [s, ∞), an honest value of the representation,
	// not a sentinel (the point-domain law).
	key(Bracket, ["regime", "income"]),
	contained(on(Earned, "regime"), on(Regime, "id")),
	key(Residency, ["person", "span"]),
	// Residency exclusion: income counts only where earned inside a residency
	// period — pointwise coverage, the same judgment as recipe 15's.
	contained(on(Earned, ["person", "span"]), on(Residency, ["person", "span"]))
])

// the marginal bracket (membership probes the disjoint bracket set). Tax
// owed is host arithmetic over the bracket walk — arithmetic beyond the
// measure is refused (the ledger).
const marginal = query(Tax).rule((r) =>
	r
		.match(Regime, { id: r.var("reg"), year: r.param("y"), status: r.param("s") })
		.match(Bracket, { regime: r.var("reg"), income: r.var("b"), rate_bps: r.var("rate_bps") })
		.where(r.pointIn(r.param("taxable"), r.var("b")))
		.select("rate_bps")
)
```

## 18. Free time and coalescing

Guarantee: Lean theorem + runtime query semantics — `pack` coalesces answer
intervals (`lean/Bumbledb/Query/Aggregates.lean: pack_canonical`,
`pack_extensional`); it asserts no stored disjointness, completeness, or
maintenance behavior.

`pack` is Snodgrass's coalesce as an aggregate — maximal disjoint segments per
group, one answer per (group, segment). Coalescing is never a write rule: the
engine stores the claims it was given.

```ts
const PersonId = u64.as("PersonId")

const Person = relation("Person", { id: PersonId.fresh, name: str })
const Claim = relation("Claim", { person: PersonId, span: interval(i64) })

const FreeTime = schema("FreeTime", { Person, Claim }, [
	contained(on(Claim, "person"), on(Person, "id"))
	// No pointwise key, on purpose: claims overlap freely and pack coalesces
	// at read time. Wanting them stored-disjoint is recipe 1's key instead.
])

// busy time, coalesced (adjacent segments merge — the half-open law):
const busy = query(FreeTime).rule((r) =>
	r.match(Claim, { person: r.var("person"), span: r.var("span") }).select("person", r.pack("span"))
)
// raw claimed time (overlaps double-count — often the wrong question):
const claimed = query(FreeTime).rule((r) =>
	r.match(Claim, { person: r.var("person"), span: r.var("span") }).select("person", r.sum(r.duration("span")))
)
// Coalesced totals = the two-query composition (pack, then a host fold) —
// aggregates never nest; free time (gaps) is the two-line host walk over
// sorted packed answers — both refusals recorded in the ledger.
```

## The write side

## 19. The ledger

Guarantee: Lean theorem + runtime invariant for bounded sums
(`lean/Bumbledb/Query/Aggregates.lean: checkedSum_sound`); host discipline
for double entry — statements resolve posting references, not arithmetic agreement.

The census workload. Balance is a query, never a column.

```ts
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
	// A stored balance column equaling Sum(postings) is the arithmetic-
	// agreement statement — refused (the ledger): statements prove presence
	// and topology, never that a value equals a computation. Balance is host
	// arithmetic over sum; a materialized rollup is recipe 21's shape.
])

// balances (bind the fresh id — set semantics collapses duplicates):
const balances = query(Ledger).rule((r) =>
	r
		.match(Posting, { id: r.var("id"), account: r.var("account"), minor: r.var("minor") })
		.select("account", r.sum("minor"))
)
// double-entry audit (host asserts every total is 0 — discipline, not schema):
const doubleEntry = query(Ledger).rule((r) =>
	r
		.match(Posting, { id: r.var("id"), entry: r.var("entry"), minor: r.var("minor") })
		.select("entry", r.sum("minor"))
)
```

## 20. Conditional writes

Guarantee: Lean theorem + generation-witness/runtime premise + host retry
discipline — snapshot-derived writes detect movement
(`lean/Bumbledb/Txn.lean: writeFrom_moved`, `witness_conflict_distinct`);
final-state point reads need no earlier witness.

The generation witness: read the model, propose a delta, commit iff the model
you read is still the model. In the SDK the whole loop is `db.writeWitnessed`
— retry on movement is built in (every generation move is self-inflicted by
the host's own interleaved writes), and `abandon(payload)` declines to commit
without issuing anything.

```ts
const JobId = u64.as("JobId")

const State = closed("State", ["Queued", "Running", "Done"])
const Job = relation("Job", { id: JobId.fresh, state: State.id, payload: str })
const Lease = relation("Lease", { job: JobId, worker: u64, until: i64 })

const Jobs = schema("Jobs", { State, Job, Lease }, [
	contained(on(Job, "state"), on(State, "id")),
	key(Lease, ["job"]),
	// A lease exists iff its job is Running (recipe 13's conditional target):
	// claiming a job and leasing it commit together or not at all.
	mirrors(on(Lease, "job"), on(Job.where({ state: State.Running }), "id"))
])

// update-where's premise — "still Queued" is the witness:
const stillQueued = query(Jobs).rule((r) =>
	r.match(Job, { id: r.var("id"), state: State.Queued, payload: r.var("payload") }).select("id", "payload")
)

// The witnessed loop: premise reads via `snap`, the delta via `tx`; on a
// moved generation the WHOLE callback reruns on a fresh snapshot. The other
// two idioms: insert-select is the same shape (query source answers, insert
// the derived facts); key-shaped read-modify-write uses `tx.get`/`tx.contains`
// — final-state point reads need no earlier witness.
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
```

## 21. Derived relations

Guarantee: Lean theorem + validator/runtime premises for soundness
(`lean/Bumbledb/Txn.lean: derived_soundness_vs_freshness`); host
discipline for completeness — containment rejects unsupported facts but never refreshes omissions.

The materialized view as a relation under statements — unsoundness the schema
can name is uncommittable; incompleteness remains representable until the host
refreshes it.

```ts
const Arm = closed("Arm", ["Busy", "Ooo"])
const Claim = relation("Claim", { source: u64, person: u64, arm: Arm.id, span: interval(i64) })
const BusySpan = relation("BusySpan", { person: u64, span: interval(i64) })

const Rollup = schema("Rollup", { Arm, Claim, BusySpan }, [
	contained(on(Claim, "arm"), on(Arm, "id")),
	key(Claim, ["source"]),
	key(Claim, ["person", "span"]),
	key(BusySpan, ["person", "span"]), // packed ⇒ disjoint: statable
	// Soundness, pointwise: every stored rollup point is covered by busy
	// claims — an UNSOUND rollup (claiming busy time that isn't, or surviving
	// its sources' deletion) cannot commit, judged on every touching commit.
	contained(on(BusySpan, ["person", "span"]), on(Claim.where({ arm: Arm.Busy }), ["person", "span"]))
])

// Maintenance is the third witness idiom (recipe 20): re-run the deriving
// query on a snapshot, diff, commit witnessed — the rollup cannot commit
// against sources it didn't actually read. The deriving query (pack IS the
// coalesce):
const deriving = query(Rollup).rule((r) =>
	r.match(Claim, { person: r.var("person"), span: r.var("span"), arm: Arm.Busy }).select("person", r.pack("span"))
)
```

## 22. Union reads

Guarantee: Lean theorem + represented planner/runtime premise — rule union is
set-idempotent (`lean/Bumbledb/Query/Denotation.lean: union_idempotent`);
key-backed DU arms justify the disjointness optimization
(`lean/Bumbledb/Exec/Dedup.lean: disjoint_witness_licence`).

The whole-DU read is a set of rules: one head, one rule per arm — disjunction
is data at the top, never an execution node.

```ts
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

// One query, two rules (set union). The exclusivity theorem (recipe 2) is
// spent a third time here: rules selecting different `kind` handles are
// provably disjoint, so the executor elides cross-rule dedup — the free lunch.
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
```

## 23. The anti-recipes: five gravestones

Guarantee: intentionally refused — each gravestone names unsupported vocabulary
and its representable replacement; none asserts an engine theorem.

What not to model. Each gravestone cites its replacement; the block's
relations are the replacements, compiled.

```ts
const GravestoneEventId = u64.as("GravestoneEventId")

// GRAVESTONE: successor pointers (a `next` column). A linked list inside a
// relation is control flow smuggled into data. REPLACEMENT: the ordering
// triple (recipe 9).
const Step = relation("Step", { flow: u64, pos: u64, action: str })
// GRAVESTONE: floats for scores, rates, money. Permanently refused (the
// ledger). REPLACEMENT: fixed-point i64 — basis points (recipe 4).
const Score = relation("Score", { subject: u64, bps: i64 })
// GRAVESTONE: conditional keys ("at most one active run per student") —
// rejected as FDs. REPLACEMENT: the relation split, whose ordinary key IS
// the invariant (recipe 13's arm shape).
const ActiveRun = relation("ActiveRun", { student: u64, run: u64 })
// GRAVESTONE: clip-at-query intervals (facts spanning period boundaries,
// every reader clipping). REPLACEMENT: split at write (recipe 17).
const Usage = relation("Usage", { meter: u64, period: u64, used: interval(i64) })
// GRAVESTONE: uuid keys. uuidv7 is identity + clash-avoidance + clock in
// one lie. REPLACEMENT: fresh (minted identity) + an explicit i64 time column.
const Event = relation("Event", { id: GravestoneEventId.fresh, at: i64 })

const Gravestones = schema("Gravestones", { Step, Score, ActiveRun, Usage, Event }, [
	key(Step, ["flow", "pos"]),
	key(Score, ["subject"]),
	key(ActiveRun, ["student"]),
	key(Usage, ["meter", "used"])
])
```

## Host-driven closure

## 24. The closure idiom

Guarantee: host discipline for the loop — the finite `seen` set proves
termination for the host run; the engine-native form beside it executes
whole under the fixpoint driver, budget-bounded
(`lean/Bumbledb/Exec/Fixpoint.lean: program_eval_sound`).

Reachability, in two dialects. The host-loop idiom remains the depth-bounded
answer: the loop runs depth-many rounds and each round is one ∈-set query —
an `inSet` probe, microsecond-class. The frontier discipline below *is*
semi-naive evaluation's Δ, spent where a loop is a loop: the host. The
engine-native form is the same closure as one stratified `program()`.

```ts
const NodeId = u64.as("NodeId")

const Node = relation("Node", { id: NodeId.fresh, name: str })
// One parent per child — a forest (recipe 10's edge shape); a root is a
// node whose Parent fact is absent (recipe 3's honest 0..1).
const Parent = relation("Parent", { child: NodeId, parent: NodeId })

const Closure = schema("Closure", { Node, Parent }, [
	key(Parent, ["child"]),
	contained(on(Parent, "child"), on(Node, "id")),
	contained(on(Parent, "parent"), on(Node, "id"))
])

// The loop's one query — the frontier's children, one ∈-set probe:
const step = query(Closure).rule((r) => r.match(Parent, { child: r.var("c"), parent: r.inSet("frontier") }).select("c"))
```

The loop (the compiled, driven copy is in `test/cookbook.test.ts`, over a
three-level forest with the exact reachable set asserted):

```ts
const seen = new Set<bigint>([root])
let frontier: readonly bigint[] = [root]
for (;;) {
	const next = db.execute(stepPrepared, { frontier }) // one set-param query
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
```

Termination is the host's theorem: `seen` grows strictly or the loop breaks,
inside a finite node set. When the idiom's costs bite — **unbounded or large
depth**, or **closure composed into a larger plan** — write the engine-native
form instead: `?root` seeds the predicate, and the output joins the finished
set back through the theory's own domain relation (an `idb` atom is a join
position, so the head rides the `Node` atom):

```ts
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
	return p.output((r) => r.match(Node, { id: r.var("c") }).idb(seeded, r.var("c")).select("c"))
})
const reachPrepared = db.prepare(reach)
```

(the test drives both dialects and asserts the same reachable sets, root for
root). What stays host-side is the **chain-window class** — interval
intersection along paths — which the recursion surface fences out: the idiom
carries the window in the host's frontier, one intersection per hop, and that
composition has no engine form.

## 25. The chart of accounts

Guarantee: host discipline + runtime aggregate semantics — the host computes
closure, then one checked `sum` (`lean/Bumbledb/Query/Aggregates.lean:
checkedSum_sound`); the engine-native form folds over a *finished* lower
stratum, the one aggregation shape the strata roster admits.

The ledger workload's real recursion case, in the same two dialects: a
hierarchical chart of accounts and a subtree rollup. The host composition —
recipe 24's loop accumulates the subtree's ∈-set, then **one `sum` query over
the accumulated set** folds the postings. The engine aggregates, the host
composes (aggregates never nest). The engine-native form is one program:
aggregation *through* a cycle is refused, but a fold over a recursive
predicate from a **higher stratum** reads a finished set and is ordinary.

```ts
const AccountId = u64.as("AccountId")
const PostingId = u64.as("PostingId")

const Account = relation("Account", { id: AccountId.fresh, name: str })
const AccountParent = relation("AccountParent", { child: AccountId, parent: AccountId })
const Posting = relation("Posting", { id: PostingId.fresh, account: AccountId, minor: i64 })

const Accounts = schema("Accounts", { Account, AccountParent, Posting }, [
	key(AccountParent, ["child"]), // one parent per account
	contained(on(AccountParent, "child"), on(Account, "id")),
	contained(on(AccountParent, "parent"), on(Account, "id")),
	contained(on(Posting, "account"), on(Account, "id"))
])

// The two queries the host rollup composes:
//   the frontier step (recipe 24's loop, verbatim):
const frontierStep = query(Accounts).rule((r) =>
	r.match(AccountParent, { child: r.var("c"), parent: r.inSet("frontier") }).select("c")
)
//   the rollup over the accumulated subtree (bind the fresh id — recipe
//   19's discipline, spent again; equal postings to one account both count):
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
```

## 26. Exact partition

Guarantee: Lean theorem + validator/runtime premises — mutual point coverage
plus pointwise keys realizes exact partition
(`lean/Bumbledb/Dependencies.lean: exact_partition_iff`).

An exact partition needs both coverage directions. The first containment below
is the intent-level reference; the two pointwise keys make each side disjoint;
the final pair proves equal point supports per policy — forward coverage
forbids gaps and reverse coverage forbids overhang.

The explicit `key(Policy, ["id", "live"])` is load-bearing. Containment
targets resolve by their exact projected field set, so the fresh `{id}` key
cannot serve the `{id, live}` target and the engine infers no key closure.

```ts
const PolicyId = u64.as("PolicyId")

const Policy = relation("Policy", { id: PolicyId.fresh, live: interval(i64) })
const Version = relation("Version", { policy: PolicyId, valid: interval(i64) })

const ExactPartition = schema("ExactPartition", { Policy, Version }, [
	contained(on(Version, "policy"), on(Policy, "id")), // reference intent
	key(Version, ["policy", "valid"]), // disjoint versions
	key(Policy, ["id", "live"]), // exact target key, not implied by {id}
	contained(on(Policy, ["id", "live"]), on(Version, ["policy", "valid"])), // no gaps in the policy source span
	contained(on(Version, ["policy", "valid"]), on(Policy, ["id", "live"])) // no version overhang
])
```

Together the mutual containments prove equal point supports for each policy;
the pointwise keys make those supports genuine partitions rather than
overlapping covers. Touching half-open segments remain legal, and the same
construction works with any scalar-prefix arity before the final interval
position.

## 27. Derived facts, maintained

Guarantee: host discipline + validator/runtime premises — freshness comes from
the generation witness; containment proves surviving rollup facts sound only
(`lean/Bumbledb/Txn.lean: derived_soundness_vs_freshness`).

A stored rollup is an ordinary relation with an ordinary soundness statement.
Here `pack` derives maximal busy spans, while containment prevents any stored
`BusySpan` point that has no busy claim behind it. That is soundness, not a
refresh theorem: a missing span remains representable until the host
maintenance loop fills it.

```ts
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

// Derive the desired rollup on the maintenance snapshot:
const deriving = query(MaintainedRollup).rule((r) =>
	r
		.match(Claim, { source: r.var("source"), person: r.var("person"), arm: Arm.Busy, span: r.var("span") })
		.select("person", r.pack("span"))
)
```

The host loop is `db.writeWitnessed`: derive on the attempt's snapshot, diff,
build the delta — recipe 20's third idiom. On a moved generation the SDK
throws away the attempt and reruns the whole callback on a fresh snapshot; it
never retries a stale diff. Dependencies prove every surviving stored span
sound, while the witness proves which source state the derivation saw;
neither mechanism proves completeness. The engine's compiled copy
(`maintain_busy_spans` in `cookbook.rs`) drives the retry-and-repack loop.

## Operating the store

## 28. Migration is ETL

Guarantee: Lean theorem + validator/runtime premises + host discipline —
fingerprints refuse reinterpretation, final-state judgments validate each load
(`lean/Bumbledb/Txn.lean: etl_lands_valid`), and the host owns
the semantic transform and dependency-safe load order.

There is no in-place migration and never will be: a schema is a theory, the
store records the theory's fingerprint, and `Db.open` under a changed theory
is a hard fingerprint mismatch — the engine refuses to reinterpret facts it
judged under different laws. Migration is extract, transform, load: `scan`
exports every fact of a relation as typed values under one snapshot, the host
transforms, and inserts (fresh ids resupplied — identity survives) land into
a store created under the new theory, judged whole by the ordinary
final-state judgment: load containment targets first, and a chunk that lands
is already valid.

The v2 theory below adds what v1 never recorded — *when* a salary applied —
as an interval with a pointwise key: one salary per employee per instant. The
transform supplies the missing dimension (a ray from the migration epoch).

```ts
const EmployeeId = u64.as("EmployeeId")

// The old theory, judged and fingerprinted:
const EmployeeV1 = relation("Employee", { id: EmployeeId.fresh, name: str })
const SalaryV1 = relation("Salary", { employee: EmployeeId, amount: i64 })
const PayrollV1 = schema("PayrollV1", { Employee: EmployeeV1, Salary: SalaryV1 }, [
	contained(on(SalaryV1, "employee"), on(EmployeeV1, "id"))
])

// The new theory adds what v1 never recorded:
const Employee = relation("Employee", { id: EmployeeId.fresh, name: str })
const Salary = relation("Salary", { employee: EmployeeId, amount: i64, applies: interval(i64) })
const Payroll = schema("Payroll", { Employee, Salary }, [
	contained(on(Salary, "employee"), on(Employee, "id")),
	key(Salary, ["employee", "applies"]) // one salary per instant
])

// The post-migration read — salaries in force at an instant:
const inForceAt = query(Payroll).rule((r) =>
	r
		.match(Employee, { id: r.var("e"), name: r.var("name") })
		.match(Salary, { employee: r.var("e"), amount: r.var("amount"), applies: r.var("w") })
		.where(r.pointIn(r.param("at"), r.var("w")))
		.select("name", "amount")
)
```

The engine's compiled test drives the whole loop (export under one snapshot,
the fingerprint refusal, load order, identity, mint catch-up, judgment); the
SDK pin asserts the two theories carry two distinct fingerprints — the
refusal's premise. For stores whose creating schema is gone, `Db.exhume`
reads the store's own persisted descriptor — the one schema-independent read
path, the E half of ETL.

## Composition

## 29. The zone ledger

Guarantee: Lean theorem + validator/runtime premises — per-kind mutual point
coverage realizes each arm's exact partition
(`lean/Bumbledb/Dependencies.lean: exact_partition_iff`) over one
disjointness witness, and the mixed-width `==` positions type by element
domain (`lean/Bumbledb/Schema.lean: Value.points_one_tag_u64`); witness
segmentation is host discipline (the honesty note below).

Recipe 9's sidecar, composed: a ledger whose timeline divides into zones of
two kinds — unit zones (`interval(u64, 1n)`) and pair zones
(`interval(u64, 2n)`), each kind carrying its own payload sidecar. The
discriminated-union pattern (recipe 2) applied at interval positions: a
kind-discriminated `Zone` witness relation owns **cross-sidecar disjointness**
through its one pointwise key, and since each sidecar's point support equals
its kind's zone support (the per-kind `mirrors`), a unit slot can never
overlap a pair slot even though they live in different relations. The arm
widths are enforced **by type**: a `UnitSlot` value is width 1 or does not
exist — no runtime width check, nothing to enforce at commit.

```ts
const LedgerId = u64.as("LedgerId")

const Kind = closed("Kind", ["Unit", "Pair"])
const Ledger = relation("Ledger", { id: LedgerId.fresh, name: str })
// The witness: every zone of the ledger, kind-discriminated; its one
// pointwise key is the cross-sidecar disjointness proof.
const Zone = relation("Zone", { ledger: LedgerId, kind: Kind.id, at: interval(u64) })
const UnitSlot = relation("UnitSlot", { ledger: LedgerId, at: interval(u64, 1n), entry: u64 })
const PairSlot = relation("PairSlot", { ledger: LedgerId, at: interval(u64, 2n), entry: u64 })

const ZoneLedger = schema("ZoneLedger", { Kind, Ledger, Zone, UnitSlot, PairSlot }, [
	contained(on(Zone, "ledger"), on(Ledger, "id")),
	contained(on(Zone, "kind"), on(Kind, "id")),
	key(Zone, ["ledger", "at"]), // all zones disjoint, whatever the kind
	key(UnitSlot, ["ledger", "at"]),
	key(PairSlot, ["ledger", "at"]),
	// Each kind's zones carry exactly its sidecar's points — mixed widths,
	// one element domain:
	mirrors(on(Zone.where({ kind: Kind.Unit }), ["ledger", "at"]), on(UnitSlot, ["ledger", "at"])),
	mirrors(on(Zone.where({ kind: Kind.Pair }), ["ledger", "at"]), on(PairSlot, ["ledger", "at"]))
])
```

The honesty note — **coalescing insensitivity**: the `mirrors` judgments
compare point supports, not rows. A single Unit-kind zone `[4,6)` beside two
unit slots `[4,5)`, `[5,6)` satisfies both directions, because nothing forces
the witness rows to mirror the sidecar's segmentation — only its points. If
per-row correspondence matters, the host writes zones at slot granularity;
the schema proves disjointness and coverage either way.
