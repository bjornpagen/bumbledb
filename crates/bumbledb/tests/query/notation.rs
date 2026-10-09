//! Render goldens: each query lowers, validates against a store of its
//! theory, and renders to the pinned normalized text; the normalized text
//! reparses to the same query.
use bumbledb::query;

use crate::common::pin;

/// The benchmark ledger.
pub mod ledger {
    bumbledb::schema! {
        pub Ledger;

        closed relation Currency as CurrencyId = { Usd, Eur, Gbp };
        closed relation Source as SourceId = { Manual, Import, System };
        closed relation Tag as TagId = { Fee, Rebate, Adjustment };

        relation Holder {
            id: u64 as HolderId,
            name: str,
        }
        relation Account {
            id: u64 as AccountId,
            holder: u64 as HolderId,
            currency: u64 as CurrencyId,
        }
        relation Instrument {
            id: u64 as InstrumentId,
            symbol: str,
        }
        relation JournalEntry {
            id: u64 as JournalEntryId,
            source: u64 as SourceId,
            created_at: i64,
        }
        relation Posting {
            id: u64 as PostingId,
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
            id: u64 as OrgId,
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

        Holder(id)       -> Holder;
        Account(id)      -> Account;
        Instrument(id)   -> Instrument;
        JournalEntry(id) -> JournalEntry;
        Posting(id)      -> Posting;
        Org(id)          -> Org;

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

/// The benchmark calendar.
mod calendar {
    bumbledb::schema! {
        pub Scheduling;

        closed relation Rsvp as RsvpId = { Accepted, Tentative, Declined };
        closed relation ClaimKind as ClaimKindId = { Busy, Ooo };

        relation Account {
            id: u64 as CalAccountId,
            name: str,
        }
        relation Person {
            id: u64 as CalPersonId,
            account: u64 as CalAccountId,
            name: str,
        }
        relation Calendar {
            id: u64 as CalendarId,
            owner: u64 as CalPersonId,
        }
        relation Event {
            id: u64 as CalEventId,
            calendar: u64 as CalendarId,
            span: interval<i64>,
            created_at: i64,
            hash: bytes<32>,
        }
        relation Attendance {
            id: u64 as AttendanceId,
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
            id: u64 as RoomId,
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

        Account(id)    -> Account;
        Person(id)     -> Person;
        Calendar(id)   -> Calendar;
        Event(id)      -> Event;
        Attendance(id) -> Attendance;
        Room(id)       -> Room;

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

/// A year/regime/bracket walk.
mod tax {
    bumbledb::schema! {
        pub Tax;

        closed relation Status as StatusId = { Draft, Active, Repealed };

        relation Year {
            id: u64 as YearId,
            span: interval<i64>,
        }
        relation Regime {
            id: u64 as RegimeId,
            year: u64 as YearId,
            status: u64 as StatusId,
        }
        relation Bracket {
            regime: u64 as RegimeId,
            income: interval<i64>,
            rate_bps: i64,
        }

        Year(id)   -> Year;
        Regime(id) -> Regime;

        Regime(year)    <= Year(id);
        Regime(status)  <= Status(id);
        Bracket(regime) <= Regime(id);
    }
}

use calendar::{ClaimKind, Scheduling};
use ledger::Ledger;
use tax::Tax;

/// Busy ∪ Ooo: two rules, one head, a window param. A qualified handle
/// (`ClaimKind::Busy`) renders bare, and the bare spelling reparses through
/// the field's closed relation whatever that relation is named.
#[test]
fn calendar_union_golden() {
    let normalized = "(v0, v1) | Claim(person: v0, span: v1, arm == Busy), Allen(v1, INTERSECTS, ?0);\n\
         (v0, v1) | Claim(person: v0, span: v1, arm == Ooo), Allen(v1, INTERSECTS, ?0);";
    let unavailable = query!(Scheduling {
        (person, span) | Claim(person, span, arm == ClaimKind::Busy),
                         Allen(span, INTERSECTS, ?window);
        (person, span) | Claim(person, span, arm == ClaimKind::Ooo),
                         Allen(span, INTERSECTS, ?window);
    });
    assert_eq!(pin("calendar-union", Scheduling, &unavailable), normalized);
    let reparsed = query!(Scheduling {
        (v0, v1) | Claim(person: v0, span: v1, arm == Busy), Allen(v1, INTERSECTS, ?0);
        (v0, v1) | Claim(person: v0, span: v1, arm == Ooo), Allen(v1, INTERSECTS, ?0);
    });
    assert_eq!(*reparsed, *unavailable);
}

/// The calendar union lowers to exactly the IR a host writes by hand
/// through the theory's relation constants.
#[test]
fn calendar_union_lowers_to_the_exact_ir() {
    use bumbledb::{
        AllenMask, Atom, CmpOp, Comparison, ConditionTree, FindTerm, ParamId, Rule, Term, Value,
        VarId,
    };
    let lowered = query!(Scheduling {
        (person, span) | Claim(person, span, arm == ClaimKind::Busy),
                         Allen(span, INTERSECTS, ?window);
        (person, span) | Claim(person, span, arm == Ooo),
                         Allen(span, INTERSECTS, ?window);
    });
    let arm_rule = |arm: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(Scheduling::Claim.relation()),
            bindings: vec![
                (Scheduling::Claim.person, Term::Var(VarId(0))),
                (Scheduling::Claim.span, Term::Var(VarId(1))),
                (Scheduling::Claim.arm, Term::Literal(Value::U64(arm))),
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
    let rules = vec![
        arm_rule(ClaimKind::Busy.id().0),
        arm_rule(ClaimKind::Ooo.id().0),
    ];
    let head = rules[0].head();
    assert_eq!(*lowered, bumbledb::Query::cq(vec![], head, rules));
}

/// A three-atom walk with two point memberships and a param selection.
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

/// The renderer's output (`v{id}` variables, positional `?N` params, atoms
/// then conditions) reparses to a query that renders back to itself.
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

/// A self-join with explicit variables on both ends, an order comparison
/// and a literal mask.
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

/// Negation plus a bare handle selection: holders of USD accounts with no
/// postings.
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

/// The bare and qualified handle spellings lower to the same IR, and the
/// rendered bare handle is a fixed point.
#[test]
fn closed_reference_handles_are_a_fixed_point() {
    let normalized = "(v0) | Regime(id: v0, status == Active);";
    let active = query!(Tax {
        (r) | Regime(id: r, status == Active);
    });
    assert_eq!(pin("active-regimes", Tax, &active), normalized);
    let reparsed = query!(Tax {
        (v0) | Regime(id: v0, status == Active);
    });
    assert_eq!(
        pin("active-regimes-fixed-point", Tax, &reparsed),
        normalized
    );
    let qualified = query!(Tax {
        (v0) | Regime(id: v0, status == Status::Active);
    });
    assert_eq!(*qualified, *reparsed);
}

/// A string literal selection lowers to the `Value::String` it spells and
/// renders back to the same literal.
#[test]
fn string_literal_selection_golden() {
    let named = query!(Ledger {
        (h) | Holder(id: h, name == "Ada \"the\" Countess\n");
    });
    let rule = &named.rules()[0];
    assert_eq!(
        rule.atoms[0].bindings[1].1,
        bumbledb::Term::Literal(bumbledb::Value::String("Ada \"the\" Countess\n".into()))
    );
    assert_eq!(
        pin("string-literal", Ledger, &named),
        "(v0) | Holder(id: v0, name == \"Ada \\\"the\\\" Countess\\n\");"
    );
}

/// Every named aggregate head form in one rule; the render drops the
/// column names.
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

/// `Pack`, the coalescing fold.
#[test]
fn pack_round_trip() {
    let packed = query!(Scheduling {
        (person, busy: Pack(span)) | Claim(person, span);
    });
    assert_eq!(
        pin("packed", Scheduling, &packed),
        "(v0, Pack(v1)) | Claim(person: v0, span: v1);"
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

/// The org closure over `OrgParent`: rec rules render with the nameless
/// `rec(…)` prefix and derived-table atoms as `interior {id}`.
const ORG_REACH_NORMALIZED: &str = "rec(v0, v1) | OrgParent(child: v0, parent: v1);\n\
     rec(v0, v2) | OrgParent(child: v0, parent: v1), interior 0(v1, v2);\n\
     (v0, v1) | interior 0(v0, v1);";

#[test]
fn recursive_reach_golden() {
    let reachable = query!(Ledger {
        rec reach(c, a) | OrgParent(child: c, parent: a);
        rec reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);
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

/// Derived-table names never reach the IR: the rec is
/// `InteriorId(interiors.len())` and `reach(m, a)` binds positions 0 and 1.
#[test]
fn recursive_lowers_to_the_exact_ir() {
    use bumbledb::ir::HeadTerm;
    use bumbledb::{
        Atom, AtomSource, FieldId, FindTerm, InteriorId, NonEmpty, Rec, RecRule, RecStep, Rule,
        Term, VarId,
    };
    let lowered = query!(Ledger {
        rec reach(c, a) | OrgParent(child: c, parent: a);
        rec reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);
        (c, a) | reach(c, a);
    });
    let parent_atom = |child: u16, parent: u16| Atom {
        source: AtomSource::Edb(Ledger::OrgParent.relation()),
        bindings: vec![
            (Ledger::OrgParent.child, Term::Var(VarId(child))),
            (Ledger::OrgParent.parent, Term::Var(VarId(parent))),
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
    let expected = bumbledb::Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![parent_atom(0, 1)],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
                atoms: vec![parent_atom(0, 1)],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![rule([0, 1], vec![reach_atom(0, 1)])],
    };
    assert_eq!(*lowered.query(), expected);
}

/// Named params get dense ids by first occurrence in walk order: interiors,
/// rec base arms, recursive arms, then main.
#[test]
fn rec_then_main_mint_param_ids_in_walk_order() {
    use bumbledb::ir::HeadTerm;
    use bumbledb::{
        Atom, AtomSource, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, InteriorId,
        NonEmpty, ParamId, Rec, RecRule, RecStep, Rule, Term, VarId,
    };
    let lowered = query!(Ledger {
        rec reach(c, a) | OrgParent(child: c, parent: a);
        rec reach(c, a) | OrgParent(child: c, parent: m), reach(m, a), a != ?skip;
        (c, a) | reach(c, a), c == ?root;
    });
    let parent_atom = |child: u16, parent: u16| Atom {
        source: AtomSource::Edb(Ledger::OrgParent.relation()),
        bindings: vec![
            (Ledger::OrgParent.child, Term::Var(VarId(child))),
            (Ledger::OrgParent.parent, Term::Var(VarId(parent))),
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
    let expected = bumbledb::Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![parent_atom(0, 1)],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
                atoms: vec![parent_atom(0, 1)],
                // rec arms walk before main: `?skip` is ParamId(0).
                conditions: vec![cond(CmpOp::Ne, 2, 0)],
            }),
        }),
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![rule(
            [0, 1],
            vec![reach_atom(0, 1)],
            // `?root` in main is second: ParamId(1).
            vec![cond(CmpOp::Eq, 0, 1)],
        )],
    };
    assert_eq!(*lowered.query(), expected);
}

/// Sparse positions (`2: x`), position selections (`1 == …`) and position
/// set membership (`0 in ?p`) round-trip.
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

    // A position has no field, so its handle is qualified and renders as
    // the row id.
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

/// A mask unions basics with `|`; `field in ?N` binds a set param.
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

/// Radix prefixes, `_` separators and suffixes are accepted at every
/// integer position; the render is canonical decimal.
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

/// `and(…)`/`or(…)` trees of comparisons round-trip.
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

/// `Allen` and point membership nest under `or`/`and`.
const MANDATE_TOUCH_NORMALIZED: &str = "(v0) | Mandate(org: v0, active: v1), \
     or(Allen(v1, INTERSECTS, ?0), ?1 in v1);";

#[test]
fn condition_tree_comparison_leaves_round_trip() {
    let touching = query!(Ledger {
        (org) | Mandate(org, active),
                or(Allen(active, INTERSECTS, ?window), ?p in active);
    });
    assert_eq!(
        pin("mandate-touch", Ledger, &touching),
        MANDATE_TOUCH_NORMALIZED
    );
    let reparsed = query!(Ledger {
        (v0) | Mandate(org: v0, active: v1),
               or(Allen(v1, INTERSECTS, ?0), ?1 in v1);
    });
    assert_eq!(
        pin("mandate-touch-fixed-point", Ledger, &reparsed),
        MANDATE_TOUCH_NORMALIZED
    );
}

/// Nested `and`/`or` lower to the IR's `ConditionTree` as written.
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
            source: bumbledb::AtomSource::Edb(Ledger::Posting.relation()),
            bindings: vec![
                (Ledger::Posting.id, Term::Var(VarId(0))),
                (Ledger::Posting.amount, Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Or(vec![
            leaf(CmpOp::Eq, -100),
            ConditionTree::And(vec![leaf(CmpOp::Gt, -50), leaf(CmpOp::Lt, -10)]),
        ])],
    };
    assert_eq!(*banded, bumbledb::Query::single(rule));
}

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

    let point_in = query!(IntervalLit {
        (x) | R(x, w), 5 in 0..10;
    });
    let interval = Interval::<u64>::new(0, 10).expect("nonempty");
    assert_eq!(
        *point_in,
        bumbledb::Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Edb(IntervalLit::R.relation()),
                bindings: vec![
                    (IntervalLit::R.x, Term::Var(VarId(0))),
                    (IntervalLit::R.w, Term::Var(VarId(1))),
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

/// Cycle detection over a linear reachability relation.
mod cycle_graph {
    bumbledb::schema! {
        pub CycleGraph;

        closed relation State as StateId = { Upheld, Broken };

        relation Grp {
            id: u64 as GrpId,
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

        Grp(id) -> Grp;
        Produces(grp) <= Grp(id);
        Requires(consumer) <= Grp(id);
        Requires(state) <= State(id);
    }
}

use cycle_graph::CycleGraph;

#[test]
fn reach_diagonal_golden() {
    let cycle = query!(CycleGraph {
        rec reach(from, to) | Produces(grp: from, capability: cap),
            Requires(consumer: to, capability: cap, state == State::Upheld), from != to;
        rec reach(from, to) | Produces(grp: from, capability: cap),
            Requires(consumer: mid, capability: cap, state == State::Upheld),
            Requires(consumer: to, state == State::Upheld), from != mid, reach(mid, to);
        (node) | Grp(id: node), reach(node, node);
    });
    assert_eq!(
        pin("reach-diagonal", CycleGraph, &cycle),
        "rec(v0, v2) | Produces(grp: v0, capability: v1), \
Requires(consumer: v2, capability: v1, state == Upheld), v0 != v2;\n\
rec(v0, v3) | Produces(grp: v0, capability: v1), \
Requires(consumer: v2, capability: v1, state == Upheld), \
Requires(consumer: v3, state == Upheld), interior 0(v2, v3), v0 != v2;\n\
(v0) | Grp(id: v0), interior 0(v0, v0);"
    );
}
