//! The calendar read families (docs/architecture/60-validation.md § the
//! calendar benchmark): six timed queries, one per landed representation,
//! each naming what it times. Exact IR, seeded param policies,
//! hand-written SQL goldens, per-family `SQLite` index DDL — the same
//! identity discipline as the ledger families (`crate::families`);
//! `digest()` keys the verify stamp on this list.
//!
//! `free_busy` is the one family the IR→SQL translator cannot express
//! (`Pack` — [`crate::translate::Inexpressible::PackAggregate`]): it is
//! **reported translator-unpaired, never dropped** — its `SQLite` side is
//! the hand-written window-function coalesce below (`SQLite`'s honest
//! best shot at Snodgrass coalescing), verified row-identical against
//! the engine and the naive model before any timing.

use bumbledb::{
    AggOp, AllenMask, Atom, CmpOp, Comparison, ConditionTree, FindTerm, MaskTerm, ParamId, Query,
    Rule, Term, Value, VarId,
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
        op: CmpOp::Allen {
            mask: MaskTerm::Literal(mask),
        },
        lhs,
        rhs,
    })
}

/// One calendar family. `hand_param_slots` marks the translator-unpaired
/// case: `None` means the `SQLite` side is `translate()` output (pinned
/// equal to `golden_sql` by test); `Some(slots)` means the family's SQL
/// **is** `golden_sql` with these positional slots — the no-silent-caps
/// rule's visible form.
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
    /// The family's `SQLite` side for one draw: translator output for
    /// the paired families, the hand-written coalesce for `free_busy`.
    ///
    /// # Errors
    ///
    /// Translation errors, stringified (never for hand-paired families).
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

/// `busy_scan` — **times the Allen mask against a param window over an
/// O(n) scan** (PRDs 03/04: the mask kernel; `00-product.md`'s
/// range-accelerator trigger names this family as its evidence).
/// `Q(p, s) :- Claim(person = p, arm = Busy, span = s),
/// Allen(s, ?0, INTERSECTS)`.
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

/// The busiest stretch of the corpus timeline (the Zipf head's chains
/// reach `CAL_BASE + ~2.2 × 10⁷`; the tail's end early, so early windows
/// are dense).
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
        // The pre-epoch miss: no claim (rays included) starts before
        // CAL_BASE, so nothing intersects.
        scalar_draw(vec![window(CAL_BASE - 2 * HOUR, HOUR)]),
    ]
}

/// `meets_chain` — **times the named-relation probes: the singleton
/// basics `MEETS` (a chain join) and `DURING` (a window filter)** —
/// the mask's singleton-cost-equals-composite-cost claim, measured
/// (PRD 03). `Q(a, b) :- Claim(person = ?0, span = a),
/// Claim(person = ?0, span = b), Allen(a, b, MEETS),
/// Allen(a, ?1, DURING)` — back-to-back segments of one person's chain
/// inside a window (the generator abuts every third boundary, so chains
/// exist by construction).
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

/// `rsvp_union` — **times the DU whole-read: a three-rule program, one
/// rule per RSVP arm through one spanning union seen-set** (rules as
/// data, one sink, and cross-rule set semantics). The distinct `rsvp`
/// selections still prove the arms disjoint and introspection reports that
/// knowledge, but execution deliberately keeps the spanning set after
/// the measured refutation in `docs/architecture/40-execution.md`.
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
        head: vec![bumbledb::HeadTerm::Var, bumbledb::HeadTerm::Var],
        rules: vec![arm(RSVP_ACCEPTED), arm(RSVP_TENTATIVE), arm(RSVP_DECLINED)],
    }
}

fn rsvp_union_params(_: &GenConfig) -> Vec<Draw> {
    // Param-less whole read: one empty draw.
    vec![scalar_draw(vec![])]
}

