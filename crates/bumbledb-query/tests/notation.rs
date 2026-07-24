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

/// [`pin`]'s program twin: prepared through the program boundary, then
/// rendered by `render_program` (interior predicates `p{id}`, output
/// bare).
fn pin_program<S: Theory + Copy>(tag: &str, theory: S, program: &bumbledb::Program) -> String {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), theory).expect("create the theory's store");
    db.prepare(program).expect("the golden program validates");
    let schema: Schema = theory.descriptor().validate().expect("a landed theory");
    bumbledb::ir::render::render_program(&schema, program)
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
        AllenMask, Atom, CmpOp, Comparison, ConditionTree, FindTerm, MaskTerm, ParamId, Rule, Term,
        Value, VarId,
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

/// Arg restriction is writable and renderer-total in all three important
/// shapes: singleton carry, coherent multi-carry, and the key carrying itself.
/// Each source spelling renders to the normalized form, whose positional form
/// reparses and renders byte-identically.
#[test]
fn arg_heads_round_trip_singleton_composite_and_self_carry() {
    let singleton = query!(Ledger {
        (ArgMax(id, at)) | Posting(id, at);
    });
    let singleton_normalized = "(ArgMax(v0, v1)) | Posting(id: v0, at: v1);";
    assert_eq!(
        pin("arg-singleton", Ledger, &singleton),
        singleton_normalized
    );
    let singleton_reparsed = query!(Ledger {
        (ArgMax(v0, v1)) | Posting(id: v0, at: v1);
    });
    assert_eq!(
        pin("arg-singleton-fixed-point", Ledger, &singleton_reparsed),
        singleton_normalized
    );

    let composite = query!(Ledger {
        (account, ArgMax(id, at), ArgMax(amount, at))
            | Posting(id, account, amount, at);
    });
    let composite_normalized = "(v1, ArgMax(v0, v3), ArgMax(v2, v3)) | \
        Posting(id: v0, account: v1, amount: v2, at: v3);";
    assert_eq!(
        pin("arg-composite", Ledger, &composite),
        composite_normalized
    );
    let composite_reparsed = query!(Ledger {
        (v1, ArgMax(v0, v3), ArgMax(v2, v3))
            | Posting(id: v0, account: v1, amount: v2, at: v3);
    });
    assert_eq!(
        pin("arg-composite-fixed-point", Ledger, &composite_reparsed),
        composite_normalized
    );

    let self_carry = query!(Ledger {
        (ArgMin(at, at)) | Posting(at);
    });
    let self_carry_normalized = "(ArgMin(v0, v0)) | Posting(at: v0);";
    assert_eq!(
        pin("arg-self-carry", Ledger, &self_carry),
        self_carry_normalized
    );
    let self_carry_reparsed = query!(Ledger {
        (ArgMin(v0, v0)) | Posting(at: v0);
    });
    assert_eq!(
        pin("arg-self-carry-fixed-point", Ledger, &self_carry_reparsed),
        self_carry_normalized
    );
}

/// Grammar exposure does not weaken the semantic boundary: Arg restriction
/// remains single-rule because its key is rule-scoped outside the head.
#[test]
fn arg_across_rules_is_the_typed_notation_level_refusal() {
    let query = query!(Ledger {
        (ArgMax(id, at)) | Posting(id, account == 3, at);
        (ArgMax(id, at)) | Posting(id, account == 7, at);
    });
    let dir = TempDir::new("arg-across-rules");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let Err(error) = db.prepare(&query) else {
        panic!("Arg restriction across rules must be refused");
    };
    assert!(matches!(
        error,
        bumbledb::Error::Validation(bumbledb::error::ValidationError::ArgAcrossRules { rules: 2 })
    ));
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

/// The named-head program (the notation's recursion form): named heads
/// declare predicates, a body atom names one (bare idents bind head
/// POSITIONS, ordered dense — left to right from 0, positional never
/// nominal), and bare rules ARE the output. The org-hierarchy closure
/// over `OrgParent`, rendered: interior rules carry the synthesized
/// `p0` name, output rules render bare, dense predicate atoms render
/// as bare idents — and that normalized text reparses to the same
/// bytes.
const ORG_REACH_NORMALIZED: &str = "p0(v0, v1) | OrgParent(child: v0, parent: v1);\n\
     p0(v0, v2) | OrgParent(child: v0, parent: v1), p0(v1, v2);\n\
     (v0, v1) | p0(v0, v1);";

#[test]
fn named_head_program_golden() {
    let reachable = query!(Ledger {
        reach(c, a) | OrgParent(child: c, parent: a);
        reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);
        (c, a) | reach(c, a);
    });
    assert_eq!(
        pin_program("org-reach", Ledger, &reachable),
        ORG_REACH_NORMALIZED
    );
}

#[test]
fn named_head_normalized_text_is_a_fixed_point() {
    let reparsed = query!(Ledger {
        p0(v0, v1) | OrgParent(child: v0, parent: v1);
        p0(v0, v2) | OrgParent(child: v0, parent: v1), p0(v1, v2);
        (v0, v1) | p0(v0, v1);
    });
    assert_eq!(
        pin_program("org-reach-fixed-point", Ledger, &reparsed),
        ORG_REACH_NORMALIZED
    );
}

