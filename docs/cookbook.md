# The cookbook: modeling intuition as schemas

Worked schemas for anyone writing a theory. Each recipe states the invariant
the database enforces and what stays the application's responsibility; keys,
containments, capacities and query operators keep their ordinary semantics.

Each recipe is one self-contained Rust block. `CookbookDoctests` in
`crates/bumbledb/src/lib.rs` includes this file, so `cargo test --doc`
compiles and runs every block: `schema!` checks the declaration at compile
time, the runtime checker validates it, every query validates against it, and
the recipes that open a store run their writes and reads. A recipe that
disagrees with the engine fails the build.

## Foundations

## 1. The minimal interval schema

Guarantee: the pointwise key enforces per-service disjointness. Checked interval constructors reject empty values.

One fact per outage window; the pointwise key is the whole temporal design.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Uptime;

    relation Service { id: u64 as ServiceId, name: str }
    relation Outage  { service: u64 as ServiceId, window: interval<i64> }

    Service(id) -> Service;
    Outage(service) <= Service(id);
    Outage(service, window) -> Outage;
}

// The Rust query notation constructs an AST directly. Down at instant `t` — point membership
// (`in`) is a typing rule.
let down_at = bumbledb::query!(Uptime {
    (service) | Outage(service, window: w), ?t in w;
});

// Overlapping an incident window — one Allen mask, no operator zoo.
let overlapping = bumbledb::query!(Uptime {
    (service, w) | Outage(service, window: w), Allen(w, INTERSECTS, ?incident);
});

let schema = Uptime.descriptor().validate().expect("the schema checks");
for query in [&*down_at, &*overlapping] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

Total downtime can be computed in native stages: `measure` each bounded interval,
then sum in a following stage. Use `pack` first when overlapping outages should
count once. Native `FindTerm::Segments` constructs intervals;
`ScalarExpr::Measure` computes width. The [staged recipes](../ts/COOKBOOK.md#13-derive-slices-measure-them-and-round-the-total)
show the public algebra and contribution identity end to end.

## 2. Discriminated unions

Guarantee: key-backed equality requires exactly one corresponding row on each side. Both projections must resolve to declared keys.

Sum-typed entities: a closed-relation discriminator plus per-arm child
relations, connected by bidirectional conditional containments.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Grading;

    closed relation Kind as KindId = { Deterministic, CustomOperator };

    relation Task { id: u64 as TaskId, kind: u64 as KindId }
    relation DeterministicGrading  { task: u64 as TaskId, tolerance: i64 }
    relation CustomOperatorGrading { task: u64 as TaskId, operator: str }

    Task(id) -> Task;
    Task(kind) <= Kind(id);
    DeterministicGrading(task)  -> DeterministicGrading;
    CustomOperatorGrading(task) -> CustomOperatorGrading;
    Task(id | kind == Deterministic)  == DeterministicGrading(task);
    Task(id | kind == CustomOperator) == CustomOperatorGrading(task);
}

