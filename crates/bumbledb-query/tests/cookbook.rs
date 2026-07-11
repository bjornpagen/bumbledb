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

use bumbledb::ir::render::render;
use bumbledb::{Db, Query, Schema, Theory};
use bumbledb_query::query;

use std::path::{Path, PathBuf};

const COOKBOOK: &str = include_str!("../../../docs/cookbook.md");

/// A self-cleaning temp directory (the notation-test shape; deps stay zero).
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bumbledb-cookbook-test-{tag}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create test dir");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

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

    relation Task { id: u64 as TaskId, fresh, kind: enum Kind { Deterministic, CustomOperator } }
    relation DeterministicGrading  { task: u64 as TaskId, tolerance: i64 }
    relation CustomOperatorGrading { task: u64 as TaskId, operator: str }

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

    relation Account { id: u64 as AccountId, fresh, name: str }
    relation Posting {
        id: u64 as PostingId, fresh,
        account: u64 as AccountId,
        currency: enum Currency { Usd, Eur, Gbp },
        minor: i64 as Minor,
    }

    Posting(account) <= Account(id);
});

recipe!(r05, Content, {
    pub Content;

    relation Document {
        id: u64 as DocumentId, fresh,
        name: str,
        payload: bytes<32> as PayloadHash,
    }
    relation Replica { payload: bytes<32> as PayloadHash, region: enum Region { Us, Eu } }

    Document(payload) -> Document;
    Replica(payload) <= Document(payload);
});

recipe!(r06, Playlists, {
    pub Playlists;

    relation Playlist { id: u64 as PlaylistId, fresh, name: str }
    relation Entry { playlist: u64 as PlaylistId, pos: u64, track: str }

    Entry(playlist) <= Playlist(id);
    Entry(playlist, pos) -> Entry;
});

