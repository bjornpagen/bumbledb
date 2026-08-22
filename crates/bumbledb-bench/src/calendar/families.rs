//! (`Pack` — [`crate::translate::Inexpressible::PackAggregate`]): it is
//! **reported translator-unpaired, never dropped** — its `SQLite` side is the
//! engine and the naive model before any timing.

use bumbledb::{
    AllenMask, Atom, CmpOp, Comparison, ConditionTree, FindTerm, ParamId, Query, Rule, Term, Value,
    VarId,
};

use crate::calendar::corpus_gen::{CAL_BASE, CAL_HORIZON, CalSizes, HOUR, created_at};
use crate::calendar::{ARM_BUSY, RSVP_ACCEPTED, RSVP_DECLINED, RSVP_TENTATIVE, ids};
use crate::corpus_gen::GenConfig;
use crate::families::{Draw, FamilyIndex, Kind, scalar_draw};
use crate::fixture::var;
use crate::translate::{ParamSlot, Translated};

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

fn allen(lhs: Term, rhs: Term, mask: AllenMask) -> ConditionTree {
    ConditionTree::Leaf(Comparison {
        op: CmpOp::Allen { mask },
        lhs,
        rhs,
    })
}

pub struct CalFamily {
    pub name: &'static str,
    pub kind: Kind,
    pub query: fn() -> Query,
    pub params: fn(&GenConfig) -> Vec<Draw>,
    pub golden_sql: &'static str,
    pub hand_param_slots: Option<&'static [ParamSlot]>,
    pub param_policy: &'static str,
    pub indexes: &'static [FamilyIndex],
}

impl CalFamily {
    /// # Errors
    pub fn sql_for(
        &self,
        query: &Query,
        draw: &[crate::naive::ParamValue],
    ) -> Result<Translated, String> {
        match self.hand_param_slots {
            Some(slots) => Ok(Translated {
                sql: self.golden_sql.to_owned(),
                params: slots.to_vec(),
            }),
            None => crate::translate::translate(
                query,
                crate::calendar::schema(),
                &crate::families::set_bindings(draw),
            ),
        }
    }
}

fn busy_scan_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::CLAIM),
            bindings: vec![
                (ids::claim::PERSON, var(0)),
                (ids::claim::ARM, Term::Literal(Value::U64(ARM_BUSY))),
                (ids::claim::SPAN, var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![allen(var(1), param(0), AllenMask::INTERSECTS)],
    })
}

const ACTIVE_SPAN: i64 = 22_000_000;

fn window(at: i64, width: i64) -> Value {
    Value::IntervalI64(bumbledb::Interval::<i64>::new(at, at + width).expect("nonempty interval"))
}

fn busy_scan_params(_: &GenConfig) -> Vec<Draw> {
    let width = ACTIVE_SPAN / 64;
    vec![
        scalar_draw(vec![window(CAL_BASE + ACTIVE_SPAN / 16, width)]),
        scalar_draw(vec![window(CAL_BASE + ACTIVE_SPAN / 4, width)]),
        scalar_draw(vec![window(CAL_BASE + ACTIVE_SPAN / 2, width)]),
        scalar_draw(vec![window(CAL_BASE - 2 * HOUR, HOUR)]),
    ]
}

fn meets_chain_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::CLAIM),
                bindings: vec![(ids::claim::PERSON, param(0)), (ids::claim::SPAN, var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::CLAIM),
                bindings: vec![(ids::claim::PERSON, param(0)), (ids::claim::SPAN, var(2))],
            },
        ],
        negated: vec![],
        conditions: vec![
            allen(var(1), var(2), AllenMask::MEETS),
            allen(var(1), param(1), AllenMask::DURING),
        ],
    })
}

