# The cookbook — modeling intuition as schemas

Worked schemas for the owner and any agent writing a theory. **This document is
illustrative, never normative**: the compiled tests are the authority — every
block here is executed by the sync suites, and a recipe that disagrees with the
engine is amended in the same change.

Every schema below compiles and validates verbatim against the current engine,
and every query fence compiles via `query!`, prepares against a real store of
its recipe's schema, and round-trips through `ir::render` —
`crates/bumbledb-query/tests/cookbook.rs` duplicates each block token-for-token
and a sync test pins the duplication, so a recipe edited here without the test
following breaks the build.

Each recipe states the enforced database invariant and any application responsibility. Keys, containments, capacities, and query operators retain their ordinary runtime semantics.

## Foundations

## 1. The minimal interval schema

Guarantee: the pointwise key enforces per-service disjointness. Checked interval constructors reject empty values.

One fact per outage window; the pointwise key is the whole temporal design.

```rust
bumbledb::schema! {
    pub Uptime;

    relation Service { id: u64 as ServiceId, name: str }
    relation Outage  { service: u64 as ServiceId, window: interval<i64> }

    Service(id) -> Service;
    Outage(service) <= Service(id);
    Outage(service, window) -> Outage;
}
```

The Rust query notation constructs an AST directly. Down at
instant `t` — point membership (`in`) is a typing rule:

```rust
let down_at = query!(Uptime {
    (service) | Outage(service, window: w), ?t in w;
});
```

Overlapping an incident window — one Allen mask, no operator zoo:

```rust
let overlapping = query!(Uptime {
    (service, w) | Outage(service, window: w), Allen(w, INTERSECTS, ?incident);
});
```

