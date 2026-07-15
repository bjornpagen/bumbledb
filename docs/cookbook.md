# The cookbook — modeling intuition as schemas

Worked schemas for the owner and any agent writing a theory. **This document is
illustrative, never normative**: where a recipe and an architecture chapter
disagree, the chapter wins and the recipe is amended in the same change
(`docs/architecture/README.md` rule 5). The chapters it defers to:
`docs/architecture/10-data-model.md` (the six value types and the closed-relation
form, the interval denotation, the modeling discipline, derived relations),
`30-dependencies.md` (the two
judgments and their theorems), `20-query-ir.md` (query semantics; § the query
notation is the grammar the recipes' query comments are written in), `70-api.md`
(the `schema!` grammar, conditional writes). Refusals cited below live in
the architecture chapters they cite.

Every schema below compiles and validates verbatim against the current engine —
`crates/bumbledb-query/tests/cookbook.rs` duplicates each block token-for-token
and a sync test pins the duplication, so a recipe edited here without the test
following breaks the build.

Guarantee labels that name Lean results cite the checked spec in `lean/` by
theorem name (`lean/Bumbledb/….lean: name` — `scripts/spec-census.sh` verifies
every citation resolves); the label always names any additional Rust premise.

## Foundations

## 1. The minimal interval schema

Guarantee: Lean theorem + validator/runtime premise — the pointwise key
enforces per-service disjointness (`lean/Bumbledb/Dependencies.lean:
pointwise_key_disjoint`); checked intervals supply nonempty values
(`lean/Bumbledb/Values.lean: interval_nonempty`).

One fact per outage window; the pointwise key is the whole temporal design.

```rust
bumbledb::schema! {
    pub Uptime;

    relation Service { id: u64 as ServiceId, fresh, name: str }
    // The window is one value, not a (start, end) column pair: the denotation
    // (a set of points, half-open) is what the judgments below read through.
    relation Outage  { service: u64 as ServiceId, window: interval<i64> }

    Outage(service) <= Service(id);
    // The pointwise key: per service, no two outages share a point — every
    // pair satisfies DISJOINT. SQL:2011's WITHOUT OVERLAPS, as a theorem.
    Outage(service, window) -> Outage;

    // Queries (the notation — 20-query-ir.md § the query notation):
    //   down at instant t (point membership (`in`) is a typing rule):
    //     (service) | Outage(service, window: w), ?t in w;
    //   overlapping an incident window (one Allen mask, no operator zoo):
    //     (service, w) | Outage(service, window: w), Allen(w, INTERSECTS, ?incident);
    //   total downtime per service (the denotation's one arithmetic):
    //     (service, Sum(Duration(window))) | Outage(service, window);
}
```

## 2. Discriminated unions

Guarantee: Lean theorem + validator/runtime premises — key-backed equality
gives unique source/target correspondence (`lean/Bumbledb/Dependencies.lean:
keyed_eq_unique_correspondence`); both projections must resolve to declared keys.

Sum-typed entities: a closed-relation discriminator plus per-arm child
relations, glued by bidirectional conditional containments
(`30-dependencies.md` § the derivations).

```rust
bumbledb::schema! {
    pub Grading;

    // The discriminator vocabulary is a closed relation: its ground axioms are
    // axioms, and the host enum `Kind` is emitted for rustc's matching.
    closed relation Kind as KindId = { Deterministic, CustomOperator };

    relation Task { id: u64 as TaskId, fresh, kind: u64 as KindId }
    relation DeterministicGrading  { task: u64 as TaskId, tolerance: i64 }
    relation CustomOperatorGrading { task: u64 as TaskId, operator: str }

    Task(kind) <= Kind(id);                                // the discriminator resolves
    DeterministicGrading(task)  -> DeterministicGrading;   // one arm fact per parent
    CustomOperatorGrading(task) -> CustomOperatorGrading;
    // Totality (==, left to right): a Deterministic task HAS its arm fact —
    // same commit, always. Arm validity (right to left): an arm fact's parent
    // exists WITH that kind — composite-FK-plus-CHECK, one statement.
    Task(id | kind == Deterministic)  == DeterministicGrading(task);
    Task(id | kind == CustomOperator) == CustomOperatorGrading(task);
    // Exclusivity is a theorem, not a statement: one id in two arms would
    // force `kind` to equal two handles against the fresh key on id.
    // The executor spends the same theorem again — recipe 22's free lunch.
}
```

## 3. 0..1 optional attributes

Guarantee: Lean theorem + validator/runtime premises — the child key proves at
most one fact (`lean/Bumbledb/Dependencies.lean: functionality_unique_witness`)
and containment requires its parent (`lean/Bumbledb/Dependencies.lean:
contains_iff_view_subset`); absence remains legal.

No nulls, anywhere. Optional data is an absent fact in a child relation; the
child's key plus a one-way containment *is* "nullable column", done honestly.

```rust
bumbledb::schema! {
    pub Optionality;

    relation Business { id: u64 as BusinessId, fresh, name: str }
    relation MailingAddress { business: u64 as BusinessId, line: str, city: str }

    MailingAddress(business) -> MailingAddress;   // at most one address...
    MailingAddress(business) <= Business(id);     // ...and only for a real business
    // One-way <= on purpose: absence is the fact that isn't. The all-or-nothing
    // column group (line+city together or neither) is unstatable TO VIOLATE —
    // the fact carries both fields or does not exist.

    // Negation is plain anti-join (no null branch exists in any operator):
    //   (b) | Business(id: b), !MailingAddress(business: b);
}
```

## 4. Money

Guarantee: host discipline + validator premises — fixed-point scale and
currency grouping live in host newtypes; containments only resolve references.

Fixed-point i64 minor units; the host newtype owns scale and currency. Floats
are permanently refused (the ledger); proration and FX are host arithmetic.

```rust
bumbledb::schema! {
    pub Money;

    closed relation Currency as CurrencyId = { Usd, Eur, Gbp };

    relation Account { id: u64 as AccountId, fresh, name: str }
    // Minor units in i64 (±92 quadrillion cents); `as Minor` is the host
    // newtype — rustc polices cross-domain confusion, not the engine
    // (hard structural typing, 10-data-model.md).
    relation Posting {
        id: u64 as PostingId, fresh,
        account: u64 as AccountId,
        currency: u64 as CurrencyId,
        minor: i64 as Minor,
    }

    Posting(account)  <= Account(id);
    Posting(currency) <= Currency(id);

    // Multi-currency totals: currency is a group key, never summed across —
    // Sum folds in i128 with one final range check, so totals cannot wrap
    // silently. Bind the fresh id: set semantics would collapse two equal
    // (account, currency, minor) postings without it.
    //   (account, currency, total: Sum(minor)) | Posting(id, account, currency, minor);
}
```

## 5. Content addressing

Guarantee: validator/runtime premises + host discipline — the payload key and
containments enforce identity/reference shape; hashing and blob durability stay external.

The decision rule for byte-shaped data: **intern what repeats (`str`); inline
what identifies (`bytes<N>`)** — `10-data-model.md` § the type layer.

```rust
bumbledb::schema! {
    pub Content;

    closed relation Region as RegionId = { Us, Eu };

    relation Document {
        id: u64 as DocumentId, fresh,
        name: str,                          // repeats: interned, id-equality
        payload: bytes<32> as PayloadHash,  // identifies: the blake3 of the
    }                                       // external blob — inline, never interned
    relation Replica { payload: bytes<32> as PayloadHash, region: u64 as RegionId }

    Document(payload) -> Document;          // content-addressed: one doc per digest
    Replica(payload) <= Document(payload);
    Replica(region)  <= Region(id);
    // bytes<N> is identity-only (Eq/Ne, membership): a digest's lexicographic
    // order is an encoding artifact, refused as semantics (10-data-model.md).
    // Large objects: facts stay fixed-width; the payload lives in external
    // storage, referenced by identity (the large-object refusal).

    //   (id) | Document(id, payload == ?digest);   // a bytes param self-encodes
}
```

## Vocabularies

## 6. The vocabulary

Guarantee: Lean theorem + validator/runtime premise — the sealed closed
extension is constant at every instance (`lean/Bumbledb/Schema.lean:
den_closed_constant`) and the compiled member-set containment admits only
declared priority handles.

The enum idiom's replacement, first-class: a vocabulary is a **closed
relation** — its ground axioms are declared in the schema, sealed at
validate, frozen by the fingerprint, virtual in storage
(`10-data-model.md` § closed relations). The store holds zero vocabulary
bytes, and handles are the literals on every surface: statements, queries,
Plan introspection, errors.

```rust
bumbledb::schema! {
    pub Tickets;

    // Tier 1: handles only. The macro emits the host enum `Priority`,
    // welded to declaration-order ids — an emission, not a type:
    // the engine's vocabulary stays relational; rustc's pattern matching
    // keeps working on the projection.
    closed relation Priority as PriorityId = { Low, Normal, Urgent };

    relation Ticket {
        id: u64 as TicketId, fresh,
        priority: u64 as PriorityId,
        opened_at: i64,
    }

    // A closed reference is an ordinary u64 under the handle newtype plus
    // one containment; the judgment compiles at validate to a member-set
    // test — one AND, one bit test, no probe (30-dependencies.md).
    Ticket(priority) <= Priority(id);

    // Handles are literals in queries exactly as in statements, and the
    // renderer prints them back — the round trip runs on names:
    //   (t) | Ticket(id: t, priority == Urgent);
    // A query atom over the vocabulary itself folds at prepare; the join
    // has zero runtime existence (40-execution.md § the grounding).
    // The boundary law: intrinsic meaning goes here (changing it is a new
    // theory); policy that drifts without a rebuild is an ordinary
    // relation — a vocabulary is never written, only declared.
}
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

    // Tier 2: payload columns state what each word MEANS, next to the
    // word. A rubric change is a new theory — exactly right for meaning.
    closed relation Kind as KindId {
        mastered: bool,
        rank: u64,
    } = {
        DirectPass { mastered: true,  rank: 30 },
        JudgedPass { mastered: true,  rank: 20 },
        Failed     { mastered: false, rank: 10 },
    };

    relation Attempt { id: u64 as AttemptId, fresh, kind: u64 as KindId }
    relation Certificate { attempt: u64 as AttemptId, kind: u64 as KindId }

    Attempt(kind) <= Kind(id);
    Certificate(attempt) -> Certificate;
    Certificate(attempt) <= Attempt(id);
    // ψ reads the payload: certificates carry mastered kinds only — the
    // member set {DirectPass, JudgedPass} compiles at validate and the
    // judgment is O(1) at commit (recipe 8 is this statement's own recipe).
    Certificate(kind) <= Kind(id | mastered == true);

    // The classification read duplicates no flag onto Attempt — ψ walks
    // the vocabulary's payload in the query too:
    //   (a) | Attempt(id: a, kind: k), Kind(id: k, mastered == true);
    // The Kind atom folds at prepare into a plan-constant handle set on
    // its sibling; plan introspection prints the set, not a count:
    //   folded: Kind{mastered == true} → {DirectPass, JudgedPass}
}
```

## 8. The sub-vocabulary

Guarantee: validator/runtime premise — ψ over the sealed extension compiles the
exact paging member set; a nonmember write is commit-rejected.

The ψ-selected containment: a reference constrained to the facts of a
vocabulary that satisfy a payload selection. Because the target is closed,
the enforcement plan is not a probe strategy — it is **the answer set
itself**, compiled at validate (`30-dependencies.md` § enforcement, whose
worked example this recipe rot-proofs).

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
        id: u64 as IncidentId, fresh,
        severity: u64 as SeverityId,
    }
    relation Escalation {
        incident: u64 as IncidentId,
        severity: u64 as SeverityId,
        at: i64,
    }

    Incident(severity) <= Severity(id);
    Escalation(incident) <= Incident(id);
    // The sub-vocabulary: an escalation carries a PAGING severity, by
    // statement. ψ over the sealed extension compiles to the member set
    // {Critical, Fatal}; the judgment is one bit test per touched fact,
    // and an escalation at severity == Info aborts the commit.
    Escalation(severity) <= Severity(id | pages == true);

    //   who is being paged (the same ψ, on the read side):
    //     (i) | Escalation(incident: i, severity: s), Severity(id: s, pages == true);
}
```

## Structure

## 9. Ordered collections

Guarantee: validator/runtime premise + host discipline — the composite key
permits one occupant per slot; ordering the result remains a host operation.

The linked-list verdict: successor pointers are control flow smuggled into
data — every reorder becomes a dependent chain of writes. Order is a value.

```rust
bumbledb::schema! {
    pub Playlists;

    relation Playlist { id: u64 as PlaylistId, fresh, name: str }
    // Explicit position column; write gapped strides (1024, 2048, ...) so
    // insertion between neighbors is one write, renumbering an amortized rarity.
    relation Entry { playlist: u64 as PlaylistId, pos: u64, track: str }

    Entry(playlist) <= Playlist(id);
    Entry(playlist, pos) -> Entry;          // one occupant per slot

    // Results are sets; the host sorts by pos (ordering is presentation —
    // the architecture README's OPEN item, not a query feature):
    //   (pos, track) | Entry(playlist == ?list, pos, track);
}
```

## 10. Trees and ASTs

Guarantee: Lean theorem + validator/runtime premises for key-backed arms
(`lean/Bumbledb/Dependencies.lean: keyed_eq_unique_correspondence`); host
discipline for acyclicity — statements prove arm/edge shape, never a tree theorem.

Node header + per-kind arms (recipe 2's pattern); every edge resolves; the
shape theorems come from FDs on the edge relations.

```rust
bumbledb::schema! {
    pub Ast;

    closed relation Kind as KindId = { Lit, Add };

    relation Node { id: u64 as NodeId, fresh, kind: u64 as KindId }
    relation Lit  { node: u64 as NodeId, value: i64 }
    relation Add  { node: u64 as NodeId, lhs: u64 as NodeId, rhs: u64 as NodeId }
    relation Parent { child: u64 as NodeId, parent: u64 as NodeId }

    Node(kind) <= Kind(id);
    Lit(node) -> Lit;
    Add(node) -> Add;
    // Every node's arm is total, valid, and exclusive (recipe 2's theorems):
    Node(id | kind == Lit) == Lit(node);
    Node(id | kind == Add) == Add(node);
    // Every child edge resolves — no dangling subtrees, judged at commit:
    Add(lhs) <= Node(id);
    Add(rhs) <= Node(id);
    // Functional parent (one parent per child) ⇒ the reachable shape is
    // paths-or-cycles; acyclicity itself is outside the ∀∃ vocabulary —
    // host discipline, recorded (statements never reference predicates,
    // 30-dependencies.md). Transitive reach is recipe 24's closure, in
    // either dialect, or a precomputed relation the host maintains.
    Parent(child) -> Parent;
    Parent(child)  <= Node(id);
    Parent(parent) <= Node(id);

    //   (v) | Add(node == ?n, lhs: l), Lit(node: l, value: v);
}
```

## 11. Typed graphs

Guarantee: validator/runtime premises — endpoint containments type each edge
and composite keys deduplicate pairs; no transitive graph property is claimed.

One relation per edge kind: the edge vocabulary is closed and checked —
endpoint containments pin which node kinds each edge may touch.

```rust
bumbledb::schema! {
    pub Graph;

    relation Person { id: u64 as PersonId, fresh, name: str }
    relation Repo   { id: u64 as RepoId, fresh, name: str }
    relation Follows   { follower: u64 as PersonId, followee: u64 as PersonId }
    relation Maintains { person: u64 as PersonId, repo: u64 as RepoId }

    Follows(follower) <= Person(id);        // a Person→Person edge, by statement —
    Follows(followee) <= Person(id);        // a Follows fact cannot touch a Repo
    Follows(follower, followee) -> Follows; // at most one edge per pair
    Maintains(person) <= Person(id);
    Maintains(repo)   <= Repo(id);
    Maintains(person, repo) -> Maintains;

    // Mutual follows — joins are explicit `field: v` on both ends (the
    // punning law); `<` keeps each pair once:
    //   (a, b) | Follows(follower: a, followee: b),
    //            Follows(follower: b, followee: a), a < b;
}
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

    relation Entity { id: u64 as EntityId, fresh, name: str }
    relation Transform  { entity: u64 as EntityId, x: i64, y: i64 }
    relation Velocity   { entity: u64 as EntityId, dx: i64, dy: i64 }
    relation Renderable { entity: u64 as EntityId, mesh: str }

    Transform(entity)  -> Transform;        // each component 0..1 per entity
    Transform(entity)  <= Entity(id);
    Velocity(entity)   -> Velocity;
    Velocity(entity)   <= Entity(id);
    Renderable(entity) -> Renderable;
    // An archetype rule is one containment: every Renderable has a Transform
    // (and, through it, an Entity — containment composes).
    Renderable(entity) <= Transform(entity);

    // The physics join is the component intersection:
    //   (e, x, y, dx, dy) | Transform(entity: e, x, y), Velocity(entity: e, dx, dy);
}
```

## 13. State machines

Guarantee: Lean theorem + validator/runtime premises for the shipped arm
(`lean/Bumbledb/Dependencies.lean: keyed_eq_unique_correspondence`); host
discipline for allowed transitions — equality pins state evidence, not paths.

States are a discriminated union; per-state data lives in arms; and the
conditional reference target — a reference to "an order *that is shipped*" —
is one selected containment, the statement SQL cannot write.

```rust
bumbledb::schema! {
    pub Orders;

    closed relation State as StateId = { Cart, Placed, Shipped };

    relation Order { id: u64 as OrderId, fresh, state: u64 as StateId }
    relation Placement { order: u64 as OrderId, at: i64 }
    relation Shipment  { order: u64 as OrderId, carrier: str, at: i64 }

    Order(state) <= State(id);
    Placement(order) -> Placement;
    Shipment(order)  -> Shipment;
    // History accretes: a Shipped order keeps its Placement — one-way <=
    // admits arms from earlier states surviving the transition.
    Placement(order) <= Order(id);
    // The conditional target, both ways: every Shipment references an order
    // THAT IS Shipped (validity), and every Shipped order has its Shipment
    // (totality) — the transition and its evidence commit together.
    Shipment(order) == Order(id | state == Shipped);
    // Transition predicates ("only Placed may ship") are host code under the
    // generation witness — recipe 20; the schema pins the states, not the paths.

    //   (id, carrier) | Order(id, state == Shipped), Shipment(order: id, carrier);
}
```

## Time and coverage

## 14. The calendar core

Guarantee: Lean theorem + validator/runtime premises — accepted equality is
key-backed correspondence (`lean/Bumbledb/Dependencies.lean:
keyed_eq_unique_correspondence`), while pointwise keys/coverage enforce only
declared hard policy.

Policy as schema: hard rules are pointwise keys, soft rules are the statements
you decline to write.

```rust
bumbledb::schema! {
    pub Calendar;

    closed relation Rsvp as RsvpId = { Accepted, Tentative, Declined };
    closed relation Arm as ArmId = { Busy, Ooo };

    relation Person { id: u64 as PersonId, fresh, name: str }
    relation Room   { id: u64 as RoomId, fresh, name: str }
    relation Event  { id: u64 as EventId, fresh, span: interval<i64> }
    relation Attendance {
        id: u64 as AttendanceId, fresh,
        event: u64 as EventId,
        person: u64 as PersonId,
        rsvp: u64 as RsvpId,
    }
    relation Claim {
        source: u64,
        person: u64 as PersonId,
        arm: u64 as ArmId,
        span: interval<i64>,
    }
    relation Booking   { room: u64 as RoomId, event: u64 as EventId, span: interval<i64> }
    relation WorkHours { person: u64 as PersonId, hours: interval<i64> }

    Attendance(event)  <= Event(id);
    Attendance(person) <= Person(id);
    Attendance(rsvp)   <= Rsvp(id);
    Attendance(event, person) -> Attendance;    // one RSVP per (event, person)
    Claim(source) -> Claim;
    Claim(person) <= Person(id);
    Claim(arm)    <= Arm(id);
    // HARD: rooms cannot double-book — the pointwise key (recipe 1's theorem).
    Booking(room, span) -> Booking;
    // SOFT: people CAN double-book — `Claim(person, span) -> Claim` is simply
    // not declared. Policy is the presence or absence of one statement.
    // Accepting an invitation IS claiming the time (totality + validity):
    Attendance(id | rsvp == Accepted) == Claim(source | arm == Busy);
    // Busy time lies inside working hours, pointwise — coverage rides the
    // target's own key (disjoint + ordered is a theorem, not a request):
    WorkHours(person, hours) -> WorkHours;
    Claim(person, span | arm == Busy) <= WorkHours(person, hours);
    Booking(room)  <= Room(id);
    Booking(event) <= Event(id);

    //   (room, s) | Booking(room, span: s), Allen(s, INTERSECTS, ?want);
    //   (person, s) | Claim(person, span: s), Allen(s, INTERSECTS, ?window);
}
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