fn meets_chain_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = CalSizes::of(cfg.scale);
    let full = Value::IntervalI64(
        bumbledb::Interval::<i64>::new(CAL_BASE - HOUR, CAL_HORIZON).expect("nonempty interval"),
    );
    let quarter = Value::IntervalI64(
        bumbledb::Interval::<i64>::new(CAL_BASE - HOUR, CAL_BASE + ACTIVE_SPAN / 4)
            .expect("nonempty interval"),
    );
    vec![
        scalar_draw(vec![Value::U64(0), full.clone()]),
        scalar_draw(vec![Value::U64(sizes.persons / 2), full.clone()]),
        scalar_draw(vec![Value::U64(63), quarter]),
        scalar_draw(vec![Value::U64(sizes.persons + 1_000_000), full]),
    ]
}

/// The distinct `rsvp` selections still prove the arms disjoint and
/// introspection reports that knowledge, but execution deliberately keeps the
/// spanning set after the measured refutation in.
fn rsvp_union_query() -> Query {
    let arm = |ordinal: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::ATTENDANCE),
            bindings: vec![
                (ids::attendance::EVENT, var(0)),
                (ids::attendance::PERSON, var(1)),
                (ids::attendance::RSVP, Term::Literal(Value::U64(ordinal))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    };
    Query {
        interiors: vec![],
        head: vec![bumbledb::HeadTerm::Var, bumbledb::HeadTerm::Var],
        rules: vec![arm(RSVP_ACCEPTED), arm(RSVP_TENTATIVE), arm(RSVP_DECLINED)],
        rec: None,
    }
}

fn rsvp_union_params(_: &GenConfig) -> Vec<Draw> {
    vec![scalar_draw(vec![])]
}

fn conflict_pairs_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::PERSON),
                bindings: vec![(ids::person::ID, var(0)), (ids::person::ACCOUNT, param(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::CLAIM),
                bindings: vec![(ids::claim::PERSON, var(0)), (ids::claim::SPAN, var(2))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::PERSON),
                bindings: vec![(ids::person::ID, var(1)), (ids::person::ACCOUNT, param(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::CLAIM),
                bindings: vec![(ids::claim::PERSON, var(1)), (ids::claim::SPAN, var(3))],
            },
        ],
        negated: vec![],
        conditions: vec![allen(var(2), var(3), AllenMask::INTERSECTS)],
    })
}

fn conflict_pairs_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = CalSizes::of(cfg.scale);
    vec![
        scalar_draw(vec![Value::U64(0)]),
        scalar_draw(vec![Value::U64(1)]),
        scalar_draw(vec![Value::U64(sizes.accounts / 2)]),
        scalar_draw(vec![Value::U64(sizes.accounts + 1_000_000)]),
    ]
}

fn conflict_free_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::PERSON),
                bindings: vec![(ids::person::ID, var(0)), (ids::person::ACCOUNT, param(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EVENT),
                bindings: vec![(ids::event::CREATED_AT, param(1))],
            },
        ],
        negated: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::CLAIM),
            bindings: vec![(ids::claim::PERSON, var(0)), (ids::claim::SPAN, param(1))],
        }],
        conditions: vec![],
    })
}

fn conflict_free_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = CalSizes::of(cfg.scale);
    let instant = |event: u64| Value::I64(created_at(cfg.seed, event % sizes.events.max(1)));
    vec![
        scalar_draw(vec![Value::U64(0), instant(0)]),
        scalar_draw(vec![Value::U64(1), instant(17)]),
        scalar_draw(vec![
            Value::U64(sizes.accounts / 2),
            instant(sizes.events / 2),
        ]),
        scalar_draw(vec![Value::U64(sizes.accounts + 1_000_000), instant(3)]),
    ]
}

/// `Q(p, Pack(s)):- Person(id = p, account = ?0), Claim(person = p, span = s),
/// Allen(s, ?1, INTERSECTS)` — one row per (person, maximal busy-or-OOO
/// segment) among claims touching the window; free time is the host's two-line
/// gap walk over the sorted output (the recorded `Gaps` refusal).
fn free_busy_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(2) }],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::PERSON),
                bindings: vec![(ids::person::ID, var(0)), (ids::person::ACCOUNT, param(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::CLAIM),
                bindings: vec![(ids::claim::PERSON, var(0)), (ids::claim::SPAN, var(2))],
            },
        ],
        negated: vec![],
        conditions: vec![allen(var(2), param(1), AllenMask::INTERSECTS)],
    })
}

