//! The temporal scenario: the Allen kernel on its own turf — stabbing,
//! overlap joins, mixed masks, rays, coalesce. The corpus makes the stress cases
//! INVARIANTS instead of query-side filters: rays are `end == i64::MAX`
//! interval values (the engine's own ray representation), every bounded
//! span ends strictly inside the fixed horizon (the corpus law,
//! `corpus::spans`), and MEETS/DURING witnesses are planted
//! deterministically on the low keys — so the ray family is just the
//! stabbing query at a post-horizon instant, and the mixed-mask family's
//! both arms are asserted, not hoped. The special cases live in the
//! coordinates, not in branches.

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    AggOp, AllenMask, Atom, CmpOp, Comparison, ConditionTree, FindTerm, MaskTerm, ParamId, Query,
    RelationId, Rule, Term, Value, VarId,
};

use super::{DEFAULT_CAP, Scenario, ScenarioQuery, Surface, Twin};
use crate::fixture::var;

mod corpus;

#[cfg(test)]
mod tests;

bumbledb::schema! {
    pub Temporal;

    relation Key {
        id: u64 as TpKeyId, fresh,
    }
    relation Span {
        id: u64 as TpSpanId, fresh,
        key: u64 as TpKeyId,
        span: interval<i64>,
        weight: i64,
    }

    Span(key) <= Key(id);
}

/// The validated scenario schema, memoized for the inspection surfaces
/// (DDL rendering, typing); the store is created from [`Temporal`]'s
/// descriptor (`scenarios::load`).
///
/// # Panics
///
/// Never in practice: the declared scenario schema is valid.
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        Temporal
            .descriptor()
            .validate()
            .expect("the scenario schema is valid")
    })
}

/// Relation and field ids by declaration order.
pub mod ids {
    use bumbledb::{FieldId, RelationId};

    pub const KEY: RelationId = RelationId(0);
    pub const SPAN: RelationId = RelationId(1);

    pub mod key {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
    }
    pub mod span {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const KEY: FieldId = FieldId(1);
        pub const SPAN: FieldId = FieldId(2);
        pub const WEIGHT: FieldId = FieldId(3);
    }
}

/// One corpus row-stream list — the [`Scenario::rows`] shape.
type Rows = Vec<(RelationId, Box<dyn Iterator<Item = Vec<Value>>>)>;

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

fn lt(lhs: Term, rhs: Term) -> ConditionTree {
    ConditionTree::Leaf(Comparison {
        op: CmpOp::Lt,
        lhs,
        rhs,
    })
}

/// Point membership as a predicate: `lhs ∋ rhs` — the interval on the
/// left, the point on the right (the IR's retained lowering order).
fn point_in(lhs: Term, rhs: Term) -> ConditionTree {
    ConditionTree::Leaf(Comparison {
        op: CmpOp::PointIn,
        lhs,
        rhs,
    })
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

/// The count head shared by every folded family.
fn count() -> Vec<FindTerm> {
    vec![FindTerm::Aggregate {
        op: AggOp::Count,
        over: None,
    }]
}

/// t1/t4 — the stabbing probe, one IR shape for both families:
/// `Q(key, id) :- Span(id, key, span = v), v ∋ ?0` — which spans cover
/// the instant. The instant rides a `PointIn` condition, never the
/// interval-field binding: the bivalent-anchor rule (`ir.rs` `Atom`
/// docs; the ledger's `mandate_at_instant` comment) reads a LONE
/// interval-position param as interval value *equality*, so the
/// predicate form is the representation under which `?0` is I64-typed
/// point membership (`start ≤ t < end`) — the translate goldens'
/// `point_in` precedent. t4 is this same query at post-horizon
/// instants: the corpus law (every bounded span ends inside the
/// horizon) makes its answer set exactly the rays, with no ray
/// predicate anywhere.
fn stab() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::SPAN),
            bindings: vec![
                (ids::span::ID, var(1)),
                (ids::span::KEY, var(0)),
                (ids::span::SPAN, var(2)),
            ],
        }],
        negated: vec![],
        conditions: vec![point_in(var(2), param(0))],
    })
}

/// t2 — the pairwise span-overlap self-join per key, counted:
/// `Count :- Span(id = a, key = k, span = u), Span(id = b, key = k,
/// span = v), a < b, Allen(u, v, INTERSECTS)` — the strict id order
/// counts each unordered pair once.
fn overlap_join() -> Query {
    Query::single(Rule {
        finds: count(),
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::SPAN),
                bindings: vec![
                    (ids::span::ID, var(0)),
                    (ids::span::KEY, var(2)),
                    (ids::span::SPAN, var(3)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::SPAN),
                bindings: vec![
                    (ids::span::ID, var(1)),
                    (ids::span::KEY, var(2)),
                    (ids::span::SPAN, var(4)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![
            lt(var(0), var(1)),
            allen(var(3), var(4), AllenMask::INTERSECTS),
        ],
    })
}

/// t5 — the per-key coalescing fold: `Q(Pack(v)) :- Span(key = ?0,
/// span = v)` — Snodgrass coalescing of one key's spans into maximal
/// disjoint half-open segments (adjacency merges; a packed ray is a
/// ray; the empty binding set packs to the empty answer set). The one
/// head SQL cannot express: the translator refuses `Pack`
/// (`Inexpressible::PackAggregate`), so the `SQLite` side is the
/// hand-written islands SQL ([`HAND_T5`]) — the `free_busy` precedent.
fn pack_key() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Pack,
            over: Some(VarId(0)),
        }],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::SPAN),
            bindings: vec![(ids::span::KEY, param(0)), (ids::span::SPAN, var(0))],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// t3 — the mixed-mask pair join on one key: `Q(a, b) :- Span(id = a,
