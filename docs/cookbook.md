# The cookbook — modeling intuition as schemas

Worked schemas for the owner and any agent writing a theory. **This document is
illustrative, never normative**: where a recipe and an architecture chapter
disagree, the chapter wins and the recipe is amended in the same change
(`docs/architecture/README.md` rule 5). The chapters it defers to:
`docs/architecture/10-data-model.md` (the seven types, the interval denotation,
the modeling discipline, derived relations), `30-dependencies.md` (the two
judgments and their theorems), `20-query-ir.md` (query semantics; § the query
notation is the grammar the recipes' query comments are written in), `70-api.md`
(the `schema!` grammar, conditional writes). Refusals cited below live in
`docs/prd-algebra/README.md`'s ledger.

Every schema below compiles and validates verbatim against the current engine —
`crates/bumbledb-query/tests/cookbook.rs` duplicates each block token-for-token
and a sync test pins the duplication, so a recipe edited here without the test
following breaks the build.

## Foundations

## 1. The minimal interval schema

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
    //   down at instant t (membership is a typing rule, not syntax):
    //     (service) | Outage(service, window: w), ?t in w;
    //   overlapping an incident window (one Allen mask, no operator zoo):
    //     (service, w) | Outage(service, window: w), Allen(w, INTERSECTS, ?incident);
    //   total downtime per service (the denotation's one arithmetic):
    //     (service, Sum(Duration(window))) | Outage(service, window);
}
```

## 2. Discriminated unions

Sum-typed entities: a discriminator enum plus per-arm child relations, glued by
bidirectional conditional containments (`30-dependencies.md` § the derivations).

```rust
bumbledb::schema! {
    pub Grading;

    relation Task { id: u64 as TaskId, fresh, kind: enum Kind { Deterministic, CustomOperator } }
    relation DeterministicGrading  { task: u64 as TaskId, tolerance: i64 }
    relation CustomOperatorGrading { task: u64 as TaskId, operator: str }

    DeterministicGrading(task)  -> DeterministicGrading;   // one arm row per parent
    CustomOperatorGrading(task) -> CustomOperatorGrading;
    // Totality (==, left to right): a Deterministic task HAS its arm row —
    // same commit, always. Arm validity (right to left): an arm row's parent
    // exists WITH that kind — composite-FK-plus-CHECK, one statement.
    Task(id | kind == Deterministic)  == DeterministicGrading(task);
    Task(id | kind == CustomOperator) == CustomOperatorGrading(task);
    // Exclusivity is a theorem, not a statement: one id in two arms would
    // force `kind` to equal two variants against the fresh key on id.
    // The executor spends the same theorem again — recipe 19's free lunch.
}
```

## 3. 0..1 optional attributes

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

Fixed-point i64 minor units; the host newtype owns scale and currency. Floats
are permanently refused (the ledger); proration and FX are host arithmetic.

```rust
bumbledb::schema! {
    pub Money;

    relation Account { id: u64 as AccountId, fresh, name: str }
    // Minor units in i64 (±92 quadrillion cents); `as Minor` is the host
    // newtype — rustc polices cross-domain confusion, not the engine
    // (hard structural typing, 10-data-model.md).
    relation Posting {
        id: u64 as PostingId, fresh,
        account: u64 as AccountId,
        currency: enum Currency { Usd, Eur, Gbp },
        minor: i64 as Minor,
    }

    Posting(account) <= Account(id);

    // Multi-currency totals: currency is a group key, never summed across —
    // Sum folds in i128 with one final range check, so totals cannot wrap
    // silently. Bind the fresh id: set semantics would collapse two equal
    // (account, currency, minor) postings without it.
    //   (account, currency, total: Sum(minor)) | Posting(id, account, currency, minor);
}
```

## 5. Content addressing

The decision rule for byte-shaped data: **intern what repeats (`str`); inline
what identifies (`bytes<N>`)** — `10-data-model.md` § the type layer.

```rust
bumbledb::schema! {
    pub Content;

    relation Document {
        id: u64 as DocumentId, fresh,
        name: str,                          // repeats: interned, id-equality
        payload: bytes<32> as PayloadHash,  // identifies: the blake3 of the
    }                                       // external blob — inline, never interned
    relation Replica { payload: bytes<32> as PayloadHash, region: enum Region { Us, Eu } }

    Document(payload) -> Document;          // content-addressed: one doc per digest
    Replica(payload) <= Document(payload);
    // bytes<N> is identity-only (Eq/Ne, membership): a digest's lexicographic
    // order is an encoding artifact, refused as semantics (10-data-model.md).
    // Large objects: facts stay fixed-width; the payload lives in external
    // storage, referenced by identity (the large-object refusal).

    //   (id) | Document(id, payload == ?digest);   // a bytes param self-encodes
}
```

## Structure

## 6. Ordered collections

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

## 7. Trees and ASTs

Node header + per-kind arms (recipe 2's pattern); every edge resolves; the
shape theorems come from FDs on the edge relations.

```rust
bumbledb::schema! {
    pub Ast;

    relation Node { id: u64 as NodeId, fresh, kind: enum Kind { Lit, Add } }
    relation Lit  { node: u64 as NodeId, value: i64 }
    relation Add  { node: u64 as NodeId, lhs: u64 as NodeId, rhs: u64 as NodeId }
    relation Parent { child: u64 as NodeId, parent: u64 as NodeId }

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
    // host discipline, recorded. Recursion stays refused (README OPEN item);
    // transitive reach is a precomputed relation the host maintains.
    Parent(child) -> Parent;
    Parent(child)  <= Node(id);
    Parent(parent) <= Node(id);

    //   (v) | Add(node == ?n, lhs: l), Lit(node: l, value: v);
}
```

## 8. Typed graphs

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
    Follows(followee) <= Person(id);        // a Follows row cannot touch a Repo
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

## 9. Entity-component

The 0..1 idiom (recipe 3) at scale: components are sidecar relations; an
entity has a component iff the fact exists; a new component kind is a new
relation, not a wider row.

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

## 10. State machines

States are a discriminated union; per-state data lives in arms; and the
conditional reference target — a reference to "an order *that is shipped*" —
is one selected containment, the statement SQL cannot write.

```rust
bumbledb::schema! {
    pub Orders;

    relation Order { id: u64 as OrderId, fresh, state: enum State { Cart, Placed, Shipped } }
    relation Placement { order: u64 as OrderId, at: i64 }
    relation Shipment  { order: u64 as OrderId, carrier: str, at: i64 }

    Placement(order) -> Placement;
    Shipment(order)  -> Shipment;
    // History accretes: a Shipped order keeps its Placement — one-way <=
    // admits arms from earlier states surviving the transition.
    Placement(order) <= Order(id);
    // The conditional target, both ways: every Shipment references an order
    // THAT IS Shipped (validity), and every Shipped order has its Shipment
    // (totality) — the transition and its evidence commit together.
    Shipment(order) == Order(id | state == Shipped);
    // Transition guards ("only Placed may ship") are host code under the
    // generation witness — recipe 17; the schema pins the states, not the paths.

    //   (id, carrier) | Order(id, state == Shipped), Shipment(order: id, carrier);
}
```

## Time and tilings

## 11. The calendar core

Policy as schema: hard rules are pointwise keys, soft rules are the statements
you decline to write.

```rust
bumbledb::schema! {
    pub Calendar;

    relation Person { id: u64 as PersonId, fresh, name: str }
    relation Room   { id: u64 as RoomId, fresh, name: str }
    relation Event  { id: u64 as EventId, fresh, span: interval<i64> }
    relation Attendance {
        id: u64 as AttendanceId, fresh,
        event: u64 as EventId,
        person: u64 as PersonId,
        rsvp: enum Rsvp { Accepted, Tentative, Declined },
    }
    relation Claim {
        source: u64,
        person: u64 as PersonId,
        arm: enum Arm { Busy, Ooo },
        span: interval<i64>,
    }
    relation Booking   { room: u64 as RoomId, event: u64 as EventId, span: interval<i64> }
    relation WorkHours { person: u64 as PersonId, hours: interval<i64> }

    Attendance(event)  <= Event(id);
    Attendance(person) <= Person(id);
    Attendance(event, person) -> Attendance;    // one RSVP per (event, person)
    Claim(source) -> Claim;
    Claim(person) <= Person(id);
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

## 12. Effective-dated configuration

Versioned rules: no overlaps (pointwise key), no gaps (coverage), and "in
force on date t" is one membership probe.

```rust
bumbledb::schema! {
    pub Pricing;

    relation Policy  { id: u64 as PolicyId, fresh, live: interval<i64> }
    relation Version { policy: u64 as PolicyId, rate_bps: i64, valid: interval<i64> }

    Version(policy) <= Policy(id);
    // No overlapping versions: at any instant, at most one rate is the law.
    Version(policy, valid) -> Version;
    // No gaps: every point of the policy's lifetime is covered by versions —
    // together with the key above, versions TILE the lifetime (recipe 13).
    Policy(id, live) <= Version(policy, valid);

    //   in force on date t — one membership probe:
    //     (rate_bps) | Version(policy == ?p, rate_bps, valid: v), ?t in v;
    //   clean successions (half-open makes MEETS exact, no ±1 fudge):
    //     (a, b) | Version(policy: p, valid: a), Version(policy: p, valid: b),
    //              Allen(a, MEETS, b);
}
```

## 13. Tilings

Pay periods, shifts, estimated-tax quarters: **disjoint + covering = a
tiling** — no overlaps, no holes, two statements.

```rust
bumbledb::schema! {
    pub Payroll;

    relation FiscalYear { id: u64 as FiscalYearId, fresh, span: interval<i64> }
    relation PayPeriod  { year: u64 as FiscalYearId, seq: u64, span: interval<i64> }

    PayPeriod(year) <= FiscalYear(id);
    PayPeriod(year, seq)  -> PayPeriod;     // sequence numbers stay unique
    PayPeriod(year, span) -> PayPeriod;     // disjoint: no shared instant
    FiscalYear(id, span) <= PayPeriod(year, span);  // covering: no holes

    //   the period holding date t:
    //     (seq) | PayPeriod(year == ?y, seq, span: s), ?t in s;
}
```

## 14. Federal income tax

Brackets are intervals over money; the top bracket is a ray; regimes key on
(year, status); and proration happens at write time, never at query time.

```rust
bumbledb::schema! {
    pub Tax;

    relation Regime {
        id: u64 as RegimeId, fresh,
        year: i64,
        status: enum Status { Single, MarriedJoint, HeadOfHousehold },
    }
    relation Bracket { regime: u64 as RegimeId, income: interval<i64>, rate_bps: i64 }
    relation Residency { person: u64, span: interval<i64> }
    // Tile at write: an Earned fact never spans a year boundary — writers
    // split (prorate) at the boundary, so no reader ever clips. The
    // representation move that deletes clip-at-query (gravestone, recipe 20).
    relation Earned { person: u64, regime: u64 as RegimeId, span: interval<i64>, minor: i64 }

    Regime(year, status) -> Regime;         // one regime per (year, filing status)
    Bracket(regime) <= Regime(id);
    // Brackets tile [0, ∞): disjoint per regime, and the TOP BRACKET IS A RAY —
    // end == MAX denotes [s, ∞), an honest value of the representation, not a
    // sentinel (the point-domain law, 10-data-model.md).
    Bracket(regime, income) -> Bracket;
    Earned(regime) <= Regime(id);
    Residency(person, span) -> Residency;
    // Residency exclusion: income counts only where earned inside a residency
    // period — pointwise coverage, the same judgment as recipe 12's.
    Earned(person, span) <= Residency(person, span);

    //   the marginal bracket (membership walks the tiling):
    //     (rate_bps) | Regime(id: r, year == ?y, status == ?s),
    //                  Bracket(regime: r, income: b, rate_bps), ?taxable in b;
    // Tax owed is host arithmetic over the bracket walk — arithmetic beyond
    // the measure is refused (the ledger).
}
```

## 15. Free time and coalescing

`Pack` is Snodgrass's coalesce as an aggregate — maximal disjoint segments per
group, one row per (group, segment). Coalescing is never a write rule: the
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
    // sorted packed rows — both refusals recorded in the ledger.
}
```

## The write side

## 16. The ledger

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
    // arithmetic over Sum; a materialized rollup is recipe 18's shape.

    //   balances (bind the fresh id — set semantics collapses duplicates):
    //     (account, total: Sum(minor)) | Posting(id, account, minor);
    //   double-entry audit (host asserts every total is 0 — discipline, not schema):
    //     (entry, Sum(minor)) | Posting(id, entry, minor);
}
```

