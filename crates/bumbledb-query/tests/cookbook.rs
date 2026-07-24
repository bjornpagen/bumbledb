//! Rot-proofing for `docs/cookbook.md` (the intuition unit): every cookbook
//! schema compiles and **validates** verbatim against the current engine,
//! the roster is enumerated with a count assertion (a doc recipe without a
//! test entry fails), and every doc query fence compiles via `query!`,
//! prepares against a real store of its recipe's schema, and round-trips
//! through `ir::render`, notation.rs-style.
//!
//! Include-or-duplicate: **duplicate** — markdown cannot be `include!`d at
//! item position, so each block is duplicated here and the sync test pins
//! the duplication token-for-token against the doc: editing either copy
//! without the other fails `doc_blocks_match_the_compiled_copies`. The doc's
//! rust fences are classified: each recipe has ONE schema fence (starts
//! `bumbledb::schema!`; compared comment-stripped — the token stream never
//! carries comments) and zero-or-more query fences (`let <name> = query!`;
//! compared WITHOUT comment-stripping — they are code, and the stringified
//! twin cannot carry a comment, so a commented query fence is drift).

use bumbledb::schema::ValidateDescriptor as _;
use std::collections::BTreeSet;

use bumbledb::ir::render::{render, render_program};
use bumbledb::ir::{Program, Value};
use bumbledb::{
    AnswerValue, Answers, BindValue, Db, Fact, ParamArg, PreparedQuery, Query, Schema, Snapshot,
    Theory,
};
use bumbledb_query::query;

const COOKBOOK: &str = include_str!("../../../docs/cookbook.md");

mod common;
use common::TempDir;

/// One module per recipe: the schema compiled, its token source pinned for
/// the doc-sync test, a validation entry point for the roster test, and the
/// recipe's query fences — each compiled by `query!` from the SAME tokens
/// its `QUERIES` pin stringifies (the duplicate-and-pin law, query side),
/// with its `ir::render` golden beside it. The emitted host enums live in
/// the module, so bare handles (`priority == Urgent`) resolve in place.
macro_rules! recipe {
    ($m:ident, $theory:ident, { $($t:tt)* }) => {
        recipe!($m, $theory, { $($t)* }, queries {});
    };
    ($m:ident, $theory:ident, { $($t:tt)* }, queries {
        $( $qname:ident: { $($q:tt)* } => $golden:literal; )*
    }) => {
        mod $m {
            bumbledb::schema! { $($t)* }
            pub const SOURCE: &str = stringify!($($t)*);
            /// The doc's query fences, reconstructed from the compiling tokens.
            pub const QUERIES: &[&str] = &[$(concat!(
                "let ", stringify!($qname), " = query!(", stringify!($theory),
                " { ", stringify!($($q)*), " });"
            )),*];
            /// The `ir::render` goldens, one per query fence, in doc order.
            pub const RENDERS: &[&str] = &[$($golden),*];
            pub fn validate() -> Result<bumbledb::Schema, bumbledb::error::SchemaError> {
                use bumbledb::Theory as _;
                use bumbledb::schema::ValidateDescriptor as _;
                $theory.descriptor().validate()
            }
            /// Prepares every query fence against a real store of this
            /// recipe's theory and renders it — the `pin()` discipline.
            pub fn pin() -> Vec<String> {
                crate::pin_all(concat!("cookbook-pin-", stringify!($m)), $theory, &[$(
                    crate::PinnedQuery::from(bumbledb_query::query!($theory { $($q)* }))
                ),*])
            }
        }
    };
}

recipe!(r01, Uptime, {
    pub Uptime;

    relation Service { id: u64 as ServiceId, fresh, name: str }
    relation Outage  { service: u64 as ServiceId, window: interval<i64> }

    Outage(service) <= Service(id);
    Outage(service, window) -> Outage;
}, queries {
    down_at: { (service) | Outage(service, window: w), ?t in w; }
        => "(v0) | Outage(service: v0, window: v1), ?0 in v1;";
    overlapping: { (service, w) | Outage(service, window: w), Allen(w, INTERSECTS, ?incident); }
        => "(v0, v1) | Outage(service: v0, window: v1), Allen(v1, INTERSECTS, ?0);";
    downtime: { (service, Sum(Duration(window))) | Outage(service, window); }
        => "(v0, Sum(Duration(v1))) | Outage(service: v0, window: v1);";
});

recipe!(r02, Grading, {
    pub Grading;

    closed relation Kind as KindId = { Deterministic, CustomOperator };

    relation Task { id: u64 as TaskId, fresh, kind: u64 as KindId }
    relation DeterministicGrading  { task: u64 as TaskId, tolerance: i64 }
    relation CustomOperatorGrading { task: u64 as TaskId, operator: str }

    Task(kind) <= Kind(id);
    DeterministicGrading(task)  -> DeterministicGrading;
    CustomOperatorGrading(task) -> CustomOperatorGrading;
    Task(id | kind == Deterministic)  == DeterministicGrading(task);
    Task(id | kind == CustomOperator) == CustomOperatorGrading(task);
});

recipe!(r03, Optionality, {
    pub Optionality;

    relation Business { id: u64 as BusinessId, fresh, name: str }
    relation MailingAddress { business: u64 as BusinessId, line: str, city: str }

    MailingAddress(business) -> MailingAddress;
    MailingAddress(business) <= Business(id);
}, queries {
    unaddressed: { (b) | Business(id: b), !MailingAddress(business: b); }
        => "(v0) | Business(id: v0), !MailingAddress(business: v0);";
});

recipe!(r04, Money, {
    pub Money;

    closed relation Currency as CurrencyId = { Usd, Eur, Gbp };

    relation Account { id: u64 as AccountId, fresh, name: str }
    relation Posting {
        id: u64 as PostingId, fresh,
        account: u64 as AccountId,
        currency: u64 as CurrencyId,
        minor: i64 as Minor,
    }

    Posting(account)  <= Account(id);
    Posting(currency) <= Currency(id);
}, queries {
    totals: { (account, currency, total: Sum(minor)) | Posting(id, account, currency, minor); }
        => "(v1, v2, Sum(v3)) | Posting(id: v0, account: v1, currency: v2, minor: v3);";
});

recipe!(r05, Content, {
    pub Content;

    closed relation Region as RegionId = { Us, Eu };

    relation Document {
        id: u64 as DocumentId, fresh,
        name: str,
        payload: bytes<32> as PayloadHash,
    }
    relation Replica { payload: bytes<32> as PayloadHash, region: u64 as RegionId }

    Document(payload) -> Document;
    Replica(payload) <= Document(payload);
    Replica(region)  <= Region(id);
}, queries {
    by_digest: { (id) | Document(id, payload == ?digest); }
        => "(v0) | Document(id: v0, payload == ?0);";
});

recipe!(r06, Tickets, {
    pub Tickets;

    closed relation Priority as PriorityId = { Low, Normal, Urgent };

    relation Ticket {
        id: u64 as TicketId, fresh,
        priority: u64 as PriorityId,
        opened_at: i64,
    }

    Ticket(priority) <= Priority(id);
}, queries {
    urgent: { (t) | Ticket(id: t, priority == Urgent); }
        => "(v0) | Ticket(id: v0, priority == Urgent);";
});

recipe!(r07, Review, {
    pub Review;

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
    Certificate(kind) <= Kind(id | mastered == true);
}, queries {
    mastered: { (a) | Attempt(id: a, kind: k), Kind(id: k, mastered == true); }
        => "(v0) | Attempt(id: v0, kind: v1), Kind(id: v1, mastered == true);";
});

recipe!(r08, Oncall, {
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
    Escalation(severity) <= Severity(id | pages == true);
}, queries {
    paged: { (i) | Escalation(incident: i, severity: s), Severity(id: s, pages == true); }
        => "(v0) | Escalation(incident: v0, severity: v1), Severity(id: v1, pages == true);";
});

recipe!(r09, Playlists, {
    pub Playlists;

    relation Playlist { id: u64 as PlaylistId, fresh, name: str }
    relation Extent { playlist: u64 as PlaylistId, span: interval<u64> }
    relation Slot { playlist: u64 as PlaylistId, slot: interval<u64, 1>, track: str }

    Extent(playlist) <= Playlist(id);
    Slot(playlist)   <= Playlist(id);
    Extent(playlist) -> Extent;
    Extent(playlist, span) -> Extent;
    Slot(playlist, slot) -> Slot;
    Extent(playlist, span) == Slot(playlist, slot);
}, queries {
    at_pos: { (track) | Slot(playlist == ?list, slot: s, track), ?pos in s; }
        => "(v1) | Slot(playlist == ?0, slot: v0, track: v1), ?1 in v0;";
});

recipe!(r10, Ast, {
    pub Ast;

    closed relation Kind as KindId = { Lit, Add };

    relation Node { id: u64 as NodeId, fresh, kind: u64 as KindId }
    relation Lit  { node: u64 as NodeId, value: i64 }
    relation Add  { node: u64 as NodeId, lhs: u64 as NodeId, rhs: u64 as NodeId }
    relation Parent { child: u64 as NodeId, parent: u64 as NodeId }

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
}, queries {
    lhs_lit: { (v) | Add(node == ?n, lhs: l), Lit(node: l, value: v); }
        => "(v1) | Add(node == ?0, lhs: v0), Lit(node: v0, value: v1);";
});

