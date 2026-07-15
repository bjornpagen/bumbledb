//! Rot-proofing for `docs/cookbook.md` (the intuition unit): every cookbook
//! schema compiles and **validates** verbatim against the current engine,
//! the roster is enumerated with a count assertion (a doc recipe without a
//! test entry fails), and the teachable notation queries round-trip through
//! `query!` + prepare + `ir::render`, notation.rs-style.
//!
//! Include-or-duplicate: **duplicate** — markdown cannot be `include!`d at
//! item position, so each block is duplicated here and the sync test pins
//! the duplication token-for-token against the doc (comments and whitespace
//! aside — the token stream never carries them): editing either copy
//! without the other fails `doc_blocks_match_the_compiled_copies`.

use std::collections::BTreeSet;

use bumbledb::ir::Value;
use bumbledb::ir::render::render;
use bumbledb::{
    AnswerValue, Answers, BindValue, Db, Fact, ParamArg, PreparedQuery, Query, Schema, Snapshot,
    Theory,
};
use bumbledb_query::query;

const COOKBOOK: &str = include_str!("../../../docs/cookbook.md");

mod common;
use common::TempDir;

/// One module per recipe: the schema compiled, its token source pinned for
/// the doc-sync test, and a validation entry point for the roster test.
macro_rules! recipe {
    ($m:ident, $theory:ident, { $($t:tt)* }) => {
        mod $m {
            bumbledb::schema! { $($t)* }
            pub const SOURCE: &str = stringify!($($t)*);
            pub fn validate() -> Result<bumbledb::Schema, bumbledb::error::SchemaError> {
                use bumbledb::Theory as _;
                $theory.descriptor().validate()
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
});

recipe!(r15, Pricing, {
    pub Pricing;

    relation Policy  { id: u64 as PolicyId, fresh, live: interval<i64> }
    relation Version { policy: u64 as PolicyId, rate_bps: i64, valid: interval<i64> }

    Version(policy) <= Policy(id);
    Version(policy, valid) -> Version;
    Policy(id, live) <= Version(policy, valid);
});

recipe!(r16, Payroll, {
    pub Payroll;

    relation FiscalYear { id: u64 as FiscalYearId, fresh, span: interval<i64> }
    relation PayPeriod  { year: u64 as FiscalYearId, seq: u64, span: interval<i64> }

    PayPeriod(year) <= FiscalYear(id);
    PayPeriod(year, seq)  -> PayPeriod;
    PayPeriod(year, span) -> PayPeriod;
    FiscalYear(id, span) <= PayPeriod(year, span);
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
});

recipe!(r18, FreeTime, {
    pub FreeTime;

    relation Person { id: u64 as PersonId, fresh, name: str }
    relation Claim  { person: u64 as PersonId, span: interval<i64> }

    Claim(person) <= Person(id);
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

/// The roster, exhaustively — one entry per doc recipe, in doc order.
struct Recipe {
    title: &'static str,
    source: &'static str,
    validate: fn() -> Result<Schema, bumbledb::error::SchemaError>,
}

const ROSTER: [Recipe; 29] = [
    Recipe {
        title: "The minimal interval schema",
        source: r01::SOURCE,
        validate: r01::validate,
    },
    Recipe {
        title: "Discriminated unions",
        source: r02::SOURCE,
        validate: r02::validate,
    },
    Recipe {
        title: "0..1 optional attributes",
        source: r03::SOURCE,
        validate: r03::validate,
    },
    Recipe {
        title: "Money",
        source: r04::SOURCE,
        validate: r04::validate,
    },
    Recipe {
        title: "Content addressing",
        source: r05::SOURCE,
        validate: r05::validate,
    },
    Recipe {
        title: "The vocabulary",
        source: r06::SOURCE,
        validate: r06::validate,
    },
    Recipe {
        title: "The classification",
        source: r07::SOURCE,
        validate: r07::validate,
    },
    Recipe {
        title: "The sub-vocabulary",
        source: r08::SOURCE,
        validate: r08::validate,
    },
    Recipe {
        title: "Ordered collections",
        source: r09::SOURCE,
        validate: r09::validate,
    },
    Recipe {
        title: "Trees and ASTs",
        source: r10::SOURCE,
        validate: r10::validate,
    },
    Recipe {
        title: "Typed graphs",
        source: r11::SOURCE,
        validate: r11::validate,
    },
    Recipe {
        title: "Entity-component",
        source: r12::SOURCE,
        validate: r12::validate,
    },
    Recipe {
        title: "State machines",
        source: r13::SOURCE,
        validate: r13::validate,
    },
    Recipe {
        title: "The calendar core",
        source: r14::SOURCE,
        validate: r14::validate,
    },
    Recipe {
        title: "Effective-dated configuration",
        source: r15::SOURCE,
        validate: r15::validate,
    },
    Recipe {
        title: "Disjoint covers",
        source: r16::SOURCE,
        validate: r16::validate,
    },
    Recipe {
        title: "Federal income tax",
        source: r17::SOURCE,
        validate: r17::validate,
    },
    Recipe {
        title: "Free time and coalescing",
        source: r18::SOURCE,
        validate: r18::validate,
    },
    Recipe {
        title: "The ledger",
        source: r19::SOURCE,
        validate: r19::validate,
    },
    Recipe {
        title: "Conditional writes",
        source: r20::SOURCE,
        validate: r20::validate,
    },
    Recipe {
        title: "Derived relations",
        source: r21::SOURCE,
        validate: r21::validate,
    },
    Recipe {
        title: "Union reads",
        source: r22::SOURCE,
        validate: r22::validate,
    },
    Recipe {
        title: "The anti-recipes: five gravestones",
        source: r23::SOURCE,
        validate: r23::validate,
    },
    Recipe {
        title: "The closure idiom",
        source: r24::SOURCE,
        validate: r24::validate,
    },
    Recipe {
        title: "The chart of accounts",
        source: r25::SOURCE,
        validate: r25::validate,
    },
    Recipe {
        title: "Exact partition",
        source: r26::SOURCE,
        validate: r26::validate,
    },
    Recipe {
        title: "Derived facts, maintained",
        source: r27::SOURCE,
        validate: r27::validate,
    },
    Recipe {
        title: "Migration is ETL",
        source: r28::SOURCE,
        validate: r28::validate,
    },
    Recipe {
        title: "The zone ledger",
        source: r29::SOURCE,
        validate: r29::validate,
    },
];

/// Comments and whitespace out; what remains is exactly what the token
/// stream carries, so a stringified duplicate compares against a doc block.
fn normalize(text: &str) -> String {
    text.lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .flat_map(str::chars)
        .filter(|c| !c.is_whitespace())
        .collect()
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

/// The doc's fenced `rust` blocks, in order.
fn doc_blocks() -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in COOKBOOK.lines() {
        match &mut current {
            None if line.trim() == "```rust" => current = Some(String::new()),
            None => {}
            Some(block) if line.trim() == "```" => {
                blocks.push(std::mem::take(block));
                current = None;
            }
            Some(block) => {
                block.push_str(line);
                block.push('\n');
            }
        }
    }
    blocks
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
        assert_eq!(*n, i + 1, "recipe numbering is 1..=29 in order");
        assert_eq!(title, recipe.title, "recipe {} title", i + 1);
    }
}

/// Every doc schema block is token-identical to its compiled duplicate.
#[test]
fn doc_blocks_match_the_compiled_copies() {
    let blocks = doc_blocks();
    let labels = doc_labels();
    assert_eq!(
        blocks.len(),
        ROSTER.len(),
        "one schema block per recipe, in roster order"
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
    for (i, (block, recipe)) in blocks.iter().zip(ROSTER.iter()).enumerate() {
        let expected = format!("bumbledb::schema!{{{}}}", normalize(recipe.source));
        assert_eq!(
            normalize(block),
            expected,
            "recipe {} ({}) drifted between doc and test",
            i + 1,
            recipe.title
        );
    }
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
    assert_eq!(schema.keys().iter().filter(|key| key.pointwise).count(), 2);
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

/// Renders after proving the query real: prepared against a `Db` of the
/// theory (prepare runs the validation roster) — the notation-test `pin`.
fn pin<S: Theory + Copy>(tag: &str, theory: S, query: &Query) -> String {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), theory).expect("create the theory's store");
    db.prepare(query).expect("the cookbook query validates");
    let schema: Schema = theory.descriptor().validate().expect("a landed theory");
    render(&schema, query)
}

/// Recipe 1: the measure under `Sum` — total downtime per service.
#[test]
fn r01_duration_sum_round_trips() {
    let downtime = query!(r01::Uptime {
        (service, Sum(Duration(window))) | Outage(service, window);
    });
    assert_eq!(
        pin("r01-downtime", r01::Uptime, &downtime),
        "(v0, Sum(Duration(v1))) | Outage(service: v0, window: v1);"
    );
}

/// Recipe 3: negation is plain anti-join — businesses without an address.
#[test]
fn r03_negation_round_trips() {
    let bare = query!(r03::Optionality {
        (b) | Business(id: b), !MailingAddress(business: b);
    });
    assert_eq!(
        pin("r03-bare", r03::Optionality, &bare),
        "(v0) | Business(id: v0), !MailingAddress(business: v0);"
    );
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

/// Recipe 9: positional access is membership — "what plays at position
/// `?pos`" is one point-in probe against the unit slot.
#[test]
fn r09_positional_access_round_trips() {
    let at_pos = query!(r09::Playlists {
        (track) | Slot(playlist == ?list, slot: s, track), ?pos in s;
    });
    assert_eq!(
        pin("r09-at-pos", r09::Playlists, &at_pos),
        "(v1) | Slot(playlist == ?0, slot: v0, track: v1), ?1 in v0;"
    );
}

/// Recipe 14: the room-conflict probe — one Allen mask against a param.
#[test]
fn r14_booking_probe_round_trips() {
    let conflicts = query!(r14::Calendar {
        (room, s) | Booking(room, span: s), Allen(s, INTERSECTS, ?want);
    });
    assert_eq!(
        pin("r14-conflicts", r14::Calendar, &conflicts),
        "(v0, v1) | Booking(room: v0, span: v1), Allen(v1, INTERSECTS, ?0);"
    );
}

/// Recipe 15: "in force on date t" is one membership probe.
#[test]
fn r15_in_force_round_trips() {
    let in_force = query!(r15::Pricing {
        (rate_bps) | Version(policy == ?p, rate_bps, valid: v), ?t in v;
    });
    assert_eq!(
        pin("r15-in-force", r15::Pricing, &in_force),
        "(v0) | Version(policy == ?0, rate_bps: v0, valid: v1), ?1 in v1;"
    );
}

/// Recipe 17: the marginal bracket — membership probes the disjoint bracket set.
#[test]
fn r17_marginal_bracket_round_trips() {
    let marginal = query!(r17::Tax {
        (rate_bps) | Regime(id: r, year == ?y, status == ?s),
                     Bracket(regime: r, income: b, rate_bps), ?taxable in b;
    });
    assert_eq!(
        pin("r17-marginal", r17::Tax, &marginal),
        "(v2) | Regime(id: v0, year == ?0, status == ?1), \
         Bracket(regime: v0, income: v1, rate_bps: v2), ?2 in v1;"
    );
}

/// Recipe 18: `Pack` is the coalescing fold — busy time per person.
#[test]
fn r18_pack_round_trips() {
    let busy = query!(r18::FreeTime {
        (person, busy: Pack(span)) | Claim(person, span);
    });
    assert_eq!(
        pin("r18-busy", r18::FreeTime, &busy),
        "(v0, Pack(v1)) | Claim(person: v0, span: v1);"
    );
}

/// Recipe 19: balances — bind the fresh id or set semantics collapses
/// equal (account, minor) pairs.
#[test]
fn r19_balances_round_trips() {
    let balances = query!(r19::Ledger {
        (account, total: Sum(minor)) | Posting(id, account, minor);
    });
    assert_eq!(
        pin("r19-balances", r19::Ledger, &balances),
        "(v1, Sum(v2)) | Posting(id: v0, account: v1, minor: v2);"
    );
}

/// Recipe 22: the whole-DU read — one head, one rule per arm; the
/// exclusivity theorem elides cross-rule dedup. The bare handles resolve
/// through the `Kind` host enum in scope, and the renderer prints them
/// back as the same bare handles — the round trip runs on names.
#[test]
fn r22_union_read_round_trips() {
    use r22::Kind;
    let methods = query!(r22::Payments {
        (id, n) | Payment(id, kind == Card), Card(payment: id, last4: n);
        (id, n) | Payment(id, kind == Ach), Ach(payment: id, routing: n);
    });
    assert_eq!(
        pin("r22-methods", r22::Payments, &methods),
        "(v0, v1) | Payment(id: v0, kind == Card), Card(payment: v0, last4: v1);\n\
         (v0, v1) | Payment(id: v0, kind == Ach), Ach(payment: v0, routing: v1);"
    );
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

/// Recipe 6: the vocabulary — the bare handle is a fixed point of the
/// round trip (`Priority` is `UpperCamel` of `priority`, so the
/// renderer's own output reparses through the host enum in scope).
#[test]
fn r06_vocabulary_handle_round_trips() {
    use r06::Priority;
    let urgent = query!(r06::Tickets {
        (t) | Ticket(id: t, priority == Urgent);
    });
    assert_eq!(
        pin("r06-urgent", r06::Tickets, &urgent),
        "(v0) | Ticket(id: v0, priority == Urgent);"
    );
}

/// Recipe 7: the classification read — ψ over the vocabulary's payload,
/// no flag duplicated onto Attempt.
#[test]
fn r07_classification_round_trips() {
    let mastered = query!(r07::Review {
        (a) | Attempt(id: a, kind: k), Kind(id: k, mastered == true);
    });
    assert_eq!(
        pin("r07-mastered", r07::Review, &mastered),
        "(v0) | Attempt(id: v0, kind: v1), Kind(id: v1, mastered == true);"
    );
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
        reach(c) | Parent(child: c, parent: m), reach(0: m);
        (c) | reach(0: c);
    });
    let mut native_q = db
        .prepare_program(&native)
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
        sub(a) | AccountParent(child: a, parent: p), sub(0: p);
        (total: Sum(minor)) | Posting(id, account: a, minor), sub(0: a);
    });
    let mut native_q = db
        .prepare_program(&native)
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
        .bulk_load(r28::Employee::RELATION, employees)
        .expect("load employees");
    assert_eq!(loaded, 3);
    let loaded = v2
        .bulk_load(r28::Salary::RELATION, salaries)
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