let schema = Grading.descriptor().validate().expect("the schema checks");
let _ = schema;
```

The TypeScript `alternatives` helper constructs these ordinary laws from a
parent scalar key, closed roster, and child keys. Declare supplied keys once;
the expansion follows roster order and has the same descriptor and fingerprint
as manual laws. Composite scalar identities work. Interval identity projections
express coverage and cannot prove exactly one payload row. An arm switch plus
payload replacement is one atomic final-state change. Evidence and ownership
remain explicit laws. See the [complete declaration](../ts/COOKBOOK.md#15-exhaustive-alternatives-with-ordinary-laws).

## 3. 0..1 optional attributes

Guarantee: the child key allows at most one fact, and containment requires its parent. Absence remains legal.

No nulls, anywhere. Optional data is an absent fact in a child relation; the
child's key plus a one-way containment *is* "nullable column", done honestly.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Optionality;

    relation Business { id: u64 as BusinessId, name: str }
    relation MailingAddress { business: u64 as BusinessId, line: str, city: str }

    Business(id) -> Business;
    MailingAddress(business) -> MailingAddress;
    MailingAddress(business) <= Business(id);
}

// Negation is plain anti-join (no null branch exists in any operator).
let unaddressed = bumbledb::query!(Optionality {
    (b) | Business(id: b), !MailingAddress(business: b);
});

let schema = Optionality.descriptor().validate().expect("the schema checks");
for query in [&*unaddressed] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 4. Money

Guarantee: host discipline + validator premises — fixed-point scale and
currency grouping live in host newtypes; containments only resolve references.

Fixed-point i64 minor units; the host newtype owns scale and currency.
Native `mulDiv` supports exact bounded integer proration with explicit rounding.
FX policy, rate evidence, and unit discipline remain application responsibilities.

Multi-currency totals: currency is a group key, never summed across — `Sum`
folds in i128 with one final range check, so totals cannot wrap silently.
Bind the application-owned id: set semantics would collapse two equal
(account, currency, minor) postings without it.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Money;

    closed relation Currency as CurrencyId = { Usd, Eur, Gbp };

    relation Account { id: u64 as AccountId, name: str }
    relation Posting {
        id: u64 as PostingId,
        account: u64 as AccountId,
        currency: u64 as CurrencyId,
        minor: i64 as Minor,
    }

    Account(id) -> Account;
    Posting(account)  <= Account(id);
    Posting(currency) <= Currency(id);
}

let totals = bumbledb::query!(Money {
    (account, currency, total: Sum(minor)) | Posting(id, account, currency, minor);
});

let schema = Money.descriptor().validate().expect("the schema checks");
for query in [&*totals] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 5. Content addressing

Guarantee: validator/runtime premises + host discipline — the payload key and
containments enforce identity/reference shape; hashing and blob durability stay external.

Use `str` for text and `bytes<N>` for a fixed-width binary identifier or
digest. Text values are not an application-visible persistent intern table;
choose the type for its meaning, not an assumed storage compression ratio.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Content;

    closed relation Region as RegionId = { Us, Eu };

    relation Document {
        id: u64 as DocumentId,
        name: str,
        payload: bytes<32> as PayloadHash,
    }
    relation Replica { payload: bytes<32> as PayloadHash, region: u64 as RegionId }

    Document(id) -> Document;
    Document(payload) -> Document;
    Replica(payload) <= Document(payload);
    Replica(region)  <= Region(id);
}

// The lookup by digest — a bytes param self-encodes.
let by_digest = bumbledb::query!(Content {
    (id) | Document(id, payload == ?digest);
});

let schema = Content.descriptor().validate().expect("the schema checks");
for query in [&*by_digest] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## Vocabularies

## 6. The vocabulary

Guarantee: the closed extension is fixed by the schema. Member-set containment admits only the declared priority handles.

The enum idiom's replacement, first-class: a vocabulary is a **closed
relation** — its ground axioms are declared in the schema, sealed at
validate, and included in schema identity. Closed facts come from the schema,
not an ordinary mutable relation. Handles are literals in statements, queries,
Plan introspection, errors.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Tickets;

    closed relation Priority as PriorityId = { Low, Normal, Urgent };

    relation Ticket {
        id: u64 as TicketId,
        priority: u64 as PriorityId,
        opened_at: i64,
    }

    Ticket(id) -> Ticket;
    Ticket(priority) <= Priority(id);
}

// Handles are literals in queries exactly as in statements, and the renderer prints them back
// — the round trip runs on names. A query atom over the vocabulary can be resolved during
// preparation.
let urgent = bumbledb::query!(Tickets {
    (t) | Ticket(id: t, priority == Urgent);
});

let schema = Tickets.descriptor().validate().expect("the schema checks");
for query in [&*urgent] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 7. The classification

Guarantee: validator/runtime premise — closed payload facts and ψ-selected
containment restrict certificates to the compiled mastered-handle set.

The fused form: the vocabulary carries its intrinsic facts as **payload
columns** — one ground axiom per handle, values sealed with the schema, read by
ψ-selections. Axioms are declared, never written: an ordinary relation the
application seeded at startup would need every deployment to re-verify it.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Review;

    closed relation Kind as KindId {
        mastered: bool,
        rank: u64,
    } = {
        DirectPass { mastered: true,  rank: 30 },
        JudgedPass { mastered: true,  rank: 20 },
        Failed     { mastered: false, rank: 10 },
    };

    relation Attempt { id: u64 as AttemptId, kind: u64 as KindId }
    relation Certificate { attempt: u64 as AttemptId, kind: u64 as KindId }

    Attempt(id) -> Attempt;
    Attempt(kind) <= Kind(id);
    Certificate(attempt) -> Certificate;
    Certificate(attempt) <= Attempt(id);
    Certificate(kind) <= Kind(id | mastered == true);
}

// The classification read duplicates no flag onto `Attempt` — ψ walks the vocabulary's payload
// in the query too.
let mastered = bumbledb::query!(Review {
    (a) | Attempt(id: a, kind: k), Kind(id: k, mastered == true);
});

let schema = Review.descriptor().validate().expect("the schema checks");
for query in [&*mastered] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

The `Kind` atom folds at prepare into a plan-constant handle set on its
sibling; plan introspection prints the set, not a count:
`folded: Kind{mastered == true} → {DirectPass, JudgedPass}`.

## 8. The sub-vocabulary

Guarantee: validator/runtime premise — ψ over the sealed extension compiles the
exact paging member set; a nonmember write is commit-rejected.

The ψ-selected containment: a reference constrained to the facts of a
vocabulary that satisfy a payload selection. Because the target is closed,
the enforcement plan is not a probe strategy — it is **the answer set
itself**, compiled during schema validation.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Oncall;

    closed relation Severity as SeverityId {
        pages: bool,
    } = {
        Info     { pages: false },
        Warning  { pages: false },
        Critical { pages: true },
        Fatal    { pages: true },
    };

    relation Incident {
        id: u64 as IncidentId,
        severity: u64 as SeverityId,
    }
    relation Escalation {
        incident: u64 as IncidentId,
        severity: u64 as SeverityId,
        at: i64,
    }

    Incident(id) -> Incident;
    Incident(severity) <= Severity(id);
    Escalation(incident) <= Incident(id);
    Escalation(severity) <= Severity(id | pages == true);
}

// Who is being paged — the same ψ, on the read side.
let paged = bumbledb::query!(Oncall {
    (i) | Escalation(incident: i, severity: s), Severity(id: s, pages == true);
});

let schema = Oncall.descriptor().validate().expect("the schema checks");
for query in [&*paged] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## Structure

## 9. Ordered collections

Guarantee: mutual point coverage plus pointwise keys enforces an exact partition. Interval positions match by element domain regardless of width refinement. The application orders results for presentation.

A linked list is control flow smuggled into data: with a `next` pointer per
item, every reorder becomes a dependent chain of writes. Order is a value.
The idiomatic ordered collection is an interval partition, spelled as a
**triple**:

- **the entity** — `Playlist`, ordinary identity;
- **the extent as a 0..1 child** — not a field on the entity, and the reason
  is forcing, not taste: *empty lists exist, empty intervals do not*. A
  playlist with no tracks has no span to record, and interval values are
  nonempty by construction, so the span lives in an optional child (recipe
  3's shape) whose presence *is* nonemptiness;
- **the unit-slot child** — each track occupies `interval<u64, 1>` (the
  width is the type: one stored word, a wrong-width value unrepresentable),
  and the exact-partition `==` (recipe 26) forces the slots to tile the
  extent exactly: a gap aborts, an overlap aborts (the pointwise key
  convicts it before coverage even runs).

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Playlists;

    relation Playlist { id: u64 as PlaylistId, name: str }
    relation Extent { playlist: u64 as PlaylistId, span: interval<u64> }
    relation Slot { playlist: u64 as PlaylistId, slot: interval<u64, 1>, track: str }

    Playlist(id) -> Playlist;
    Extent(playlist) <= Playlist(id);
    Slot(playlist)   <= Playlist(id);
    Extent(playlist) -> Extent;
    Extent(playlist, span) -> Extent;
    Slot(playlist, slot) -> Slot;
    Extent(playlist, span) == Slot(playlist, slot);
}

// Positional access is membership — "what plays at position `?pos`".
let at_pos = bumbledb::query!(Playlists {
    (track) | Slot(playlist == ?list, slot: s, track), ?pos in s;
});

let schema = Playlists.descriptor().validate().expect("the schema checks");
for query in [&*at_pos] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

Middle insert is honest about its cost: making room at position `k` shifts
every later slot and grows the extent — O(k) writes in **one delta**, judged
once at commit, so the partition never passes through an invalid
intermediate state. That is the price of exact density; pay it knowingly.

If middle inserts dominate and exact density is not worth their cost, the
demoted escape hatch is the spread slot: a scalar `pos: u64` column written
in gapped strides (1024, 2048, ...) under the same composite key, insertion
between neighbors one write, renumbering an amortized rarity. That is the
whole hatch — bumbledb has no lexicographic fractional indexing, because
string order is refused: there is no
"between two strings" to allocate.

## 10. Trees and ASTs

Guarantee: key-backed arms enforce each variant's row correspondence. The application enforces acyclicity; these declarations alone do not establish a tree.

Node header + per-kind arms (recipe 2's pattern); every edge resolves; the
shape theorems come from FDs on the edge relations.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Ast;

    closed relation Kind as KindId = { Lit, Add };

    relation Node { id: u64 as NodeId, kind: u64 as KindId }
    relation Lit  { node: u64 as NodeId, value: i64 }
    relation Add  { node: u64 as NodeId, lhs: u64 as NodeId, rhs: u64 as NodeId }
    relation Parent { child: u64 as NodeId, parent: u64 as NodeId }

    Node(id) -> Node;
    Node(kind) <= Kind(id);
    Lit(node) -> Lit;
    Add(node) -> Add;
    Node(id | kind == Lit) == Lit(node);
    Node(id | kind == Add) == Add(node);
    Add(lhs) <= Node(id);
    Add(rhs) <= Node(id);
    Parent(child) -> Parent;
    Parent(child)  <= Node(id);
    Parent(parent) <= Node(id);
}

// A node's left operand, when it is a literal — the arm join.
let lhs_lit = bumbledb::query!(Ast {
    (v) | Add(node == ?n, lhs: l), Lit(node: l, value: v);
});

let schema = Ast.descriptor().validate().expect("the schema checks");
for query in [&*lhs_lit] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 11. Typed graphs

Guarantee: validator/runtime premises — endpoint containments type each edge
and composite keys deduplicate pairs; no transitive graph property is claimed.

One relation per edge kind: the edge vocabulary is closed and checked —
endpoint containments pin which node kinds each edge may touch.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Graph;

    relation Person { id: u64 as PersonId, name: str }
    relation Repo   { id: u64 as RepoId, name: str }
    relation Follows   { follower: u64 as PersonId, followee: u64 as PersonId }
    relation Maintains { person: u64 as PersonId, repo: u64 as RepoId }

    Person(id) -> Person;
    Repo(id) -> Repo;
    Follows(follower) <= Person(id);
    Follows(followee) <= Person(id);
    Follows(follower, followee) -> Follows;
    Maintains(person) <= Person(id);
    Maintains(repo)   <= Repo(id);
    Maintains(person, repo) -> Maintains;
}

// Mutual follows — joins are explicit `field: v` on both ends (the punning law); `<` keeps
// each pair once.
let mutual = bumbledb::query!(Graph {
    (a, b) | Follows(follower: a, followee: b),
             Follows(follower: b, followee: a), a < b;
});

let schema = Graph.descriptor().validate().expect("the schema checks");
for query in [&*mutual] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 12. Entity-component

Guarantee: definition + validator/runtime premises — component keys give 0..1
and containments require the stated entity/archetype facts.

The 0..1 idiom (recipe 3) at scale: components are child relations; an
entity has a component iff the fact exists; a new component kind is a new
relation, not a wider fact.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Ecs;

    relation Entity { id: u64 as EntityId, name: str }
    relation Transform  { entity: u64 as EntityId, x: i64, y: i64 }
    relation Velocity   { entity: u64 as EntityId, dx: i64, dy: i64 }
    relation Renderable { entity: u64 as EntityId, mesh: str }

    Entity(id) -> Entity;
    Transform(entity)  -> Transform;
    Transform(entity)  <= Entity(id);
    Velocity(entity)   -> Velocity;
    Velocity(entity)   <= Entity(id);
    Renderable(entity) -> Renderable;
    Renderable(entity) <= Transform(entity);
}

// The physics join is the component intersection.
let physics = bumbledb::query!(Ecs {
    (e, x, y, dx, dy) | Transform(entity: e, x, y), Velocity(entity: e, dx, dy);
});

let schema = Ecs.descriptor().validate().expect("the schema checks");
for query in [&*physics] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 13. State machines

Guarantee: key-backed equality requires the selected state's evidence. The application enforces which state transitions are allowed.

States are a discriminated union; per-state data lives in arms; and the
conditional reference target — a reference to "an order *that is shipped*" —
is one selected containment, the statement SQL cannot write.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Orders;

    closed relation State as StateId = { Cart, Placed, Shipped };

    relation Order { id: u64 as OrderId, state: u64 as StateId }
    relation Placement { order: u64 as OrderId, at: i64 }
    relation Shipment  { order: u64 as OrderId, carrier: str, at: i64 }

    Order(id) -> Order;
    Order(state) <= State(id);
    Placement(order) -> Placement;
    Shipment(order)  -> Shipment;
    Placement(order) <= Order(id);
    Shipment(order) == Order(id | state == Shipped);
}

// Shipped orders with their carriers.
let shipped = bumbledb::query!(Orders {
    (id, carrier) | Order(id, state == Shipped), Shipment(order: id, carrier);
});

let schema = Orders.descriptor().validate().expect("the schema checks");
for query in [&*shipped] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## Time and coverage

## 14. The calendar core

Guarantee: equality enforces key-backed correspondence. Pointwise keys and coverage enforce the declared hard constraints.

Policy as schema: hard rules are pointwise keys, soft rules are the statements
you decline to write.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Calendar;

    closed relation Rsvp as RsvpId = { Accepted, Tentative, Declined };
    closed relation Arm as ArmId = { Busy, Ooo };

    relation Person { id: u64 as PersonId, name: str }
    relation Room   { id: u64 as RoomId, name: str }
    relation Event  { id: u64 as EventId, span: interval<i64> }
    relation Attendance {
        id: u64 as AttendanceId,
        event: u64 as EventId,
        person: u64 as PersonId,
        rsvp: u64 as RsvpId,
    }
    relation Claim {
        source: u64 as AttendanceId,
        person: u64 as PersonId,
        arm: u64 as ArmId,
        span: interval<i64>,
    }
    relation Booking   { room: u64 as RoomId, event: u64 as EventId, span: interval<i64> }
    relation WorkHours { person: u64 as PersonId, hours: interval<i64> }

    Person(id) -> Person;
    Room(id) -> Room;
    Event(id) -> Event;
    Attendance(id) -> Attendance;
    Attendance(event)  <= Event(id);
    Attendance(person) <= Person(id);
    Attendance(rsvp)   <= Rsvp(id);
    Attendance(event, person) -> Attendance;
    Claim(source) -> Claim;
    Claim(person) <= Person(id);
    Claim(arm)    <= Arm(id);
    Booking(room, span) -> Booking;
    Attendance(id | rsvp == Accepted) == Claim(source | arm == Busy);
    WorkHours(person, hours) -> WorkHours;
    Claim(person, span | arm == Busy) <= WorkHours(person, hours);
    Booking(room)  <= Room(id);
    Booking(event) <= Event(id);
}

// The conflict probes — one Allen mask against a param, on rooms and on people.
let room_conflicts = bumbledb::query!(Calendar {
    (room, s) | Booking(room, span: s), Allen(s, INTERSECTS, ?want);
});

let busy_people = bumbledb::query!(Calendar {
    (person, s) | Claim(person, span: s), Allen(s, INTERSECTS, ?window);
});

let schema = Calendar.descriptor().validate().expect("the schema checks");
for query in [&*room_conflicts, &*busy_people] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 15. Effective-dated configuration

Guarantee: pointwise keys prevent overlap, and one-way containment requires source coverage. Target intervals may extend beyond that source.

Versioned rules: no overlaps (pointwise key), no gaps in the policy's source
lifetime (one-way coverage; version overhang remains legal), and "in force on
date t" is one membership probe.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Pricing;

    relation Policy  { id: u64 as PolicyId, live: interval<i64> }
    relation Version { policy: u64 as PolicyId, rate_bps: i64, valid: interval<i64> }

    Policy(id) -> Policy;
    Version(policy) <= Policy(id);
    Version(policy, valid) -> Version;
    Policy(id, live) <= Version(policy, valid);
}

// In force on date `t` — one membership probe.
let in_force = bumbledb::query!(Pricing {
    (rate_bps) | Version(policy == ?p, rate_bps, valid: v), ?t in v;
});

// Clean successions — half-open makes MEETS exact, no ±1 fudge.
let successions = bumbledb::query!(Pricing {
    (a, b) | Version(policy: p, valid: a), Version(policy: p, valid: b),
             Allen(a, MEETS, b);
});

let schema = Pricing.descriptor().validate().expect("the schema checks");
for query in [&*in_force, &*successions] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 16. Disjoint covers

Guarantee: containment requires source coverage. A target may overhang the source; exact partition requires constraints in both directions.

Pay periods, shifts, estimated-tax quarters: a pointwise key plus one-way
coverage is a **disjoint cover** — no overlaps among pay periods and no holes
in the fiscal year's source span. Pay periods may extend beyond that span;
target overhang is legal under this statement. Historically this pattern was
called a tiling here; that was stronger than the judgment actually proved.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Payroll;

    relation FiscalYear { id: u64 as FiscalYearId, span: interval<i64> }
    relation PayPeriod  { year: u64 as FiscalYearId, seq: u64, span: interval<i64> }

    FiscalYear(id) -> FiscalYear;
    PayPeriod(year) <= FiscalYear(id);
    PayPeriod(year, seq)  -> PayPeriod;
    PayPeriod(year, span) -> PayPeriod;
    FiscalYear(id, span) <= PayPeriod(year, span);
}

// The period holding date `t`.
let holding = bumbledb::query!(Payroll {
    (seq) | PayPeriod(year == ?y, seq, span: s), ?t in s;
});

let schema = Payroll.descriptor().validate().expect("the schema checks");
for query in [&*holding] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 17. Federal income tax

Guarantee: validator/runtime premises + host discipline — keys prove bracket
disjointness and statements prove residency coverage; full bracket coverage and proration are host duties.

Brackets are intervals over money; the top bracket is a ray; regimes key on
(year, status). Native staged queries can derive finite overlaps and prorated
amounts from the stored facts.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Tax;

    closed relation Status as StatusId = { Single, MarriedJoint, HeadOfHousehold };

    relation Regime {
        id: u64 as RegimeId,
        year: i64,
        status: u64 as StatusId,
    }
    relation Bracket { regime: u64 as RegimeId, income: interval<i64>, rate_bps: i64 }
    relation Residency { person: u64, span: interval<i64> }
    relation Earned { person: u64, regime: u64 as RegimeId, span: interval<i64>, minor: i64 }

    Regime(id) -> Regime;
    Regime(status) <= Status(id);
    Regime(year, status) -> Regime;
    Bracket(regime) <= Regime(id);
    Bracket(regime, income) -> Bracket;
    Earned(regime) <= Regime(id);
    Residency(person, span) -> Residency;
    Earned(person, span) <= Residency(person, span);
}

// The marginal bracket — membership probes the disjoint bracket set.
let marginal = bumbledb::query!(Tax {
    (rate_bps) | Regime(id: r, year == ?y, status == ?s),
                 Bracket(regime: r, income: b, rate_bps), ?taxable in b;
});

let schema = Tax.descriptor().validate().expect("the schema checks");
for query in [&*marginal] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

A native nonrecursive pipeline can intersect earning intervals with each bracket,
measure overlaps, preserve contribution identity, sum weighted amounts, and apply
`mulDiv` once to the total. Keep earning and band identity through the weighted
projection so equal-valued contributions both count. Rounding slices separately
changes the calculation. See the [executable TypeScript recipe](../ts/COOKBOOK.md#13-derive-slices-measure-them-and-round-the-total).

`ScalarExpr::MulDiv { a, b, divisor, rounding }` uses an exact wide product and
checks its 64-bit result after rounding. `Rounding` has `TowardZero`,
`NearestTiesAwayFromZero`, and `NearestTiesToEven`. A positive divisor and matching
integer kinds are required. Existing checked multiplication still rejects its own
intermediate overflow. Queries use this native arithmetic directly; application transformations use
ordinary imperative code and can query these expressions.

## 18. Free time and coalescing

Guarantee: `Pack` coalesces result intervals. It does not impose stored disjointness, completeness, or automatic maintenance.

`Pack` is Snodgrass's coalesce as an aggregate — maximal disjoint segments per
group, one answer per (group, segment). Coalescing is never a write rule: the
engine stores the claims it was given.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub FreeTime;

    relation Person { id: u64 as PersonId, name: str }
    relation Claim  { person: u64 as PersonId, span: interval<i64> }

    Person(id) -> Person;
    Claim(person) <= Person(id);
}

// Busy time, coalesced — adjacent segments merge, the half-open law.
let busy = bumbledb::query!(FreeTime {
    (person, busy: Pack(span)) | Claim(person, span);
});

let schema = FreeTime.descriptor().validate().expect("the schema checks");
for query in [&*busy] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

Raw claimed time can be measured natively. Summing overlapping claims
double-counts; use the existing `pack` stage when the intended quantity is union coverage.

Coalesced totals use successive `pack`, `measure`, and `sum` stages.
Binary `difference` constructs up to two gaps against one interval. Subtracting
an arbitrary relation of busy intervals still requires a coverage sweep;
unioning pairwise differences is incorrect.

`Interval::intersection` returns zero or one interval; `Interval::difference`
returns zero, one, or two maximal nonempty pieces. Query `FindTerm::Segments`
uses those endpoint-only operations on bound inputs; independent producers form
a Cartesian product. Their results preserve element kind and discard unproved
fixed-width refinements. A following stage can join, negate, measure, or pack
those outputs. The current macro grammar does not parse general computed trees;
the public structural Rust IR is the authoring path.

Subtracting `[3,7)` from `[0,10)` produces `[0,3)` and `[7,10)`. Subtracting
the whole interval produces no rows. Each subtraction has one interval operand;
unioning separate differences is not subtraction of combined coverage.

Integer measure is exact `u64` width for bounded signed or unsigned intervals.
Integer maximum endpoints denote rays. Dense measure preserves the native
once-rounded endpoint difference; finite overflow and unboundedness refuse
distinctly. Clip a ray before measuring it. Later filtering cannot hide an
upstream failure. See the [difference, pack, and measure pipeline](../ts/COOKBOOK.md#14-subtract-a-window-coalesce-coverage-and-measure).

## The write side

## 19. The ledger

Guarantee: checked sums reject overflow. Posting references are database constraints; the application enforces double-entry arithmetic agreement.

The ledger workload. Balance is a query, never a column.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Ledger;

    relation Account      { id: u64 as AccountId, name: str }
    relation JournalEntry { id: u64 as JournalEntryId, at: i64, memo: str }
    relation Posting {
        id: u64 as PostingId,
        entry: u64 as JournalEntryId,
        account: u64 as AccountId,
        minor: i64,
    }

    Account(id) -> Account;
    JournalEntry(id) -> JournalEntry;
    Posting(id) -> Posting;
    Posting(entry)   <= JournalEntry(id);
    Posting(account) <= Account(id);
}

// Balances — bind the application-owned id, or set semantics collapses duplicates.
let balances = bumbledb::query!(Ledger {
    (account, total: Sum(minor)) | Posting(id, account, minor);
});

// The double-entry audit — the host asserts every total is 0; discipline, not schema.
let audit = bumbledb::query!(Ledger {
    (entry, Sum(minor)) | Posting(id, entry, minor);
});

let schema = Ledger.descriptor().validate().expect("the schema checks");
for query in [&*balances, &*audit] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 20. Conditional writes

Guarantee: a generation witness rejects a write derived from a snapshot that has since moved. The application owns retries. Point reads inside a write see its final state and need no earlier witness.

The generation witness: read the model, propose a delta, commit only if the
model you read is still the model. Three idioms:

- **Update-where**: query the premise on a snapshot, then `delete(old)` and
  `insert(new)` for each matched fact in `write_from(&witness)`. "Still
  Queued" is the witness.
- **Insert-select**: query source answers on a snapshot and insert the derived
  facts with `write_from`, witnessing the same snapshot.
- **Read-modify-write, key-shaped**: point reads (`get`, `contains`) inside
  `write` see the final state, so a per-fact premise needs no witness.

A moved witness is `WriteOutcome::Moved`; nothing was written, and the host
starts again from a new snapshot rather than retrying the stale delta.

```rust
use bumbledb::{AnswerValue, Db, WorkContext, WriteOutcome, WriteTx};