```rust
bumbledb::schema! {
    pub Pricing;

    relation Policy  { id: u64 as PolicyId, fresh, live: interval<i64> }
    relation Version { policy: u64 as PolicyId, rate_bps: i64, valid: interval<i64> }

    Version(policy) <= Policy(id);
    // No overlapping versions: at any instant, at most one rate is the law.
    Version(policy, valid) -> Version;
    // No gaps in the policy lifetime: every source point is covered by versions.
    // Together with the key above this is a disjoint cover, not an exact
    // partition: Version intervals may overhang the Policy lifetime (recipe 16).
    Policy(id, live) <= Version(policy, valid);

    //   in force on date t — one membership probe:
    //     (rate_bps) | Version(policy == ?p, rate_bps, valid: v), ?t in v;
    //   clean successions (half-open makes MEETS exact, no ±1 fudge):
    //     (a, b) | Version(policy: p, valid: a), Version(policy: p, valid: b),
    //              Allen(a, MEETS, b);
}
```

## 16. Disjoint covers

Guarantee: Lean theorem/countermodel + validator/runtime premise —
`lean/Bumbledb/Dependencies.lean: coverage_is_support_inclusion` proves source
coverage, not exact partition (`lean/Bumbledb/Countermodels.lean:
one_way_overhang`).

