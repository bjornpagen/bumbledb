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
//! travel as text), plus a compact Tax-shaped fixture.

use bumbledb::ir::render::render;
use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{Db, Query, Schema, Theory};
use bumbledb_query::query;

mod common;
use common::TempDir;

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
            source: u64 as AttendanceId,
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

/// The Tax-shaped fixture (the notation unit's second example
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

/// The calendar union example: Busy ∪ Ooo is
/// the Claim relation's two arms — two rules, one head, a window param.
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
        AllenMask, Atom, CmpOp, Comparison, ConditionTree, FindTerm, ParamId, Rule, Term, Value,
        VarId,
    };
    let lowered = query!(Scheduling {
        (person, span) | Claim(person, span, arm == ClaimKind::Busy),
                         Allen(span, INTERSECTS, ?window);
    });
    let arm_rule = |arm: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(Scheduling::CLAIM),
            bindings: vec![
                (Scheduling::CLAIM_PERSON, Term::Var(VarId(0))),
                (Scheduling::CLAIM_SPAN, Term::Var(VarId(1))),
                (Scheduling::CLAIM_ARM, Term::Literal(Value::U64(arm))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::INTERSECTS,
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

/// The Tax fixture's three-atom walk, with two
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
/// `v{id}` variables, positional `?N` params, atoms-then-conditions —
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

/// The calendar self-join with
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

/// Every named-aggregate head form in one rule; the names stay at the
/// call site (result columns are positional — the render drops them).
#[test]
fn aggregate_heads_golden() {
    let balances = query!(Ledger {
        (account, total: Sum(amount), n: Count, lo: Min(amount), hi: Max(amount))
            | Posting(entry, account, amount);
    });
    assert_eq!(
        pin("balances", Ledger, &balances),
        "(v1, Sum(v2), Count, Min(v2), Max(v2)) | \
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

    let durations = query!(Scheduling {
        (Duration(span)) | Claim(span);
    });
    let normalized = "(Duration(v0)) | Claim(span: v0);";
    assert_eq!(pin("durations", Scheduling, &durations), normalized);
    let reparsed = query!(Scheduling {
        (Duration(v0)) | Claim(span: v0);
    });
    assert_eq!(
        pin("durations-fixed-point", Scheduling, &reparsed),
        normalized
    );
}

/// Every scalar comparison operator and a scalar param survive the same
/// lowering/rendering fixed point.
#[test]
fn scalar_comparisons_round_trip() {
    let comparisons = query!(Ledger {
        (id) | Posting(id, entry, account, instrument, amount, at),
               id == ?wanted, entry != 0, account < 10, instrument <= 10,
               amount > -10, at >= -10;
    });
    let normalized = "(v0) | Posting(id: v0, entry: v1, account: v2, instrument: v3, amount: v4, at: v5), \
        v0 == ?0, v1 != 0, v2 < 10, v3 <= 10, v4 > -10, v5 >= -10;";
    assert_eq!(pin("scalar-comparisons", Ledger, &comparisons), normalized);
    let reparsed = query!(Ledger {
        (v0) | Posting(id: v0, entry: v1, account: v2, instrument: v3, amount: v4, at: v5),
               v0 == ?0, v1 != 0, v2 < 10, v3 <= 10, v4 > -10, v5 >= -10;
    });
    assert_eq!(
        pin("scalar-comparisons-fixed-point", Ledger, &reparsed),
        normalized
    );
}

/// The `recursive` form (the notation's one linear rec): consecutive
/// `recursive` lines union into one Rec (a line whose body names the
/// derived table is a rec arm, else base); bare rules are main. The
/// org-hierarchy closure over `OrgParent`, rendered: rec rules carry
/// the nameless `rec(...)` prefix, main rules render bare, dense
/// interior atoms render as `interior {id}` — and that normalized text
/// reparses to the same bytes.
const ORG_REACH_NORMALIZED: &str = "rec(v0, v1) | OrgParent(child: v0, parent: v1);\n\
     rec(v0, v2) | OrgParent(child: v0, parent: v1), interior 0(v1, v2);\n\
     (v0, v1) | interior 0(v0, v1);";

#[test]
fn recursive_reach_golden() {
    let reachable = query!(Ledger {
        recursive reach(c, a) | OrgParent(child: c, parent: a);
        recursive reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);
        (c, a) | reach(c, a);
    });
    assert_eq!(pin("org-reach", Ledger, &reachable), ORG_REACH_NORMALIZED);
}

#[test]
fn recursive_normalized_text_is_a_fixed_point() {
    let reparsed = query!(Ledger {
        rec(v0, v1) | OrgParent(child: v0, parent: v1);
        rec(v0, v2) | OrgParent(child: v0, parent: v1), interior 0(v1, v2);
        (v0, v1) | interior 0(v0, v1);
    });
    assert_eq!(
        pin("org-reach-fixed-point", Ledger, &reparsed),
        ORG_REACH_NORMALIZED
    );
}

/// The recursive lowering pinned as data: names are macro-local and
/// never enter the IR — the emitted value carries bare `InteriorId`s,
/// `Interior` sources, and head-position `FieldId`s. The rec id is
/// `InteriorId(interiors.len())`. The ordered dense spelling IS that
/// lowering: `reach(m, a)` is bindings `[(0, m), (1, a)]`.
#[test]
fn recursive_lowers_to_the_exact_ir() {
    use bumbledb::ir::HeadTerm;
    use bumbledb::{Atom, AtomSource, FieldId, FindTerm, InteriorId, Rec, Rule, Term, VarId};
    let lowered = query!(Ledger {
        recursive reach(c, a) | OrgParent(child: c, parent: a);
        recursive reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);
        (c, a) | reach(c, a);
    });
    let parent_atom = |child: u16, parent: u16| Atom {
        source: AtomSource::Edb(Ledger::ORG_PARENT),
        bindings: vec![
            (Ledger::ORG_PARENT_CHILD, Term::Var(VarId(child))),
            (Ledger::ORG_PARENT_PARENT, Term::Var(VarId(parent))),
        ],
    };
    let reach_atom = |a: u16, b: u16| Atom {
        source: AtomSource::Interior(InteriorId(0)),
        bindings: vec![
            (FieldId(0), Term::Var(VarId(a))),
            (FieldId(1), Term::Var(VarId(b))),
        ],
    };
    let rule = |finds: [u16; 2], atoms: Vec<Atom>| Rule {
        finds: finds.map(|v| FindTerm::Var(VarId(v))).to_vec(),
        atoms,
        negated: vec![],
        conditions: vec![],
    };
    let expected = bumbledb::Query::Reach {
        interiors: vec![],
        rec: Rec {
            head: vec![HeadTerm::Var, HeadTerm::Var],
            base: vec![rule([0, 1], vec![parent_atom(0, 1)])],
            rec: vec![rule([0, 2], vec![parent_atom(0, 1), reach_atom(1, 2)])],
        },
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![rule([0, 1], vec![reach_atom(0, 1)])],
    };
    assert_eq!(lowered, expected);
}

/// Named params get dense ids by first occurrence in IR walk order —
/// interiors, then rec base, then rec arms, then main. Consecutive
/// `recursive` lines union; non-consecutive reuse is a compile error, so
/// groups cannot interleave around main.
#[test]
fn rec_then_main_mint_param_ids_in_walk_order() {
    use bumbledb::ir::HeadTerm;
    use bumbledb::{
        Atom, AtomSource, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, InteriorId, ParamId,
        Rec, Rule, Term, VarId,
    };
    let lowered = query!(Ledger {
        recursive reach(c, a) | OrgParent(child: c, parent: a);
        recursive reach(c, a) | OrgParent(child: c, parent: m), reach(m, a), a != ?skip;
        (c, a) | reach(c, a), c == ?root;
    });
    let parent_atom = |child: u16, parent: u16| Atom {
        source: AtomSource::Edb(Ledger::ORG_PARENT),
        bindings: vec![
            (Ledger::ORG_PARENT_CHILD, Term::Var(VarId(child))),
            (Ledger::ORG_PARENT_PARENT, Term::Var(VarId(parent))),
        ],
    };
    let reach_atom = |a: u16, b: u16| Atom {
        source: AtomSource::Interior(InteriorId(0)),
        bindings: vec![
            (FieldId(0), Term::Var(VarId(a))),
            (FieldId(1), Term::Var(VarId(b))),
        ],
    };
    let cond = |op: CmpOp, var: u16, param: u16| {
        ConditionTree::Leaf(Comparison {
            op,
            lhs: Term::Var(VarId(var)),
            rhs: Term::Param(ParamId(param)),
        })
    };
    let rule = |finds: [u16; 2], atoms: Vec<Atom>, conditions: Vec<ConditionTree>| Rule {
        finds: finds.map(|v| FindTerm::Var(VarId(v))).to_vec(),
        atoms,
        negated: vec![],
        conditions,
    };
    let expected = bumbledb::Query::Reach {
        interiors: vec![],
        rec: Rec {
            head: vec![HeadTerm::Var, HeadTerm::Var],
            base: vec![rule([0, 1], vec![parent_atom(0, 1)], vec![])],
            rec: vec![rule(
                [0, 2],
                vec![parent_atom(0, 1), reach_atom(1, 2)],
                // rec arms walk before main: `?skip` is ParamId(0).
                vec![cond(CmpOp::Ne, 2, 0)],
            )],
        },
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![rule(
            [0, 1],
            vec![reach_atom(0, 1)],
            // `?root` in main is second: ParamId(1).
            vec![cond(CmpOp::Eq, 0, 1)],
        )],
    };
    assert_eq!(lowered, expected);
}

/// The indexed spellings survive for what the ordered form cannot say —
/// sparse positions (`2: x`), position selections (`1 == …`), and
/// position set membership (`0 in ?p`) — and render as `i:`/selection
/// forms while dense interior atoms render as `interior {id}`. Both
/// normalized texts reparse to their own bytes: the fixed-point law
/// holds on both sides of the split. `interior` is the non-recursive
/// derived table.
#[test]
fn sparse_and_selection_positions_round_trip() {
    let sparse = query!(Ledger {
        interior posted(id, account, amount) | Posting(id, account, amount);
        (x) | posted(2: x, 0 in ?wanted);
    });
    let sparse_normalized = "interior 0(v0, v1, v2) | Posting(id: v0, account: v1, amount: v2);\n\
         (v0) | interior 0(2: v0, 0 in ?0);";
    assert_eq!(pin("sparse-positions", Ledger, &sparse), sparse_normalized);
    let sparse_reparsed = query!(Ledger {
        interior 0(v0, v1, v2) | Posting(id: v0, account: v1, amount: v2);
        (v0) | interior 0(2: v0, 0 in ?0);
    });
    assert_eq!(
        pin("sparse-positions-fixed-point", Ledger, &sparse_reparsed),
        sparse_normalized
    );

    // A position selection carries no field name, so its handle is
    // written qualified; the renderer prints the row id by value (the
    // handle's home is the field-carrying selection form).
    let selected = query!(Ledger {
        interior acct(id, currency) | Account(id, currency);
        (a) | acct(0: a, 1 == Currency::Usd);
    });
    let selected_normalized = "interior 0(v0, v1) | Account(id: v0, currency: v1);\n\
         (v0) | interior 0(0: v0, 1 == 0);";
    assert_eq!(
        pin("selected-positions", Ledger, &selected),
        selected_normalized
    );
    let selected_reparsed = query!(Ledger {
        interior 0(v0, v1) | Account(id: v0, currency: v1);
        (v0) | interior 0(0: v0, 1 == 0);
    });
    assert_eq!(
        pin("selected-positions-fixed-point", Ledger, &selected_reparsed),
        selected_normalized
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

/// Integer literals are rustc's (ruled 2026-07-23, R8): radix prefixes
/// and `_` separators are notation at every integer position — suffixed
/// or bare — and the renderer normalizes to canonical decimal, so the
/// round-trip law is canonical-form, not verbatim.
#[test]
fn radix_literals_normalize_to_canonical_decimal() {
    let banded = query!(Ledger {
        (id) | Posting(id, entry == 0x10, amount),
               amount > -0b101, amount != -1_000, id < 0o17u64;
    });
    let normalized = "(v0) | Posting(id: v0, entry == 16, amount: v1), \
         v1 > -5, v1 != -1000, v0 < 15;";
    assert_eq!(pin("radix-literals", Ledger, &banded), normalized);
    let reparsed = query!(Ledger {
        (v0) | Posting(id: v0, entry == 16, amount: v1), v1 > -5, v1 != -1000, v0 < 15;
    });
    assert_eq!(
        pin("radix-literals-fixed-point", Ledger, &reparsed),
        normalized
    );
}

/// The condition-tree grammar (ruled 2026-07-23, R9): `and(..)`/`or(..)`
/// are notation, one item per tree, comparison leaves exactly as the IR's
/// `ConditionTree` — and the renderer's functional forms reparse, closing
/// the round trip over the full input grammar.
const AMOUNT_BAND_NORMALIZED: &str = "(v0) | Posting(id: v0, amount: v1), \
     or(v1 == -100, and(v1 > -50, v1 < -10));";

#[test]
fn condition_tree_golden() {
    let banded = query!(Ledger {
        (id) | Posting(id, amount), or(amount == -100, and(amount > -50, amount < -10));
    });
    assert_eq!(pin("amount-band", Ledger, &banded), AMOUNT_BAND_NORMALIZED);
}

#[test]
fn condition_tree_normalized_text_is_a_fixed_point() {
    let reparsed = query!(Ledger {
        (v0) | Posting(id: v0, amount: v1), or(v1 == -100, and(v1 > -50, v1 < -10));
    });
    assert_eq!(
        pin("amount-band-fixed-point", Ledger, &reparsed),
        AMOUNT_BAND_NORMALIZED
    );
}

/// The tree's leaf vocabulary is every comparison — `Allen`, point
/// membership, and the measure nest under `or`/`and` exactly as the TS
/// condition grammar admits them (one condition language, two identical
/// surfaces).
const MANDATE_TOUCH_NORMALIZED: &str = "(v0) | Mandate(org: v0, active: v1), \
     or(Allen(v1, INTERSECTS, ?0), and(?1 in v1, Duration(v1) >= 3600));";

#[test]
fn condition_tree_comparison_leaves_round_trip() {
    let touching = query!(Ledger {
        (org) | Mandate(org, active),
                or(Allen(active, INTERSECTS, ?window), and(?p in active, Duration(active) >= 3600));
    });
    assert_eq!(
        pin("mandate-touch", Ledger, &touching),
        MANDATE_TOUCH_NORMALIZED
    );
    let reparsed = query!(Ledger {
        (v0) | Mandate(org: v0, active: v1),
               or(Allen(v1, INTERSECTS, ?0), and(?1 in v1, Duration(v1) >= 3600));
    });
    assert_eq!(
        pin("mandate-touch-fixed-point", Ledger, &reparsed),
        MANDATE_TOUCH_NORMALIZED
    );
}

/// The tree lowering pinned as data: nested `and`/`or` construct the
/// IR's `ConditionTree` verbatim — validation distributes to DNF
/// engine-side, so the macro never hand-lowers a disjunction.
#[test]
fn condition_tree_lowers_to_the_exact_ir() {
    use bumbledb::{Atom, CmpOp, Comparison, ConditionTree, FindTerm, Rule, Term, Value, VarId};
    let banded = query!(Ledger {
        (id) | Posting(id, amount), or(amount == -100, and(amount > -50, amount < -10));
    });
    let leaf = |op: CmpOp, value: i64| {
        ConditionTree::Leaf(Comparison {
            op,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::I64(value)),
        })
    };
    let rule = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(Ledger::POSTING),
            bindings: vec![
                (Ledger::POSTING_ID, Term::Var(VarId(0))),
                (Ledger::POSTING_AMOUNT, Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Or(vec![
            leaf(CmpOp::Eq, -100),
            ConditionTree::And(vec![leaf(CmpOp::Gt, -50), leaf(CmpOp::Lt, -10)]),
        ])],
    };
    assert_eq!(banded, bumbledb::Query::single(rule));
}

/// `start..end` interval literals must emit `Value::Interval*(Interval::new(...))`,
/// not a two-argument enum constructor.
mod interval_lit {
    bumbledb::schema! {
        pub IntervalLit;
        relation R { x: u64, w: interval<u64> }
    }
}

#[test]
fn interval_literals_compile_prepare_and_render() {
    use bumbledb::{
        AllenMask, Atom, CmpOp, Comparison, ConditionTree, FindTerm, Interval, Rule, Term, Value,
        VarId,
    };
    use interval_lit::IntervalLit;

    // Two literals: compiles (the E0061 hole) and lowers to Interval::new.
    // Prepare refuses a constant comparison; the grammar is still usable
    // at a variable position below.
    let point_in = query!(IntervalLit {
        (x) | R(x, w), 5 in 0..10;
    });
    let interval = Interval::<u64>::new(0, 10).expect("nonempty");
    assert_eq!(
        point_in,
        bumbledb::Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Edb(IntervalLit::R),
                bindings: vec![
                    (IntervalLit::R_X, Term::Var(VarId(0))),
                    (IntervalLit::R_W, Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![ConditionTree::Leaf(Comparison {
                op: CmpOp::PointIn,
                lhs: Term::Literal(Value::IntervalU64(interval)),
                rhs: Term::Literal(Value::U64(5)),
            })],
        })
    );

    let allen = query!(IntervalLit {
        (x) | R(x, w), Allen(w, INTERSECTS, 0..10);
    });
    assert_eq!(
        pin("interval-literal-allen", IntervalLit, &allen),
        "(v0) | R(x: v0, w: v1), Allen(v1, INTERSECTS, 0..10);"
    );
    assert!(matches!(
        &allen.rules()[0].conditions[0],
        ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen { mask },
            rhs: Term::Literal(Value::IntervalU64(_)),
            ..
        }) if *mask == AllenMask::INTERSECTS
    ));

    let eq = query!(IntervalLit {
        (x) | R(x, w == 1..2);
    });
    assert_eq!(
        pin("interval-literal-eq", IntervalLit, &eq),
        "(v0) | R(x: v0, w == 1..2);"
    );
}

