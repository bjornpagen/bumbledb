//! Round-trip goldens: `render(lower(text))` equals the normalized text,
//! byte-exactly — the anti-drift discipline (one grammar, three
//! consumers: `ir::render` emits it, `query!` parses it, the cookbook
//! writes in it). Every golden also **validates**: the lowered query is
//! prepared against a real `Db` of its theory, so the pinned strings are
//! real queries, not render-only shapes.
//!
//! The theories are the landed benchmark theories — the ledger
//! (`bumbledb-bench/src/schema.rs`) and the ALG-16 calendar
//! (`bumbledb-bench/src/calendar.rs`) — transcribed here declaration for
//! declaration (the bench crate is quarantined; its schemas are data and
//! travel as text), plus the PRD's Tax-shaped fixture.

use bumbledb::ir::render::render;
use bumbledb::{Db, Query, Schema, Theory};
use bumbledb_query::query;

use std::path::{Path, PathBuf};

/// A self-cleaning temp directory (the engine testutil's shape; deps
/// stay zero here too).
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bumbledb-query-test-{tag}"));
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

/// The benchmark ledger, transcribed.
mod ledger {
    bumbledb::schema! {
        pub Ledger;

        closed relation Currency as CurrencyId = { Usd, Eur, Gbp };
        closed relation Source as SourceId = { Manual, Import, System };
        closed relation Tag as TagId = { Fee, Rebate, Adjustment };

        relation Holder {
            id: u64 as HolderId, fresh,
            name: str,
        }
        relation Account {
            id: u64 as AccountId, fresh,
            holder: u64 as HolderId,
            currency: u64 as CurrencyId,
        }
        relation Instrument {
            id: u64 as InstrumentId, fresh,
            symbol: str,
        }
        relation JournalEntry {
            id: u64 as JournalEntryId, fresh,
            source: u64 as SourceId,
            created_at: i64,
        }
        relation Posting {
            id: u64 as PostingId, fresh,
            entry: u64 as JournalEntryId,
            account: u64 as AccountId,
            instrument: u64 as InstrumentId,
            amount: i64,
            at: i64,
        }
        relation PostingTag {
            posting: u64 as PostingId,
            tag: u64 as TagId,
        }
        relation Org {
            id: u64 as OrgId, fresh,
            name: str,
        }
        relation OrgParent {
            child: u64 as OrgId,
            parent: u64 as OrgId,
        }
        relation Mandate {
            account: u64 as AccountId,
            org: u64 as OrgId,
            active: interval<i64>,
        }

        Account(holder)      <= Holder(id);
        Account(currency)    <= Currency(id);
        Posting(entry)       <= JournalEntry(id);
        Posting(account)     <= Account(id);
        Posting(instrument)  <= Instrument(id);
        PostingTag(posting)  <= Posting(id);
        PostingTag(tag)      <= Tag(id);
        JournalEntry(source) <= Source(id);
        OrgParent(child)     <= Org(id);
        OrgParent(parent)    <= Org(id);
        Mandate(account)     <= Account(id);
        Mandate(org)         <= Org(id);
        Mandate(account, active) -> Mandate;
    }
}

