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
    pub Rings;

    relation Party {
        id: u64 as RgPartyId, fresh,
        kind: u64 as RgPartyKindId,
    }
    relation Transfer {
        id: u64 as RgTransferId, fresh,
        src: u64 as RgPartyId,
        dst: u64 as RgPartyId,
        amount: i64,
        span: interval<i64>,
    }
    relation Bomb1 {
        src: u64 as RgB1NodeId,
        dst: u64 as RgB1NodeId,
    }
    relation Bomb2 {
        src: u64 as RgB2NodeId,
        dst: u64 as RgB2NodeId,
    }

    closed relation Kind as RgPartyKindId = { Person, Company, Exchange, Mixer };

    Party(kind) <= Kind(id);
    Transfer(src) <= Party(id);
    Transfer(dst) <= Party(id);
    Bomb1(src, dst) -> Bomb1;
    Bomb2(src, dst) -> Bomb2;
}

/// # Panics
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        Rings
            .descriptor()
            .validate()
            .expect("the scenario schema is valid")
    })
}

pub mod ids {
    use bumbledb::{FieldId, RelationId};

    pub const PARTY: RelationId = RelationId(0);
    pub const TRANSFER: RelationId = RelationId(1);
    pub const BOMB1: RelationId = RelationId(2);
    pub const BOMB2: RelationId = RelationId(3);

    pub const PARTY_KIND: RelationId = RelationId(4);

    pub mod party {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const KIND: FieldId = FieldId(1);
    }
    pub mod transfer {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const SRC: FieldId = FieldId(1);
        pub const DST: FieldId = FieldId(2);
        pub const AMOUNT: FieldId = FieldId(3);
        pub const SPAN: FieldId = FieldId(4);
    }