recipe!(r11, Graph, {
    pub Graph;

    relation Person { id: u64 as PersonId, fresh, name: str }
    relation Repo   { id: u64 as RepoId, fresh, name: str }
    relation Follows   { follower: u64 as PersonId, followee: u64 as PersonId }
    relation Maintains { person: u64 as PersonId, repo: u64 as RepoId }

    Follows(follower) <= Person(id);
    Follows(followee) <= Person(id);
    Follows(follower, followee) -> Follows;
    Maintains(person) <= Person(id);
    Maintains(repo)   <= Repo(id);
    Maintains(person, repo) -> Maintains;
}, queries {
    mutual: { (a, b) | Follows(follower: a, followee: b),
                       Follows(follower: b, followee: a), a < b; }
        => "(v0, v1) | Follows(follower: v0, followee: v1), \
            Follows(follower: v1, followee: v0), v0 < v1;";
});

recipe!(r12, Ecs, {
    pub Ecs;

    relation Entity { id: u64 as EntityId, fresh, name: str }
    relation Transform  { entity: u64 as EntityId, x: i64, y: i64 }
    relation Velocity   { entity: u64 as EntityId, dx: i64, dy: i64 }
    relation Renderable { entity: u64 as EntityId, mesh: str }

    Transform(entity)  -> Transform;
    Transform(entity)  <= Entity(id);
    Velocity(entity)   -> Velocity;
    Velocity(entity)   <= Entity(id);
    Renderable(entity) -> Renderable;
    Renderable(entity) <= Transform(entity);
}, queries {
    physics: { (e, x, y, dx, dy) | Transform(entity: e, x, y), Velocity(entity: e, dx, dy); }
        => "(v0, v1, v2, v3, v4) | Transform(entity: v0, x: v1, y: v2), \
            Velocity(entity: v0, dx: v3, dy: v4);";
});

recipe!(r13, Orders, {
    pub Orders;

    closed relation State as StateId = { Cart, Placed, Shipped };

    relation Order { id: u64 as OrderId, fresh, state: u64 as StateId }
    relation Placement { order: u64 as OrderId, at: i64 }
    relation Shipment  { order: u64 as OrderId, carrier: str, at: i64 }

    Order(state) <= State(id);
    Placement(order) -> Placement;
    Shipment(order)  -> Shipment;
    Placement(order) <= Order(id);
    Shipment(order) == Order(id | state == Shipped);
}, queries {
    shipped: { (id, carrier) | Order(id, state == Shipped), Shipment(order: id, carrier); }
        => "(v0, v1) | Order(id: v0, state == Shipped), Shipment(order: v0, carrier: v1);";
});

recipe!(r14, Calendar, {
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
        source: u64 as AttendanceId,
        person: u64 as PersonId,
        arm: u64 as ArmId,
        span: interval<i64>,
    }
    relation Booking   { room: u64 as RoomId, event: u64 as EventId, span: interval<i64> }
    relation WorkHours { person: u64 as PersonId, hours: interval<i64> }

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
}, queries {
    room_conflicts: { (room, s) | Booking(room, span: s), Allen(s, INTERSECTS, ?want); }
        => "(v0, v1) | Booking(room: v0, span: v1), Allen(v1, INTERSECTS, ?0);";
    busy_people: { (person, s) | Claim(person, span: s), Allen(s, INTERSECTS, ?window); }
        => "(v0, v1) | Claim(person: v0, span: v1), Allen(v1, INTERSECTS, ?0);";
});

recipe!(r15, Pricing, {
    pub Pricing;

    relation Policy  { id: u64 as PolicyId, fresh, live: interval<i64> }
    relation Version { policy: u64 as PolicyId, rate_bps: i64, valid: interval<i64> }

    Version(policy) <= Policy(id);
    Version(policy, valid) -> Version;
    Policy(id, live) <= Version(policy, valid);
}, queries {
    in_force: { (rate_bps) | Version(policy == ?p, rate_bps, valid: v), ?t in v; }
        => "(v0) | Version(policy == ?0, rate_bps: v0, valid: v1), ?1 in v1;";
    successions: { (a, b) | Version(policy: p, valid: a), Version(policy: p, valid: b),
                            Allen(a, MEETS, b); }
        => "(v1, v2) | Version(policy: v0, valid: v1), Version(policy: v0, valid: v2), \
            Allen(v1, MEETS, v2);";
});

recipe!(r16, Payroll, {
    pub Payroll;

    relation FiscalYear { id: u64 as FiscalYearId, fresh, span: interval<i64> }
    relation PayPeriod  { year: u64 as FiscalYearId, seq: u64, span: interval<i64> }

    PayPeriod(year) <= FiscalYear(id);
    PayPeriod(year, seq)  -> PayPeriod;
    PayPeriod(year, span) -> PayPeriod;
    FiscalYear(id, span) <= PayPeriod(year, span);
}, queries {
    holding: { (seq) | PayPeriod(year == ?y, seq, span: s), ?t in s; }
        => "(v0) | PayPeriod(year == ?0, seq: v0, span: v1), ?1 in v1;";
});

recipe!(r17, Tax, {
    pub Tax;

    closed relation Status as StatusId = { Single, MarriedJoint, HeadOfHousehold };

    relation Regime {
        id: u64 as RegimeId, fresh,
        year: i64,
        status: u64 as StatusId,
    }
    relation Bracket { regime: u64 as RegimeId, income: interval<i64>, rate_bps: i64 }
    relation Residency { person: u64, span: interval<i64> }
    relation Earned { person: u64, regime: u64 as RegimeId, span: interval<i64>, minor: i64 }

    Regime(status) <= Status(id);
    Regime(year, status) -> Regime;
    Bracket(regime) <= Regime(id);
    Bracket(regime, income) -> Bracket;
    Earned(regime) <= Regime(id);
    Residency(person, span) -> Residency;
    Earned(person, span) <= Residency(person, span);
}, queries {
    marginal: { (rate_bps) | Regime(id: r, year == ?y, status == ?s),
                             Bracket(regime: r, income: b, rate_bps), ?taxable in b; }
        => "(v2) | Regime(id: v0, year == ?0, status == ?1), \
            Bracket(regime: v0, income: v1, rate_bps: v2), ?2 in v1;";
});

recipe!(r18, FreeTime, {
    pub FreeTime;

    relation Person { id: u64 as PersonId, fresh, name: str }
    relation Claim  { person: u64 as PersonId, span: interval<i64> }

    Claim(person) <= Person(id);
}, queries {
    busy: { (person, busy: Pack(span)) | Claim(person, span); }
        => "(v0, Pack(v1)) | Claim(person: v0, span: v1);";
    claimed: { (person, Sum(Duration(span))) | Claim(person, span); }
        => "(v0, Sum(Duration(v1))) | Claim(person: v0, span: v1);";
});

recipe!(r19, Ledger, {
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
}, queries {
    balances: { (account, total: Sum(minor)) | Posting(id, account, minor); }
        => "(v1, Sum(v2)) | Posting(id: v0, account: v1, minor: v2);";
    audit: { (entry, Sum(minor)) | Posting(id, entry, minor); }
        => "(v1, Sum(v2)) | Posting(id: v0, entry: v1, minor: v2);";
});

recipe!(r20, Jobs, {
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
    Lease(job) == Job(id | state == Running);
}, queries {
    queued: { (id, payload) | Job(id, state == Queued, payload); }
        => "(v0, v1) | Job(id: v0, state == Queued, payload: v1);";
});

recipe!(r21, Rollup, {
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
}, queries {
    deriving: { (person, busy: Pack(span)) | Claim(person, span, arm == Busy); }
        => "(v0, Pack(v1)) | Claim(person: v0, span: v1, arm == Busy);";
});

recipe!(r22, Payments, {
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
}, queries {
    methods: { (id, n) | Payment(id, kind == Card), Card(payment: id, last4: n);
               (id, n) | Payment(id, kind == Ach), Ach(payment: id, routing: n); }
        => "(v0, v1) | Payment(id: v0, kind == Card), Card(payment: v0, last4: v1);\n\
            (v0, v1) | Payment(id: v0, kind == Ach), Ach(payment: v0, routing: v1);";
});

recipe!(r23, Gravestones, {
    pub Gravestones;

    relation Step { flow: u64, pos: u64, action: str }
    relation Score { subject: u64, bps: i64 }
    relation ActiveRun { student: u64, run: u64 }
    relation Usage { meter: u64, period: u64, used: interval<i64> }
    relation Event { id: u64 as GravestoneEventId, fresh, at: i64 }

    Step(flow, pos)    -> Step;
    Score(subject)     -> Score;
    ActiveRun(student) -> ActiveRun;
    Usage(meter, used) -> Usage;
});

recipe!(r24, Closure, {
    pub Closure;

    relation Node   { id: u64 as NodeId, fresh, name: str }
    relation Parent { child: u64 as NodeId, parent: u64 as NodeId }

    Parent(child) -> Parent;
    Parent(child)  <= Node(id);
    Parent(parent) <= Node(id);
}, queries {
    children: { (c) | Parent(child: c, parent in ?frontier); }
        => "(v0) | Parent(child: v0, parent in ?0);";
    native: { reach(c) | Node(id: c), c == ?root;
              reach(c) | Parent(child: c, parent: m), reach(m);
              (c) | reach(c); }
        => "p0(v0) | Node(id: v0), v0 == ?0;\n\
            p0(v0) | Parent(child: v0, parent: v1), p0(v1);\n\
            (v0) | p0(v0);";
});