Pay periods, shifts, estimated-tax quarters: a pointwise key plus one-way
coverage is a **disjoint cover** — no overlaps among pay periods and no holes
in the fiscal year's source span. Pay periods may extend beyond that span;
target overhang is legal under this statement. Historically this pattern was
called a tiling here; that was stronger than the judgment actually proved.

```rust
bumbledb::schema! {
    pub Payroll;

    relation FiscalYear { id: u64 as FiscalYearId, fresh, span: interval<i64> }
    relation PayPeriod  { year: u64 as FiscalYearId, seq: u64, span: interval<i64> }

    PayPeriod(year) <= FiscalYear(id);
    PayPeriod(year, seq)  -> PayPeriod;     // sequence numbers stay unique
    PayPeriod(year, span) -> PayPeriod;     // disjoint: no shared instant
    // Covering: no holes in the fiscal year's span; pay-period overhang is legal.
    FiscalYear(id, span) <= PayPeriod(year, span);

    //   the period holding date t:
    //     (seq) | PayPeriod(year == ?y, seq, span: s), ?t in s;
}
```

## 17. Federal income tax

Guarantee: validator/runtime premises + host discipline — keys prove bracket
disjointness and statements prove residency coverage; full bracket coverage and proration are host duties.

Brackets are intervals over money; the top bracket is a ray; regimes key on
(year, status); and proration happens at write time, never at query time.