bumbledb::schema! {
    pub Jobs;

    closed relation State as StateId = { Queued, Running, Done };

    relation Job {
        id: u64 as JobId,
        state: u64 as StateId,
        payload: str,
    }
    relation Lease { job: u64 as JobId, worker: u64, until: i64 }

    Job(id) -> Job;
    Job(state) <= State(id);
    Lease(job) -> Lease;
    Lease(job) == Job(id | state == Running);
}

let queued = bumbledb::query!(Jobs {
    (id, payload) | Job(id, state == Queued, payload);
});

let dir = std::env::temp_dir().join(format!("cookbook-jobs-{}.bdb", std::process::id()));
let db = Db::create(&dir, Jobs, WorkContext::new())?.expect("the empty store admits");
db.write(WorkContext::new(), |tx| {
    tx.insert([&Job { id: JobId(1), state: State::Queued.id(), payload: "resize" }])
})?
.expect("the job is queued");

// Update-where: find the premise on a snapshot and keep its witness.
let work = WorkContext::new();
let snapshot = db.snapshot(&work)?;
let claimed: Vec<(u64, String)> = {
    let frame = snapshot.frame(&work);
    let mut prepared = frame.prepare(&queued)?;
    let answers = frame.execute_collect(&mut prepared, &queued.bind(bumbledb::params! {}))?;
    answers
        .answers()
        .map(|answer| match (answer.get(0), answer.get(1)) {
            (AnswerValue::U64(id), AnswerValue::String(payload)) => (id, payload.to_owned()),
            other => panic!("the find is (u64, str), not {other:?}"),
        })
        .collect()
};
let witness = snapshot.witness();
drop(snapshot);