/// The program lowering pinned as data: predicate names are macro-local
/// and never enter the IR — the emitted value carries bare `PredId`s,
/// `Idb` sources, and head-position `FieldId`s, exactly what a host
/// writes by hand. The ordered dense spelling IS that lowering:
/// `reach(m, a)` is bindings `[(0, m), (1, a)]`, positions left to
/// right from 0.
#[test]
fn named_head_program_lowers_to_the_exact_ir() {
    use bumbledb::ir::HeadTerm;
    use bumbledb::{Atom, AtomSource, FieldId, FindTerm, PredId, Rule, Term, VarId};
    let lowered = query!(Ledger {
        reach(c, a) | OrgParent(child: c, parent: a);
        reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);
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
        source: AtomSource::Idb(PredId(0)),
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
    let expected = bumbledb::Program {
        predicates: vec![
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![
                    rule([0, 1], vec![parent_atom(0, 1)]),
                    rule([0, 2], vec![parent_atom(0, 1), reach_atom(1, 2)]),
                ],
            },
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![rule([0, 1], vec![reach_atom(0, 1)])],
            },
        ],
        output: PredId(1),
    };
    assert_eq!(lowered, expected);
}

/// Named params get dense ids by first occurrence in SOURCE order,
/// query-global — never by group-emission order (finding 015): rules of
/// one predicate may interleave around another group, the host binds
/// positionally against the text it wrote, and a permutation between
/// same-typed params is silent wrong bindings no roster can catch.
/// `?root` (source-first, in the output rule) must take id 0 even though
/// the `reach` group emits first.
#[test]
fn interleaved_groups_mint_param_ids_in_source_order() {
    use bumbledb::ir::HeadTerm;
    use bumbledb::{
        Atom, AtomSource, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, ParamId, PredId,
        Rule, Term, VarId,
    };
    let lowered = query!(Ledger {
        reach(c, a) | OrgParent(child: c, parent: a);
        (c, a) | reach(c, a), c == ?root;
        reach(c, a) | OrgParent(child: c, parent: m), reach(m, a), a != ?skip;
    });
    let parent_atom = |child: u16, parent: u16| Atom {
        source: AtomSource::Edb(Ledger::ORG_PARENT),
        bindings: vec![
            (Ledger::ORG_PARENT_CHILD, Term::Var(VarId(child))),
            (Ledger::ORG_PARENT_PARENT, Term::Var(VarId(parent))),
        ],
    };
    let reach_atom = |a: u16, b: u16| Atom {
        source: AtomSource::Idb(PredId(0)),
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
    let expected = bumbledb::Program {
        predicates: vec![
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![
                    rule([0, 1], vec![parent_atom(0, 1)], vec![]),
                    rule(
                        [0, 2],
                        vec![parent_atom(0, 1), reach_atom(1, 2)],
                        // `?skip` is second in source order: ParamId(1).
                        vec![cond(CmpOp::Ne, 2, 1)],
                    ),
                ],
            },
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![rule(
                    [0, 1],
                    vec![reach_atom(0, 1)],
                    // `?root` is first in source order: ParamId(0).
                    vec![cond(CmpOp::Eq, 0, 0)],
                )],
            },
        ],
        output: PredId(1),
    };
    assert_eq!(lowered, expected);
}

/// The indexed spellings survive for what the ordered form cannot say —
/// sparse positions (`2: x`), position selections (`1 == …`), and
/// position set membership (`0 in ?p`) — and render as `i:`/selection
/// forms while dense predicate atoms render bare. Both normalized texts
/// reparse to their own bytes: the fixed-point law holds on both sides
/// of the split.
#[test]
fn sparse_and_selection_positions_round_trip() {
    let sparse = query!(Ledger {
        posted(id, account, amount) | Posting(id, account, amount);
        (x) | posted(2: x, 0 in ?wanted);
    });
    let sparse_normalized = "p0(v0, v1, v2) | Posting(id: v0, account: v1, amount: v2);\n\
         (v0) | p0(2: v0, 0 in ?0);";
    assert_eq!(
        pin_program("sparse-positions", Ledger, &sparse),
        sparse_normalized
    );
    let sparse_reparsed = query!(Ledger {
        p0(v0, v1, v2) | Posting(id: v0, account: v1, amount: v2);
        (v0) | p0(2: v0, 0 in ?0);
    });
    assert_eq!(
        pin_program("sparse-positions-fixed-point", Ledger, &sparse_reparsed),
        sparse_normalized
    );

    // A position selection carries no field name, so its handle is
    // written qualified; the renderer prints the row id by value (the
    // handle's home is the field-carrying selection form).
    let selected = query!(Ledger {
        acct(id, currency) | Account(id, currency);
        (a) | acct(0: a, 1 == Currency::Usd);
    });
    let selected_normalized = "p0(v0, v1) | Account(id: v0, currency: v1);\n\
         (v0) | p0(0: v0, 1 == 0);";
    assert_eq!(
        pin_program("selected-positions", Ledger, &selected),
        selected_normalized
    );
    let selected_reparsed = query!(Ledger {
        p0(v0, v1) | Account(id: v0, currency: v1);
        (v0) | p0(0: v0, 1 == 0);
    });
    assert_eq!(
        pin_program("selected-positions-fixed-point", Ledger, &selected_reparsed),
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
    use bumbledb::{
        Atom, CmpOp, Comparison, ConditionTree, FindTerm, Rule, Term, Value, VarId,
    };
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