/// `conflict_pairs` — **times the Allen-mask self-join** (PRD 04 — the
/// configuration kernel under a true interval-pair join).
/// `Q(p1, p2) :- Person(id = p1, account = ?0), Claim(person = p1,
/// span = u), Person(id = p2, account = ?0), Claim(person = p2,
/// span = v), Allen(u, v, INTERSECTS)` — person pairs of one account
/// concurrently claimed (reflexive pairs included, as in the ledger's
/// `mandate_overlap`).
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
        scalar_draw(vec![Value::U64(0)]), // the Zipf head's account
        scalar_draw(vec![Value::U64(1)]),
        scalar_draw(vec![Value::U64(sizes.accounts / 2)]),
        scalar_draw(vec![Value::U64(sizes.accounts + 1_000_000)]),
    ]
}

/// `conflict_free` — **times the anti-probe with a point-membership
/// binding** (PRD 04 + negation: the conflict family's conflict-free
/// variant). `Q(p) :- Person(id = p, account = ?0),
/// Event(created_at = ?1), ¬Claim(person = p, span ∋ ?1)` — persons of
/// one account with no claim covering the instant; the `Event` atom is
/// the instant's scalar anchor (the bivalent-anchor rule — a lone
/// interval-position param would read as interval value equality),
/// exactly the ledger's `mandate_at_instant` shape, negated.
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

/// `free_busy` — **times `Pack`, the coalescing fold** (PRDs 11/12: the
/// shared segment sweep's second continuation), per person per window.
/// `Q(p, Pack(s)) :- Person(id = p, account = ?0), Claim(person = p,
/// span = s), Allen(s, ?1, INTERSECTS)` — one row per (person, maximal
/// busy-or-OOO segment) among claims touching the window; free time is
/// the host's two-line gap walk over the sorted output (the recorded
/// `Gaps` refusal). Translator-unpaired: the `SQLite` side is the
/// hand-written window-function coalesce ([`FREE_BUSY_SQL`]).
fn free_busy_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(2)),
            },
        ],
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

/// `claim_hours` — **times the measure: `Sum(Duration)` grouped by claim
/// arm** (PRD 10 — the one arithmetic the point-set denotation defines),
/// under the `Allen(DISJOINT)` ray filter against `[CAL_HORIZON, ∞)`:
/// rays have no finite measure, and the filter keeps exactly the bounded
/// claims (every bounded end sits below the horizon), the documented
/// host idiom. The `source` binding is the claim key, so the fold's
/// distinct-bindings elision engages (key coverage — the `balance`
/// regime). `Q(arm, Sum(Duration(s))) :- Claim(source = c, arm, span = s),
/// Allen(s, [CAL_HORIZON, ∞), DISJOINT)`.
fn claim_hours_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::AggregateMeasure {
                op: AggOp::Sum,
                over: VarId(2),
            },
        ],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::CLAIM),
            bindings: vec![
                (ids::claim::SOURCE, var(1)),
                (ids::claim::ARM, var(0)),
                (ids::claim::SPAN, var(2)),
            ],
        }],
        negated: vec![],
        conditions: vec![allen(
            var(2),
            Term::Literal(Value::IntervalI64(
                bumbledb::Interval::<i64>::ray(CAL_HORIZON).expect("calendar ray"),
            )),
            AllenMask::DISJOINT,
        )],
    })
}

fn claim_hours_params(_: &GenConfig) -> Vec<Draw> {
    // Param-less full fold: one empty draw.
    vec![scalar_draw(vec![])]
}

/// `slot_scan` — **times the mask kernel over the fixed-width interval
/// type** (the order purge's `interval<i64, w>`: the encoding stores the
/// start word only, the end derives as `start + w`): the `busy_scan`
/// shape moved onto the 8-byte lane, so the pair prices the fixed
/// encoding against the general 16-byte form under the identical O(n)
/// scan. `Q(r, s) :- Slot(room = r, span = s), Allen(s, ?0, INTERSECTS)`.
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