let claim = |tx: &mut WriteTx<'_, Jobs>| -> bumbledb::Result<()> {
    for (id, payload) in &claimed {
        tx.delete([&Job { id: JobId(*id), state: State::Queued.id(), payload }])?;
        tx.insert([&Job { id: JobId(*id), state: State::Running.id(), payload }])?;
        tx.insert([&Lease { job: JobId(*id), worker: 7, until: 60 }])?;
    }
    Ok(())
};
let committed = db.write_from(WorkContext::new(), &witness, &claim)?.expect("nothing moved");
assert!(committed.changed);

// The claim moved the store, so the same witness now refuses the whole write.
let retry = db.write_from(WorkContext::new(), &witness, &claim)?;
assert!(matches!(retry, WriteOutcome::Moved { .. }));

// Key-shaped read-modify-write: the point read sees the write's own final state.
db.write(WorkContext::new(), |tx| {
    let until = tx.get(LeaseByJob { job: JobId(1) })?.expect("a running job has a lease").until;
    tx.delete([&Lease { job: JobId(1), worker: 7, until }])?;
    tx.insert([&Lease { job: JobId(1), worker: 7, until: until + 60 }])?;
    Ok(())
})?
.expect("the lease is extended");

drop(db);
std::fs::remove_dir_all(&dir)?;
Ok::<(), Box<dyn std::error::Error>>(())
```

## 21. Derived relations

Guarantee: containment rejects unsupported stored facts. The application maintains completeness and refreshes missing derived facts.

The materialized view as a relation under statements — unsoundness the schema
can name is uncommittable; incompleteness remains representable until the host
refreshes it.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Rollup;

    closed relation Arm as ArmId = { Busy, Ooo };

    relation Claim {
        source: u64,
        person: u64,
        arm: u64 as ArmId,
        span: interval<i64>,
    }
    relation BusySpan { person: u64, span: interval<i64> }

    Claim(arm) <= Arm(id);
    Claim(source) -> Claim;
    Claim(person, span) -> Claim;
    BusySpan(person, span) -> BusySpan;
    BusySpan(person, span) <= Claim(person, span | arm == Busy);
}

// Maintenance is the third witness idiom (recipe 20): re-run the deriving query on a
// snapshot, diff, `write_from(&witness)`; the rollup cannot commit against sources it did not
// read (recipe 27 runs the loop). The deriving query (`Pack` IS the coalesce).
let deriving = bumbledb::query!(Rollup {
    (person, busy: Pack(span)) | Claim(person, span, arm == Busy);
});

let schema = Rollup.descriptor().validate().expect("the schema checks");
for query in [&*deriving] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 22. Union reads

Guarantee: rule union uses set semantics. Deduplication absorbs repeated derivations while retaining distinct bindings needed by aggregates.

The whole-DU read is a set of rules: one head, one rule per arm — disjunction
is data at the top, never an execution node.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Payments;

    closed relation Kind as KindId = { Card, Ach };

    relation Payment { id: u64 as PaymentId, kind: u64 as KindId }
    relation Card { payment: u64 as PaymentId, last4: u64 }
    relation Ach  { payment: u64 as PaymentId, routing: u64 }

    Payment(id) -> Payment;
    Payment(kind) <= Kind(id);
    Card(payment) -> Card;
    Ach(payment)  -> Ach;
    Payment(id | kind == Card) == Card(payment);
    Payment(id | kind == Ach)  == Ach(payment);
}

// One query, two rules (set union). The exclusivity theorem (recipe 2) is spent a third time
// here: rules selecting different `kind` values are disjoint. The union still has set
// semantics; do not depend on bag-like duplicates when composing rules.
let methods = bumbledb::query!(Payments {
    (id, n) | Payment(id, kind == Card), Card(payment: id, last4: n);
    (id, n) | Payment(id, kind == Ach), Ach(payment: id, routing: n);
});

let schema = Payments.descriptor().validate().expect("the schema checks");
for query in [&*methods] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 23. The anti-recipes

Guarantee: none of its own. Each entry names a shape not to model and its
representable replacement; the replacements carry the guarantees of the
recipes they cite.

What not to model, and what to model instead. The block declares the
replacements.

- **A `next` pointer per row.** Order is a value: `Step(flow, pos) -> Step`
  keys each position, and recipe 9 makes the positions an exact partition.
- **A float where the quantity is exact.** A score in basis points is an
  `i64` in a declared unit (recipe 4); `f64` is for measured quantities.
- **An `active` flag.** "The current run" is a 0..1 relation keyed by its
  owner, `ActiveRun(student) -> ActiveRun`, which a key can police and a
  boolean column cannot (recipe 3).
- **A counter updated in place.** Consumption is an interval fact under a
  pointwise key, `Usage(meter, used) -> Usage`, and totals are queries
  (recipe 19).
- **Identity or time issued by the database.** The application supplies the
  `id` and the `at` of an `Event`; the database issues no ids and reads no
  clock.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Replacements;

    relation Step { flow: u64, pos: u64, action: str }
    relation Score { subject: u64, bps: i64 }
    relation ActiveRun { student: u64, run: u64 }
    relation Usage { meter: u64, period: u64, used: interval<i64> }
    relation Event { id: u64 as EventId, at: i64 }

    Event(id) -> Event;
    Step(flow, pos)    -> Step;
    Score(subject)     -> Score;
    ActiveRun(student) -> ActiveRun;
    Usage(meter, used) -> Usage;
}

let schema = Replacements.descriptor().validate().expect("the schema checks");
let _ = schema;
```