/// key = ?0, span = u), Span(id = b, key = ?0, span = v),
/// Allen(u, v, DURING ∪ MEETS)` — `BitOr` composes the mask, the
/// disjunction is data. No self-pair guard is needed: equal intervals
/// satisfy neither DURING nor MEETS, so `a == b` cannot answer.
fn mixed_mask() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::SPAN),
                bindings: vec![
                    (ids::span::ID, var(0)),
                    (ids::span::KEY, param(0)),
                    (ids::span::SPAN, var(2)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::SPAN),
                bindings: vec![
                    (ids::span::ID, var(1)),
                    (ids::span::KEY, param(0)),
                    (ids::span::SPAN, var(3)),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![allen(var(2), var(3), AllenMask::DURING | AllenMask::MEETS)],
    })
}

/// t1's instant policy — size-independent (fixed instants over the
/// fixed value horizon; the last set is the miss: nothing — rays
/// included — starts before `TP_BASE`, so a pre-base instant answers
/// nothing).
fn stab_params(_seed: u64) -> Vec<Vec<Value>> {
    use corpus::{TP_BASE, TP_HORIZON};
    vec![
        vec![Value::I64(TP_BASE + TP_HORIZON / 2)],
        vec![Value::I64(TP_BASE + TP_HORIZON / 4)],
        vec![Value::I64(TP_BASE + 1_000)],
        vec![Value::I64(TP_BASE - 10_000_000)],
    ]
}

/// t4's instant policy: POST-horizon instants — every bounded span has
/// ended (the corpus law), so only rays answer; the miss sits before
/// `TP_BASE`, where nothing has started.
fn ray_params(_seed: u64) -> Vec<Vec<Value>> {
    use corpus::{TP_BASE, TP_HORIZON};
    vec![
        vec![Value::I64(TP_BASE + TP_HORIZON + 1_000)],
        vec![Value::I64(TP_BASE + TP_HORIZON + 500_000)],
        vec![Value::I64(TP_BASE + 2 * TP_HORIZON)],
        vec![Value::I64(TP_BASE - 1)],
    ]
}

/// t3/t5's key policy: the heavy key 0 (the deterministic Zipf head),
/// two planted-witness keys, and the key miss — all present at both
/// scales (the planted witnesses live on keys 0..8). For t5 the miss is
/// the empty-fold case: an empty binding set packs to the empty answer
/// set, never a NULL row.
fn key_params(_seed: u64) -> Vec<Vec<Value>> {
    vec![
        vec![Value::U64(0)],
        vec![Value::U64(1)],
        vec![Value::U64(5)],
        vec![Value::U64(1_000_000)],
    ]
}

// ---------------------------------------------------------------------
// Hand-written SQL lanes (docs/architecture/60-validation.md): pinned
// constants, never regenerated from the translator; every lane passes
// the same uncapped multiset gate as the canonical translations.
// ---------------------------------------------------------------------

/// t2's hand-tuned twin: the canonical translation captured verbatim
/// (via a temporary printing test, the `HAND_R2` procedure), with the
/// single 9-basic `INTERSECTS` OR-block replaced by the two-comparison
/// overlap `(LS < RE AND RS < LE)` over the same column aliases —
/// everything else identical, so the lanes differ ONLY in the mask
/// rendering (the never-flatter-ourselves law: `SQLite` gets its best
/// shot beside the canonical OR-chain, both gated, both reported).
const HAND_T2: &str = "SELECT COUNT(*) FROM (SELECT DISTINCT t0.\"id\" AS v0, t1.\"id\" AS v1, t0.\"key\" AS v2, t0.\"span_start\" AS v3_start, t0.\"span_end\" AS v3_end, t1.\"span_start\" AS v4_start, t1.\"span_end\" AS v4_end FROM \"Span\" AS t0, \"Span\" AS t1 WHERE t0.\"key\" = t1.\"key\" AND t0.\"id\" < t1.\"id\" AND (t0.\"span_start\" < t1.\"span_end\" AND t1.\"span_start\" < t0.\"span_end\")) HAVING COUNT(*) > 0";