recipe!(r07, Ast, {
    pub Ast;

    relation Node { id: u64 as NodeId, fresh, kind: enum Kind { Lit, Add } }
    relation Lit  { node: u64 as NodeId, value: i64 }
    relation Add  { node: u64 as NodeId, lhs: u64 as NodeId, rhs: u64 as NodeId }
    relation Parent { child: u64 as NodeId, parent: u64 as NodeId }

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

recipe!(r08, Graph, {
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

recipe!(r09, Ecs, {
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

recipe!(r10, Orders, {
    pub Orders;

    relation Order { id: u64 as OrderId, fresh, state: enum State { Cart, Placed, Shipped } }
    relation Placement { order: u64 as OrderId, at: i64 }
    relation Shipment  { order: u64 as OrderId, carrier: str, at: i64 }

    Placement(order) -> Placement;
    Shipment(order)  -> Shipment;
    Placement(order) <= Order(id);
    Shipment(order) == Order(id | state == Shipped);
});

recipe!(r11, Calendar, {
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
    Attendance(event, person) -> Attendance;
    Claim(source) -> Claim;
    Claim(person) <= Person(id);
    Booking(room, span) -> Booking;
    Attendance(id | rsvp == Accepted) == Claim(source | arm == Busy);
    WorkHours(person, hours) -> WorkHours;
    Claim(person, span | arm == Busy) <= WorkHours(person, hours);
    Booking(room)  <= Room(id);
    Booking(event) <= Event(id);
});

recipe!(r12, Pricing, {
    pub Pricing;

    relation Policy  { id: u64 as PolicyId, fresh, live: interval<i64> }
    relation Version { policy: u64 as PolicyId, rate_bps: i64, valid: interval<i64> }

    Version(policy) <= Policy(id);
    Version(policy, valid) -> Version;
    Policy(id, live) <= Version(policy, valid);
});

recipe!(r13, Payroll, {
    pub Payroll;

    relation FiscalYear { id: u64 as FiscalYearId, fresh, span: interval<i64> }
    relation PayPeriod  { year: u64 as FiscalYearId, seq: u64, span: interval<i64> }

    PayPeriod(year) <= FiscalYear(id);
    PayPeriod(year, seq)  -> PayPeriod;
    PayPeriod(year, span) -> PayPeriod;
    FiscalYear(id, span) <= PayPeriod(year, span);
});

recipe!(r14, Tax, {
    pub Tax;

    relation Regime {
        id: u64 as RegimeId, fresh,
        year: i64,
        status: enum Status { Single, MarriedJoint, HeadOfHousehold },
    }
    relation Bracket { regime: u64 as RegimeId, income: interval<i64>, rate_bps: i64 }
    relation Residency { person: u64, span: interval<i64> }
    relation Earned { person: u64, regime: u64 as RegimeId, span: interval<i64>, minor: i64 }

    Regime(year, status) -> Regime;
    Bracket(regime) <= Regime(id);
    Bracket(regime, income) -> Bracket;
    Earned(regime) <= Regime(id);
    Residency(person, span) -> Residency;
    Earned(person, span) <= Residency(person, span);
});

recipe!(r15, FreeTime, {
    pub FreeTime;

    relation Person { id: u64 as PersonId, fresh, name: str }
    relation Claim  { person: u64 as PersonId, span: interval<i64> }

    Claim(person) <= Person(id);
});

recipe!(r16, Ledger, {
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

recipe!(r17, Jobs, {
    pub Jobs;

    relation Job {
        id: u64 as JobId, fresh,
        state: enum State { Queued, Running, Done },
        payload: str,
    }
    relation Lease { job: u64 as JobId, worker: u64, until: i64 }

    Lease(job) -> Lease;
    Lease(job) == Job(id | state == Running);
});

recipe!(r18, Rollup, {
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
    BusySpan(person, span) -> BusySpan;
    BusySpan(person, span) <= Claim(person, span | arm == Busy);
});

recipe!(r19, Payments, {
    pub Payments;

    relation Payment { id: u64 as PaymentId, fresh, kind: enum Kind { Card, Ach } }
    relation Card { payment: u64 as PaymentId, last4: u64 }
    relation Ach  { payment: u64 as PaymentId, routing: u64 }

    Card(payment) -> Card;
    Ach(payment)  -> Ach;
    Payment(id | kind == Card) == Card(payment);
    Payment(id | kind == Ach)  == Ach(payment);
});

recipe!(r20, Gravestones, {
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

/// The roster, exhaustively — one entry per doc recipe, in doc order.
struct Recipe {
    title: &'static str,
    source: &'static str,
    validate: fn() -> Result<Schema, bumbledb::error::SchemaError>,
}

const ROSTER: [Recipe; 20] = [
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
        title: "Ordered collections",
        source: r06::SOURCE,
        validate: r06::validate,
    },
    Recipe {
        title: "Trees and ASTs",
        source: r07::SOURCE,
        validate: r07::validate,
    },
    Recipe {
        title: "Typed graphs",
        source: r08::SOURCE,
        validate: r08::validate,
    },
    Recipe {
        title: "Entity-component",
        source: r09::SOURCE,
        validate: r09::validate,
    },
    Recipe {
        title: "State machines",
        source: r10::SOURCE,
        validate: r10::validate,
    },
    Recipe {
        title: "The calendar core",
        source: r11::SOURCE,
        validate: r11::validate,
    },
    Recipe {
        title: "Effective-dated configuration",
        source: r12::SOURCE,
        validate: r12::validate,
    },
    Recipe {
        title: "Tilings",
        source: r13::SOURCE,
        validate: r13::validate,
    },
    Recipe {
        title: "Federal income tax",
        source: r14::SOURCE,
        validate: r14::validate,
    },
    Recipe {
        title: "Free time and coalescing",
        source: r15::SOURCE,
        validate: r15::validate,
    },
    Recipe {
        title: "The ledger",
        source: r16::SOURCE,
        validate: r16::validate,
    },
    Recipe {
        title: "Conditional writes",
        source: r17::SOURCE,
        validate: r17::validate,
    },
    Recipe {
        title: "Derived relations",
        source: r18::SOURCE,
        validate: r18::validate,
    },
    Recipe {
        title: "Union reads",
        source: r19::SOURCE,
        validate: r19::validate,
    },
    Recipe {
        title: "The anti-recipes: five gravestones",
        source: r20::SOURCE,
        validate: r20::validate,
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
        assert_eq!(*n, i + 1, "recipe numbering is 1..=20 in order");
        assert_eq!(title, recipe.title, "recipe {} title", i + 1);
    }
}

/// Every doc schema block is token-identical to its compiled duplicate.
#[test]
fn doc_blocks_match_the_compiled_copies() {
    let blocks = doc_blocks();
    assert_eq!(
        blocks.len(),
        ROSTER.len(),
        "one schema block per recipe, in roster order"
    );
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

/// Recipe 11: the room-conflict probe — one Allen mask against a param.
#[test]
fn r11_booking_probe_round_trips() {
    let conflicts = query!(r11::Calendar {
        (room, s) | Booking(room, span: s), Allen(s, INTERSECTS, ?want);
    });
    assert_eq!(
        pin("r11-conflicts", r11::Calendar, &conflicts),
        "(v0, v1) | Booking(room: v0, span: v1), Allen(v1, INTERSECTS, ?0);"
    );
}

/// Recipe 12: "in force on date t" is one membership probe.
#[test]
fn r12_in_force_round_trips() {
    let in_force = query!(r12::Pricing {
        (rate_bps) | Version(policy == ?p, rate_bps, valid: v), ?t in v;
    });
    assert_eq!(
        pin("r12-in-force", r12::Pricing, &in_force),
        "(v0) | Version(policy == ?0, rate_bps: v0, valid: v1), ?1 in v1;"
    );
}

/// Recipe 14: the marginal bracket — membership walks the tiling.
#[test]
fn r14_marginal_bracket_round_trips() {
    let marginal = query!(r14::Tax {
        (rate_bps) | Regime(id: r, year == ?y, status == ?s),
                     Bracket(regime: r, income: b, rate_bps), ?taxable in b;
    });
    assert_eq!(
        pin("r14-marginal", r14::Tax, &marginal),
        "(v2) | Regime(id: v0, year == ?0, status == ?1), \
         Bracket(regime: v0, income: v1, rate_bps: v2), ?2 in v1;"
    );
}

/// Recipe 15: `Pack` is the coalescing fold — busy time per person.
#[test]
fn r15_pack_round_trips() {
    let busy = query!(r15::FreeTime {
        (person, busy: Pack(span)) | Claim(person, span);
    });
    assert_eq!(
        pin("r15-busy", r15::FreeTime, &busy),
        "(v0, Pack(v1)) | Claim(person: v0, span: v1);"
    );
}

/// Recipe 16: balances — bind the fresh id or set semantics collapses
/// equal (account, minor) pairs.
#[test]
fn r16_balances_round_trips() {
    let balances = query!(r16::Ledger {
        (account, total: Sum(minor)) | Posting(id, account, minor);
    });
    assert_eq!(
        pin("r16-balances", r16::Ledger, &balances),
        "(v1, Sum(v2)) | Posting(id: v0, account: v1, minor: v2);"
    );
}

/// Recipe 19: the whole-DU read — one head, one rule per arm; the
/// exclusivity theorem elides cross-rule dedup.
#[test]
fn r19_union_read_round_trips() {
    let methods = query!(r19::Payments {
        (id, n) | Payment(id, kind == Card), Card(payment: id, last4: n);
        (id, n) | Payment(id, kind == Ach), Ach(payment: id, routing: n);
    });
    assert_eq!(
        pin("r19-methods", r19::Payments, &methods),
        "(v0, v1) | Payment(id: v0, kind == Card), Card(payment: v0, last4: v1);\n\
         (v0, v1) | Payment(id: v0, kind == Ach), Ach(payment: v0, routing: v1);"
    );
}