## Host-driven closure

## 24. The closure idiom

Guarantee: the host loop terminates because its seen set grows strictly
inside a finite node set. The native form runs the linear reach driver under
the execution budgets.

Reachability, in two dialects. Most real hierarchies (org charts, charts of
accounts, category trees) are depth-bounded, so a host loop runs depth-many
rounds and each round is one query with a set parameter. The frontier
discipline is semi-naive evaluation's delta, spent in the host.

When the loop's costs bite (unbounded or large depth, or a closure that must
join further inside one plan), write the engine-native form: the same closure
as one linear `rec`. `?root` seeds it, the bare rule is the required main, and
the driver runs the rounds inside one plan. Cycle detection with `reach(x, x)`
is the same family: a linear rec plus a main join of the finished table.

What stays host-side is interval intersection along paths: the recursion
surface excludes it, so the host carries the window in its frontier, one
intersection per hop.

```rust
use std::collections::BTreeSet;

use bumbledb::{AnswerValue, Answers, Db, Value, WorkContext};

bumbledb::schema! {
    pub Closure;

    relation Node   { id: u64 as NodeId, name: str }
    relation Parent { child: u64 as NodeId, parent: u64 as NodeId }

    Node(id) -> Node;
    Parent(child) -> Parent;
    Parent(child)  <= Node(id);
    Parent(parent) <= Node(id);
}

// The loop's one query: the frontier's children.
let children = bumbledb::query!(Closure {
    (c) | Parent(child: c, parent in ?frontier);
});

let native = bumbledb::query!(Closure {
    rec reach(c) | Node(id: c), c == ?root;
    rec reach(c) | Parent(child: c, parent: m), reach(m);
    (c) | reach(c);
});

let dir = std::env::temp_dir().join(format!("cookbook-closure-{}.bdb", std::process::id()));
let db = Db::create(&dir, Closure, WorkContext::new())?.expect("the empty store admits");
db.write(WorkContext::new(), |tx| {
    for (id, name) in [(1, "root"), (2, "a"), (3, "b"), (4, "a.1"), (5, "a.1.x"), (6, "other")] {
        tx.insert([&Node { id: NodeId(id), name }])?;
    }
    for (child, parent) in [(2, 1), (3, 1), (4, 2), (5, 4)] {
        tx.insert([&Parent { child: NodeId(child), parent: NodeId(parent) }])?;
    }
    Ok(())
})?
.expect("the tree commits");

let ids = |answers: &Answers| -> BTreeSet<u64> {
    answers
        .answers()
        .map(|answer| match answer.get(0) {
            AnswerValue::U64(id) => id,
            other => panic!("a node id, not {other:?}"),
        })
        .collect()
};

// The host loop: one set-param query per depth.
let mut step = db.prepare(&children, WorkContext::new())?;
let mut seen = BTreeSet::from([1]);
let mut frontier = vec![Value::U64(1)];
while !frontier.is_empty() {
    let next = db.read(WorkContext::new(), |frame| {
        frame.execute_collect(&mut step, &children.bind(bumbledb::params! { frontier: frontier.as_slice() }))
    })?;
    frontier = ids(&next).into_iter().filter(|id| seen.insert(*id)).map(Value::U64).collect();
}

// The native form: the same rounds inside one plan.
let mut reach = db.prepare(&native, WorkContext::new())?;
let reached = db.read(WorkContext::new(), |frame| {
    frame.execute_collect(&mut reach, &native.bind(bumbledb::params! { root: 1u64 }))
})?;
assert_eq!(ids(&reached), seen);
assert_eq!(seen, BTreeSet::from([1, 2, 3, 4, 5]));

drop((step, reach, db));
std::fs::remove_dir_all(&dir)?;
Ok::<(), Box<dyn std::error::Error>>(())
```

## 25. The chart of accounts

Guarantee: the host computes the closure before one checked sum. The native
form likewise aggregates over the finished recursive relation.

The ledger's real recursion case, in the same two dialects: a hierarchical
chart of accounts and a subtree rollup. In the host composition, recipe 24's
loop accumulates the subtree as a set, then one `Sum` query over that set
folds the postings: the engine aggregates, the host composes. The native form
is one query. Aggregation through the rec cycle is refused
(`AggregateInInterior` on a rec head), but a fold over the finished rec is an
ordinary main: the rec converges first, then the fold runs once.