/// The tuned lane value for t2: [`HAND_T2`] with the canonical
/// placeholder row mirrored exactly — t2 is parameterless, so the row
/// is empty (asserted equal to the canonical `.params` in `tests`).
fn t2_tuned() -> crate::translate::Translated {
    crate::translate::Translated {
        sql: HAND_T2.to_owned(),
        params: vec![],
    }
}

/// t5's `SQLite` lane: the hand-written islands-and-gaps
/// window-function coalesce, adapted from the calendar `free_busy`
/// golden (`calendar/families.rs: FREE_BUSY` — verified row-identical
/// against the engine's `Pack` there): one key, so the person partition
/// drops; no window filter, so the innermost select is the bare
/// distinct span set of `"key" = ?1`. Order the distinct spans, start a
/// new island where a start exceeds the running max end (`s <= MAX(e)`
/// merges — half-open adjacency, exactly `Pack`'s law), fold each
/// island to `(MIN(s), MAX(e))` — one row per maximal segment, matching
/// the engine's relation-shaped `Pack` answers; the result signature is
/// the single Interval column, which `compare::from_sqlite` reassembles
/// from the two INTEGER halves. Rays: `end == i64::MAX` is an ordinary
/// INTEGER, so `MAX()` over it is correct — a packed ray is a ray. The
/// empty key (the miss draw): zero inner rows `GROUP BY` to zero
/// segments, the engine's empty answer set.
const HAND_T5: &str = "SELECT MIN(s), MAX(e) FROM (SELECT s, e, SUM(head) OVER (ORDER BY s, e ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS island FROM (SELECT s, e, CASE WHEN s <= MAX(e) OVER (ORDER BY s, e ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING) THEN 0 ELSE 1 END AS head FROM (SELECT DISTINCT t0.\"span_start\" AS s, t0.\"span_end\" AS e FROM \"Span\" AS t0 WHERE t0.\"key\" = ?1))) GROUP BY island";

/// The hand lane value for t5: [`HAND_T5`] with one placeholder slot —
/// the SQL names the key once as `?1`, bound from param 0.
fn t5_hand() -> crate::translate::Translated {
    crate::translate::Translated {
        sql: HAND_T5.to_owned(),
        params: vec![crate::translate::ParamSlot::Whole(ParamId(0))],
    }
}

fn queries() -> Vec<ScenarioQuery> {
    vec![
        ScenarioQuery {
            name: "t1_stab",
            surface: Surface::Query(stab),
            params: stab_params,
            about: "interval stabbing: point-in-span membership probe",
            twin: Twin::Canonical,
            cap: None,
        },
        ScenarioQuery {
            name: "t2_overlap_join",
            surface: Surface::Query(overlap_join),
            params: |_| vec![vec![]],
            about: "pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite",
            twin: Twin::Tuned(t2_tuned),
            cap: Some(DEFAULT_CAP),
        },
        ScenarioQuery {
            name: "t3_mixed_mask",
            surface: Surface::Query(mixed_mask),
            params: key_params,
            about: "mixed-mask (DURING ∪ MEETS) pair join on one key — the composite-mask disjunction as data",
            twin: Twin::Canonical,
            cap: None,
        },
        ScenarioQuery {
            name: "t4_ray_stab",
            surface: Surface::Query(stab),
            params: ray_params,
            about: "open-ended rays: past the horizon only rays answer — the ray case lives in the corpus coordinates, not in a filter",
            twin: Twin::Canonical,
            cap: None,
        },
        ScenarioQuery {
            name: "t5_pack_key",
            surface: Surface::Query(pack_key),
            params: key_params,
            about: "Pack/coalesce: Snodgrass coalescing per key — SQLite's lane is the hand-written islands SQL (the free_busy precedent)",
            twin: Twin::Hand(t5_hand),
            cap: None,
        },
    ]
}

/// One registration shared by the full world and its smoke twin — the
/// SAME queries and param policies over corpora that differ only in
/// row counts (`corpus::Sizes`), so the tier-0 smoke gate exercises
/// exactly what the night run times.
fn build(rows: fn(u64) -> Rows) -> Scenario {
    Scenario {
        name: "temporal",
        about: "the Allen kernel on its own turf: stabbing, overlap joins, mixed masks, rays, coalesce",
        schema,
        descriptor: || bumbledb::Theory::descriptor(Temporal),
        rows,
        extra_indexes: &[
            "CREATE INDEX ix_tp_span_key ON \"Span\"(\"key\")",
            "CREATE INDEX ix_tp_span_key_start ON \"Span\"(\"key\", \"span_start\", \"span_end\")",
            "CREATE INDEX ix_tp_span_start_end ON \"Span\"(\"span_start\", \"span_end\")",
        ],
        queries,
    }
}

/// The scenario registration (the full corpus).
#[must_use]
pub fn scenario() -> Scenario {
    build(corpus::rows_full)
}

/// The smoke twin: identical schema, queries, params, and indexes over
/// the tiny corpus — the oracle-gate entry for the world's tests.
#[cfg(test)]
pub fn scenario_smoke() -> Scenario {
    build(corpus::rows_smoke)
}