## 17. Conditional writes

The generation witness (`70-api.md` § conditional writes): read the model,
propose a delta, commit iff the model you read is still the model.

```rust
bumbledb::schema! {
    pub Jobs;

    relation Job {
        id: u64 as JobId, fresh,
        state: enum State { Queued, Running, Done },
        payload: str,
    }
    relation Lease { job: u64 as JobId, worker: u64, until: i64 }

    Lease(job) -> Lease;
    // A lease exists iff its job is Running (recipe 10's conditional target):
    // claiming a job and leasing it commit together or not at all.
    Lease(job) == Job(id | state == Running);

    // The three witness idioms, each snapshot-query → compute →
    // write_from(&snap) → host retry on GenerationMoved:
    //   update-where: query the premise on a snapshot, then delete(old) +
    //     insert(new) per matched fact — "still Queued" is the witness:
    //       (id, payload) | Job(id, state == Queued, payload);
    //   insert-select: query source rows, insert the derived facts — the
    //     data-modifying CTE with its premises witnessed instead of locked.
    //   read-modify-write, key-shaped: WriteTx point reads (get/contains) see
    //     the final state — per-fact premises need no witness, never retry.
}
```

## 18. Derived relations

The materialized view as a relation under statements — staleness the schema
can name is uncommittable (`10-data-model.md` § derived relations owns this).