/// Primer-shaped cycle detector: linear `reach(from, to)` with extra EDB
/// on the step arm; main is `reach(x, x)` — a join of the finished rec,
/// not a second SCC. Empty answers = DAG. In-tree lock; the Primer repo
/// recut is out of this cut.
mod primer {
    bumbledb::schema! {
        pub Primer;

        closed relation State as StateId = { Upheld, Broken };

        relation Grp {
            id: u64 as GrpId, fresh,
        }
        relation Produces {
            grp: u64 as GrpId,
            capability: u64,
        }
        relation Requires {
            consumer: u64 as GrpId,
            capability: u64,
            state: u64 as StateId,
        }

        Produces(grp) <= Grp(id);
        Requires(consumer) <= Grp(id);
        Requires(state) <= State(id);
    }
}

use primer::{Primer, State};

#[test]
fn primer_shaped_reach_diagonal_golden() {
    let cycle = query!(Primer {
        recursive reach(from, to) | Produces(grp: from, capability: cap),
            Requires(consumer: to, capability: cap, state == State::Upheld), from != to;
        recursive reach(from, to) | Produces(grp: from, capability: cap),
            Requires(consumer: mid, capability: cap, state == State::Upheld),
            Requires(consumer: to, state == State::Upheld), from != mid, reach(mid, to);
        (node) | Grp(id: node), reach(node, node);
    });
    assert_eq!(
        pin("primer-reach-diagonal", Primer, &cycle),
        "rec(v0, v2) | Produces(grp: v0, capability: v1), \
Requires(consumer: v2, capability: v1, state == Upheld), v0 != v2;\n\
rec(v0, v3) | Produces(grp: v0, capability: v1), \
Requires(consumer: v2, capability: v1, state == Upheld), \
Requires(consumer: v3, state == Upheld), interior 0(v2, v3), v0 != v2;\n\
(v0) | Grp(id: v0), interior 0(v0, v0);"
    );
}