The rollup binds the application-owned posting id, so two equal postings to
one account both count (recipe 19).

```rust
use std::collections::BTreeSet;

use bumbledb::{AnswerValue, Answers, Db, Value, WorkContext};

bumbledb::schema! {
    pub Accounts;

    relation Account { id: u64 as AccountId, name: str }
    relation AccountParent { child: u64 as AccountId, parent: u64 as AccountId }
    relation Posting {
        id: u64 as PostingId,
        account: u64 as AccountId,
        minor: i64,
    }

    Account(id) -> Account;
    AccountParent(child) -> AccountParent;
    AccountParent(child)  <= Account(id);
    AccountParent(parent) <= Account(id);
    Posting(account) <= Account(id);
}

let native = bumbledb::query!(Accounts {
    rec sub(a) | Account(id: a), a == ?root;
    rec sub(a) | AccountParent(child: a, parent: p), sub(p);
    (total: Sum(minor)) | Posting(id, account: a, minor), sub(a);
});

// The host composition's two queries: the frontier step and the rollup.
let children = bumbledb::query!(Accounts {
    (c) | AccountParent(child: c, parent in ?frontier);
});
let rollup = bumbledb::query!(Accounts {
    (total: Sum(minor)) | Posting(id, account in ?subtree, minor);
});

let dir = std::env::temp_dir().join(format!("cookbook-accounts-{}.bdb", std::process::id()));
let db = Db::create(&dir, Accounts, WorkContext::new())?.expect("the empty store admits");
db.write(WorkContext::new(), |tx| {
    for (id, name) in [(1, "assets"), (2, "cash"), (3, "petty cash"), (4, "receivables")] {
        tx.insert([&Account { id: AccountId(id), name }])?;
    }
    for (child, parent) in [(2, 1), (3, 2), (4, 1)] {
        tx.insert([&AccountParent { child: AccountId(child), parent: AccountId(parent) }])?;
    }
    for (id, account, minor) in [(1, 2, 100), (2, 3, 50), (3, 3, 50), (4, 4, -30), (5, 1, 5)] {
        tx.insert([&Posting { id: PostingId(id), account: AccountId(account), minor }])?;
    }
    Ok(())
})?
.expect("the chart commits");

let total = |answers: &Answers| match answers.get(0, 0) {
    AnswerValue::I64(total) => total,
    other => panic!("an i64 total, not {other:?}"),
};

// Host composition: close the subtree of "cash", then sum over it.
let mut step = db.prepare(&children, WorkContext::new())?;
let mut subtree = BTreeSet::from([2]);
let mut frontier = vec![Value::U64(2)];
while !frontier.is_empty() {
    let next = db.read(WorkContext::new(), |frame| {
        frame.execute_collect(&mut step, &children.bind(bumbledb::params! { frontier: frontier.as_slice() }))
    })?;
    frontier = next
        .answers()
        .filter_map(|answer| match answer.get(0) {
            AnswerValue::U64(id) => subtree.insert(id).then_some(Value::U64(id)),
            other => panic!("an account id, not {other:?}"),
        })
        .collect();
}
let members: Vec<Value> = subtree.iter().copied().map(Value::U64).collect();
let mut sum = db.prepare(&rollup, WorkContext::new())?;
let host = db.read(WorkContext::new(), |frame| {
    frame.execute_collect(&mut sum, &rollup.bind(bumbledb::params! { subtree: members.as_slice() }))
})?;

// Native: one query, the fold over the finished rec.
let mut one = db.prepare(&native, WorkContext::new())?;
let engine = db.read(WorkContext::new(), |frame| {
    frame.execute_collect(&mut one, &native.bind(bumbledb::params! { root: 2u64 }))
})?;

assert_eq!(total(&host), 200);
assert_eq!(total(&engine), 200);

drop((step, sum, one, db));
std::fs::remove_dir_all(&dir)?;
Ok::<(), Box<dyn std::error::Error>>(())
```

## 26. Exact partition

Guarantee: mutual point coverage plus pointwise keys enforces an exact partition.

An exact partition needs both coverage directions. The first containment below
is the intent-level reference; the two pointwise keys make each side disjoint;
the final pair proves equal point supports per policy — forward coverage forbids
gaps and reverse coverage forbids overhang. This is not mere tiling language:
it is the five ordinary statements witnessing
`exactTiling_iff_exactPointPartition`.

The explicit `Policy(id, live) -> Policy` is load-bearing. Containment targets
resolve by their exact projected field set, so a declared `{id}` key cannot serve
the `{id, live}` target and the engine infers no key closure.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub ExactPartition;

    relation Policy  { id: u64 as PolicyId, live: interval<i64> }
    relation Version { policy: u64 as PolicyId, valid: interval<i64> }

    Policy(id) -> Policy;
    Version(policy) <= Policy(id);
    Version(policy, valid) -> Version;
    Policy(id, live) -> Policy;
    Policy(id, live) <= Version(policy, valid);
    Version(policy, valid) <= Policy(id, live);
}

let schema = ExactPartition.descriptor().validate().expect("the schema checks");
let _ = schema;
```

Together the mutual containments prove equal point supports for each policy;
the pointwise keys make those supports genuine partitions rather than overlapping
covers. Touching half-open segments remain legal, and the same construction works
with any scalar-prefix arity before the final interval position.

## 27. Derived facts, maintained

Guarantee: a generation witness checks freshness. Containment validates
surviving rollup facts; the application maintains missing ones.

A stored rollup is an ordinary relation with an ordinary soundness statement.
Here `Pack` derives maximal busy spans, while containment prevents any stored
`BusySpan` point that has no busy claim behind it. That is soundness, not a
refresh theorem: a missing span remains representable until the host
maintenance loop fills it.

The loop is snapshot, derive, diff, `write_from(&witness)`. On
`WriteOutcome::Moved` it throws away the derived set and the diff and starts
from a new snapshot; it never retries a stale diff. Containment proves every
surviving stored span sound, and the witness proves which source state the
derivation saw; neither proves completeness. Below, a claim lands between the
first derivation and its commit, the loop observes one retry, and the second
round stores the recomputed span.

```rust
use std::collections::BTreeSet;

use bumbledb::{AnswerValue, Db, Interval, WorkContext, WriteOutcome};

bumbledb::schema! {
    pub MaintainedRollup;

    closed relation Arm as ArmId = { Busy, Ooo };

    relation Claim {
        source: u64,
        person: u64,
        arm: u64 as ArmId,
        span: interval<i64>,
    }
    relation BusySpan { person: u64, span: interval<i64> }

    Claim(arm) <= Arm(id);
    Claim(source) -> Claim;
    Claim(person, span) -> Claim;
    BusySpan(person, span) -> BusySpan;
    BusySpan(person, span) <= Claim(person, span | arm == Busy);
}

let deriving = bumbledb::query!(MaintainedRollup {
    (person, busy: Pack(span)) | Claim(source, person, arm == Busy, span);
});

let busy = |source, start, end| Claim {
    source,
    person: 1,
    arm: Arm::Busy.id(),
    span: Interval::new(start, end).expect("a nonempty span"),
};

let dir = std::env::temp_dir().join(format!("cookbook-rollup-{}.bdb", std::process::id()));
let db = Db::create(&dir, MaintainedRollup, WorkContext::new())?.expect("the empty store admits");
db.write(WorkContext::new(), |tx| tx.insert([&busy(1, 0, 10), &busy(2, 10, 20)]))?
    .expect("the claims commit");