Total downtime can be computed in native stages: `measure` each bounded interval,
then sum in a following stage. Use `pack` first when overlapping outages should
count once. Native `FindTerm::Segments` constructs intervals;
`ScalarExpr::Measure` computes width. The [staged recipes](../ts/COOKBOOK.md#14-derive-slices-measure-them-and-round-the-total)
show the public algebra and contribution identity end to end.

## 2. Discriminated unions

Guarantee: key-backed equality requires exactly one corresponding row on each side. Both projections must resolve to declared keys.

Sum-typed entities: a closed-relation discriminator plus per-arm child
relations, connected by bidirectional conditional containments.

```rust
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
```

The TypeScript `alternatives` helper constructs these ordinary laws from a
parent scalar key, closed roster, and child keys. Declare supplied keys once;
the expansion follows roster order and has the same descriptor and fingerprint
as manual laws. Composite scalar identities work. Interval identity projections
express coverage and cannot prove exactly one payload row. An arm switch plus
payload replacement is one atomic final-state change. Evidence and ownership
remain explicit laws. See the [complete declaration](../ts/COOKBOOK.md#16-exhaustive-alternatives-with-ordinary-laws).

## 3. 0..1 optional attributes

Guarantee: the child key allows at most one fact, and containment requires its parent. Absence remains legal.

No nulls, anywhere. Optional data is an absent fact in a child relation; the
child's key plus a one-way containment *is* "nullable column", done honestly.

```rust
bumbledb::schema! {
    pub Optionality;

    relation Business { id: u64 as BusinessId, name: str }
    relation MailingAddress { business: u64 as BusinessId, line: str, city: str }

    Business(id) -> Business;
    MailingAddress(business) -> MailingAddress;
    MailingAddress(business) <= Business(id);
}
```

Negation is plain anti-join (no null branch exists in any operator):

```rust
let unaddressed = query!(Optionality {
    (b) | Business(id: b), !MailingAddress(business: b);
});
```

## 4. Money

Guarantee: host discipline + validator premises — fixed-point scale and
currency grouping live in host newtypes; containments only resolve references.

Fixed-point i64 minor units; the host newtype owns scale and currency.
Native `mulDiv` supports exact bounded integer proration with explicit rounding.
FX policy, rate evidence, and unit discipline remain application responsibilities.

```rust
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
```

Multi-currency totals: currency is a group key, never summed across — `Sum`
folds in i128 with one final range check, so totals cannot wrap silently.
Bind the application-owned id: set semantics would collapse two equal
(account, currency, minor) postings without it.

```rust
let totals = query!(Money {
    (account, currency, total: Sum(minor)) | Posting(id, account, currency, minor);
});
```

## 5. Content addressing

Guarantee: validator/runtime premises + host discipline — the payload key and
containments enforce identity/reference shape; hashing and blob durability stay external.

Use `str` for text and `bytes<N>` for a fixed-width binary identifier or
digest. Text values are not an application-visible persistent intern table;
choose the type for its meaning, not an assumed storage compression ratio.

```rust
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
```

The lookup by digest — a bytes param self-encodes:

```rust
let by_digest = query!(Content {
    (id) | Document(id, payload == ?digest);
});
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
```

Handles are literals in queries exactly as in statements, and the renderer
prints them back — the round trip runs on names. A query atom over the
vocabulary can be resolved during preparation:

```rust
let urgent = query!(Tickets {
    (t) | Ticket(id: t, priority == Urgent);
});
```

## 7. The classification

Guarantee: validator/runtime premise — closed payload facts and ψ-selected
containment restrict certificates to the compiled mastered-handle set.

The fused form: the vocabulary carries its intrinsic facts as **payload
columns** — one ground axiom per handle, values sealed with the schema, read by
ψ-selections. The old shape — an ordinary relation the application wrote at
startup and every deployment re-verified — is deleted outright: axioms are
declared, never written.

```rust
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
```

The classification read duplicates no flag onto `Attempt` — ψ walks the
vocabulary's payload in the query too:

```rust
let mastered = query!(Review {
    (a) | Attempt(id: a, kind: k), Kind(id: k, mastered == true);
});
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
```

Who is being paged — the same ψ, on the read side:

```rust
let paged = query!(Oncall {
    (i) | Escalation(incident: i, severity: s), Severity(id: s, pages == true);
});
```

## Structure

## 9. Ordered collections

Guarantee: mutual point coverage plus pointwise keys enforces an exact partition. Interval positions match by element domain regardless of width refinement. The application orders results for presentation.

The linked-list verdict: successor pointers are control flow smuggled into
data — every reorder becomes a dependent chain of writes. Order is a value.
The idiomatic ordered collection is an interval partition, spelled as a
**triple**:

- **the entity** — `Playlist`, ordinary identity;
- **the extent as a 0..1 child** — not a field on the entity, and the reason
  is forcing, not taste: *empty lists exist, empty intervals do not*. A
  playlist with no tracks has no span to record, and interval values are
  nonempty by construction, so the span lives in an optional child (recipe
  3's shape) whose presence *is* nonemptiness;
- **the unit-slot sidecar** — each track occupies `interval<u64, 1>` (the
  width is the type: one stored word, a wrong-width value unrepresentable),
  and the exact-partition `==` (recipe 26) forces the slots to tile the
  extent exactly: a gap aborts, an overlap aborts (the pointwise key
  convicts it before coverage even runs).

```rust
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
```

Positional access is membership — "what plays at position `?pos`":

```rust
let at_pos = query!(Playlists {
    (track) | Slot(playlist == ?list, slot: s, track), ?pos in s;
});
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
```

A node's left operand, when it is a literal — the arm join:

```rust
let lhs_lit = query!(Ast {
    (v) | Add(node == ?n, lhs: l), Lit(node: l, value: v);
});
```

## 11. Typed graphs

Guarantee: validator/runtime premises — endpoint containments type each edge
and composite keys deduplicate pairs; no transitive graph property is claimed.

One relation per edge kind: the edge vocabulary is closed and checked —
endpoint containments pin which node kinds each edge may touch.

```rust
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
```

Mutual follows — joins are explicit `field: v` on both ends (the punning
law); `<` keeps each pair once:

```rust
let mutual = query!(Graph {
    (a, b) | Follows(follower: a, followee: b),
             Follows(follower: b, followee: a), a < b;
});
```

## 12. Entity-component

Guarantee: definition + validator/runtime premises — component keys give 0..1
and containments require the stated entity/archetype facts.

The 0..1 idiom (recipe 3) at scale: components are sidecar relations; an
entity has a component iff the fact exists; a new component kind is a new
relation, not a wider fact.

```rust
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
```

The physics join is the component intersection:

```rust
let physics = query!(Ecs {
    (e, x, y, dx, dy) | Transform(entity: e, x, y), Velocity(entity: e, dx, dy);
});
```

## 13. State machines

Guarantee: key-backed equality requires the selected state's evidence. The application enforces which state transitions are allowed.

States are a discriminated union; per-state data lives in arms; and the
conditional reference target — a reference to "an order *that is shipped*" —
is one selected containment, the statement SQL cannot write.

```rust
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
```

Shipped orders with their carriers:

```rust
let shipped = query!(Orders {
    (id, carrier) | Order(id, state == Shipped), Shipment(order: id, carrier);
});
```

## Time and coverage

## 14. The calendar core

Guarantee: equality enforces key-backed correspondence. Pointwise keys and coverage enforce the declared hard constraints.

Policy as schema: hard rules are pointwise keys, soft rules are the statements
you decline to write.

```rust
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
```

The conflict probes — one Allen mask against a param, on rooms and on people:

```rust
let room_conflicts = query!(Calendar {
    (room, s) | Booking(room, span: s), Allen(s, INTERSECTS, ?want);
});
```

```rust
let busy_people = query!(Calendar {
    (person, s) | Claim(person, span: s), Allen(s, INTERSECTS, ?window);
});
```

## 15. Effective-dated configuration

Guarantee: pointwise keys prevent overlap, and one-way containment requires source coverage. Target intervals may extend beyond that source.

Versioned rules: no overlaps (pointwise key), no gaps in the policy's source
lifetime (one-way coverage; version overhang remains legal), and "in force on
date t" is one membership probe.

```rust
bumbledb::schema! {
    pub Pricing;

    relation Policy  { id: u64 as PolicyId, live: interval<i64> }
    relation Version { policy: u64 as PolicyId, rate_bps: i64, valid: interval<i64> }

    Policy(id) -> Policy;
    Version(policy) <= Policy(id);
    Version(policy, valid) -> Version;
    Policy(id, live) <= Version(policy, valid);
}
```

In force on date `t` — one membership probe:

```rust
let in_force = query!(Pricing {
    (rate_bps) | Version(policy == ?p, rate_bps, valid: v), ?t in v;
});
```

Clean successions — half-open makes MEETS exact, no ±1 fudge:

```rust
let successions = query!(Pricing {
    (a, b) | Version(policy: p, valid: a), Version(policy: p, valid: b),
             Allen(a, MEETS, b);
});
```

## 16. Disjoint covers

Guarantee: containment requires source coverage. A target may overhang the source; exact partition requires constraints in both directions.

Pay periods, shifts, estimated-tax quarters: a pointwise key plus one-way
coverage is a **disjoint cover** — no overlaps among pay periods and no holes
in the fiscal year's source span. Pay periods may extend beyond that span;
target overhang is legal under this statement. Historically this pattern was
called a tiling here; that was stronger than the judgment actually proved.

```rust
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
```

The period holding date `t`:

```rust
let holding = query!(Payroll {
    (seq) | PayPeriod(year == ?y, seq, span: s), ?t in s;
});
```

## 17. Federal income tax

Guarantee: validator/runtime premises + host discipline — keys prove bracket
disjointness and statements prove residency coverage; full bracket coverage and proration are host duties.

Brackets are intervals over money; the top bracket is a ray; regimes key on
(year, status). Native staged queries can derive finite overlaps and prorated
amounts from the stored facts.

```rust
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
```

The marginal bracket — membership probes the disjoint bracket set:

```rust
let marginal = query!(Tax {
    (rate_bps) | Regime(id: r, year == ?y, status == ?s),
                 Bracket(regime: r, income: b, rate_bps), ?taxable in b;
});
```

A native nonrecursive pipeline can intersect earning intervals with each bracket,
measure overlaps, preserve contribution identity, sum weighted amounts, and apply
`mulDiv` once to the total. Keep earning and band identity through the weighted
projection so equal-valued contributions both count. Rounding slices separately
changes the calculation. See the [executable TypeScript recipe](../ts/COOKBOOK.md#14-derive-slices-measure-them-and-round-the-total).

`ScalarExpr::MulDiv { a, b, divisor, rounding }` uses an exact wide product and
checks its 64-bit result after rounding. `Rounding` has `TowardZero`,
`NearestTiesAwayFromZero`, and `NearestTiesToEven`. A positive divisor and matching
integer kinds are required. Existing checked multiplication still rejects its own
intermediate overflow. The same expression runs in queries and migrations.

## 18. Free time and coalescing

Guarantee: `Pack` coalesces result intervals. It does not impose stored disjointness, completeness, or automatic maintenance.

`Pack` is Snodgrass's coalesce as an aggregate — maximal disjoint segments per
group, one answer per (group, segment). Coalescing is never a write rule: the
engine stores the claims it was given.

```rust
bumbledb::schema! {
    pub FreeTime;

    relation Person { id: u64 as PersonId, name: str }
    relation Claim  { person: u64 as PersonId, span: interval<i64> }

    Person(id) -> Person;
    Claim(person) <= Person(id);
}
```

Busy time, coalesced — adjacent segments merge, the half-open law:

```rust
let busy = query!(FreeTime {
    (person, busy: Pack(span)) | Claim(person, span);
});
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
upstream failure. See the [difference, pack, and measure pipeline](../ts/COOKBOOK.md#15-subtract-a-window-coalesce-coverage-and-measure).

## The write side

## 19. The ledger

Guarantee: checked sums reject overflow. Posting references are database constraints; the application enforces double-entry arithmetic agreement.

The census workload. Balance is a query, never a column.

```rust
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
```

Balances — bind the application-owned id, or set semantics collapses duplicates:

```rust
let balances = query!(Ledger {
    (account, total: Sum(minor)) | Posting(id, account, minor);
});
```

The double-entry audit — the host asserts every total is 0; discipline, not
schema:

```rust
let audit = query!(Ledger {
    (entry, Sum(minor)) | Posting(id, entry, minor);
});
```

## 20. Conditional writes

Guarantee: a generation witness rejects a write based on a moved instance. The application owns retries. Final-state point reads need no earlier witness.

The generation witness: read the model,
propose a delta, commit iff the model you read is still the model.

```rust
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
```

Three write idioms. The first two are instance-derived and therefore use
instance-query → compute → `write_from(&witness)` → host retry on
`ConditionalWrite::Moved`. **Update-where**: query the premise on a read
instance, then
`delete(old)` + `insert(new)` per matched fact — "still Queued" is the
witness:

```rust
let queued = query!(Jobs {
    (id, payload) | Job(id, state == Queued, payload);
});
```

**Insert-select**: query source answers, insert the derived facts,
`write_from` witnessing the `ReadInstance`.
**Read-modify-write, key-shaped**: WriteTx point reads (get/contains) see
the final state — per-fact premises need no earlier `Witness`.

## 21. Derived relations

Guarantee: containment rejects unsupported stored facts. The application maintains completeness and refreshes missing derived facts.

The materialized view as a relation under statements — unsoundness the schema
can name is uncommittable; incompleteness remains representable until the host
refreshes it.

```rust
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
```

Maintenance is the third witness idiom (recipe 20): re-run the deriving
query on a read instance, diff, `write_from(&witness)` — the rollup cannot commit
against sources it didn't actually read. The deriving query (`Pack` IS the
coalesce):

```rust
let deriving = query!(Rollup {
    (person, busy: Pack(span)) | Claim(person, span, arm == Busy);
});
```

## 22. Union reads

Guarantee: rule union uses set semantics. Deduplication absorbs repeated derivations while retaining distinct bindings needed by aggregates.

The whole-DU read is a set of rules: one head, one rule per arm — disjunction
is data at the top, never an execution node.

```rust
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
```

One query, two rules (set union). The exclusivity theorem (recipe 2) is
spent a third time here: rules selecting different `kind` values are
disjoint. The union still has set semantics; do not depend on bag-like
duplicates when composing rules:

```rust
let methods = query!(Payments {
    (id, n) | Payment(id, kind == Card), Card(payment: id, last4: n);
    (id, n) | Payment(id, kind == Ach), Ach(payment: id, routing: n);
});
```

## 23. The anti-recipes: five gravestones

Guarantee: intentionally refused — each gravestone names unsupported vocabulary
and its representable replacement; none asserts an engine theorem.

What not to model. Each gravestone cites its replacement; the block's
relations are the replacements, compiled.

```rust
bumbledb::schema! {
    pub Gravestones;

    relation Step { flow: u64, pos: u64, action: str }
    relation Score { subject: u64, bps: i64 }
    relation ActiveRun { student: u64, run: u64 }
    relation Usage { meter: u64, period: u64, used: interval<i64> }
    relation Event { id: u64 as GravestoneEventId, at: i64 }

    Event(id) -> Event;
    Step(flow, pos)    -> Step;
    Score(subject)     -> Score;
    ActiveRun(student) -> ActiveRun;
    Usage(meter, used) -> Usage;
}
```

## Host-driven closure

## 24. The closure idiom

Guarantee: the application loop terminates over its finite seen set. The native form runs through the linear reach driver with execution budgets.

Reachability, in two dialects. The host-loop idiom remains the
depth-bounded answer: the censused hierarchies are **depth-bounded**, so
the loop runs depth-many rounds and each round is one ∈-set query — a
`ParamSet` probe—against the engine. Measure the actual workload. The
frontier discipline below *is* semi-naive evaluation's Δ, spent where a loop
is a loop: the host. The engine-native form (below) is the same closure as
one linear rec: `rec` declares
the rec, the bare rule is the required main, and the driver runs the rounds
inside one plan. Cycle detection with
`reach(x, x)` is the same family — linear rec plus a main join of the
finished table, not a second rec.

```rust
bumbledb::schema! {
    pub Closure;

    relation Node   { id: u64 as NodeId, name: str }
    relation Parent { child: u64 as NodeId, parent: u64 as NodeId }

    Node(id) -> Node;
    Parent(child) -> Parent;
    Parent(child)  <= Node(id);
    Parent(parent) <= Node(id);
}
```

The loop's one query — the frontier's children, one ∈-set probe:

```rust
let children = query!(Closure {
    (c) | Parent(child: c, parent in ?frontier);
});
```

The loop (the compiled, tested copy is `reachable` in `cookbook.rs`, driven
over a three-level tree with the exact reachable set asserted):

```text
frontier = {root};  seen = {root}
loop:
    next = query(parent ∈ frontier, child)   // one set-param query
    new  = next − seen
    if new.is_empty() { break }
    seen ∪= new; frontier = new
```

Termination is the host's theorem: `seen` grows strictly or the loop breaks,
inside a finite node set. When the idiom's costs bite — **unbounded or
large depth** (the per-round query cost stops being noise), or **closure
composed into a larger plan** (the reachable set must join further inside
one plan) — write the engine-native form instead: the same closure, one
linear rec under the reach driver — `?root` seeds the rec, the bare rule is
the required main —

```rust
let native = query!(Closure {
    rec reach(c) | Node(id: c), c == ?root;
    rec reach(c) | Parent(child: c, parent: m), reach(m);
    (c) | reach(c);
});
```

— and `db.prepare(&native)?` runs the rounds inside one plan
(the compiled copy runs beside the loop in `cookbook.rs`, both dialects
asserting the same reachable sets, root for root). What stays host-side is
the **chain-window class** — interval intersection along paths — which the
recursion surface excludes: the idiom carries the window in the host's frontier,
one intersection per hop, and that composition has no engine form.

## 25. The chart of accounts

Guarantee: the application computes closure before a checked sum. The native form likewise aggregates over the completed recursive relation.

The ledger workload's real recursion case, in the same two dialects: a
hierarchical chart of accounts and a subtree rollup. The host composition —
recipe 24's loop accumulates the subtree's ∈-set, then **one `Sum` query
over the accumulated set** folds the postings. The engine aggregates, the
host composes (aggregates never nest — recipe 18's refusal family). The
engine-native form is one query: aggregation *through* the rec cycle is refused
(`AggregateInInterior` on a rec head), but a fold over a **finished** rec is
ordinary main —

```rust
let native = query!(Accounts {
    rec sub(a) | Account(id: a), a == ?root;
    rec sub(a) | AccountParent(child: a, parent: p), sub(p);
    (total: Sum(minor)) | Posting(id, account: a, minor), sub(a);
});
```

— the rec converges first, then the main fold runs once
over the finished subtree (the compiled copy in `cookbook.rs` asserts both
dialects against the hand-computed sums).

```rust
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
```

The two queries the host rollup composes. The frontier step (recipe 24's
loop, verbatim):

```rust
let children = query!(Accounts {
    (c) | AccountParent(child: c, parent in ?frontier);
});
```

The rollup over the accumulated subtree (bind the application-owned id — recipe 19's
discipline, spent again):

```rust
let rollup = query!(Accounts {
    (total: Sum(minor)) | Posting(id, account in ?subtree, minor);
});
```

The rollup is two prepared queries with the recipe-24 loop between them;
the test drives a three-level hierarchy with postings and asserts the
hand-computed subtree sum — equal postings to one account both count,
because the application-owned id keeps their bindings distinct.

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
```

Together the mutual containments prove equal point supports for each policy;
the pointwise keys make those supports genuine partitions rather than overlapping
covers. Touching half-open segments remain legal, and the same construction works
with any scalar-prefix arity before the final interval position.

## 27. Derived facts, maintained

Guarantee: a generation witness checks freshness. Containment validates surviving rollup facts; the application maintains missing ones.

A stored rollup is an ordinary relation with an ordinary soundness statement.
Here `Pack` derives maximal busy spans, while containment prevents any stored
`BusySpan` point that has no busy claim behind it. That is soundness, not a
refresh theorem: a missing span remains representable until the host maintenance
loop fills it.

```rust
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
```

Derive the desired rollup on the maintenance `ReadInstance`:

```rust
let deriving = query!(MaintainedRollup {
    (person, busy: Pack(span)) | Claim(source, person, arm == Busy, span);
});
```

The host loop is instance → derive → diff → `write_from(&witness)`. On
`ConditionalWrite::Moved`, it throws away the derived set and diff and starts from a new
instance; it never retries a stale diff. Dependencies prove every surviving
stored span sound, while the witness proves which source state the derivation
saw; neither mechanism proves completeness. The compiled copy is
`maintain_busy_spans` in `cookbook.rs`; its lock moves the source generation
between derive and commit, observes one retry, and then asserts the recomputed
packed span.

## Operating the store

The public engine is two things. **The store** (`Db`) is mutable, durable,
and leased: `create` / `open` take the writer lock; `read` hands a
`ReadInstance` for the callback; `write` hands a `WriteTx`. **The value**
(`OwnedInstance`) is immutable, proven, and owned: `InstanceBuilder` loads
and `admit`s, then `Db::from_instance` publishes the packed catalog as a
new store. There is no third duration.

## 28. Migration is ETL

Guarantee: schema fingerprints prevent reinterpretation, and final-state judgment validates each load. The application owns the transformation and dependency-safe load order.

Core storage does not reinterpret an existing store under a new schema:
the store records the theory's fingerprint, and `Db::open` under a changed
theory is a hard `SchemaMismatch` — the engine refuses to reinterpret facts
it judged under different laws. The host possesses both theories. There is
no theory-less open of a store whose schema you do not have — a real
migration is reads and writes with two schemas you both possess.
Migration is extract, transform, load:
`scan` exports every fact of a relation as typed values under one `ReadInstance`
(one generation — the export is a consistent instant), the host transforms,
and `insert_dyn` inside `write` imports into a store created under the new theory. The
engine owns both ends. This recipe demonstrates a host-authored transform;
the TypeScript log layer also supplies generated migration plans and a
local-history runner. Migration orchestration belongs above the core.

Three laws make the loop honest. **Load containment targets first** — every
`write` commits through the ordinary final-state judgment, so a `Salary` fact
whose `Employee` has not landed yet is a rejection (with the complete
violation set cited), not a deferral. **Application identity survives** —
`insert_dyn` takes explicit values for every field, so facts retain their
ids across the move. The application chooses an unused id for later inserts;
the database has no allocator, reservation API, or catch-up counter. **The new
theory judges the old data** — every dependency of the new schema holds of
every migrated fact, or that `write` aborts whole. A migration that lands is
already valid; there is no "migrate now, validate later."

The v2 theory below adds what v1 (shown as text) never recorded — *when* a
salary applied — as an interval with a pointwise functionality: one salary
per employee per instant. The transform supplies the missing dimension (a
ray from the migration epoch), which is the honest reading of "the old
amount, still in force."

```text
pub PayrollV1;                     // the old theory, judged and fingerprinted

relation Employee { id: u64 as EmployeeId, name: str }
relation Salary   { employee: u64 as EmployeeId, amount: i64 }

Employee(id) -> Employee;
Salary(employee) <= Employee(id);
```

```rust
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
```

The post-migration read — salaries in force at an instant:

```rust
let in_force = query!(Payroll {
    (name, amount) | Employee(id: e, name),
                     Salary(employee: e, amount, applies: w), ?at in w;
});
```

The compiled test drives the whole loop: seed a v1 store, export both
relations under one `ReadInstance`, drop the v1 handle and prove the
fingerprint refusal (`Db::open` of the v1 store under `Payroll` is
`SchemaMismatch`), append the ray to each salary, then `insert_dyn`
inside `write` — employees before salaries — then insert with a new
application-owned id. It verifies identity (the v1 ids answer the v2 query),
coexistence with subsequent application IDs, and judgment (the
migrated store answers under the new theory's guarantees).

## Composition

## 29. The zone ledger

Guarantee: per-kind mutual coverage and a shared pointwise key enforce each arm's exact partition. Interval positions match by element domain. The application chooses the witness segmentation described below.

Recipe 9's sidecar, composed: a ledger whose timeline divides into zones of
two kinds — unit zones (`interval<u64, 1>`) and pair zones
(`interval<u64, 2>`), each kind carrying its own payload sidecar. The
discriminated-union pattern (recipe 2) applied at interval positions: a
kind-discriminated `Zone` witness relation owns **cross-sidecar
disjointness** through its one pointwise key — all zones of a ledger are
pairwise disjoint whatever their kind, and since each sidecar's point
support equals its kind's zone support (the per-kind `==`), a unit slot can
never overlap a pair slot even though they live in different relations. The
arm widths are enforced **by type**: a `UnitSlot` value is width 1 or does
not exist, a `PairSlot` width 2 — no runtime width check, nothing to
enforce at commit.

```rust
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
```

The honesty note — **coalescing insensitivity**: the `==` judgments compare
point supports, not rows. A single Unit-kind zone `[4,6)` beside two unit
slots `[4,5)`, `[5,6)` satisfies both directions, because nothing forces the
witness rows to mirror the sidecar's segmentation — only its points. If
per-row correspondence matters, the host writes zones at slot granularity;
the schema proves disjointness and coverage either way.

## Point reads

## 30. The keyed read

Guarantee: validator/runtime premises — a declared key FD admits at most one
fact per determinant tuple (the key phase of the commit judgment), and every
keyed point read answers exactly that fact or nothing, on both scopes
(`crates/bumbledb/tests/keyed_get.rs`).

The key is a **law**, and the read surface is that law made callable. The
schema says `Course(grp) -> Course` — one course per group — so "the
course of a group" is a well-posed question with at most one answer, and
the store already enforces that on every commit:

```rust
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
```

Every declared `R(x, ..) -> R` on an ordinary relation emits a generated
**key struct** named by the derived-name rule (`{R}By{Fields}`, each snake
segment Pascal-cased) — here `CourseByGrp { grp }` — implementing `Key`
with its statement id computed at expansion. The point read is that struct
handed to `get` on either scope: `instance.get(CourseByGrp { grp })` inside
`db.read`, and `tx.get(CourseByGrp { grp })` inside `db.write`, where the
transaction side answers the FINAL state (base plus pending delta:
read-your-writes, a pending delete answers `None`). The declared ID newtype is
the primary key made callable the same way: `instance.get(id)` / `tx.get(id)`
through a `CourseId` value reads the `Course` fact carrying that key.
A wrong column, wrong newtype, or wrong relation is a compile error, never
a runtime shape check.

The anti-pattern this recipe retires: a scan-and-find where a key law
exists — `instance.scan_facts::<Course>()` folded host-side, hunting for a
`grp` — re-derives in the host what the store already enforces. The
uniqueness the fold quietly assumes IS the declared FD; spell the law and
the point read comes with it.

## Capacity laws

## 31. The power budget

Guarantee: capacity bounds each pool's summed draw by its own row, using indexed target lookup and a measure walk for each touched group. Pinned-column containment requires a device's watts to equal its model's at every commit.

Per-group capacity is one statement: the weight bracket names the measure
on the SOURCE row, and the dependent bound reads each group's ceiling from
the TARGET row (hi slot only — ruled 2026-07-24, C6; bound idents resolve
by name against the target's full roster, ruled 2026-07-24, C1).

```rust
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
```

The path spelling `[model.watts]` is a typed refusal whose diagnostic names
the local-field alternative: the weight vocabulary is closed at the row. A weight read
through a reference would be a maintained copy of another relation's field,
and a catalog edit would silently re-weigh deployed fleets. Pinned, the
inconsistent commit refuses at the device site and the migration is
explicit.

Utilization is a query, never a column (the ledger's law, recipe 19):

```rust
let draw = query!(Racks {
    (pool, total: Sum(watts)) | Device(id, pool, watts);
});
```

## 32. Calendar capacity

Guarantee: duration weights sum booking lengths against the room's span. Explicit scalar weights and durations may use scalar or duration bounds in the same application unit. Unweighted counts cannot use duration bounds. Unbounded weights or bounds fail with a typed error at the law site.

"Total booked time per room stays within the room's span" — one statement.
The interval enters through the measure argument, never the group key (the
v0 projection refusal survives narrowed).

```rust
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