fn free_busy_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = CalSizes::of(cfg.scale);
    let wide = Value::IntervalI64(
        bumbledb::Interval::<i64>::new(CAL_BASE - HOUR, CAL_BASE + ACTIVE_SPAN)
            .expect("nonempty interval"),
    );
    let narrow = window(CAL_BASE + ACTIVE_SPAN / 8, ACTIVE_SPAN / 64);
    vec![
        scalar_draw(vec![Value::U64(0), wide.clone()]),
        scalar_draw(vec![Value::U64(0), narrow]),
        scalar_draw(vec![Value::U64(sizes.accounts / 2), wide.clone()]),
        scalar_draw(vec![Value::U64(sizes.accounts + 1_000_000), wide]),
    ]
}

fn slot_scan_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::SLOT),
            bindings: vec![(ids::slot::ROOM, var(0)), (ids::slot::SPAN, var(1))],
        }],
        negated: vec![],
        conditions: vec![allen(var(1), param(0), AllenMask::INTERSECTS)],
    })
}

fn grid_span(sizes: &CalSizes) -> i64 {
    i64::try_from(sizes.slots_per_room / 3 + 1).expect("fits") * 8 * HOUR
}

fn slot_scan_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = CalSizes::of(cfg.scale);
    let span = grid_span(&sizes);
    let width = span / 16;
    vec![
        scalar_draw(vec![window(CAL_BASE + span / 8, width)]),
        scalar_draw(vec![window(CAL_BASE + span / 2, width)]),
        scalar_draw(vec![window(CAL_BASE + span * 7 / 8, width)]),
        // The pre-epoch miss: no slot starts before CAL_BASE, and the
        scalar_draw(vec![window(CAL_BASE - 2 * HOUR, HOUR)]),
    ]
}

/// Min-of-3 keeps the low tail symmetrically, so gates are unaffected; the
/// statistic itself is frozen with the published protocol.
fn slot_booking_overlap_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::SLOT),
                bindings: vec![(ids::slot::ROOM, param(0)), (ids::slot::SPAN, var(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::BOOKING),
                bindings: vec![(ids::booking::ROOM, param(0)), (ids::booking::SPAN, var(1))],
            },
        ],
        negated: vec![],
        conditions: vec![allen(var(0), var(1), AllenMask::INTERSECTS)],
    })
}

fn slot_booking_overlap_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = CalSizes::of(cfg.scale);
    vec![
        scalar_draw(vec![Value::U64(0)]),
        scalar_draw(vec![Value::U64(1)]),
        scalar_draw(vec![Value::U64(sizes.rooms / 2)]),
        scalar_draw(vec![Value::U64(sizes.rooms + 1_000_000)]),
    ]
}

macro_rules! intersects_param {
    ($s:literal, $e:literal, $a:literal, $b:literal) => {
        concat!(
            "((", $s, " < ", $a, " AND ", $a, " < ", $e, " AND ", $e, " < ", $b, ")", " OR (", $s,
            " = ", $a, " AND ", $e, " < ", $b, ")", " OR (", $a, " < ", $s, " AND ", $e, " < ", $b,
            ")", " OR (", $a, " < ", $s, " AND ", $e, " = ", $b, ")", " OR (", $s, " = ", $a,
            " AND ", $e, " = ", $b, ")", " OR (", $s, " < ", $a, " AND ", $e, " = ", $b, ")",
            " OR (", $s, " < ", $a, " AND ", $b, " < ", $e, ")", " OR (", $s, " = ", $a, " AND ",
            $b, " < ", $e, ")", " OR (", $a, " < ", $s, " AND ", $s, " < ", $b, " AND ", $b, " < ",
            $e, "))"
        )
    };
}

pub const BUSY_SCAN: &str = concat!(
    "SELECT DISTINCT t0.\"person\", t0.\"span_start\", t0.\"span_end\" FROM \"Claim\" AS t0 ",
    "WHERE t0.\"arm\" = 0 AND ",
    intersects_param!("t0.\"span_start\"", "t0.\"span_end\"", "?1", "?2")
);