/// The slot grid's covered stretch: one gapped-gapped-abutting triple
/// per `8 × HOUR` ([`crate::calendar::corpus_gen::slot_span`]).
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
        // fixed type has no rays, so nothing intersects.
        scalar_draw(vec![window(CAL_BASE - 2 * HOUR, HOUR)]),
    ]
}

/// `slot_booking_overlap` — **times the fixed × general Allen join**:
/// one room's fixed-width grid against its general-interval bookings —
/// the two interval encodings meeting under one `INTERSECTS` condition
/// (the width is the type; the join is over values, so the pair is
/// legal by the shared element domain). `Q(s, v) :- Slot(room = ?0,
/// span = s), Booking(room = ?0, span = v), Allen(s, v, INTERSECTS)`.
///
/// **The cross-process p50 bimodality is the rotation-boundary
/// tail-max, not an engine mode (mechanism hunt, 2026-07-17).** With
/// 256 samples rotated round-robin over these 4 draws (64 each), the
/// nearest-rank p50 is `sorted[127]` — and this family's two fastest
/// draw populations (the pre-epoch miss ≈ 208 ns, the `rooms/2` room
/// ≈ 18.8 µs) fill ranks 0–127 exactly, with the next population at
/// ≈ 280 µs. The reported p50 is therefore the MAX of the 64 `rooms/2`
/// samples: an extreme order statistic that swung 19.2–40.0 µs across
/// 30 fresh processes while every draw median held within ±0.5%
/// (d2 18792–18980 ns). One number per process ⇒ whole-process
/// "modes"; two independent tail-max draws ⇒ the observed 0.34–2.01
/// per-pair A/B ratios on identical binaries. Falsified alternatives:
/// same binary + same store still flips (10 processes); regenerated
/// stores are byte-identical (same blake3 across 8 regens) and flip
/// identically; a relinked binary (~8k text symbols moved) leaves
/// every draw median unchanged and flips within-arm — store
/// page-state and the code-placement relink lottery are both refuted.
/// Min-of-3 keeps the low tail symmetrically, so gates are unaffected;
/// the statistic itself is frozen with the published protocol.
/// `postings_without_tag` (families/read.rs) is the same mechanism at
/// the same rank.
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
        scalar_draw(vec![Value::U64(0)]), // the Zipf head's room
        scalar_draw(vec![Value::U64(1)]),
        scalar_draw(vec![Value::U64(sizes.rooms / 2)]),
        scalar_draw(vec![Value::U64(sizes.rooms + 1_000_000)]),
    ]
}

// ---------------------------------------------------------------------
// Hand-written SQL goldens (docs/architecture/60-validation.md): never
// regenerated from the translator; the pin test arbitrates.
// ---------------------------------------------------------------------

/// The 9 sharing basics of `INTERSECTS`, OR'd, over `(ls, le)` × the
/// param window `(?a, ?b)` — written once, spliced per family.
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

/// `busy_scan`: the busy arm pinned, the mask's 9 sharing basics OR'd
/// against the window's two placeholders.
pub const BUSY_SCAN: &str = concat!(
    "SELECT DISTINCT t0.\"person\", t0.\"span_start\", t0.\"span_end\" FROM \"Claim\" AS t0 ",
    "WHERE t0.\"arm\" = 0 AND ",
    intersects_param!("t0.\"span_start\"", "t0.\"span_end\"", "?1", "?2")
);

/// `meets_chain`: two singleton basics — `MEETS` as the chain join,
/// `DURING` as the window filter.
pub const MEETS_CHAIN: &str = "SELECT DISTINCT t0.\"span_start\", t0.\"span_end\", t1.\"span_start\", t1.\"span_end\" FROM \"Claim\" AS t0, \"Claim\" AS t1 WHERE t0.\"person\" = ?1 AND t1.\"person\" = ?1 AND ((t0.\"span_end\" = t1.\"span_start\")) AND ((?2 < t0.\"span_start\" AND t0.\"span_end\" < ?3))";