/// The ALG-16 calendar, transcribed.
mod calendar {
    bumbledb::schema! {
        pub Scheduling;

        closed relation Rsvp as RsvpId = { Accepted, Tentative, Declined };
        closed relation ClaimKind as ClaimKindId = { Busy, Ooo };

        relation Account {
            id: u64 as CalAccountId, fresh,
            name: str,
        }
        relation Person {
            id: u64 as CalPersonId, fresh,
            account: u64 as CalAccountId,
            name: str,
        }
        relation Calendar {
            id: u64 as CalendarId, fresh,
            owner: u64 as CalPersonId,
        }
        relation Event {
            id: u64 as CalEventId, fresh,
            calendar: u64 as CalendarId,
            span: interval<i64>,
            created_at: i64,
            hash: bytes<32>,
        }
        relation Attendance {
            id: u64 as AttendanceId, fresh,
            event: u64 as CalEventId,
            person: u64 as CalPersonId,
            rsvp: u64 as RsvpId,
        }
        relation Claim {
            source: u64,
            person: u64 as CalPersonId,
            arm: u64 as ClaimKindId,
            span: interval<i64>,
        }
        relation Room {
            id: u64 as RoomId, fresh,
            name: str,
        }
        relation Booking {
            room: u64 as RoomId,
            event: u64 as CalEventId,
            span: interval<i64>,
        }
        relation WorkHours {
            person: u64 as CalPersonId,
            hours: interval<i64>,
        }

        Person(account)     <= Account(id);
        Calendar(owner)     <= Person(id);
        Event(calendar)     <= Calendar(id);
        Attendance(event)   <= Event(id);
        Attendance(person)  <= Person(id);
        Attendance(rsvp)    <= Rsvp(id);
        Attendance(event, person) -> Attendance;
        Claim(person)       <= Person(id);
        Claim(arm)          <= ClaimKind(id);
        Claim(source)       -> Claim;
        Claim(person, span) -> Claim;
        Attendance(id | rsvp == Accepted) == Claim(source | arm == Busy);
        Claim(person, span | arm == Busy) <= WorkHours(person, hours);
        Booking(room)       <= Room(id);
        Booking(event)      <= Event(id);
        Booking(room, span) -> Booking;
        WorkHours(person)   <= Person(id);
        WorkHours(person, hours) -> WorkHours;
    }
}

/// The PRD's Tax-shaped fixture (the notation unit's second example
/// wants a year/regime/bracket walk; `status`'s closed relation is named
/// `UpperCamel` of its field so the bare-handle spelling stays available).
mod tax {
    bumbledb::schema! {
        pub Tax;

        closed relation Status as StatusId = { Draft, Active, Repealed };

        relation Year {
            id: u64 as YearId, fresh,
            span: interval<i64>,
        }
        relation Regime {
            id: u64 as RegimeId, fresh,
            year: u64 as YearId,
            status: u64 as StatusId,
        }
        relation Bracket {
            regime: u64 as RegimeId,
            income: interval<i64>,
            rate_bps: i64,
        }

        Regime(year)    <= Year(id);
        Regime(status)  <= Status(id);
        Bracket(regime) <= Regime(id);
    }
}

// The host enums ride along: a query-text handle (`ClaimKind::Busy`,
// bare `Usd`) resolves through the host enum in scope at the query site.
use calendar::{ClaimKind, Scheduling};
use ledger::{Currency, Ledger};
use tax::{Status, Tax};

/// Renders after proving the query real: prepared against a `Db` of the
/// theory (prepare runs the validation roster).
fn pin<S: Theory + Copy>(tag: &str, theory: S, query: &Query) -> String {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), theory).expect("create the theory's store");
    db.prepare(query).expect("the golden query validates");
    let schema: Schema = theory.descriptor().validate().expect("a landed theory");
    render(&schema, query)
}

/// The PRD's first example adapted to the landed calendar: Busy ∪ Ooo is
/// the Claim relation's two arms — two clauses, one head, a window param.
/// The qualified handle spelling (`ClaimKind::Busy`) resolves through the
/// host enum's welded row id; the renderer prints the row id back as its
/// BARE handle, resolved through the theory's sealed extension (a
/// rendered query is renderable without the host enums). `ClaimKind` is
/// not named `UpperCamel` of `arm`, so the rendered bare spelling
/// reparses only through the qualified form — the bare fixed point is
/// the naming convention's dividend, pinned on the Tax golden below.
#[test]
fn calendar_union_golden() {
    let unavailable = query!(Scheduling {
        (person, span) | Claim(person, span, arm == ClaimKind::Busy),
                         Allen(span, INTERSECTS, ?window);
        (person, span) | Claim(person, span, arm == ClaimKind::Ooo),
                         Allen(span, INTERSECTS, ?window);
    });
    assert_eq!(
        pin("calendar-union", Scheduling, &unavailable),
        "(v0, v1) | Claim(person: v0, span: v1, arm == Busy), Allen(v1, INTERSECTS, ?0);\n\
         (v0, v1) | Claim(person: v0, span: v1, arm == Ooo), Allen(v1, INTERSECTS, ?0);"
    );
}