```rust
bumbledb::schema! {
    pub Tax;

    closed relation Status as StatusId = { Single, MarriedJoint, HeadOfHousehold };

    relation Regime {
        id: u64 as RegimeId, fresh,
        year: i64,
        status: u64 as StatusId,
    }
    relation Bracket { regime: u64 as RegimeId, income: interval<i64>, rate_bps: i64 }
    relation Residency { person: u64, span: interval<i64> }
    // Split at write: an Earned fact never spans a year boundary — writers
    // split (prorate) at the boundary, so no reader ever clips. The
    // representation move that deletes clip-at-query (gravestone, recipe 23).
    relation Earned { person: u64, regime: u64 as RegimeId, span: interval<i64>, minor: i64 }

    Regime(status) <= Status(id);
    Regime(year, status) -> Regime;         // one regime per (year, filing status)
    Bracket(regime) <= Regime(id);
    // Brackets are disjoint per regime. Seed data conventionally covers [0, ∞)
    // and the top bracket is a ray, but this key proves disjointness only — it
    // does not prove coverage. end == MAX denotes [s, ∞), an honest value of
    // the representation, not a sentinel (the point-domain law, 10-data-model.md).
    Bracket(regime, income) -> Bracket;
    Earned(regime) <= Regime(id);
    Residency(person, span) -> Residency;
    // Residency exclusion: income counts only where earned inside a residency
    // period — pointwise coverage, the same judgment as recipe 15's.
    Earned(person, span) <= Residency(person, span);

    //   the marginal bracket (membership probes the disjoint bracket set):
    //     (rate_bps) | Regime(id: r, year == ?y, status == ?s),
    //                  Bracket(regime: r, income: b, rate_bps), ?taxable in b;
    // Tax owed is host arithmetic over the bracket walk — arithmetic beyond
    // the measure is refused (the ledger).
}
```