recipe!(r25, Accounts, {
    pub Accounts;

    relation Account { id: u64 as AccountId, fresh, name: str }
    relation AccountParent { child: u64 as AccountId, parent: u64 as AccountId }
    relation Posting {
        id: u64 as PostingId, fresh,
        account: u64 as AccountId,
        minor: i64,
    }

    AccountParent(child) -> AccountParent;
    AccountParent(child)  <= Account(id);
    AccountParent(parent) <= Account(id);
    Posting(account) <= Account(id);
}, queries {
    native: { sub(a) | Account(id: a), a == ?root;
              sub(a) | AccountParent(child: a, parent: p), sub(p);
              (total: Sum(minor)) | Posting(id, account: a, minor), sub(a); }
        => "p0(v0) | Account(id: v0), v0 == ?0;\n\
            p0(v0) | AccountParent(child: v0, parent: v1), p0(v1);\n\
            (Sum(v2)) | Posting(id: v0, account: v1, minor: v2), p0(v1);";
    children: { (c) | AccountParent(child: c, parent in ?frontier); }
        => "(v0) | AccountParent(child: v0, parent in ?0);";
    rollup: { (total: Sum(minor)) | Posting(id, account in ?subtree, minor); }
        => "(Sum(v1)) | Posting(id: v0, account in ?0, minor: v1);";
});

recipe!(r26, ExactPartition, {
    pub ExactPartition;

    relation Policy  { id: u64 as PolicyId, fresh, live: interval<i64> }
    relation Version { policy: u64 as PolicyId, valid: interval<i64> }

    Version(policy) <= Policy(id);
    Version(policy, valid) -> Version;
    Policy(id, live) -> Policy;
    Policy(id, live) <= Version(policy, valid);
    Version(policy, valid) <= Policy(id, live);
});

recipe!(r27, MaintainedRollup, {
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
}, queries {
    deriving: { (person, busy: Pack(span)) | Claim(source, person, arm == Busy, span); }
        => "(v1, Pack(v2)) | Claim(source: v0, person: v1, arm == Busy, span: v2);";
});

mod composite_partition {
    bumbledb::schema! {
        pub CompositePartition;

        relation Domain  { group: u64, lane: u64, live: interval<i64> }
        relation Segment { group: u64, lane: u64, valid: interval<i64> }

        Segment(group, lane, valid) -> Segment;
        Domain(group, lane, live) -> Domain;
        Domain(group, lane, live) <= Segment(group, lane, valid);
        Segment(group, lane, valid) <= Domain(group, lane, live);
    }
}

recipe!(r28, Payroll, {
    pub Payroll;

    relation Employee { id: u64 as EmployeeId, fresh, name: str }
    relation Salary {
        employee: u64 as EmployeeId,
        amount: i64,
        applies: interval<i64>,
    }

    Salary(employee) <= Employee(id);
    Salary(employee, applies) -> Salary;
}, queries {
    in_force: { (name, amount) | Employee(id: e, name),
                                 Salary(employee: e, amount, applies: w), ?at in w; }
        => "(v1, v2) | Employee(id: v0, name: v1), \
            Salary(employee: v0, amount: v2, applies: v3), ?0 in v3;";
});

/// Recipe 28's OLD theory — the v1 store the migration exports from. Not
/// a roster entry (the doc shows it as text, not a pinned schema block):
/// the recipe's pinned schema is the v2 target above; v1 exists so the
/// compiled test can drive the whole ETL loop against two real theories.
mod r28_old {
    bumbledb::schema! {
        pub PayrollV1;

        relation Employee { id: u64 as EmployeeId, fresh, name: str }
        relation Salary   { employee: u64 as EmployeeId, amount: i64 }

        Salary(employee) <= Employee(id);
    }
}

recipe!(r29, ZoneLedger, {
    pub ZoneLedger;

    closed relation Kind as KindId = { Unit, Pair };

    relation Ledger   { id: u64 as LedgerId, fresh, name: str }
    relation Zone     { ledger: u64 as LedgerId, kind: u64 as KindId, at: interval<u64> }
    relation UnitSlot { ledger: u64 as LedgerId, at: interval<u64, 1>, entry: u64 }
    relation PairSlot { ledger: u64 as LedgerId, at: interval<u64, 2>, entry: u64 }

    Zone(ledger) <= Ledger(id);
    Zone(kind)   <= Kind(id);
    Zone(ledger, at) -> Zone;
    UnitSlot(ledger, at) -> UnitSlot;
    PairSlot(ledger, at) -> PairSlot;
    Zone(ledger, at | kind == Unit) == UnitSlot(ledger, at);
    Zone(ledger, at | kind == Pair) == PairSlot(ledger, at);
});

recipe!(r30, KeyedRead, {
    pub KeyedRead;

    relation Grp     { id: u64 as GrpId, fresh, label: str }
    relation Program {
        id: u64 as ProgramId, fresh,
        grp: u64 as GrpId,
        title: str,
    }

    Program(grp) <= Grp(id);
    Program(grp) -> Program;
});

/// The roster, exhaustively — one entry per doc recipe, in doc order: the
/// schema pin, the validation entry, and the query-fence pins (doc-fence
/// sources, render goldens, and the prepare-and-render `pin`).
struct Recipe {
    title: &'static str,
    source: &'static str,
    validate: fn() -> Result<Schema, bumbledb::error::SchemaError>,
    queries: &'static [&'static str],
    renders: &'static [&'static str],
    pin: fn() -> Vec<String>,
}

/// One roster entry, wired to its recipe module's pinned surfaces.
macro_rules! entry {
    ($m:ident, $title:literal) => {
        Recipe {
            title: $title,
            source: $m::SOURCE,
            validate: $m::validate,
            queries: $m::QUERIES,
            renders: $m::RENDERS,
            pin: $m::pin,
        }
    };
}

const ROSTER: [Recipe; 30] = [
    entry!(r01, "The minimal interval schema"),
    entry!(r02, "Discriminated unions"),
    entry!(r03, "0..1 optional attributes"),
    entry!(r04, "Money"),
    entry!(r05, "Content addressing"),
    entry!(r06, "The vocabulary"),
    entry!(r07, "The classification"),
    entry!(r08, "The sub-vocabulary"),
    entry!(r09, "Ordered collections"),
    entry!(r10, "Trees and ASTs"),
    entry!(r11, "Typed graphs"),
    entry!(r12, "Entity-component"),
    entry!(r13, "State machines"),
    entry!(r14, "The calendar core"),
    entry!(r15, "Effective-dated configuration"),
    entry!(r16, "Disjoint covers"),
    entry!(r17, "Federal income tax"),
    entry!(r18, "Free time and coalescing"),
    entry!(r19, "The ledger"),
    entry!(r20, "Conditional writes"),
    entry!(r21, "Derived relations"),
    entry!(r22, "Union reads"),
    entry!(r23, "The anti-recipes: five gravestones"),
    entry!(r24, "The closure idiom"),
    entry!(r25, "The chart of accounts"),
    entry!(r26, "Exact partition"),
    entry!(r27, "Derived facts, maintained"),
    entry!(r28, "Migration is ETL"),
    entry!(r29, "The zone ledger"),
    entry!(r30, "The keyed read"),
];

