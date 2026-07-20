//! The temporal scenario: the Allen kernel on its own turf — stabbing,
//! overlap joins, mixed masks, rays. The corpus makes the stress cases
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

use super::{DEFAULT_CAP, Scenario, ScenarioQuery, Twin};
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

/// t3's key policy: the heavy key 0 (the deterministic Zipf head), two
/// planted-witness keys, and the key miss — all present at both scales
/// (the planted witnesses live on keys 0..8).
fn key_params(_seed: u64) -> Vec<Vec<Value>> {
    vec![
        vec![Value::U64(0)],
        vec![Value::U64(1)],
        vec![Value::U64(5)],
        vec![Value::U64(1_000_000)],
    ]
}

fn queries() -> Vec<ScenarioQuery> {
    vec![
        ScenarioQuery {
            name: "t1_stab",
            query: stab,
            params: stab_params,
            about: "interval stabbing: point-in-span membership probe",
            twin: Twin::Canonical,
            cap: None,
        },
        ScenarioQuery {
            name: "t2_overlap_join",
            query: overlap_join,
            params: |_| vec![vec![]],
            about: "pairwise span-overlap self-join per key, counted — the Allen OR-chain's price on SQLite",
            twin: Twin::Canonical,
            cap: Some(DEFAULT_CAP),
        },
        ScenarioQuery {
            name: "t3_mixed_mask",
            query: mixed_mask,
            params: key_params,
            about: "mixed-mask (DURING ∪ MEETS) pair join on one key — the composite-mask disjunction as data",
            twin: Twin::Canonical,
            cap: None,
        },
        ScenarioQuery {
            name: "t4_ray_stab",
            query: stab,
            params: ray_params,
            about: "open-ended rays: past the horizon only rays answer — the ray case lives in the corpus coordinates, not in a filter",
            twin: Twin::Canonical,
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