## 18. Free time and coalescing

Guarantee: Lean theorem + runtime query semantics — `Pack` coalesces answer
intervals (`lean/Bumbledb/Query/Aggregates.lean: pack_canonical`,
`pack_extensional`); it asserts no stored disjointness, completeness, or
maintenance behavior.

`Pack` is Snodgrass's coalesce as an aggregate — maximal disjoint segments per
group, one answer per (group, segment). Coalescing is never a write rule: the
engine stores the claims it was given.

```rust
bumbledb::schema! {
    pub FreeTime;

    relation Person { id: u64 as PersonId, fresh, name: str }
    relation Claim  { person: u64 as PersonId, span: interval<i64> }

    Claim(person) <= Person(id);
    // No pointwise key, on purpose: claims overlap freely and Pack coalesces
    // at read time. Wanting them stored-disjoint is recipe 1's key instead.

    //   busy time, coalesced (adjacent segments merge — the half-open law):
    //     (person, busy: Pack(span)) | Claim(person, span);
    //   raw claimed time (overlaps double-count — often the wrong question):
    //     (person, Sum(Duration(span))) | Claim(person, span);
    // Coalesced totals = the two-query composition (Pack, then a host fold) —
    // aggregates never nest; free time (gaps) is the two-line host walk over
    // sorted packed answers — both refusals recorded in the ledger.
}
```

## The write side

## 19. The ledger

Guarantee: Lean theorem + runtime invariant for bounded sums
(`lean/Bumbledb/Query/Aggregates.lean: checkedSum_sound`); host discipline
for double entry — statements resolve posting references, not arithmetic agreement.

The census workload. Balance is a query, never a column.