let mut derive = db.prepare(&deriving, WorkContext::new())?;
let mut rounds = 0;
loop {
    rounds += 1;
    let work = WorkContext::new();
    let snapshot = db.snapshot(&work)?;
    let (desired, stored) = {
        let frame = snapshot.frame(&work);
        let answers = frame.execute_collect(&mut derive, &deriving.bind(bumbledb::params! {}))?;
        let desired: BTreeSet<(u64, i64, i64)> = answers
            .answers()
            .map(|answer| match (answer.get(0), answer.get(1)) {
                (AnswerValue::U64(person), AnswerValue::IntervalI64(span)) => {
                    (person, span.start(), span.end())
                }
                other => panic!("the find is (u64, interval<i64>), not {other:?}"),
            })
            .collect();
        let stored = frame
            .scan_facts::<BusySpan>()?
            .map(|fact| fact.map(|fact| (fact.person, fact.span.start(), fact.span.end())))
            .collect::<bumbledb::Result<BTreeSet<_>>>()?;
        (desired, stored)
    };
    let witness = snapshot.witness();
    drop(snapshot);

    if rounds == 1 {
        // Another writer lands a claim between the derivation and its commit.
        db.write(WorkContext::new(), |tx| tx.insert([&busy(3, 20, 30)]))?
            .expect("the late claim commits");
    }

    let span = |start, end| Interval::new(start, end).expect("a derived span is nonempty");
    let outcome = db.write_from(WorkContext::new(), &witness, |tx| {
        for &(person, start, end) in stored.difference(&desired) {
            tx.delete([&BusySpan { person, span: span(start, end) }])?;
        }
        for &(person, start, end) in desired.difference(&stored) {
            tx.insert([&BusySpan { person, span: span(start, end) }])?;
        }
        Ok(())
    })?;
    match outcome {
        WriteOutcome::Committed(_) => break,
        WriteOutcome::Moved { .. } => continue,
        WriteOutcome::Rejected(violations) => panic!("a derived span is sound: {violations}"),
    }
}
assert_eq!(rounds, 2);

let spans = db.read(WorkContext::new(), |frame| {
    frame
        .scan_facts::<BusySpan>()?
        .map(|fact| fact.map(|fact| (fact.span.start(), fact.span.end())))
        .collect::<bumbledb::Result<Vec<_>>>()
})?;
assert_eq!(spans, [(0, 30)]);

drop((derive, db));
std::fs::remove_dir_all(&dir)?;
Ok::<(), Box<dyn std::error::Error>>(())
```

## Operating the store

The embedded engine is two things. **The store** (`Db`) is mutable, durable
and leased: `create` and `open` take the writer lock, `read` hands a
`ReadFrame` to its callback, `snapshot` pins an `OwnedRead`, and `write`
hands a `WriteTx`. **The value** (`OwnedInstance`) is immutable, judged and
owned: an `InstanceBuilder` loads facts and `admit`s them, then
`Db::from_instance` publishes the instance as a new store.

## 28. Migration is ETL

Guarantee: schema fingerprints prevent reinterpretation, and final-state judgment validates each load. The application owns the transformation and a dependency-safe load order.

A store records its theory's fingerprint, and `Db::open` under a changed
theory is a hard `SchemaMismatch`: the engine refuses to reinterpret facts it
judged under different laws. An embedded migration is therefore reads and
writes with two theories the host possesses: `scan` exports every fact of a
relation as values under one snapshot (a consistent instant), the host
transforms them, and `insert_dyn` inside `write` imports them into a store
created under the new theory. A hosted `Database` instead ships migrations
generated by `bumbledb generate` and records each step in its log; see
[durable databases](../ts/README.md#durable-databases).

Three laws keep the loop honest. **Load containment targets first**: every
`write` commits through the ordinary final-state judgment, so a `Salary` fact
whose `Employee` has not landed yet is a rejection that cites the violation,
not a deferral. **Application identity survives**: `insert_dyn` takes explicit
values for every field, so facts keep their ids across the move; the database
has no allocator. **The new theory judges the old data**: every statement of
the new schema holds of every migrated fact, or the `write` is rejected
whole. A migration that lands is already valid.

The new theory records what the old one never did, *when* a salary applies,
as an interval with a pointwise key: one salary per employee per instant. The
transform supplies the missing dimension as a ray from the migration epoch,
the honest reading of "the old amount, still in force".

```rust
use bumbledb::{Db, ErrorKind, Interval, Value, WorkContext};

mod v1 {
    bumbledb::schema! {
        pub PayrollV1;

        relation Employee { id: u64 as EmployeeId, name: str }
        relation Salary   { employee: u64 as EmployeeId, amount: i64 }

        Employee(id) -> Employee;
        Salary(employee) <= Employee(id);
    }
}

bumbledb::schema! {
    pub Payroll;

    relation Employee { id: u64 as EmployeeId, name: str }
    relation Salary {
        employee: u64 as EmployeeId,
        amount: i64,
        applies: interval<i64>,
    }

    Employee(id) -> Employee;
    Salary(employee) <= Employee(id);
    Salary(employee, applies) -> Salary;
}

// The post-migration read: salaries in force at an instant.
let in_force = bumbledb::query!(Payroll {
    (name, amount) | Employee(id: e, name),
                     Salary(employee: e, amount, applies: w), ?at in w;
});

let root = std::env::temp_dir().join(format!("cookbook-payroll-{}", std::process::id()));
std::fs::create_dir_all(&root)?;
let old_dir = root.join("v1.bdb");
let old = Db::create(&old_dir, v1::PayrollV1, WorkContext::new())?.expect("the empty store admits");
old.write(WorkContext::new(), |tx| {
    tx.insert([
        &v1::Employee { id: v1::EmployeeId(1), name: "ada" },
        &v1::Employee { id: v1::EmployeeId(2), name: "grace" },
    ])?;
    tx.insert([
        &v1::Salary { employee: v1::EmployeeId(1), amount: 120 },
        &v1::Salary { employee: v1::EmployeeId(2), amount: 130 },
    ])
})?
.expect("the old payroll commits");

// Extract both relations under one snapshot.
let (employees, salaries) = old.read(WorkContext::new(), |frame| {
    let employees = frame.scan(v1::PayrollV1::Employee.relation())?.collect::<bumbledb::Result<Vec<_>>>()?;
    let salaries = frame.scan(v1::PayrollV1::Salary.relation())?.collect::<bumbledb::Result<Vec<_>>>()?;
    Ok((employees, salaries))
})?;
drop(old);

// The old store keeps its fingerprint: the new theory cannot open it.
let refused = Db::open(&old_dir, Payroll, WorkContext::new()).err().expect("a changed theory is refused");
assert_eq!(refused.kind(), ErrorKind::SchemaMismatch);

// Transform: the old amount, in force from the migration epoch on.
let epoch = Interval::ray(1_000).expect("a ray from the epoch");
let salaries: Vec<Vec<Value>> = salaries
    .into_iter()
    .map(|row| {
        let mut values = row.values().to_vec();
        values.push(Value::IntervalI64(epoch));
        values
    })
    .collect();

// Load: containment targets first, every fact judged by the new theory.
let new = Db::create(&root.join("v2.bdb"), Payroll, WorkContext::new())?.expect("the empty store admits");
new.write(WorkContext::new(), |tx| {
    tx.insert_dyn(Payroll::Employee.relation(), &employees)?;
    tx.insert_dyn(Payroll::Salary.relation(), &salaries)?;
    Ok(())
})?
.expect("the migrated payroll satisfies the new theory");

// The old ids answer the new query.
let mut prepared = new.prepare(&in_force, WorkContext::new())?;
let paid = new.read(WorkContext::new(), |frame| {
    Ok(frame.execute_collect(&mut prepared, &in_force.bind(bumbledb::params! { at: 2_000i64 }))?.len())
})?;
assert_eq!(paid, 2);