pub const MEETS_CHAIN: &str = "SELECT DISTINCT t0.\"span_start\", t0.\"span_end\", t1.\"span_start\", t1.\"span_end\" FROM \"Claim\" AS t0, \"Claim\" AS t1 WHERE t0.\"person\" = ?1 AND t1.\"person\" = ?1 AND ((t0.\"span_end\" = t1.\"span_start\")) AND ((?2 < t0.\"span_start\" AND t0.\"span_end\" < ?3))";

pub const RSVP_UNION: &str = "SELECT DISTINCT t0.\"event\", t0.\"person\" FROM \"Attendance\" AS t0 WHERE t0.\"rsvp\" = 0 UNION SELECT DISTINCT t0.\"event\", t0.\"person\" FROM \"Attendance\" AS t0 WHERE t0.\"rsvp\" = 1 UNION SELECT DISTINCT t0.\"event\", t0.\"person\" FROM \"Attendance\" AS t0 WHERE t0.\"rsvp\" = 2";

pub const CONFLICT_PAIRS: &str = concat!(
    "SELECT DISTINCT t0.\"id\", t2.\"id\" FROM \"Person\" AS t0, \"Claim\" AS t1, ",
    "\"Person\" AS t2, \"Claim\" AS t3 WHERE t0.\"account\" = ?1 AND t0.\"id\" = t1.\"person\" ",
    "AND t2.\"account\" = ?1 AND t2.\"id\" = t3.\"person\" AND ",
    intersects_param!(
        "t1.\"span_start\"",
        "t1.\"span_end\"",
        "t3.\"span_start\"",
        "t3.\"span_end\""
    )
);

pub const CONFLICT_FREE: &str = "SELECT DISTINCT t0.\"id\" FROM \"Person\" AS t0, \"Event\" AS t1 WHERE t0.\"account\" = ?1 AND t1.\"created_at\" = ?2 AND NOT EXISTS (SELECT 1 FROM \"Claim\" AS n0 WHERE n0.\"person\" = t0.\"id\" AND n0.\"span_start\" <= ?2 AND ?2 < n0.\"span_end\")";

/// Verified row-identical against the engine's `Pack` and the naive model's
/// from-the-definition coalesce before any timing.
pub const FREE_BUSY: &str = concat!(
    "SELECT p, MIN(s), MAX(e) FROM (",
    "SELECT p, s, e, SUM(head) OVER (PARTITION BY p ORDER BY s, e ",
    "ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS island FROM (",
    "SELECT p, s, e, CASE WHEN s <= MAX(e) OVER (PARTITION BY p ORDER BY s, e ",
    "ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING) THEN 0 ELSE 1 END AS head FROM (",
    "SELECT DISTINCT t1.\"person\" AS p, t1.\"span_start\" AS s, t1.\"span_end\" AS e ",
    "FROM \"Person\" AS t0, \"Claim\" AS t1 ",
    "WHERE t0.\"account\" = ?1 AND t0.\"id\" = t1.\"person\" AND ",
    intersects_param!("t1.\"span_start\"", "t1.\"span_end\"", "?2", "?3"),
    "))) GROUP BY p, island"
);

pub const FREE_BUSY_SLOTS: &[ParamSlot] = &[
    ParamSlot::Whole(ParamId(0)),
    ParamSlot::Start(ParamId(1)),
    ParamSlot::End(ParamId(1)),
];

pub const SLOT_SCAN: &str = concat!(
    "SELECT DISTINCT t0.\"room\", t0.\"span_start\", t0.\"span_end\" FROM \"Slot\" AS t0 ",
    "WHERE ",
    intersects_param!("t0.\"span_start\"", "t0.\"span_end\"", "?1", "?2")
);

pub const SLOT_BOOKING_OVERLAP: &str = concat!(
    "SELECT DISTINCT t0.\"span_start\", t0.\"span_end\", t1.\"span_start\", t1.\"span_end\" ",
    "FROM \"Slot\" AS t0, \"Booking\" AS t1 ",
    "WHERE t0.\"room\" = ?1 AND t1.\"room\" = ?1 AND ",
    intersects_param!(
        "t0.\"span_start\"",
        "t0.\"span_end\"",
        "t1.\"span_start\"",
        "t1.\"span_end\""
    )
);

