use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    AllenMask, Atom, CmpOp, Comparison, ConditionTree, FindTerm, ParamId, Query, RelationId, Rule,
    Term, Value, VarId,
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

/// # Panics
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

fn point_in(lhs: Term, rhs: Term) -> ConditionTree {
    ConditionTree::Leaf(Comparison {
        op: CmpOp::PointIn,
        lhs,
        rhs,
    })
}

fn allen(lhs: Term, rhs: Term, mask: AllenMask) -> ConditionTree {
    ConditionTree::Leaf(Comparison {
        op: CmpOp::Allen { mask },
        lhs,
        rhs,
    })
}

fn count() -> Vec<FindTerm> {
    vec![FindTerm::Count]
}

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

fn pack_key() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Pack { over: VarId(0) }],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::SPAN),
            bindings: vec![(ids::span::KEY, param(0)), (ids::span::SPAN, var(0))],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

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

fn stab_params(_seed: u64) -> Vec<Vec<Value>> {
    use corpus::{TP_BASE, TP_HORIZON};
    vec![
        vec![Value::I64(TP_BASE + TP_HORIZON / 2)],
        vec![Value::I64(TP_BASE + TP_HORIZON / 4)],
        vec![Value::I64(TP_BASE + 1_000)],
        vec![Value::I64(TP_BASE - 10_000_000)],
    ]
}

fn ray_params(_seed: u64) -> Vec<Vec<Value>> {
    use corpus::{TP_BASE, TP_HORIZON};
    vec![
        vec![Value::I64(TP_BASE + TP_HORIZON + 1_000)],
        vec![Value::I64(TP_BASE + TP_HORIZON + 500_000)],
        vec![Value::I64(TP_BASE + 2 * TP_HORIZON)],
        vec![Value::I64(TP_BASE - 1)],
    ]
}

fn key_params(_seed: u64) -> Vec<Vec<Value>> {
    vec![
        vec![Value::U64(0)],
        vec![Value::U64(1)],
        vec![Value::U64(5)],
        vec![Value::U64(1_000_000)],
    ]
}

const HAND_T2: &str = "SELECT COUNT(*) FROM (SELECT DISTINCT t0.\"id\" AS v0, t1.\"id\" AS v1, t0.\"key\" AS v2, t0.\"span_start\" AS v3_start, t0.\"span_end\" AS v3_end, t1.\"span_start\" AS v4_start, t1.\"span_end\" AS v4_end FROM \"Span\" AS t0, \"Span\" AS t1 WHERE t0.\"key\" = t1.\"key\" AND t0.\"id\" < t1.\"id\" AND (t0.\"span_start\" < t1.\"span_end\" AND t1.\"span_start\" < t0.\"span_end\")) HAVING COUNT(*) > 0";

fn t2_tuned() -> crate::translate::Translated {
    crate::translate::Translated {
        sql: HAND_T2.to_owned(),
        params: vec![],
    }
}

const HAND_T5: &str = "SELECT MIN(s), MAX(e) FROM (SELECT s, e, SUM(head) OVER (ORDER BY s, e ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS island FROM (SELECT s, e, CASE WHEN s <= MAX(e) OVER (ORDER BY s, e ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING) THEN 0 ELSE 1 END AS head FROM (SELECT DISTINCT t0.\"span_start\" AS s, t0.\"span_end\" AS e FROM \"Span\" AS t0 WHERE t0.\"key\" = ?1))) GROUP BY island";

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

#[must_use]
pub fn scenario() -> Scenario {
    build(corpus::rows_full)
}

#[cfg(test)]
pub fn scenario_smoke() -> Scenario {
    build(corpus::rows_smoke)
}