/// Comments and whitespace out; what remains is exactly what the token
/// stream carries, so a stringified duplicate compares against a doc block.
/// Schema fences only — query fences go through `squish`.
fn normalize(text: &str) -> String {
    text.lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .flat_map(str::chars)
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Whitespace out, comments KEPT: query fences are code, so their pin is
/// exact — a comment inside a doc query fence is drift by definition (the
/// stringified twin cannot carry one), never silently ignored.
fn squish(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// The doc's numbered recipe headings, `## N. Title`, in order.
fn doc_headings() -> Vec<(usize, String)> {
    COOKBOOK
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .filter(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        .map(|rest| {
            let (n, title) = rest.split_once(". ").expect("a numbered recipe heading");
            (n.parse().expect("a recipe number"), title.to_owned())
        })
        .collect()
}

/// One doc recipe's `rust` fences, classified: the ONE schema fence (starts
/// `bumbledb::schema!`) and its query fences (`let <name> = query!(...)`),
/// in doc order — a query fence may precede its schema fence (recipe 25's
/// engine-native program sits in the prose above the block).
struct DocRecipe {
    number: usize,
    schema: String,
    queries: Vec<String>,
}

/// The doc's fenced `rust` blocks, grouped per numbered recipe heading and
/// classified — an unclassifiable fence is a failure, never a skip.
fn doc_recipes() -> Vec<DocRecipe> {
    let mut recipes: Vec<DocRecipe> = Vec::new();
    let mut fence: Option<String> = None;
    for line in COOKBOOK.lines() {
        if fence.is_none()
            && let Some(rest) = line.strip_prefix("## ")
        {
            if let Some((number, _)) = rest.split_once(". ")
                && let Ok(number) = number.parse()
            {
                recipes.push(DocRecipe {
                    number,
                    schema: String::new(),
                    queries: Vec::new(),
                });
            }
            continue;
        }
        match &mut fence {
            None if line.trim() == "```rust" => fence = Some(String::new()),
            None => {}
            Some(block) if line.trim() == "```" => {
                let block = std::mem::take(block);
                fence = None;
                let recipe = recipes
                    .last_mut()
                    .expect("a rust fence sits under a numbered recipe heading");
                let head = block.trim_start();
                if head.starts_with("bumbledb::schema!") {
                    assert!(
                        recipe.schema.is_empty(),
                        "recipe {} has two schema fences",
                        recipe.number
                    );
                    recipe.schema = block;
                } else if head.starts_with("let ") {
                    recipe.queries.push(block);
                } else {
                    panic!(
                        "recipe {} has an unclassifiable rust fence: {head:?}",
                        recipe.number
                    );
                }
            }
            Some(block) => {
                block.push_str(line);
                block.push('\n');
            }
        }
    }
    recipes
}

/// The first nonblank line after every numbered heading. The label is prose,
/// not part of the schema token stream, so the sync lock inventories it
/// separately while walking the same ordered recipe corpus.
fn doc_labels() -> Vec<(usize, String)> {
    let lines: Vec<_> = COOKBOOK.lines().collect();
    let mut labels = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("## ") else {
            continue;
        };
        let Some((number, _)) = rest.split_once(". ") else {
            continue;
        };
        let Ok(number) = number.parse() else {
            continue;
        };
        let label = lines[index + 1..]
            .iter()
            .find(|candidate| !candidate.trim().is_empty())
            .expect("a recipe heading has following prose")
            .trim();
        labels.push((number, label.to_owned()));
    }
    labels
}

/// The count assertion: the doc's numbered recipes are exactly the roster —
/// a recipe added to the doc without a test entry (or the reverse) fails here.
#[test]
fn the_doc_roster_is_exactly_this_roster() {
    let headings = doc_headings();
    assert_eq!(
        headings.len(),
        ROSTER.len(),
        "doc recipes and test entries must correspond one-to-one"
    );
    for (i, ((n, title), recipe)) in headings.iter().zip(ROSTER.iter()).enumerate() {
        assert_eq!(*n, i + 1, "recipe numbering is 1..=30 in order");
        assert_eq!(title, recipe.title, "recipe {} title", i + 1);
    }
}

/// Every doc schema block is token-identical to its compiled duplicate, and
/// every doc query fence is token-identical to its pinned twin — the same
/// duplicate-and-pin law on both fence classes, plus the query-fence census.
#[test]
fn doc_blocks_match_the_compiled_copies() {
    let recipes = doc_recipes();
    let labels = doc_labels();
    assert_eq!(
        recipes.len(),
        ROSTER.len(),
        "one classified fence corpus per recipe, in roster order"
    );
    assert_eq!(
        labels.len(),
        ROSTER.len(),
        "one guarantee label per recipe, in roster order"
    );
    for (index, (number, label)) in labels.iter().enumerate() {
        assert_eq!(*number, index + 1, "label numbering follows the roster");
        assert!(
            label.starts_with("Guarantee: "),
            "recipe {number} has no immediate Guarantee label: {label:?}"
        );
    }
    let mut query_fences = 0;
    for (i, (doc, recipe)) in recipes.iter().zip(ROSTER.iter()).enumerate() {
        assert_eq!(doc.number, i + 1, "recipe numbering follows the roster");
        assert!(
            !doc.schema.is_empty(),
            "recipe {} ({}) has no schema fence",
            i + 1,
            recipe.title
        );
        let expected = format!("bumbledb::schema!{{{}}}", normalize(recipe.source));
        assert_eq!(
            normalize(&doc.schema),
            expected,
            "recipe {} ({}) drifted between doc and test",
            i + 1,
            recipe.title
        );
        assert_eq!(
            doc.queries.len(),
            recipe.queries.len(),
            "recipe {} ({}) query fences and pinned twins must correspond one-to-one",
            i + 1,
            recipe.title
        );
        for (j, (fence, twin)) in doc.queries.iter().zip(recipe.queries.iter()).enumerate() {
            assert_eq!(
                squish(fence),
                squish(twin),
                "recipe {} ({}) query {} drifted between doc and test",
                i + 1,
                recipe.title,
                j + 1
            );
        }
        query_fences += doc.queries.len();
    }
    assert_eq!(
        query_fences, 34,
        "the doc's compiled query fences, exhaustively"
    );
}

/// Every recipe's schema validates against the current engine — the compile
/// is the macro's half, this is the validation roster's half.
#[test]
fn every_recipe_schema_validates() {
    for (i, recipe) in ROSTER.iter().enumerate() {
        (recipe.validate)().unwrap_or_else(|e| {
            panic!(
                "recipe {} ({}) failed validation: {e:?}",
                i + 1,
                recipe.title
            )
        });
    }
}

/// The 64-char lowercase hex of an engine fingerprint — the goldens spelling.
fn hex_of(fingerprint: &bumbledb::schema::fingerprint::SchemaFingerprint) -> String {
    fingerprint
        .0
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            use std::fmt::Write;
            write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
            hex
        })
}

/// The per-recipe cross-host goldens (PRD-T5): every roster schema's
/// fingerprint equals its pinned line in the ONE shared fixture,
/// `ts/test/fixtures/cookbook-fingerprints.txt` — the same file the SDK
/// cookbook suite (`ts/test/cookbook.test.ts`) asserts against, and alone
/// regenerates (`REGEN_FINGERPRINTS=1`; this side never writes it). The TS
/// side lowers a names-only spec through the napi bridge into the SAME
/// resolution/validation/blake3 code, so a divergence here is drift
/// upstream of the hasher on whichever side moved. Missing, extra, or
/// malformed fixture lines are failures, never skips.
#[test]
fn every_recipe_fingerprint_matches_the_cross_host_golden() {
    let fixture = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../ts/test/fixtures/cookbook-fingerprints.txt"
    ));
    let mut goldens = std::collections::BTreeMap::new();
    for line in fixture.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (id, hex) = line
            .split_once(' ')
            .unwrap_or_else(|| panic!("a goldens line is `rNN <64-hex>`: {line:?}"));
        let number: usize = id
            .strip_prefix('r')
            .and_then(|digits| digits.parse().ok())
            .unwrap_or_else(|| panic!("a goldens recipe id is rNN: {id:?}"));
        assert_eq!(hex.len(), 64, "a golden is 64 hex chars: {line:?}");
        assert!(
            goldens.insert(number, hex).is_none(),
            "one goldens line per recipe: {id}"
        );
    }
    assert_eq!(
        goldens.len(),
        ROSTER.len(),
        "the fixture pins exactly one line per roster recipe"
    );
    let mut drifted = Vec::new();
    for (i, recipe) in ROSTER.iter().enumerate() {
        let number = i + 1;
        let expected = goldens
            .get(&number)
            .unwrap_or_else(|| panic!("recipe {number} ({}) has no goldens line", recipe.title));
        let schema = (recipe.validate)().unwrap_or_else(|e| {
            panic!(
                "recipe {number} ({}) failed validation: {e:?}",
                recipe.title
            )
        });
        let hex = hex_of(&bumbledb::schema::fingerprint::fingerprint(&schema));
        if hex != *expected {
            drifted.push(format!(
                "recipe {number} ({}): rust {hex} != golden {expected}",
                recipe.title
            ));
        }
    }
    assert!(
        drifted.is_empty(),
        "recipes drifted from the cross-host goldens:\n{}",
        drifted.join("\n")
    );
}

fn span(start: i64, end: i64) -> bumbledb::Interval<i64> {
    bumbledb::Interval::<i64>::new(start, end).expect("nonempty half-open interval")
}

fn assert_containment_statement(error: bumbledb::Error, expected: bumbledb::schema::StatementId) {
    let bumbledb::Error::CommitRejected { violations } = error else {
        panic!("expected a containment rejection, got {error}");
    };
    let [
        bumbledb::Violation::Containment {
            statement,
            direction,
            ..
        },
    ] = violations.as_slice()
    else {
        panic!("expected one containment citation, got {violations:?}");
    };
    assert_eq!(*statement, expected);
    assert_eq!(*direction, bumbledb::error::Direction::SourceUnsatisfied);
}

fn assert_r26_schema_shape() {
    use bumbledb::schema::{StatementId, StatementView};

    // The fresh {id} key coexists with two distinct pointwise keys. Both
    // interval containments validate because their exact target field sets
    // resolve independently; no key closure is inferred.
    let schema = r26::validate().expect("the five-statement schema validates");
    assert_eq!(schema.keys().len(), 3);
    assert_eq!(
        schema.keys().iter().filter(|key| key.pointwise()).count(),
        2
    );
    for statement in [StatementId(4), StatementId(5)] {
        assert!(matches!(
            schema.statement(statement),
            StatementView::Containment(..)
        ));
    }
}