/// The lowering pinned as data, not just as text: the calendar union
/// expands to exactly the IR value a host would write by hand through
/// the id constants.
#[test]
fn calendar_union_lowers_to_the_exact_ir() {
    use bumbledb::{
        AllenMask, Atom, CmpOp, Comparison, FindTerm, MaskTerm, ParamId, PredicateTree, Rule, Term,
        Value, VarId,
    };
    let lowered = query!(Scheduling {
        (person, span) | Claim(person, span, arm == ClaimKind::Busy),
                         Allen(span, INTERSECTS, ?window);
    });
    let arm_rule = |arm: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: Scheduling::CLAIM,
            bindings: vec![
                (Scheduling::CLAIM_PERSON, Term::Var(VarId(0))),
                (Scheduling::CLAIM_SPAN, Term::Var(VarId(1))),
                (Scheduling::CLAIM_ARM, Term::Literal(Value::U64(arm))),
            ],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
            },
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    };
    assert_eq!(
        lowered,
        bumbledb::Query::single(arm_rule(ClaimKind::Busy.id().0))
    );
}

/// The PRD's second example on the Tax fixture: a three-atom walk, two
/// point-membership items, a param selection — the normalized text is
/// pinned, then reparsed below.
const TAX_RATE_NORMALIZED: &str = "(v4) | Year(id: v0, span: v1), \
     Regime(id: v2, year: v0, status == ?1), \
     Bracket(regime: v2, income: v3, rate_bps: v4), \
     ?0 in v1, ?2 in v3;";

#[test]
fn tax_rate_golden() {
    let rate = query!(Tax {
        (rate_bps) | Year(id: y, span), ?today in span,
                     Regime(id: r, year: y, status == ?s),
                     Bracket(regime: r, income, rate_bps), ?taxable in income;
    });
    assert_eq!(pin("tax-rate", Tax, &rate), TAX_RATE_NORMALIZED);
}

/// The normalized text is a fixed point: the renderer's own output —
/// `v{id}` variables, positional `?N` params, atoms-then-predicates —
/// reparses to a query that renders back to itself, byte-exactly.
#[test]
fn tax_rate_normalized_text_is_a_fixed_point() {
    let reparsed = query!(Tax {
        (v4) | Year(id: v0, span: v1),
               Regime(id: v2, year: v0, status == ?1),
               Bracket(regime: v2, income: v3, rate_bps: v4),
               ?0 in v1, ?2 in v3;
    });
    assert_eq!(
        pin("tax-rate-fixed-point", Tax, &reparsed),
        TAX_RATE_NORMALIZED
    );
}

/// The PRD's third example on the landed calendar: the self-join with
/// explicit variables on both ends (the punning law's join spelling), an
/// order comparison, a literal mask.
const CONFLICTS_NORMALIZED: &str = "(v0, v3) | Event(id: v0, calendar: v1, span: v2), \
     Event(id: v3, calendar: v1, span: v4), \
     v0 < v3, Allen(v2, INTERSECTS, v4);";

#[test]
fn conflicts_golden() {
    let conflicts = query!(Scheduling {
        (c1, c2) | Event(id: c1, calendar: k, span: d1),
                   Event(id: c2, calendar: k, span: d2),
                   c1 < c2, Allen(d1, INTERSECTS, d2);
    });
    assert_eq!(
        pin("conflicts", Scheduling, &conflicts),
        CONFLICTS_NORMALIZED
    );
}

#[test]
fn conflicts_normalized_text_is_a_fixed_point() {
    let reparsed = query!(Scheduling {
        (v0, v3) | Event(id: v0, calendar: v1, span: v2),
                   Event(id: v3, calendar: v1, span: v4),
                   v0 < v3, Allen(v2, INTERSECTS, v4);
    });
    assert_eq!(
        pin("conflicts-fixed-point", Scheduling, &reparsed),
        CONFLICTS_NORMALIZED
    );
}

/// Negation plus the bare-handle selection spelling (`currency`'s closed
/// relation is named `UpperCamel` of the field, so `Usd` resolves through
/// the `Currency` host enum): holders of USD accounts with no postings.
/// The renderer prints the row id back as the same bare handle, and that
/// handle spelling reparses — the round-trip law holds through the
/// vocabulary's names, end to end.
#[test]
fn negation_and_bare_handle_round_trip() {
    let dormant = query!(Ledger {
        (holder) | Account(id: a, holder, currency == Usd), !Posting(account: a);
    });
    let normalized = "(v1) | Account(id: v0, holder: v1, currency == Usd), !Posting(account: v0);";
    assert_eq!(pin("dormant", Ledger, &dormant), normalized);
    let reparsed = query!(Ledger {
        (v1) | Account(id: v0, holder: v1, currency == Usd), !Posting(account: v0);
    });
    assert_eq!(pin("dormant-fixed-point", Ledger, &reparsed), normalized);
}