```rust
bumbledb::schema! {
    pub Ledger;

    relation Account      { id: u64 as AccountId, fresh, name: str }
    relation JournalEntry { id: u64 as JournalEntryId, fresh, at: i64, memo: str }
    relation Posting {
        id: u64 as PostingId, fresh,
        entry: u64 as JournalEntryId,
        account: u64 as AccountId,
        minor: i64,
    }

    Posting(entry)   <= JournalEntry(id);
    Posting(account) <= Account(id);
    // A stored balance column equaling Sum(postings) is the arithmetic-
    // agreement statement — refused (the ledger): statements prove presence
    // and topology, never that a value equals a computation. Balance is host
    // arithmetic over Sum; a materialized rollup is recipe 21's shape.

    //   balances (bind the fresh id — set semantics collapses duplicates):
    //     (account, total: Sum(minor)) | Posting(id, account, minor);
    //   double-entry audit (host asserts every total is 0 — discipline, not schema):
    //     (entry, Sum(minor)) | Posting(id, entry, minor);
}
```

## 20. Conditional writes

Guarantee: Lean theorem + generation-witness/runtime premise + host retry
discipline — snapshot-derived writes detect movement
(`lean/Bumbledb/Txn.lean: writeFrom_moved`, `witness_conflict_distinct`);
final-state point reads need no earlier witness.

The generation witness (`70-api.md` § conditional writes): read the model,
propose a delta, commit iff the model you read is still the model.

```rust
bumbledb::schema! {
    pub Jobs;

    closed relation State as StateId = { Queued, Running, Done };

    relation Job {
        id: u64 as JobId, fresh,
        state: u64 as StateId,
        payload: str,
    }
    relation Lease { job: u64 as JobId, worker: u64, until: i64 }

    Job(state) <= State(id);
    Lease(job) -> Lease;
    // A lease exists iff its job is Running (recipe 13's conditional target):
    // claiming a job and leasing it commit together or not at all.
    Lease(job) == Job(id | state == Running);

    // Three write idioms. The first two are snapshot-derived and therefore
    // use snapshot-query → compute → write_from(&snap) → host retry on
    // GenerationMoved:
    //   update-where: query the premise on a snapshot, then delete(old) +
    //     insert(new) per matched fact — "still Queued" is the witness:
    //       (id, payload) | Job(id, state == Queued, payload);
    //   insert-select: query source answers, insert the derived facts — the
    //     data-modifying CTE with its premises witnessed instead of locked.
    //   read-modify-write, key-shaped: WriteTx point reads (get/contains) see
    //     the final state — per-fact premises need no earlier snapshot witness.
}
```

## 21. Derived relations

Guarantee: Lean theorem + validator/runtime premises for soundness
(`lean/Bumbledb/Txn.lean: derived_soundness_vs_freshness`); host
discipline for completeness — containment rejects unsupported facts but never refreshes omissions.

The materialized view as a relation under statements — unsoundness the schema
can name is uncommittable; incompleteness remains representable until the host
refreshes it (`10-data-model.md` § derived relations owns this).

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
    BusySpan(person, span) -> BusySpan;     // packed ⇒ disjoint: statable
    // Soundness, pointwise: every stored rollup point is covered by busy
    // claims — an UNSOUND rollup (claiming busy time that isn't, or surviving
    // its sources' deletion) cannot commit, judged on every touching commit.
    BusySpan(person, span) <= Claim(person, span | arm == Busy);

    // Maintenance is the third witness idiom (recipe 20): re-run the deriving
    // query on a snapshot, diff, write_from(&snap) — the rollup cannot commit
    // against sources it didn't actually read.
    //   the deriving query (Pack IS the coalesce):
    //     (person, busy: Pack(span)) | Claim(person, span, arm == Busy);
}
```

## 22. Union reads

Guarantee: Lean theorem + represented planner/runtime premise — rule union is
set-idempotent (`lean/Bumbledb/Query/Denotation.lean: union_idempotent`);
key-backed DU arms justify the disjointness optimization
(`lean/Bumbledb/Exec/Dedup.lean: disjoint_witness_licence`).

The whole-DU read is a set of rules: one head, one rule per arm — disjunction
is data at the top, never an execution node.

```rust
bumbledb::schema! {
    pub Payments;

    closed relation Kind as KindId = { Card, Ach };

    relation Payment { id: u64 as PaymentId, fresh, kind: u64 as KindId }
    relation Card { payment: u64 as PaymentId, last4: u64 }
    relation Ach  { payment: u64 as PaymentId, routing: u64 }

    Payment(kind) <= Kind(id);
    Card(payment) -> Card;
    Ach(payment)  -> Ach;
    Payment(id | kind == Card) == Card(payment);
    Payment(id | kind == Ach)  == Ach(payment);

    // One query, two rules (set union). The exclusivity theorem (recipe 2)
    // is spent a third time here: rules selecting different `kind` values are
    // provably disjoint, so the executor elides cross-rule dedup — the free
    // lunch (40-execution.md § set semantics).
    //   (id, n) | Payment(id, kind == Card), Card(payment: id, last4: n);
    //   (id, n) | Payment(id, kind == Ach),  Ach(payment: id, routing: n);
}
```

## 23. The anti-recipes: five gravestones

Guarantee: intentionally refused — each gravestone names unsupported vocabulary
and its representable replacement; none asserts an engine theorem.

What not to model. Each gravestone cites its replacement; the block's
relations are the replacements, compiled.

```rust
bumbledb::schema! {
    pub Gravestones;

    // GRAVESTONE: successor pointers (a `next` column). A linked list inside
    // a relation is control flow smuggled into data; every reorder is a
    // dependent chain of writes. REPLACEMENT: position columns (recipe 9).
    relation Step { flow: u64, pos: u64, action: str }
    // GRAVESTONE: floats for scores, rates, money. Permanently refused (the
    // ledger). REPLACEMENT: fixed-point i64 — basis points (recipe 4).
    relation Score { subject: u64, bps: i64 }
    // GRAVESTONE: conditional keys ("at most one active run per student") —
    // rejected as FDs. REPLACEMENT: the relation split, whose ordinary key IS
    // the invariant (30-dependencies.md; recipe 13's arm shape).
    relation ActiveRun { student: u64, run: u64 }
    // GRAVESTONE: clip-at-query intervals (facts spanning period boundaries,
    // every reader clipping). REPLACEMENT: split at write (recipe 17) — split
    // the fact at the boundary; readers stop clipping because nothing spans.
    relation Usage { meter: u64, period: u64, used: interval<i64> }
    // GRAVESTONE: uuid keys. uuidv7 is identity + clash-avoidance + clock in
    // one lie. REPLACEMENT: fresh (minted identity) + an explicit i64 time
    // column (10-data-model.md).
    relation Event { id: u64 as GravestoneEventId, fresh, at: i64 }

    Step(flow, pos)    -> Step;
    Score(subject)     -> Score;
    ActiveRun(student) -> ActiveRun;
    Usage(meter, used) -> Usage;
}
```

## Host-driven closure

## 24. The closure idiom

Guarantee: host discipline for the loop — the finite `seen` set proves
termination for the host run; the engine-native form beside it executes
whole under the fixpoint driver, budget-bounded
(`lean/Bumbledb/Exec/Fixpoint.lean: program_eval_sound`).

Reachability, in two dialects. The host-loop idiom remains the
depth-bounded answer: the censused hierarchies are **depth-bounded**, so
the loop runs depth-many rounds and each round is one ∈-set query — a
`ParamSet` probe, microsecond-class — against the engine as it stands. The
frontier discipline below *is* semi-naive evaluation's Δ, spent where a loop
is a loop: the host. The engine-native form (below) is the same closure as
one stratified program (`20-query-ir.md` § engine recursion): a named head
declares the predicate, the bare rule is the output, and the driver runs
the rounds inside one plan (`40-execution.md` § the fixpoint driver).

```rust
bumbledb::schema! {
    pub Closure;

    relation Node   { id: u64 as NodeId, fresh, name: str }
    // One parent per child — a forest (recipe 10's edge shape); a root
    // is a node whose Parent fact is absent (recipe 3's honest 0..1).
    relation Parent { child: u64 as NodeId, parent: u64 as NodeId }

    Parent(child) -> Parent;
    Parent(child)  <= Node(id);
    Parent(parent) <= Node(id);

    // The loop's one query — the frontier's children, one ∈-set probe:
    //   (c) | Parent(child: c, parent in ?frontier);
}
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
one plan) — write the engine-native form instead:

```text
// The same closure, one stratified program under the fixpoint driver:
// ?root seeds the predicate, the bare rule is the output.
let native = query!(Closure {
    reach(c) | Node(id: c), c == ?root;
    reach(c) | Parent(child: c, parent: m), reach(0: m);
    (c) | reach(0: c);
});
let mut prepared = db.prepare_program(&native)?;
```

(the compiled copy runs beside the loop in `cookbook.rs`, both dialects
asserting the same reachable sets, root for root). What stays host-side is
the **chain-window class** — interval intersection along paths — which the
recursion surface fences out (`20-query-ir.md` § engine recursion, the
chain-window fence): the idiom carries the window in the host's frontier,
one intersection per hop, and that composition has no engine form.

## 25. The chart of accounts

Guarantee: host discipline + runtime aggregate semantics — the host computes
closure, then one checked `Sum` (`lean/Bumbledb/Query/Aggregates.lean:
checkedSum_sound`); the engine-native form folds over a *finished* lower
stratum, the one aggregation shape the strata roster admits
(`20-query-ir.md` § engine recursion).

The ledger workload's real recursion case, in the same two dialects: a
hierarchical chart of accounts and a subtree rollup. The host composition —
recipe 24's loop accumulates the subtree's ∈-set, then **one `Sum` query
over the accumulated set** folds the postings. The engine aggregates, the
host composes (aggregates never nest — recipe 18's refusal family). The
engine-native form is one program: aggregation *through* a cycle is refused
(`AggregationThroughCycle`), but a fold over a recursive predicate from a
**higher stratum** reads a finished set and is ordinary —

```text
let native = query!(Accounts {
    sub(a) | Account(id: a), a == ?root;
    sub(a) | AccountParent(child: a, parent: p), sub(0: p);
    (total: Sum(minor)) | Posting(id, account: a, minor), sub(0: a);
});
```

— the closure stratum converges first, then the output's fold runs once
over the finished subtree (the compiled copy in `cookbook.rs` asserts both
dialects against the hand-computed sums).

```rust
bumbledb::schema! {
    pub Accounts;

    relation Account { id: u64 as AccountId, fresh, name: str }
    relation AccountParent { child: u64 as AccountId, parent: u64 as AccountId }
    relation Posting {
        id: u64 as PostingId, fresh,
        account: u64 as AccountId,
        minor: i64,
    }

    AccountParent(child) -> AccountParent;   // one parent per account
    AccountParent(child)  <= Account(id);
    AccountParent(parent) <= Account(id);
    Posting(account) <= Account(id);

    // The two queries the rollup composes:
    //   the frontier step (recipe 24's loop, verbatim):
    //     (c) | AccountParent(child: c, parent in ?frontier);
    //   the rollup over the accumulated subtree (bind the fresh id —
    //   recipe 19's discipline, spent again):
    //     (total: Sum(minor)) | Posting(id, account in ?subtree, minor);
}
```

The rollup is two prepared queries with the recipe-24 loop between them;
the test drives a three-level hierarchy with postings and asserts the
hand-computed subtree sum — equal postings to one account both count,
because the fresh id keeps their bindings distinct.

## 26. Exact partition

Guarantee: Lean theorem + validator/runtime premises — mutual point coverage
plus pointwise keys realizes exact partition
(`lean/Bumbledb/Dependencies.lean: exact_partition_iff`).

An exact partition needs both coverage directions. The first containment below
is the intent-level reference; the two pointwise keys make each side disjoint;
the final pair proves equal point supports per policy — forward coverage forbids
gaps and reverse coverage forbids overhang. This is not mere tiling language:
it is the five ordinary statements witnessing
`exactTiling_iff_exactPointPartition`.

The explicit `Policy(id, live) -> Policy` is load-bearing. Containment targets
resolve by their exact projected field set, so the fresh `{id}` key cannot serve
the `{id, live}` target and the engine infers no key closure.

```rust
bumbledb::schema! {
    pub ExactPartition;

    relation Policy  { id: u64 as PolicyId, fresh, live: interval<i64> }
    relation Version { policy: u64 as PolicyId, valid: interval<i64> }

    Version(policy) <= Policy(id);             // reference intent
    Version(policy, valid) -> Version;          // disjoint versions
    Policy(id, live) -> Policy;                 // exact target key, not implied by {id}
    Policy(id, live) <= Version(policy, valid); // no gaps in the policy source span
    Version(policy, valid) <= Policy(id, live); // no version overhang
}
```

Together the mutual containments prove equal point supports for each policy;
the pointwise keys make those supports genuine partitions rather than overlapping
covers. Touching half-open segments remain legal, and the same construction works
with any scalar-prefix arity before the final interval position.

## 27. Derived facts, maintained

Guarantee: host discipline + validator/runtime premises — freshness comes from
the generation witness; containment proves surviving rollup facts sound only
(`lean/Bumbledb/Txn.lean: derived_soundness_vs_freshness`).

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

    // Derive the desired rollup on the maintenance snapshot:
    //   (person, busy: Pack(span)) |
    //       Claim(source, person, arm == Busy, span);
}
```

The host loop is snapshot → derive → diff → `write_from(snapshot)`. On
`GenerationMoved`, it throws away the derived set and diff and starts from a new
snapshot; it never retries a stale diff. Dependencies prove every surviving
stored span sound, while the witness proves which source state the derivation
saw; neither mechanism proves completeness. The compiled copy is
`maintain_busy_spans` in `cookbook.rs`; its lock moves the source generation
between derive and commit, observes one retry, and then asserts the recomputed
packed span.

## Operating the store

## 28. Migration is ETL

Guarantee: Lean theorem + validator/runtime premises + host discipline —
fingerprints refuse reinterpretation, final-state judgments validate each load
(`lean/Bumbledb/Txn.lean: etl_lands_valid`), and the host owns
the semantic transform and dependency-safe load order.

There is no in-place migration and never will be: a schema is a theory,
the store records the theory's fingerprint, and `Db::open` under a changed
theory is a hard `SchemaMismatch` — the engine refuses to reinterpret facts
it judged under different laws. Migration is extract, transform, load:
`scan` exports every fact of a relation as typed values under one snapshot
(one generation — the export is a consistent instant), the host transforms,
and `bulk_load` imports into a store created under the new theory. The
engine owns both ends; the host owns exactly the middle, because the
semantic transform is the part that cannot be generic.

Three laws make the loop honest. **Load containment targets first** — every
chunk commits through the ordinary final-state judgment, so a `Salary` fact
whose `Employee` has not landed yet is a rejection (with the complete
violation set cited), not a deferral. **Fresh identity survives** —
`bulk_load` takes explicit values for every field, `fresh` ones included,
so facts keep their ids across the move, and the mint sequence catches up
past the imported high water: the next `alloc` cannot collide. **The new
theory judges the old data** — every dependency of the new schema holds of
every migrated fact, or the chunk aborts whole. A migration that lands is
already valid; there is no "migrate now, validate later."

The v2 theory below adds what v1 (shown as text) never recorded — *when* a
salary applied — as an interval with a pointwise functionality: one salary
per employee per instant. The transform supplies the missing dimension (a
ray from the migration epoch), which is the honest reading of "the old
amount, still in force."

```text
pub PayrollV1;                     // the old theory, judged and fingerprinted

relation Employee { id: u64 as EmployeeId, fresh, name: str }
relation Salary   { employee: u64 as EmployeeId, amount: i64 }

Salary(employee) <= Employee(id);
```

```rust
bumbledb::schema! {
    pub Payroll;

    relation Employee { id: u64 as EmployeeId, fresh, name: str }
    relation Salary {
        employee: u64 as EmployeeId,
        amount: i64,
        applies: interval<i64>,
    }

    Salary(employee) <= Employee(id);
    Salary(employee, applies) -> Salary;   // one salary per instant

    // The post-migration read — salaries in force at an instant:
    //   (name, amount) | Employee(id: e, name),
    //                    Salary(employee: e, amount, applies: w), ?at in w;
}
```

The compiled test drives the whole loop: seed a v1 store, export both
relations under one snapshot, drop the v1 handle and prove the
fingerprint refusal (`Db::open` of the v1 store under `Payroll` is
`SchemaMismatch`), append the ray to each salary, load employees before
salaries, then prove the three laws — identity (the v1 ids answer the v2 query), catch-up (the next
minted id clears the imported high water), and judgment (the migrated
store answers under the new theory's guarantees).