/// Recipe 26's theorem-to-runtime matrix. Point sets are written out in
/// each arm: forward coverage rejects source gaps, reverse coverage rejects
/// target overhang, and the one-way recipe deliberately accepts that overhang.
#[test]
fn r26_exact_partition_commit_matrix() {
    use bumbledb::schema::StatementId;
    use composite_partition::{CompositePartition, Domain, Segment};
    use r16::{FiscalYear, FiscalYearId, PayPeriod, Payroll};
    use r26::{ExactPartition, Policy, PolicyId, Version};

    assert_r26_schema_shape();

    // Exact and adjacent: [0,2) ∪ [2,5) = [0,5). Half-open adjacency
    // shares no point, so the Version pointwise key accepts the touching pair.
    let dir = TempDir::new("r26-exact-adjacent");
    let db = Db::create(dir.path(), ExactPartition).expect("create exact partition store");
    db.write(|tx| {
        let policy = PolicyId(1);
        tx.insert(&Policy {
            id: policy,
            live: span(0, 5),
        })?;
        tx.insert(&Version {
            policy,
            valid: span(0, 2),
        })?;
        tx.insert(&Version {
            policy,
            valid: span(2, 5),
        })
    })
    .expect("adjacent segments form an exact partition");

    // Gap only: [0,4) ∪ [5,10) leaves point support [4,5) uncovered.
    // Each Version remains inside [0,10), so only forward statement 4 fails.
    let dir = TempDir::new("r26-gap");
    let db = Db::create(dir.path(), ExactPartition).expect("create gap store");
    let error = db
        .write(|tx| {
            let policy = PolicyId(2);
            tx.insert(&Policy {
                id: policy,
                live: span(0, 10),
            })?;
            tx.insert(&Version {
                policy,
                valid: span(0, 4),
            })?;
            tx.insert(&Version {
                policy,
                valid: span(5, 10),
            })
        })
        .expect_err("the forward coverage statement rejects the gap");
    assert_containment_statement(error, StatementId(4));

    // Overhang only, the audit countermodel: source [0,10), target [0,20).
    // Forward coverage holds; reverse statement 5 rejects escaping support [10,20).
    let dir = TempDir::new("r26-overhang");
    let db = Db::create(dir.path(), ExactPartition).expect("create overhang store");
    let error = db
        .write(|tx| {
            let policy = PolicyId(3);
            tx.insert(&Policy {
                id: policy,
                live: span(0, 10),
            })?;
            tx.insert(&Version {
                policy,
                valid: span(0, 20),
            })
        })
        .expect_err("reverse coverage rejects target overhang");
    assert_containment_statement(error, StatementId(5));

    // The corrected one-way recipe pins the opposite result for that same
    // point set: FiscalYear [0,10) is covered by PayPeriod [0,20), and the
    // absent reverse statement means overhang is legal.
    let dir = TempDir::new("r16-one-way-overhang");
    let db = Db::create(dir.path(), Payroll).expect("create one-way cover store");
    db.write(|tx| {
        let year = FiscalYearId(4);
        tx.insert(&FiscalYear {
            id: year,
            span: span(0, 10),
        })?;
        tx.insert(&PayPeriod {
            year,
            seq: 1,
            span: span(0, 20),
        })
    })
    .expect("one-way source coverage permits target overhang");

    // Arity-general lock: the scalar prefix is (group, lane), followed by
    // the interval. [0,2) and [2,5) exactly partition [0,5) for (7,3).
    let dir = TempDir::new("r26-composite-prefix");
    let db = Db::create(dir.path(), CompositePartition).expect("create composite store");
    db.write(|tx| {
        tx.insert(&Domain {
            group: 7,
            lane: 3,
            live: span(0, 5),
        })?;
        tx.insert(&Segment {
            group: 7,
            lane: 3,
            valid: span(0, 2),
        })?;
        tx.insert(&Segment {
            group: 7,
            lane: 3,
            valid: span(2, 5),
        })
    })
    .expect("two-field scalar prefixes support exact partitions");
}

/// A general `u64` interval literal for the ordering-triple matrices.
fn uspan(start: u64, end: u64) -> bumbledb::Interval<u64> {
    bumbledb::Interval::<u64>::new(start, end).expect("cookbook spans are nonempty")
}

/// A unit slot value: position `p` occupies `[p, p + 1)` — the width-1
/// member of the fixed family (`Interval::fixed` discharges the Q2 bound).
fn unit(p: u64) -> bumbledb::Interval<u64> {
    bumbledb::Interval::<u64>::fixed(p, 1).expect("cookbook positions sit far below the ceiling")
}

/// Recipe 9's theorem-to-runtime matrix (the ordering triple), positive
/// arms: an exact tiling commits, and the O(k) middle insert lands as
/// one delta — the partition never passes through an invalid state.
#[test]
fn r09_ordering_triple_commit_matrix() {
    use r09::{Extent, Playlist, Playlists, Slot};

    // The tiling: span [0,3) exactly partitioned by unit slots 0, 1, 2.
    let dir = TempDir::new("r09-tiling");
    let db = Db::create(dir.path(), Playlists).expect("create playlists store");
    let list = db
        .write(|tx| {
            let list = tx.alloc()?;
            tx.insert(&Playlist {
                id: list,
                name: "road trip",
            })?;
            tx.insert(&Extent {
                playlist: list,
                span: uspan(0, 3),
            })?;
            for (position, track) in [(0, "first"), (1, "second"), (2, "third")] {
                tx.insert(&Slot {
                    playlist: list,
                    slot: unit(position),
                    track,
                })?;
            }
            Ok(list)
        })
        .expect("an exact tiling commits");

    // The middle insert, honestly O(k) and atomic: making room at
    // position 1 shifts slots 1..3 up and grows the extent — one delta.
    db.write(|tx| {
        tx.delete(&Extent {
            playlist: list,
            span: uspan(0, 3),
        })?;
        tx.insert(&Extent {
            playlist: list,
            span: uspan(0, 4),
        })?;
        for (position, track) in [(1, "second"), (2, "third")] {
            tx.delete(&Slot {
                playlist: list,
                slot: unit(position),
                track,
            })?;
        }
        for (position, track) in [(1, "interlude"), (2, "second"), (3, "third")] {
            tx.insert(&Slot {
                playlist: list,
                slot: unit(position),
                track,
            })?;
        }
        Ok(())
    })
    .expect("the shift lands as one judged delta");
}

/// Recipe 9's violating deltas: a gap aborts on the span-side coverage
/// direction of the `==`, an overlap aborts in the key phase — the two
/// negative arms of the ordering triple's matrix.
#[test]
fn r09_gap_and_overlap_deltas_abort() {
    use bumbledb::schema::StatementId;
    use r09::{Extent, Playlist, Playlists, Slot};

    // The gap: span [0,3) with slots 0 and 2 only — point 1 uncovered,
    // the span-side coverage direction of the `==` convicts (its second
    // expanded containment, statement 6).
    let dir = TempDir::new("r09-gap");
    let db = Db::create(dir.path(), Playlists).expect("create gap store");
    let error = db
        .write(|tx| {
            let list = tx.alloc()?;
            tx.insert(&Playlist {
                id: list,
                name: "gapped",
            })?;
            tx.insert(&Extent {
                playlist: list,
                span: uspan(0, 3),
            })?;
            tx.insert(&Slot {
                playlist: list,
                slot: unit(0),
                track: "first",
            })?;
            tx.insert(&Slot {
                playlist: list,
                slot: unit(2),
                track: "third",
            })?;
            Ok(())
        })
        .expect_err("a gap delta aborts");
    assert_containment_statement(error, StatementId(6));

    // The overlap: a second occupant of position 1 — the pointwise key
    // convicts in the key phase, before coverage even runs.
    let dir = TempDir::new("r09-overlap");
    let db = Db::create(dir.path(), Playlists).expect("create overlap store");
    let error = db
        .write(|tx| {
            let list = tx.alloc()?;
            tx.insert(&Playlist {
                id: list,
                name: "doubled",
            })?;
            tx.insert(&Extent {
                playlist: list,
                span: uspan(0, 2),
            })?;
            for (position, track) in [(0, "first"), (1, "second")] {
                tx.insert(&Slot {
                    playlist: list,
                    slot: unit(position),
                    track,
                })?;
            }
            tx.insert(&Slot {
                playlist: list,
                slot: unit(1),
                track: "usurper",
            })?;
            Ok(())
        })
        .expect_err("an overlap delta aborts");
    assert!(matches!(error, bumbledb::Error::CommitRejected { .. }));
}

/// Recipe 29's matrix (the zone ledger): the two-kind composition commits;
/// a cross-sidecar overlap dies on the Zone pointwise key; the coalesced
/// witness and width arms live in the honesty test below.
#[test]
fn r29_zone_ledger_commit_matrix() {
    use r29::{Kind, Ledger, PairSlot, UnitSlot, Zone, ZoneLedger};

    fn pair(p: u64) -> bumbledb::Interval<u64> {
        bumbledb::Interval::<u64>::fixed(p, 2)
            .expect("cookbook positions sit far below the ceiling")
    }

    // The composition: a unit zone and a pair zone, each arm's sidecar
    // carrying exactly its zone's points.
    let dir = TempDir::new("r29-compose");
    let db = Db::create(dir.path(), ZoneLedger).expect("create zone ledger store");
    db.write(|tx| {
        let ledger = tx.alloc()?;
        tx.insert(&Ledger {
            id: ledger,
            name: "day plan",
        })?;
        tx.insert(&Zone {
            ledger,
            kind: Kind::Unit.id(),
            at: uspan(0, 1),
        })?;
        tx.insert(&Zone {
            ledger,
            kind: Kind::Pair.id(),
            at: uspan(1, 3),
        })?;
        tx.insert(&UnitSlot {
            ledger,
            at: unit(0),
            entry: 10,
        })?;
        tx.insert(&PairSlot {
            ledger,
            at: pair(1),
            entry: 20,
        })
    })
    .expect("the two-kind composition commits");

    // Cross-sidecar disjointness: a pair zone overlapping a unit zone is
    // one pointwise key violation — the kinds never meet in a relation,
    // but their zones share the witness.
    let dir = TempDir::new("r29-cross-overlap");
    let db = Db::create(dir.path(), ZoneLedger).expect("create overlap store");
    let error = db
        .write(|tx| {
            let ledger = tx.alloc()?;
            tx.insert(&Ledger {
                id: ledger,
                name: "collided",
            })?;
            tx.insert(&Zone {
                ledger,
                kind: Kind::Unit.id(),
                at: uspan(0, 1),
            })?;
            tx.insert(&Zone {
                ledger,
                kind: Kind::Pair.id(),
                at: uspan(0, 2),
            })?;
            tx.insert(&UnitSlot {
                ledger,
                at: unit(0),
                entry: 10,
            })?;
            tx.insert(&PairSlot {
                ledger,
                at: pair(0),
                entry: 20,
            })
        })
        .expect_err("a cross-sidecar overlap aborts on the zone key");
    assert!(matches!(error, bumbledb::Error::CommitRejected { .. }));
}