/// The comprehensive closed-reference golden (the surface pass's own):
/// on a theory whose closed relation is named `UpperCamel` of its
/// referencing field (`status` → `Status`), the rendered BARE handle is
/// a fixed point — `render(lower(text)) == normalize(text)` byte-exactly
/// through the handle spelling — and the qualified spelling
/// (`Status::Active`) lowers to the identical IR, so both reparse paths
/// land on one normalized text.
#[test]
fn closed_reference_handles_are_a_fixed_point() {
    let normalized = "(v0) | Regime(id: v0, status == Active);";
    let active = query!(Tax {
        (r) | Regime(id: r, status == Active);
    });
    assert_eq!(pin("active-regimes", Tax, &active), normalized);
    // The renderer's own output reparses through the bare-handle rule
    // (UpperCamel(field) = the host enum in scope) to the fixed point.
    let reparsed = query!(Tax {
        (v0) | Regime(id: v0, status == Active);
    });
    assert_eq!(
        pin("active-regimes-fixed-point", Tax, &reparsed),
        normalized
    );
    // The qualified spelling is the same query, value for value.
    let qualified = query!(Tax {
        (v0) | Regime(id: v0, status == Status::Active);
    });
    assert_eq!(qualified, reparsed);
}

/// Every named-aggregate head form in one clause; the names stay at the
/// call site (result columns are positional — the render drops them).
#[test]
fn aggregate_heads_golden() {
    let balances = query!(Ledger {
        (account, total: Sum(amount), n: Count,
         entries: CountDistinct(entry), lo: Min(amount), hi: Max(amount))
            | Posting(entry, account, amount);
    });
    assert_eq!(
        pin("balances", Ledger, &balances),
        "(v1, Sum(v2), Count, CountDistinct(v0), Min(v2), Max(v2)) | \
         Posting(entry: v0, account: v1, amount: v2);"
    );
}

/// `Pack` (the coalescing fold) and the measure forms: a `Duration`
/// fold in the head and a measure comparison in the body.
#[test]
fn pack_and_duration_round_trip() {
    let packed = query!(Scheduling {
        (person, busy: Pack(span)) | Claim(person, span);
    });
    assert_eq!(
        pin("packed", Scheduling, &packed),
        "(v0, Pack(v1)) | Claim(person: v0, span: v1);"
    );

    let long_meetings = query!(Scheduling {
        (person, Sum(Duration(span))) | Claim(person, span), Duration(span) >= 3600;
    });
    let normalized = "(v0, Sum(Duration(v1))) | Claim(person: v0, span: v1), Duration(v1) >= 3600;";
    assert_eq!(pin("long-meetings", Scheduling, &long_meetings), normalized);
    let reparsed = query!(Scheduling {
        (v0, Sum(Duration(v1))) | Claim(person: v0, span: v1), Duration(v1) >= 3600;
    });
    assert_eq!(
        pin("long-meetings-fixed-point", Scheduling, &reparsed),
        normalized
    );
}

/// A non-composite mask unions basics with `|` (set union over the 13),
/// and a set-param binding is the membership spelling `field in ?N`.
#[test]
fn mask_union_and_set_param_round_trip() {
    let adjacent = query!(Scheduling {
        (id) | Event(id, span: s), Allen(s, BEFORE|MEETS, ?window);
    });
    let normalized = "(v0) | Event(id: v0, span: v1), Allen(v1, BEFORE|MEETS, ?0);";
    assert_eq!(pin("adjacent", Scheduling, &adjacent), normalized);
    let reparsed = query!(Scheduling {
        (v0) | Event(id: v0, span: v1), Allen(v1, BEFORE|MEETS, ?0);
    });
    assert_eq!(
        pin("adjacent-fixed-point", Scheduling, &reparsed),
        normalized
    );

    let in_region = query!(Ledger {
        (id) | Account(id, currency in ?currencies);
    });
    assert_eq!(
        pin("in-region", Ledger, &in_region),
        "(v0) | Account(id: v0, currency in ?0);"
    );
}