```rust
bumbledb::schema! {
    pub Rollup;

    relation Claim {
        source: u64,
        person: u64,
        arm: enum Arm { Busy, Ooo },
        span: interval<i64>,
    }
    relation BusySpan { person: u64, span: interval<i64> }

    Claim(source) -> Claim;
    Claim(person, span) -> Claim;
    BusySpan(person, span) -> BusySpan;     // packed ⇒ disjoint: statable
    // Soundness, pointwise: every stored rollup point is covered by busy
    // claims — an UNSOUND rollup (claiming busy time that isn't, or surviving
    // its sources' deletion) cannot commit, judged on every touching commit.
    BusySpan(person, span) <= Claim(person, span | arm == Busy);

    // Maintenance is the third witness idiom (recipe 17): re-run the deriving
    // query on a snapshot, diff, write_from(&snap) — the rollup cannot commit
    // against sources it didn't actually read.
    //   the deriving query (Pack IS the coalesce):
    //     (person, busy: Pack(span)) | Claim(person, span, arm == Busy);
}
```

## 19. Union reads

The whole-DU read is a set of rules: one head, one rule per arm — disjunction
is data at the top, never an execution node.

```rust
bumbledb::schema! {
    pub Payments;

    relation Payment { id: u64 as PaymentId, fresh, kind: enum Kind { Card, Ach } }
    relation Card { payment: u64 as PaymentId, last4: u64 }
    relation Ach  { payment: u64 as PaymentId, routing: u64 }

    Card(payment) -> Card;
    Ach(payment)  -> Ach;
    Payment(id | kind == Card) == Card(payment);
    Payment(id | kind == Ach)  == Ach(payment);

    // One query, two clauses (set union). The exclusivity theorem (recipe 2)
    // is spent a third time here: rules selecting different `kind` values are
    // provably disjoint, so the executor elides cross-rule dedup — the free
    // lunch (40-execution.md § set semantics).
    //   (id, n) | Payment(id, kind == Card), Card(payment: id, last4: n);
    //   (id, n) | Payment(id, kind == Ach),  Ach(payment: id, routing: n);
}
```

## 20. The anti-recipes: five gravestones

What not to model. Each gravestone cites its replacement; the block's
relations are the replacements, compiled.

```rust
bumbledb::schema! {
    pub Gravestones;

    // GRAVESTONE: successor pointers (a `next` column). A linked list inside
    // a relation is control flow smuggled into data; every reorder is a
    // dependent chain of writes. REPLACEMENT: position columns (recipe 6).
    relation Step { flow: u64, pos: u64, action: str }
    // GRAVESTONE: floats for scores, rates, money. Permanently refused (the
    // ledger). REPLACEMENT: fixed-point i64 — basis points (recipe 4).
    relation Score { subject: u64, bps: i64 }
    // GRAVESTONE: conditional keys ("at most one active run per student") —
    // rejected as FDs. REPLACEMENT: the relation split, whose ordinary key IS
    // the invariant (30-dependencies.md; recipe 10's arm shape).
    relation ActiveRun { student: u64, run: u64 }
    // GRAVESTONE: clip-at-query intervals (facts spanning period boundaries,
    // every reader clipping). REPLACEMENT: tile at write (recipe 14) — split
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