/// Recipe 29's honesty arms: the coalesced witness is accepted (the
/// judgments compare point supports, not rows), and a wrong-width arm
/// value is a typed shape error — the width is enforced by type.
#[test]
fn r29_coalescing_insensitivity_and_width_by_type() {
    use r29::{Kind, Ledger, UnitSlot, Zone, ZoneLedger};

    // Coalescing insensitivity, pinned: one Unit-kind zone [4,6) beside
    // two unit slots [4,5), [5,6) — equal point supports, so both `==`
    // directions hold; nothing forces row correspondence.
    let dir = TempDir::new("r29-coalesced");
    let db = Db::create(dir.path(), ZoneLedger).expect("create coalesced store");
    db.write(|tx| {
        let ledger = tx.alloc()?;
        tx.insert(&Ledger {
            id: ledger,
            name: "coalesced",
        })?;
        tx.insert(&Zone {
            ledger,
            kind: Kind::Unit.id(),
            at: uspan(4, 6),
        })?;
        tx.insert(&UnitSlot {
            ledger,
            at: unit(4),
            entry: 40,
        })?;
        tx.insert(&UnitSlot {
            ledger,
            at: unit(5),
            entry: 50,
        })
    })
    .expect("the coalesced witness satisfies both point-support directions");

    // The width is the type: a width-2 value at the unit arm is a typed
    // shape error before any judgment — unrepresentable, not rejected.
    let dir = TempDir::new("r29-wrong-width");
    let db = Db::create(dir.path(), ZoneLedger).expect("create width store");
    let error = db
        .write(|tx| {
            let ledger = tx.alloc()?;
            tx.insert(&Ledger {
                id: ledger,
                name: "wide",
            })?;
            tx.insert(&UnitSlot {
                ledger,
                at: uspan(0, 2),
                entry: 10,
            })?;
            Ok(())
        })
        .expect_err("a wrong-width arm value is a typed shape error");
    assert!(matches!(error, bumbledb::Error::FactShape(_)));
}

/// One compiled doc query, either shape `query!` lowers to: the bare-rule
/// `ir::Query` or the named-head `ir::Program` (recipe 24/25's closures).
enum PinnedQuery {
    Query(Query),
    Program(Program),
}

impl From<Query> for PinnedQuery {
    fn from(query: Query) -> Self {
        Self::Query(query)
    }
}

impl From<Program> for PinnedQuery {
    fn from(program: Program) -> Self {
        Self::Program(program)
    }
}

/// Renders after proving each query real: every query fence of one recipe,
/// prepared against one `Db` of its theory (prepare runs the validation
/// roster) — the notation-test `pin`, recipe-wide.
fn pin_all<S: Theory + Copy>(tag: &str, theory: S, queries: &[PinnedQuery]) -> Vec<String> {
    if queries.is_empty() {
        return Vec::new();
    }
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), theory).expect("create the theory's store");
    let schema: Schema = theory.descriptor().validate().expect("a landed theory");
    queries
        .iter()
        .map(|pinned| match pinned {
            PinnedQuery::Query(query) => {
                db.prepare(query).expect("the cookbook query validates");
                render(&schema, query)
            }
            PinnedQuery::Program(program) => {
                db.prepare(program).expect("the cookbook program validates");
                render_program(&schema, program)
            }
        })
        .collect()
}

/// Every doc query fence compiles via `query!`, prepares against a real
/// store of its recipe's schema, and round-trips through `ir::render` to
/// its pinned golden — the `pin()` law over the whole query roster,
/// recipe 24/25's programs included.
#[test]
fn every_doc_query_compiles_prepares_and_round_trips() {
    for (i, recipe) in ROSTER.iter().enumerate() {
        assert_eq!(
            recipe.queries.len(),
            recipe.renders.len(),
            "recipe {} ({}) pins one render golden per query",
            i + 1,
            recipe.title
        );
        let rendered = (recipe.pin)();
        assert_eq!(
            rendered.len(),
            recipe.renders.len(),
            "recipe {} ({}) renders every pinned query",
            i + 1,
            recipe.title
        );
        for (j, (rendered, golden)) in rendered.iter().zip(recipe.renders.iter()).enumerate() {
            assert_eq!(
                rendered,
                golden,
                "recipe {} ({}) query {} drifted from its render golden",
                i + 1,
                recipe.title,
                j + 1
            );
        }
    }
}

/// Recipe 3's missing negative witness: the child key is the 0..1 proof,
/// so two different address facts for one business abort together.
#[test]
fn r03_a_second_optional_child_is_rejected() {
    use r03::{Business, MailingAddress, Optionality};

    let dir = TempDir::new("r03-second-child");
    let db = Db::create(dir.path(), Optionality).expect("create optionality store");
    let error = db
        .write(|tx| {
            let business = tx.alloc()?;
            tx.insert(&Business {
                id: business,
                name: "one",
            })?;
            tx.insert(&MailingAddress {
                business,
                line: "first",
                city: "here",
            })?;
            tx.insert(&MailingAddress {
                business,
                line: "second",
                city: "there",
            })?;
            Ok(())
        })
        .expect_err("the child key permits at most one address");
    assert!(matches!(error, bumbledb::Error::CommitRejected { .. }));
}

/// Recipe 22's missing negative witness: one payment cannot inhabit both
/// key-backed DU arms because the reverse equality requires two distinct
/// discriminator values for the same fresh id.
#[test]
fn r22_a_double_arm_payment_is_rejected() {
    use r22::{Ach, Card, Kind, Payment, PaymentId, Payments};

    let dir = TempDir::new("r22-double-arm");
    let db = Db::create(dir.path(), Payments).expect("create payments store");
    let payment = PaymentId(7);
    let error = db
        .write(|tx| {
            tx.insert(&Payment {
                id: payment,
                kind: Kind::Card.id(),
            })?;
            tx.insert(&Card {
                payment,
                last4: 1234,
            })?;
            tx.insert(&Ach {
                payment,
                routing: 99,
            })?;
            Ok(())
        })
        .expect_err("one id cannot inhabit Card and Ach simultaneously");
    assert!(matches!(error, bumbledb::Error::CommitRejected { .. }));
}

/// Recipe 8: the sub-vocabulary's judgment is the compiled member set —
/// a paging escalation commits; a non-paging one aborts at commit with
/// the containment violation (the PRD-04 worked example, rot-proofed).
#[test]
fn r08_sub_vocabulary_violating_insert_aborts() {
    use r08::{Escalation, Incident, IncidentId, Oncall, Severity};
    let dir = TempDir::new("r08-subvocab");
    let db = Db::create(dir.path(), Oncall).expect("create the Oncall store");
    db.write(|tx| {
        let id: IncidentId = tx.alloc()?;
        tx.insert(&Incident {
            id,
            severity: Severity::Critical.id(),
        })?;
        tx.insert(&Escalation {
            incident: id,
            severity: Severity::Critical.id(),
            at: 1,
        })?;
        Ok(())
    })
    .expect("a paging escalation commits");
    let err = db
        .write(|tx| {
            let id: IncidentId = tx.alloc()?;
            tx.insert(&Incident {
                id,
                severity: Severity::Info.id(),
            })?;
            tx.insert(&Escalation {
                incident: id,
                severity: Severity::Info.id(),
                at: 2,
            })?;
            Ok(())
        })
        .unwrap_err();
    assert!(
        matches!(err, bumbledb::Error::CommitRejected { .. }),
        "a non-paging escalation violates the ψ-selected containment: {err:?}"
    );
}

/// Recipes 24–25's loop, compiled — the doc's pseudocode as a host
/// function: `frontier = {root}; seen = {root}`, each round binds the
/// frontier as the ∈-set param, subtracts `seen`, and stops on an empty
/// delta. Host-driven semi-naive reachability, verbatim.
fn reachable<S>(
    snap: &Snapshot<'_, S>,
    children: &mut PreparedQuery<'_, S>,
    root: u64,
) -> bumbledb::Result<BTreeSet<u64>> {
    let mut seen = BTreeSet::from([root]);
    let mut frontier = vec![root];
    let mut out = Answers::new();
    loop {
        let params: Vec<Value> = frontier.iter().map(|&n| Value::U64(n)).collect();
        snap.execute_args(children, &[ParamArg::Set(&params)], &mut out)?;
        frontier.clear();
        for answer in 0..out.len() {
            let AnswerValue::U64(child) = out.get(answer, 0) else {
                panic!("the frontier query finds one u64 column");
            };
            if seen.insert(child) {
                frontier.push(child);
            }
        }
        if frontier.is_empty() {
            return Ok(seen);
        }
    }
}