#[must_use]
pub fn all() -> &'static [CalFamily] {
    &[
        CalFamily {
            name: "busy_scan",
            kind: Kind::Gate,
            query: busy_scan_query,
            params: busy_scan_params,
            golden_sql: BUSY_SCAN,
            hand_param_slots: None,
            param_policy: "3 ~1.6%-of-span windows spread over the active span + 1 pre-epoch miss.",
            indexes: &[(
                "idx_claim_arm_span",
                "Claim",
                &["arm", "span_start", "span_end"],
            )],
        },
        CalFamily {
            name: "meets_chain",
            kind: Kind::Gate,
            query: meets_chain_query,
            params: meets_chain_params,
            golden_sql: MEETS_CHAIN,
            hand_param_slots: None,
            param_policy: "The Zipf-head person, a mid person, person 63 under a quarter window, + 1 person miss.",
            indexes: &[],
        },
        CalFamily {
            name: "rsvp_union",
            kind: Kind::Gate,
            query: rsvp_union_query,
            params: rsvp_union_params,
            golden_sql: RSVP_UNION,
            hand_param_slots: None,
            param_policy: "No params — the DU whole-read; one empty draw.",
            indexes: &[(
                "idx_attendance_rsvp",
                "Attendance",
                &["rsvp", "event", "person"],
            )],
        },
        CalFamily {
            name: "conflict_pairs",
            kind: Kind::Gate,
            query: conflict_pairs_query,
            params: conflict_pairs_params,
            golden_sql: CONFLICT_PAIRS,
            hand_param_slots: None,
            param_policy: "The head account (persons 0..8 — the dense stratum), two others, + 1 miss.",
            indexes: &[],
        },
        CalFamily {
            name: "conflict_free",
            kind: Kind::Gate,
            query: conflict_free_query,
            params: conflict_free_params,
            golden_sql: CONFLICT_FREE,
            hand_param_slots: None,
            param_policy: "3 (account, event-creation instant) pairs + 1 account miss; instants scatter over the active span.",
            indexes: &[("idx_event_created", "Event", &["created_at"])],
        },
        CalFamily {
            name: "free_busy",
            kind: Kind::Gate,
            query: free_busy_query,
            params: free_busy_params,
            golden_sql: FREE_BUSY,
            hand_param_slots: Some(FREE_BUSY_SLOTS),
            param_policy: "The head account wide + narrow, a mid account wide, + 1 miss (translator-unpaired: hand coalesce).",
            indexes: &[],
        },
        CalFamily {
            name: "slot_scan",
            kind: Kind::Report,
            query: slot_scan_query,
            params: slot_scan_params,
            golden_sql: SLOT_SCAN,
            hand_param_slots: None,
            param_policy: "3 ~6%-of-grid windows spread over the slot grid + 1 pre-epoch miss (fixed-width lane).",
            indexes: &[("idx_slot_span", "Slot", &["span_start", "span_end"])],
        },
        CalFamily {
            name: "slot_booking_overlap",
            kind: Kind::Report,
            query: slot_booking_overlap_query,
            params: slot_booking_overlap_params,
            golden_sql: SLOT_BOOKING_OVERLAP,
            hand_param_slots: None,
            param_policy: "The head room, room 1, a mid room, + 1 room miss (fixed x general Allen join).",
            indexes: &[],
        },
    ]
}

#[must_use]
pub fn digest() -> [u8; 32] {
    let mut digest = bumbledb::digest::Digest::new();
    for family in all() {
        digest.update(family.name.as_bytes());
        digest.update(format!("{:?}", (family.query)()).as_bytes());
        digest.update(family.golden_sql.as_bytes());
    }
    digest.finalize()
}