/// `rsvp_union`: one `SELECT DISTINCT` per arm joined by `UNION` — the
/// DU whole-read (`SQLite`'s `UNION` is exactly ∪ under DISTINCT
/// discipline).
pub const RSVP_UNION: &str = "SELECT DISTINCT t0.\"event\", t0.\"person\" FROM \"Attendance\" AS t0 WHERE t0.\"rsvp\" = 0 UNION SELECT DISTINCT t0.\"event\", t0.\"person\" FROM \"Attendance\" AS t0 WHERE t0.\"rsvp\" = 1 UNION SELECT DISTINCT t0.\"event\", t0.\"person\" FROM \"Attendance\" AS t0 WHERE t0.\"rsvp\" = 2";

/// `conflict_pairs`: the Allen self-join across persons of one account.
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

/// `conflict_free`: the anti-probe — `NOT EXISTS` with the instant's
/// membership formula inside, the instant anchored by the `Event` gate.
pub const CONFLICT_FREE: &str = "SELECT DISTINCT t0.\"id\" FROM \"Person\" AS t0, \"Event\" AS t1 WHERE t0.\"account\" = ?1 AND t1.\"created_at\" = ?2 AND NOT EXISTS (SELECT 1 FROM \"Claim\" AS n0 WHERE n0.\"person\" = t0.\"id\" AND n0.\"span_start\" <= ?2 AND ?2 < n0.\"span_end\")";

/// `free_busy`: `SQLite`'s honest best shot at the coalescing fold —
/// hand-written window-function islands (`SQLite` has no coalescing
/// aggregate; a recursive-CTE row walk is strictly slower, so the
/// islands form is the fairer opponent): order each person's distinct
/// claim windows, start a new island where a window's start exceeds the
/// running max end (`s <= max(prev e)` merges — half-open adjacency,
/// exactly `Pack`'s law), then fold each island to `(MIN(s), MAX(e))`.
/// Verified row-identical against the engine's `Pack` and the naive
/// model's from-the-definition coalesce before any timing.
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

/// `free_busy`'s positional slots: the account, then the window's halves
/// (the hand-written twin of the translator's `ParamSlot` order).
pub const FREE_BUSY_SLOTS: &[ParamSlot] = &[
    ParamSlot::Whole(ParamId(0)),
    ParamSlot::Start(ParamId(1)),
    ParamSlot::End(ParamId(1)),
];

/// `slot_scan`: the fixed-width lane's scan — the mask's 9 sharing
/// basics OR'd against the window's two placeholders over the slot
/// grid (`SQLite` stores both halves; the 8-byte start-only encoding is
/// the engine side's private economy, invisible to the mapped oracle).
pub const SLOT_SCAN: &str = concat!(
    "SELECT DISTINCT t0.\"room\", t0.\"span_start\", t0.\"span_end\" FROM \"Slot\" AS t0 ",
    "WHERE ",
    intersects_param!("t0.\"span_start\"", "t0.\"span_end\"", "?1", "?2")
);

/// `slot_booking_overlap`: the fixed × general Allen join within one
/// room.
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

/// `claim_hours`: the normative fold template over the distinct binding
/// set, the ray filter's 4 disjoint basics OR'd against the horizon
/// literal (`[1800000000, ∞)` — ∞ = `i64::MAX`, the largest end word).
pub const CLAIM_HOURS: &str = "SELECT v0, SUM(v2_end - v2_start) FROM (SELECT DISTINCT t0.\"arm\" AS v0, t0.\"source\" AS v1, t0.\"span_start\" AS v2_start, t0.\"span_end\" AS v2_end FROM \"Claim\" AS t0 WHERE ((t0.\"span_end\" < 1800000000) OR (t0.\"span_end\" = 1800000000) OR (9223372036854775807 = t0.\"span_start\") OR (9223372036854775807 < t0.\"span_start\"))) GROUP BY v0";