/// Recipe 24: the closure idiom — the loop above over a three-level
/// tree, asserting the exact reachable set (the stray root excluded,
/// an interior node reaching exactly its own subtree).
#[test]
fn r24_closure_idiom_reaches_the_exact_set() {
    use r24::{Closure, Node, NodeId, Parent};
    let dir = TempDir::new("r24-closure");
    let db = Db::create(dir.path(), Closure).expect("create the Closure store");
    let ids = db
        .write(|tx| {
            let mut ids: Vec<NodeId> = Vec::new();
            for name in ["root", "a", "b", "c", "d", "e", "stray"] {
                let id: NodeId = tx.alloc()?;
                tx.insert(&Node { id, name })?;
                ids.push(id);
            }
            // Three levels: root → {a, b}, a → {c, d}, b → {e};
            // `stray` is a second root the closure must never reach.
            for (child, parent) in [(1usize, 0usize), (2, 0), (3, 1), (4, 1), (5, 2)] {
                tx.insert(&Parent {
                    child: ids[child],
                    parent: ids[parent],
                })?;
            }
            Ok(ids)
        })
        .expect("seed the tree");

    let children = query!(r24::Closure {
        (c) | Parent(child: c, parent in ?frontier);
    });
    let mut prepared = db.prepare(&children).expect("prepare the frontier query");
    // The engine-native form (recipe 24's second dialect): the same
    // closure as one stratified program — the named head declares the
    // predicate, the ?root param seeds it, the bare rule is the output
    // — executed whole under the fixpoint driver.
    let native = query!(r24::Closure {
        reach(c) | Node(id: c), c == ?root;
        reach(c) | Parent(child: c, parent: m), reach(m);
        (c) | reach(c);
    });
    let mut native_q = db
        .prepare(&native)
        .expect("prepare the engine-native closure");
    let ids_of = |out: &Answers| -> BTreeSet<u64> {
        (0..out.len())
            .map(|answer| {
                let AnswerValue::U64(node) = out.get(answer, 0) else {
                    panic!("the closure finds one u64 column");
                };
                node
            })
            .collect()
    };
    db.read(|snap| {
        let from_root = reachable(snap, &mut prepared, ids[0].0)?;
        let whole_tree: BTreeSet<u64> = ids[..6].iter().map(|id| id.0).collect();
        assert_eq!(
            from_root, whole_tree,
            "the root reaches the whole tree and never the stray"
        );
        let from_a = reachable(snap, &mut prepared, ids[1].0)?;
        let a_subtree: BTreeSet<u64> = BTreeSet::from([ids[1].0, ids[3].0, ids[4].0]);
        assert_eq!(
            from_a, a_subtree,
            "an interior node reaches exactly its own subtree"
        );
        // Both dialects compute one closure — the idiom and the driver
        // agree, root for root.
        let native_root =
            ids_of(&snap.execute_collect(&mut native_q, &[BindValue::U64(ids[0].0)])?);
        assert_eq!(native_root, whole_tree, "the engine-native closure agrees");
        let native_a = ids_of(&snap.execute_collect(&mut native_q, &[BindValue::U64(ids[1].0)])?);
        assert_eq!(native_a, a_subtree, "per-root agreement holds");
        Ok(())
    })
    .expect("the closure loop reaches its fixpoint");
}

/// Recipe 25: the chart of accounts — the closure idiom composed with
/// one `Sum` over the accumulated ∈-set; the hand-computed subtree
/// rollup over a three-level hierarchy with postings.
#[test]
fn r25_subtree_rollup_matches_the_hand_computed_sum() {
    use r25::{Account, AccountId, AccountParent, Accounts, Posting, PostingId};
    let dir = TempDir::new("r25-accounts");
    let db = Db::create(dir.path(), Accounts).expect("create the Accounts store");
    let ids = db
        .write(|tx| {
            let mut ids: Vec<AccountId> = Vec::new();
            for name in ["assets", "cash", "receivables", "checking", "savings"] {
                let id: AccountId = tx.alloc()?;
                tx.insert(&Account { id, name })?;
                ids.push(id);
            }
            // Three levels: assets → {cash, receivables}, cash → {checking, savings}.
            for (child, parent) in [(1usize, 0usize), (2, 0), (3, 1), (4, 1)] {
                tx.insert(&AccountParent {
                    child: ids[child],
                    parent: ids[parent],
                })?;
            }
            // Postings — the two equal 700s to checking are distinct facts
            // (the fresh id keeps both bindings; recipe 19's discipline).
            for (account, minor) in [
                (3usize, 5_000i64),
                (3, 700),
                (3, 700),
                (4, 30),
                (1, 2),
                (2, 9_999),
                (0, 1),
            ] {
                let id: PostingId = tx.alloc()?;
                tx.insert(&Posting {
                    id,
                    account: ids[account],
                    minor,
                })?;
            }
            Ok(ids)
        })
        .expect("seed the hierarchy and its postings");

    let children = query!(r25::Accounts {
        (c) | AccountParent(child: c, parent in ?frontier);
    });
    let rollup = query!(r25::Accounts {
        (total: Sum(minor)) | Posting(id, account in ?subtree, minor);
    });
    let mut frontier_q = db.prepare(&children).expect("prepare the frontier query");
    let mut rollup_q = db.prepare(&rollup).expect("prepare the rollup");
    // The engine-native form (recipe 25's second dialect): the closure
    // stratum converges first, then the output's fold runs once over the
    // finished subtree — aggregation OF a lower stratum, the one shape
    // the strata roster admits.
    let native = query!(r25::Accounts {
        sub(a) | Account(id: a), a == ?root;
        sub(a) | AccountParent(child: a, parent: p), sub(p);
        (total: Sum(minor)) | Posting(id, account: a, minor), sub(a);
    });
    let mut native_q = db
        .prepare(&native)
        .expect("prepare the engine-native rollup");
    let native_sum = |snap: &Snapshot<'_, Accounts>,
                      native_q: &mut PreparedQuery<'_, Accounts>,
                      root: u64|
     -> bumbledb::Result<i64> {
        let out = snap.execute_collect(native_q, &[BindValue::U64(root)])?;
        assert_eq!(out.len(), 1, "one all-aggregate answer");
        let AnswerValue::I64(total) = out.get(0, 0) else {
            panic!("the native rollup sums an i64 column");
        };
        Ok(total)
    };
    let sum_over = |snap: &Snapshot<'_, Accounts>,
                    rollup_q: &mut PreparedQuery<'_, Accounts>,
                    subtree: &BTreeSet<u64>|
     -> bumbledb::Result<i64> {
        let set: Vec<Value> = subtree.iter().map(|&a| Value::U64(a)).collect();
        let mut out = Answers::new();
        snap.execute_args(rollup_q, &[ParamArg::Set(&set)], &mut out)?;
        assert_eq!(out.len(), 1, "one all-aggregate answer");
        let AnswerValue::I64(total) = out.get(0, 0) else {
            panic!("the rollup sums an i64 column");
        };
        Ok(total)
    };
    db.read(|snap| {
        // The cash subtree: {cash, checking, savings} — closure, then one Sum.
        let cash_subtree = reachable(snap, &mut frontier_q, ids[1].0)?;
        let expected: BTreeSet<u64> = BTreeSet::from([ids[1].0, ids[3].0, ids[4].0]);
        assert_eq!(cash_subtree, expected, "the cash subtree, exactly");
        // Hand-computed: checking 5000 + 700 + 700, savings 30, cash 2 = 6432;
        // receivables' 9999 and the assets root's own 1 are outside.
        assert_eq!(sum_over(snap, &mut rollup_q, &cash_subtree)?, 6_432);
        // The whole-tree rollup from the root: 6432 + 9999 + 1 = 16432.
        let all = reachable(snap, &mut frontier_q, ids[0].0)?;
        assert_eq!(sum_over(snap, &mut rollup_q, &all)?, 16_432);
        // Both dialects fold one subtree — the composed idiom and the
        // engine-native program agree, root for root.
        assert_eq!(native_sum(snap, &mut native_q, ids[1].0)?, 6_432);
        assert_eq!(native_sum(snap, &mut native_q, ids[0].0)?, 16_432);
        Ok(())
    })
    .expect("the rollup composes the closure with one Sum");
}

type BusySpanKey = (u64, i64, i64);

fn derived_busy_spans(
    snap: &Snapshot<'_, r27::MaintainedRollup>,
    query: &mut PreparedQuery<'_, r27::MaintainedRollup>,
) -> bumbledb::Result<BTreeSet<BusySpanKey>> {
    let answers = snap.execute_collect(query, &[])?;
    let mut desired = BTreeSet::new();
    for answer in 0..answers.len() {
        let AnswerValue::U64(person) = answers.get(answer, 0) else {
            panic!("the rollup person is u64");
        };
        let AnswerValue::IntervalI64(span) = answers.get(answer, 1) else {
            panic!("Pack returns an i64 interval");
        };
        desired.insert((person, span.start(), span.end()));
    }
    Ok(desired)
}

/// Recipe 27's documented host protocol: derive and diff on one snapshot,
/// commit the diff under that snapshot's witness, and discard the whole attempt
/// on movement. `before_commit` is the deterministic concurrency seam used by
/// the lock below; production callers pass a no-op.
fn maintain_busy_spans(
    db: &Db<r27::MaintainedRollup>,
    query: &mut PreparedQuery<'_, r27::MaintainedRollup>,
    mut before_commit: impl FnMut(usize) -> bumbledb::Result<()>,
) -> bumbledb::Result<usize> {
    let mut retries = 0;
    loop {
        let attempt = db.read(|snap| {
            let desired = derived_busy_spans(snap, query)?;
            let existing: BTreeSet<BusySpanKey> = snap
                .scan_facts::<r27::BusySpan>()?
                .map(|fact| fact.map(|span| (span.person, span.span.start(), span.span.end())))
                .collect::<bumbledb::Result<_>>()?;
            let removes: Vec<_> = existing.difference(&desired).copied().collect();
            let inserts: Vec<_> = desired.difference(&existing).copied().collect();
            before_commit(retries)?;
            db.write_from(snap, |tx| {
                for (person, start, end) in &removes {
                    tx.delete(&r27::BusySpan {
                        person: *person,
                        span: span(*start, *end),
                    })?;
                }
                for (person, start, end) in &inserts {
                    tx.insert(&r27::BusySpan {
                        person: *person,
                        span: span(*start, *end),
                    })?;
                }
                Ok(())
            })
        });
        match attempt {
            Ok(()) => return Ok(retries),
            Err(bumbledb::Error::GenerationMoved { .. }) => retries += 1,
            Err(error) => return Err(error),
        }
    }
}