#[must_use]
pub fn index_ddl() -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for family in all() {
        for (name, table, columns) in family.indexes {
            if !seen.insert(*name) {
                continue;
            }
            let cols = columns
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");
            out.push(format!("CREATE INDEX \"{name}\" ON \"{table}\" ({cols})"));
        }
    }
    out
}

#[must_use]
pub fn expected_indexes() -> Vec<(String, String)> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for family in all() {
        for (name, table, _) in family.indexes {
            if seen.insert(*name) {
                out.push(((*table).to_owned(), (*name).to_owned()));
            }
        }
    }
    out
}

#[must_use]
pub fn translator_unpaired() -> Vec<&'static str> {
    all()
        .iter()
        .filter(|family| family.hand_param_slots.is_some())
        .map(|family| family.name)
        .collect()
}

pub const RANDOM_DRAWS: u32 = 4;

/// # Panics
#[must_use]
pub fn random_draw(name: &str, rng: &mut crate::corpus_gen::Rng, cfg: &GenConfig) -> Option<Draw> {
    let sizes = CalSizes::of(cfg.scale);
    let window = |rng: &mut crate::corpus_gen::Rng, max_width: i64| {
        let span = u64::try_from(ACTIVE_SPAN + 2 * HOUR).expect("positive");
        let start = CAL_BASE - HOUR + i64::try_from(rng.range(span)).expect("fits");
        let width = 1 + i64::try_from(rng.range(u64::try_from(max_width).expect("positive")))
            .expect("fits");
        window(start, width)
    };
    match name {
        "busy_scan" => Some(scalar_draw(vec![window(rng, ACTIVE_SPAN / 8)])),
        "meets_chain" => Some(scalar_draw(vec![
            Value::U64(rng.range(sizes.persons * 9 / 8)),
            window(rng, ACTIVE_SPAN),
        ])),
        "rsvp_union" => None,
        "conflict_pairs" => Some(scalar_draw(vec![Value::U64(
            rng.range(sizes.accounts * 9 / 8),
        )])),
        "conflict_free" => {
            let account = Value::U64(rng.range(sizes.accounts * 9 / 8));

            let instant = if rng.range(2) == 0 {
                created_at(cfg.seed, rng.range(sizes.events.max(1)))
            } else {
                CAL_BASE + i64::try_from(rng.range(22_000_000)).expect("fits")
            };
            Some(scalar_draw(vec![account, Value::I64(instant)]))
        }
        "free_busy" => Some(scalar_draw(vec![
            Value::U64(rng.range(sizes.accounts * 9 / 8)),
            window(rng, ACTIVE_SPAN / 4),
        ])),
        "slot_scan" => {
            let span = grid_span(&sizes);
            let start = CAL_BASE - HOUR
                + i64::try_from(rng.range(u64::try_from(span + 2 * HOUR).expect("positive")))
                    .expect("fits");
            let width = 1 + i64::try_from(rng.range(u64::try_from(span / 8).expect("positive")))
                .expect("fits");
            Some(scalar_draw(vec![Value::IntervalI64(
                bumbledb::Interval::<i64>::new(start, start + width).expect("nonempty interval"),
            )]))
        }
        "slot_booking_overlap" => Some(scalar_draw(vec![Value::U64(
            rng.range(sizes.rooms * 9 / 8),
        )])),
        other => unreachable!("unregistered calendar family {other}"),
    }
}

#[must_use]
pub fn unit_draw(name: &str, seed: u64, sizes: &CalSizes) -> Draw {
    let wide = window(CAL_BASE - HOUR, CAL_HORIZON - CAL_BASE + HOUR - 1);
    match name {
        "busy_scan" | "slot_scan" => scalar_draw(vec![wide]),
        "meets_chain" | "free_busy" => scalar_draw(vec![Value::U64(0), wide]),
        "rsvp_union" => scalar_draw(vec![]),

        "conflict_pairs" | "slot_booking_overlap" => scalar_draw(vec![Value::U64(0)]),
        "conflict_free" => scalar_draw(vec![
            Value::U64(0),
            Value::I64(created_at(seed, sizes.events / 2)),
        ]),
        other => unreachable!("unregistered calendar family {other}"),
    }
}