drop((prepared, new));
std::fs::remove_dir_all(&root)?;
Ok::<(), Box<dyn std::error::Error>>(())
```

## Composition

## 29. The zone ledger

Guarantee: per-kind mutual coverage and a shared pointwise key enforce each arm's exact partition. Interval positions match by element domain. The application chooses the witness segmentation described below.

Recipe 9's unit slots, composed: a ledger whose timeline divides into zones
of two kinds — unit zones (`interval<u64, 1>`) and pair zones
(`interval<u64, 2>`), each kind carrying its own slot relation. The
discriminated-union pattern (recipe 2) applied at interval positions: a
kind-discriminated `Zone` witness relation owns **disjointness across the
slot relations** through its one pointwise key — all zones of a ledger are
pairwise disjoint whatever their kind, and since each slot relation's point
support equals its kind's zone support (the per-kind `==`), a unit slot can
never overlap a pair slot even though they live in different relations. The
arm widths are enforced **by type**: a `UnitSlot` value is width 1 or does
not exist, a `PairSlot` width 2 — no runtime width check, nothing to
enforce at commit.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub ZoneLedger;

    closed relation Kind as KindId = { Unit, Pair };

    relation Ledger   { id: u64 as LedgerId, name: str }
    relation Zone     { ledger: u64 as LedgerId, kind: u64 as KindId, at: interval<u64> }
    relation UnitSlot { ledger: u64 as LedgerId, at: interval<u64, 1>, entry: u64 }
    relation PairSlot { ledger: u64 as LedgerId, at: interval<u64, 2>, entry: u64 }

    Ledger(id) -> Ledger;
    Zone(ledger) <= Ledger(id);
    Zone(kind)   <= Kind(id);
    Zone(ledger, at) -> Zone;
    UnitSlot(ledger, at) -> UnitSlot;
    PairSlot(ledger, at) -> PairSlot;
    Zone(ledger, at | kind == Unit) == UnitSlot(ledger, at);
    Zone(ledger, at | kind == Pair) == PairSlot(ledger, at);
}

let schema = ZoneLedger.descriptor().validate().expect("the schema checks");
let _ = schema;
```

The honesty note — **coalescing insensitivity**: the `==` judgments compare
point supports, not rows. A single Unit-kind zone `[4,6)` beside two unit
slots `[4,5)`, `[5,6)` satisfies both directions, because nothing forces the
witness rows to mirror the slots' segmentation — only its points. If
per-row correspondence matters, the host writes zones at slot granularity;
the schema proves disjointness and coverage either way.

## Point reads

## 30. The keyed read

Guarantee: a declared key admits at most one fact per determinant tuple, and
every keyed point read answers exactly that fact or nothing, in a read and in
a write.

The key is a law, and the read surface is that law made callable. The schema
says `Course(grp) -> Course`, one course per group, so "the course of a group"
is a well-posed question with at most one answer, and the store enforces that
on every commit.

Every declared `R(x, ..) -> R` on an ordinary relation generates a key struct
named `{R}By{Fields}` (each snake segment Pascal-cased), here
`CourseByGrp { grp }`. The point read hands that struct to `get`:
`frame.get(CourseByGrp { grp })` inside `db.read`, and
`tx.get(CourseByGrp { grp })` inside `db.write`, where the transaction answers
its final state (read-your-writes; a pending delete answers `None`). A wrong
column, newtype or relation is a compile error, never a runtime shape check.

The anti-pattern is a scan-and-find where a key exists: folding
`frame.scan_facts::<Course>()` in the host to hunt for a `grp` re-derives the
uniqueness the store already enforces. Spell the law, and the point read comes
with it.

```rust
use bumbledb::{Db, WorkContext};

bumbledb::schema! {
    pub KeyedRead;

    relation Grp     { id: u64 as GrpId, label: str }
    relation Course {
        id: u64 as CourseId,
        grp: u64 as GrpId,
        title: str,
    }

    Grp(id) -> Grp;
    Course(grp) <= Grp(id);
    Course(grp) -> Course;
}

let dir = std::env::temp_dir().join(format!("cookbook-keyed-{}.bdb", std::process::id()));
let db = Db::create(&dir, KeyedRead, WorkContext::new())?.expect("the empty store admits");
db.write(WorkContext::new(), |tx| {
    tx.insert([&Grp { id: GrpId(1), label: "algebra" }])?;
    tx.insert([&Course { id: CourseId(10), grp: GrpId(1), title: "Linear maps" }])?;
    // The write reads its own final state.
    assert!(tx.get(CourseByGrp { grp: GrpId(1) })?.is_some());
    Ok(())
})?
.expect("the course commits");

let title = db.read(WorkContext::new(), |frame| {
    Ok(frame.get(CourseByGrp { grp: GrpId(1) })?.map(|course| course.title.to_owned()))
})?;
assert_eq!(title.as_deref(), Some("Linear maps"));

// A second course for the same group violates the key.
let second = db.write(WorkContext::new(), |tx| {
    tx.insert([&Course { id: CourseId(11), grp: GrpId(1), title: "Groups" }])
})?;
assert!(matches!(second, bumbledb::WriteOutcome::Rejected(_)));

drop(db);
std::fs::remove_dir_all(&dir)?;
Ok::<(), Box<dyn std::error::Error>>(())
```

## Capacity laws

## 31. The power budget

Guarantee: capacity bounds each pool's summed draw by its own row, using indexed target lookup and a measure walk for each touched group. Pinned-column containment requires a device's watts to equal its model's at every commit.

Per-group capacity is one statement: the weight bracket names the measure
on the SOURCE row, and the dependent bound reads each group's ceiling from
the TARGET row. Only the upper bound may read the target row, and a bound
names any field of it.

The path spelling `[model.watts]` is a typed refusal whose diagnostic names
the local-field alternative: the weight vocabulary is closed at the row. A weight read
through a reference would be a maintained copy of another relation's field,
and a catalog edit would silently re-weigh deployed fleets. Pinned, the
inconsistent commit refuses at the device site and the migration is
explicit.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Racks;

    relation Pool  { id: u64 as PoolId, supply: u64 }
    relation Model { id: u64 as ModelId, watts: u64 }
    relation Device {
        id: u64 as DeviceId,
        pool: u64 as PoolId,
        model: u64 as ModelId,
        watts: u64,
    }

    Pool(id) -> Pool;
    Device(pool) <= Pool(id);
    Model(id, watts) -> Model;
    Device(model, watts) <= Model(id, watts);
    Pool(id) <=[watts]{0..supply} Device(pool);
}

// Utilization is a query, never a column (the ledger's law, recipe 19).
let draw = bumbledb::query!(Racks {
    (pool, total: Sum(watts)) | Device(id, pool, watts);
});

let schema = Racks.descriptor().validate().expect("the schema checks");
for query in [&*draw] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

## 32. Calendar capacity

Guarantee: duration weights sum booking lengths against the room's span. Explicit scalar weights and durations may use scalar or duration bounds in the same application unit. Unweighted counts cannot use duration bounds. Unbounded weights or bounds fail with a typed error at the law site.

"Total booked time per room stays within the room's span" — one statement.
The interval enters through the measure argument, never the group key: an
interval group key is refused at the law site.

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Rooms;

    relation Room { id: u64 as RoomId, span: interval<i64> }
    relation Booking {
        id: u64 as BookingId,
        room: u64 as RoomId,
        booked: interval<i64>,
    }

    Room(id) -> Room;
    Booking(room) <= Room(id);
    Booking(room, booked) -> Booking;
    Room(id) <=[Duration(booked)]{0..Duration(span)} Booking(room);
}

let schema = Rooms.descriptor().validate().expect("the schema checks");
let _ = schema;
```

Mind the weighted `{0}` and the weighted floor: on a weighted statement
`{0}` says "the group's total is zero" (zero-measure rows may exist — the
weaker law than the unit exclusion), and `<=[w]{1..*}` ("positive total")
is not "at least one booking" — that intent is the bare containment. Choose
by the intended constraint, not by a superficially similar count expression.

Booked time can be computed with native measurement and a subsequent sum stage;
coalesce first if overlapping bookings should count once.

`Duration` measures discrete interval width; it does not imply a clock unit.
For `interval<u64>` wage-base coordinates in cents, its width is cents.
For civil-date coordinates counted in days, its width is calendar days.
An explicit `u64` measure/bound must use the same unit as the other side;
the schema does not infer seconds, days, or currency from scalar encoding.

Two self-capacity bounds over a scalar key can prove a recorded measure:
`Slice(id) <=[cents]{0..Duration(span)} Slice(id)` and
`Slice(id) <=[Duration(span)]{0..cents} Slice(id)`.
The key selects one row, so the inequalities prove `cents = end - start`.
Finite interval measurement and overflow checks still apply. This uses the
same capacity evaluator as every other weighted bound, with no conversion
or second arithmetic interpretation.