/// The registry: the calendar's nine rows — the six numbered queries,
/// the conflict family contributing its anti-probe variant as its own
/// row (`60-validation.md`'s table), plus the roster extension's two
/// fixed-width interval rows (`slot_scan`, `slot_booking_overlap` —
/// report-only: measurement infrastructure, not gate claims).
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // the registry is a table, one entry per family
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
            name: "claim_hours",
            kind: Kind::Gate,
            query: claim_hours_query,
            params: claim_hours_params,
            golden_sql: CLAIM_HOURS,
            hand_param_slots: None,
            param_policy: "No params — the ray-filtered full measure fold; one empty draw.",
            indexes: &[(
                "idx_claim_arm_span",
                "Claim",
                &["arm", "span_start", "span_end"],
            )],
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

/// The family-list digest — a verify-stamp ingredient beside the
/// ledger's ([`crate::families::digest`]); any calendar family change
/// re-baselines every stamp.
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

/// Every calendar-family-owned index, deduplicated by name, as
/// `CREATE INDEX` statements — the calendar mirror's family layer over
/// the statement-derived plan (`crate::sqlmap::schema_ddl`).
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

/// The calendar family-owned indexes as `(table, name)` pairs — the
/// fairness contract's registry beside the statement-derived set.
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

/// The translator-unpaired calendar families, by name — consumed by the
/// verify report so the gap is counted and printed, never silent (the
/// no-silent-caps rule).
#[must_use]
pub fn translator_unpaired() -> Vec<&'static str> {
    all()
        .iter()
        .filter(|family| family.hand_param_slots.is_some())
        .map(|family| family.name)
        .collect()
}

/// How many seeded random draws the verify pass adds per parameterized
/// family (the calendar's randomized slice — windows, persons,
/// accounts, and instants drawn over the corpus domains with misses).
pub const RANDOM_DRAWS: u32 = 4;

/// One seeded random draw for the verify pass's randomized calendar
/// slice: in-domain most of the time, out-of-domain misses by
/// construction (the 9/8 domain stretch). `None` for the param-less
/// families — a whole read has no draw axis to randomize.
///
/// # Panics
///
/// Never in practice: draw arithmetic stays far inside `i64`.
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
        "rsvp_union" | "claim_hours" => None,
        "conflict_pairs" => Some(scalar_draw(vec![Value::U64(
            rng.range(sizes.accounts * 9 / 8),
        )])),
        "conflict_free" => {
            let account = Value::U64(rng.range(sizes.accounts * 9 / 8));
            // Half the instants are actual event creations (membership
            // hits exist), half arbitrary points over the active span.
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
            // Windows over the slot grid's own stretch (the active-span
            // windows would mostly miss the grid).
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

/// One in-domain draw per family at unit scale (the naive lane: the
/// S-scale rotations are mostly out of the unit corpus's tiny domains;
/// these make every join produce witnesses).
#[must_use]
pub fn unit_draw(name: &str, seed: u64, sizes: &CalSizes) -> Draw {
    let wide = window(CAL_BASE - HOUR, CAL_HORIZON - CAL_BASE + HOUR - 1);
    match name {
        "busy_scan" | "slot_scan" => scalar_draw(vec![wide]),
        "meets_chain" | "free_busy" => scalar_draw(vec![Value::U64(0), wide]),
        "rsvp_union" | "claim_hours" => scalar_draw(vec![]),
        // The head account and the head room share ordinal 0.
        "conflict_pairs" | "slot_booking_overlap" => scalar_draw(vec![Value::U64(0)]),
        "conflict_free" => scalar_draw(vec![
            Value::U64(0),
            Value::I64(created_at(seed, sizes.events / 2)),
        ]),
        other => unreachable!("unregistered calendar family {other}"),
    }
}