    pub mod bomb {
        use super::FieldId;
        pub const SRC: FieldId = FieldId(0);
        pub const DST: FieldId = FieldId(1);
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

fn ge(lhs: Term, rhs: Term) -> ConditionTree {
    ConditionTree::Leaf(Comparison {
        op: CmpOp::Ge,
        lhs,
        rhs,
    })
}

fn intersects(lhs: Term, rhs: Term) -> ConditionTree {
    ConditionTree::Leaf(Comparison {
        op: CmpOp::Allen {
            mask: AllenMask::INTERSECTS,
        },
        lhs,
        rhs,
    })
}

fn count() -> Vec<FindTerm> {
    vec![FindTerm::Count]
}

fn ring_atoms(with_spans: bool) -> Vec<Atom> {
    let mut atoms = vec![
        Atom {
            source: bumbledb::AtomSource::Edb(ids::TRANSFER),
            bindings: vec![
                (ids::transfer::SRC, var(0)),
                (ids::transfer::DST, var(1)),
                (ids::transfer::AMOUNT, var(3)),
            ],
        },
        Atom {
            source: bumbledb::AtomSource::Edb(ids::TRANSFER),
            bindings: vec![(ids::transfer::SRC, var(1)), (ids::transfer::DST, var(2))],
        },
        Atom {
            source: bumbledb::AtomSource::Edb(ids::TRANSFER),
            bindings: vec![(ids::transfer::SRC, var(2)), (ids::transfer::DST, var(0))],
        },
    ];
    if with_spans {
        for (atom, span_var) in atoms.iter_mut().zip([4u16, 5, 6]) {
            atom.bindings.push((ids::transfer::SPAN, var(span_var)));
        }
    }
    atoms
}

fn wash_ring() -> Query {
    Query::single(Rule {
        finds: count(),
        atoms: ring_atoms(false),
        negated: vec![],
        conditions: vec![lt(var(0), var(1)), lt(var(0), var(2)), ge(var(3), param(0))],
    })
}

fn temporal_ring() -> Query {
    Query::single(Rule {
        finds: count(),
        atoms: ring_atoms(true),
        negated: vec![],
        conditions: vec![
            lt(var(0), var(1)),
            lt(var(0), var(2)),
            ge(var(3), param(0)),
            intersects(var(4), var(5)),
            intersects(var(5), var(6)),
            intersects(var(6), var(4)),
        ],
    })
}

const HAND_R2: &str = "SELECT COUNT(*) FROM (SELECT DISTINCT t0.\"src\" AS v0, t0.\"dst\" AS v1, t1.\"dst\" AS v2, t0.\"amount\" AS v3, t0.\"span_start\" AS v4_start, t0.\"span_end\" AS v4_end, t1.\"span_start\" AS v5_start, t1.\"span_end\" AS v5_end, t2.\"span_start\" AS v6_start, t2.\"span_end\" AS v6_end FROM \"Transfer\" AS t0, \"Transfer\" AS t1, \"Transfer\" AS t2 WHERE t0.\"dst\" = t1.\"src\" AND t1.\"dst\" = t2.\"src\" AND t0.\"src\" = t2.\"dst\" AND t0.\"src\" < t0.\"dst\" AND t0.\"src\" < t1.\"dst\" AND t0.\"amount\" >= ?1 AND (t0.\"span_start\" < t1.\"span_end\" AND t1.\"span_start\" < t0.\"span_end\") AND (t1.\"span_start\" < t2.\"span_end\" AND t2.\"span_start\" < t1.\"span_end\") AND (t2.\"span_start\" < t0.\"span_end\" AND t0.\"span_start\" < t2.\"span_end\")) HAVING COUNT(*) > 0";

fn r2_tuned() -> crate::translate::Translated {
    crate::translate::Translated {
        sql: HAND_R2.to_owned(),
        params: vec![crate::translate::ParamSlot::Whole(ParamId(0))],
    }
}

fn bomb_triangle(rel: RelationId) -> Query {
    Query::single(Rule {
        finds: count(),
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(rel),
                bindings: vec![(ids::bomb::SRC, var(0)), (ids::bomb::DST, var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(rel),
                bindings: vec![(ids::bomb::SRC, var(1)), (ids::bomb::DST, var(2))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(rel),
                bindings: vec![(ids::bomb::SRC, var(2)), (ids::bomb::DST, var(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn bomb_t1() -> Query {
    bomb_triangle(ids::BOMB1)
}

fn bomb_t2() -> Query {
    bomb_triangle(ids::BOMB2)
}

fn reciprocal() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::TRANSFER),
                bindings: vec![(ids::transfer::SRC, var(0)), (ids::transfer::DST, var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::TRANSFER),
                bindings: vec![(ids::transfer::SRC, var(1)), (ids::transfer::DST, var(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::PARTY),
                bindings: vec![(ids::party::ID, var(0)), (ids::party::KIND, param(0))],
            },
        ],
        negated: vec![],
        conditions: vec![lt(var(0), var(1))],
    })
}

fn two_path_count() -> Query {
    Query::single(Rule {
        finds: count(),
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::TRANSFER),
                bindings: vec![(ids::transfer::SRC, var(0)), (ids::transfer::DST, var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::TRANSFER),
                bindings: vec![(ids::transfer::SRC, var(1)), (ids::transfer::DST, var(2))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn amount_params(_seed: u64) -> Vec<Vec<Value>> {
    vec![
        vec![Value::I64(0)],
        vec![Value::I64(100)],
        vec![Value::I64(1000)],
        vec![Value::I64(1_000_000)],
    ]
}

fn queries() -> Vec<ScenarioQuery> {
    vec![
        ScenarioQuery {
            name: "r1_wash_ring",
            surface: Surface::Query(wash_ring),
            params: amount_params,
            about: "the equality 3-ring (wash-trade) over power-law hubs — the binary-join exponent, capped",
            twin: Twin::Canonical,
            cap: Some(DEFAULT_CAP),
        },
        ScenarioQuery {
            name: "r2_temporal_ring",
            surface: Surface::Query(temporal_ring),
            params: amount_params,
            about: "the ring + pairwise Allen INTERSECTS — the temporal-ring shape",
            twin: Twin::Tuned(r2_tuned),
            cap: Some(DEFAULT_CAP),
        },
        ScenarioQuery {
            name: "r3_bomb_t1",
            surface: Surface::Query(bomb_t1),
            params: |_| vec![vec![]],
            about: "bipartite-bomb tier 1 (m=48): K_{m,m} + one planted triangle — answer 3 by construction; sized to finish within the cap",
            twin: Twin::Canonical,
            cap: Some(DEFAULT_CAP),
        },
        ScenarioQuery {
            name: "r4_bomb_t2",
            surface: Surface::Query(bomb_t2),
            params: |_| vec![vec![]],
            about: "bipartite-bomb tier 2 (m=384): m^3≈5.7e7 closing probes — the exponent evidence; SQLite predictably exceeds the cap, reported exceeded-cap, excluded and counted",
            twin: Twin::Canonical,
            cap: Some(DEFAULT_CAP),
        },
        ScenarioQuery {
            name: "r5_reciprocal",
            surface: Surface::Query(reciprocal),
            params: |_| {
                vec![
                    vec![Value::U64(0)],
                    vec![Value::U64(1)],
                    vec![Value::U64(2)],
                    vec![Value::U64(99)],
                ]
            },
            about: "the reciprocal-pair 2-cycle, kind-filtered",
            twin: Twin::Canonical,
            cap: None,
        },
        ScenarioQuery {
            name: "r6_two_path_count",
            surface: Surface::Query(two_path_count),
            params: |_| vec![vec![]],
            about: "the denominator story: the distinct 2-path count binary joins must materialize",
            twin: Twin::Canonical,
            cap: Some(DEFAULT_CAP),
        },
    ]
}

fn build(rows: fn(u64) -> Rows) -> Scenario {
    Scenario {
        name: "rings",
        about: "hub near-bipartite graphs: cyclic joins expose the binary-join exponent",
        schema,
        descriptor: || bumbledb::Theory::descriptor(Rings),
        rows,
        extra_indexes: &[
            "CREATE INDEX ix_rg_transfer_src_dst ON \"Transfer\"(\"src\", \"dst\")",
            "CREATE INDEX ix_rg_transfer_dst ON \"Transfer\"(\"dst\")",
            "CREATE INDEX ix_rg_transfer_amount ON \"Transfer\"(\"amount\")",
            "CREATE INDEX ix_rg_bomb1_dst ON \"Bomb1\"(\"dst\")",
            "CREATE INDEX ix_rg_bomb2_dst ON \"Bomb2\"(\"dst\")",
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