/// The maintained-derived-facts recipe moves the source after the first
/// derivation. The stale diff is refused by the generation witness; the host
/// re-derives and lands the packed span from the new source state.
#[test]
fn r27_maintenance_rederives_after_generation_movement() {
    use r27::{Arm, BusySpan, Claim, MaintainedRollup};

    let dir = TempDir::new("r27-maintained-rollup");
    let db = Db::create(dir.path(), MaintainedRollup).expect("create maintained rollup store");
    db.write(|tx| {
        for (source, person, arm, claim_span) in [
            (1, 7, Arm::Busy.id(), span(0, 2)),
            (2, 7, Arm::Busy.id(), span(2, 4)),
            (9, 8, Arm::Ooo.id(), span(100, 110)),
        ] {
            tx.insert(&Claim {
                source,
                person,
                arm,
                span: claim_span,
            })?;
        }
        Ok(())
    })
    .expect("seed claims");

    let derive = query!(r27::MaintainedRollup {
        (person, busy: Pack(claim_span)) |
            Claim(source, person, arm == Busy, span: claim_span);
    });
    let mut prepared = db.prepare(&derive).expect("prepare busy-span derivation");
    let retries = maintain_busy_spans(&db, &mut prepared, |attempt| {
        if attempt == 0 {
            db.write(|tx| {
                tx.insert(&Claim {
                    source: 3,
                    person: 7,
                    arm: Arm::Busy.id(),
                    span: span(4, 6),
                })?;
                Ok(())
            })?;
        }
        Ok(())
    })
    .expect("maintenance retries and commits");
    assert_eq!(retries, 1, "the moved first derivation must be discarded");

    db.read(|snap| {
        let spans: Vec<BusySpan> = snap.scan_facts()?.collect::<bumbledb::Result<_>>()?;
        assert_eq!(
            spans,
            vec![BusySpan {
                person: 7,
                span: span(0, 6),
            }],
            "the retry derives the new complete busy support; Ooo is excluded"
        );
        Ok(())
    })
    .expect("read maintained rollup");
}

/// Recipe 28: migration is ETL — the whole loop against two real
/// theories. Seeds a v1 store, proves the fingerprint refusal, exports
/// under one snapshot, transforms (the ray supplies the missing
/// `applies` dimension), loads containment targets first, then proves
/// the three laws: identity, mint catch-up, judgment under v2.
#[test]
fn r28_migration_is_etl() {
    // The transform's one decision: v1 amounts are in force since the
    // migration epoch — a ray.
    const EPOCH: i64 = 0;
    let dir_v1 = TempDir::new("r28-v1");
    let dir_v2 = TempDir::new("r28-v2");

    // Seed the v1 store; remember the fresh high water.
    let v1 = Db::create(dir_v1.path(), r28_old::PayrollV1).expect("create the v1 store");
    let high_water = v1
        .write(|tx| {
            let mut max = 0;
            for (name, amount) in [("ada", 90_000i64), ("bo", 70_000), ("cy", 80_000)] {
                let id: r28_old::EmployeeId = tx.alloc()?;
                tx.insert(&r28_old::Employee { id, name })?;
                tx.insert(&r28_old::Salary {
                    employee: id,
                    amount,
                })?;
                max = max.max(id.0);
            }
            Ok(max)
        })
        .expect("seed the v1 store");

    // Export under ONE snapshot (one generation — a consistent instant);
    // the transform appends the ray.
    let (employees, salaries) = v1
        .read(|snap| {
            let employees: Vec<Vec<Value>> = snap
                .scan(r28_old::Employee::RELATION)?
                .collect::<bumbledb::Result<_>>()?;
            let salaries: Vec<Vec<Value>> = snap
                .scan(r28_old::Salary::RELATION)?
                .map(|fact| {
                    let mut fact = fact?;
                    fact.push(Value::IntervalI64(
                        bumbledb::Interval::<i64>::new(EPOCH, i64::MAX)
                            .expect("the migration ray is nonempty"),
                    ));
                    Ok(fact)
                })
                .collect::<bumbledb::Result<_>>()?;
            Ok((employees, salaries))
        })
        .expect("export the v1 facts");

    // The fingerprint law: with the v1 handle dropped (LMDB is one
    // handle per env), the store refuses to open under the v2 theory.
    drop(v1);
    let Err(err) = Db::open(dir_v1.path(), r28::Payroll) else {
        panic!("a changed theory must not open");
    };
    assert!(
        matches!(err, bumbledb::Error::SchemaMismatch { .. }),
        "{err:?}"
    );

    // Load containment targets first; explicit fresh values keep identity.
    let v2 = Db::create(dir_v2.path(), r28::Payroll).expect("create the v2 store");
    let loaded = v2
        .bulk_load_dyn(r28::Employee::RELATION, employees)
        .expect("load employees");
    assert_eq!(loaded, 3);
    let loaded = v2
        .bulk_load_dyn(r28::Salary::RELATION, salaries)
        .expect("load salaries");
    assert_eq!(loaded, 3);

    // The mint sequence cleared the imported high water: no collision.
    v2.write(|tx| {
        let next: r28::EmployeeId = tx.alloc()?;
        assert!(
            next.0 > high_water,
            "minted {} at or below the imported high water {high_water}",
            next.0
        );
        Ok(())
    })
    .expect("mint after import");

    // The migrated store answers under the new theory: every v1 salary
    // is in force at any post-epoch instant, keyed by its old identity.
    let in_force = query!(r28::Payroll {
        (name, amount) | Employee(id: e, name), Salary(employee: e, amount, applies: w),
                         ?at in w;
    });
    let mut prepared = v2.prepare(&in_force).expect("prepare the v2 query");
    let mut out = Answers::new();
    v2.read(|snap| {
        snap.execute_args(
            &mut prepared,
            &[ParamArg::Scalar(BindValue::I64(1))],
            &mut out,
        )
    })
    .expect("query the migrated store");
    let mut answers = BTreeSet::new();
    for row in 0..out.len() {
        let AnswerValue::String(name) = out.get(row, 0) else {
            panic!("column 0 is the name");
        };
        let AnswerValue::I64(amount) = out.get(row, 1) else {
            panic!("column 1 is the amount");
        };
        answers.insert((name.to_owned(), amount));
    }
    let expected: BTreeSet<(String, i64)> = [("ada", 90_000i64), ("bo", 70_000), ("cy", 80_000)]
        .into_iter()
        .map(|(n, a)| (n.to_owned(), a))
        .collect();
    assert_eq!(answers, expected, "the v1 facts answer under the v2 theory");
}

/// Recipe 30: the keyed read — the declared law `Program(grp) -> Program`
/// made callable. The generated key struct (`ProgramByGrp`, the
/// `{R}By{Fields}` derived-name rule) answers on BOTH scopes, the fresh
/// newtype reads the primary form, and a determinant nobody wrote misses
/// cleanly — the recipe's taught spellings, exercised verbatim.
#[test]
fn r30_keyed_read_reads_through_the_law_on_both_scopes() {
    use r30::{Grp, GrpId, KeyedRead, Program, ProgramByGrp, ProgramId};

    let dir = TempDir::new("r30-keyed-read");
    let db = Db::create(dir.path(), KeyedRead).expect("create the KeyedRead store");
    let (grp, empty_grp, program) = db
        .write(|tx| {
            let grp: GrpId = tx.alloc()?;
            tx.insert(&Grp {
                id: grp,
                label: "algebra",
            })?;
            let empty_grp: GrpId = tx.alloc()?;
            tx.insert(&Grp {
                id: empty_grp,
                label: "geometry",
            })?;
            let program: ProgramId = tx.alloc()?;
            tx.insert(&Program {
                id: program,
                grp,
                title: "linear equations",
            })?;
            Ok((grp, empty_grp, program))
        })
        .expect("seed the keyed-read store");

    db.read(|snap| {
        // The law made callable: the doc's snapshot spelling.
        assert_eq!(
            snap.get(ProgramByGrp { grp })?,
            Some(Program {
                id: program,
                grp,
                title: "linear equations",
            })
        );
        // The fresh newtype is the primary key made callable.
        assert_eq!(
            snap.get(program)?,
            Some(Program {
                id: program,
                grp,
                title: "linear equations",
            })
        );
        // A group with no program misses cleanly — no fold, no assumption.
        assert_eq!(snap.get(ProgramByGrp { grp: empty_grp })?, None);
        Ok(())
    })
    .expect("snapshot keyed reads");

    // The same spellings inside the write transaction answer the final state.
    db.write(|tx| {
        let found = tx
            .get(ProgramByGrp { grp })?
            .expect("the law answers in write scope");
        assert_eq!(found.id, program);
        assert_eq!(
            tx.get(program)?,
            Some(Program {
                id: program,
                grp,
                title: "linear equations",
            })
        );
        Ok(())
    })
    .expect("write-scope keyed reads");
}
