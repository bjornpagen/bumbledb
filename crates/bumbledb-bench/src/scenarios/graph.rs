//! The graph fan-out scenario: a power-law directed graph. Multi-hop
//! traversals are worst-case join fan-out — every hop multiplies by the
//! out-degree, and hub starts make the intermediate sets explode. This
//! is the regime where WCOJ-class execution is supposed to earn its
//! keep; `SQLite` runs the same conjunctive SQL through nested-loop +
//! B-tree plans.

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    AggOp, Atom, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, ParamId, Query, Rule, Term,
    Value, VarId,
};

use super::{Scenario, ScenarioQuery, mix};
use crate::corpus_gen::Rng;
use crate::fixture::var;

bumbledb::schema! {
    pub Graph;

    relation Node {
        id: u64 as GNodeId, fresh,
        kind: u64 as GNodeKindId,
        score: i64,
    }
    relation Edge {
        src: u64 as GNodeId,
        dst: u64 as GNodeId,
        weight: i64,
    }

    closed relation Kind as GNodeKindId = { User, Bot, Org, Page, Group };

    Node(kind) <= Kind(id);
    Edge(src) <= Node(id);
    Edge(dst) <= Node(id);
    Edge(src, dst) -> Edge;
}

/// Relation ids by declaration order.
/// The validated scenario schema, memoized for the inspection surfaces
/// (DDL rendering, typing); the store is created from [`Graph`]'s
/// descriptor (`scenarios::load`).
///
/// # Panics
///
/// Never in practice: the declared scenario schema is valid.
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        Graph
            .descriptor()
            .validate()
            .expect("the scenario schema is valid")
    })
}

pub mod ids {
    use bumbledb::RelationId;
    pub const NODE: RelationId = RelationId(0);
    pub const EDGE: RelationId = RelationId(1);
    pub const NODE_KIND: RelationId = RelationId(2);
}

pub const NODES: u64 = 100_000;
pub const EDGES: u64 = 500_000;
/// 0.1% of nodes are hubs holding ~30% of edge sources.
const HUBS: u64 = NODES / 1000;

fn node_row(seed: u64, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(seed, ids::NODE.0, i));
    vec![
        Value::U64(i),
        Value::U64(rng.range(5)),
        Value::I64(i64::try_from(rng.range(1000)).expect("small")),
    ]
}

fn edge_row(seed: u64, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(seed, ids::EDGE.0, i));
    let src = if rng.chance(3, 10) {
        rng.range(HUBS)
    } else {
        HUBS + rng.range(NODES - HUBS)
    };
    // Destination locality: half the edges land near the source (real
    // graphs cluster), half anywhere — reciprocal edges arise naturally.
    let dst = if rng.chance(1, 2) {
        (src + 1 + rng.range(64)) % NODES
    } else {
        rng.range(NODES)
    };
    vec![
        Value::U64(src),
        Value::U64(dst),
        Value::I64(i64::try_from(rng.range(100)).expect("small")),
    ]
}

/// Distinct (src, dst) pairs so both engines load the identical edge set.
fn distinct_edges(seed: u64) -> Vec<Vec<Value>> {
    let mut loaded = std::collections::HashSet::new();
    let mut out = Vec::new();
    for i in 0..EDGES {
        let row = edge_row(seed, i);
        let (Value::U64(src), Value::U64(dst)) = (&row[0], &row[1]) else {
            unreachable!("edge rows are (u64, u64, i64)");
        };
        if loaded.insert((*src, *dst)) {
            out.push(row);
        }
    }
    out
}

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

/// Param policy shared by the traversal queries: one hub, two normal
/// nodes, one miss.
fn start_params(seed: u64, salt: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 901, salt));
    vec![
        vec![Value::U64(rng.range(HUBS))],
        vec![Value::U64(HUBS + rng.range(NODES - HUBS))],
        vec![Value::U64(HUBS + rng.range(NODES - HUBS))],
        vec![Value::U64(NODES + 1_000_000)],
    ]
}

/// g1 — direct out-neighbors.
fn neighbors() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::EDGE),
            bindings: vec![(FieldId(0), param(0)), (FieldId(1), var(0))],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// g2 — two hops out.
fn two_hop() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![(FieldId(0), param(0)), (FieldId(1), var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), var(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

/// g3 — three-hop reach, counted (the intermediate explosion, folded).
fn three_hop_count() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![(FieldId(0), param(0)), (FieldId(1), var(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![(FieldId(0), var(0)), (FieldId(1), var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), var(2))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

/// g4 — mutual (reciprocal) edges among a node kind: the 2-cycle.
fn mutual() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![(FieldId(0), var(0)), (FieldId(1), var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), var(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::NODE),
                bindings: vec![(FieldId(0), var(0)), (FieldId(1), param(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

/// g5 — triangles through a start node: the 3-cycle, counted.
fn triangles_from() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![(FieldId(0), param(0)), (FieldId(1), var(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![(FieldId(0), var(0)), (FieldId(1), var(1))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), param(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

/// g6 — weighted hop with node filter: ranges on both hop and target.
fn weighted_hop() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::EDGE),
                bindings: vec![
                    (FieldId(0), param(0)),
                    (FieldId(1), var(0)),
                    (FieldId(2), var(1)),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::NODE),
                bindings: vec![(FieldId(0), var(0)), (FieldId(2), var(2))],
            },
        ],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(1),
                rhs: param(1),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(2),
                rhs: param(2),
            }),
        ],
    })
}

fn weighted_hop_params(seed: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 901, 6));
    vec![
        vec![Value::U64(rng.range(HUBS)), Value::I64(50), Value::I64(500)],
        vec![
            Value::U64(HUBS + rng.range(NODES - HUBS)),
            Value::I64(10),
            Value::I64(100),
        ],
        vec![
            Value::U64(HUBS + rng.range(NODES - HUBS)),
            Value::I64(90),
            Value::I64(900),
        ],
        vec![Value::U64(NODES + 1_000_000), Value::I64(0), Value::I64(0)],
    ]
}

/// The scenario registration.
#[must_use]
pub fn scenario() -> Scenario {
    Scenario {
        name: "graph",
        about: "power-law directed graph: multi-hop fan-out, cycles",
        schema,
        descriptor: || bumbledb::Theory::descriptor(Graph),
        rows: |seed| {
            vec![
                (
                    ids::NODE,
                    Box::new((0..NODES).map(move |i| node_row(seed, i))),
                ),
                (ids::EDGE, Box::new(distinct_edges(seed).into_iter())),
            ]
        },
        extra_indexes: &[
            "CREATE INDEX ix_edge_dst ON \"Edge\"(\"dst\")",
            "CREATE INDEX ix_edge_weight ON \"Edge\"(\"weight\")",
            "CREATE INDEX ix_node_kind ON \"Node\"(\"kind\")",
            "CREATE INDEX ix_node_score ON \"Node\"(\"score\")",
        ],
        queries: || {
            vec![
                ScenarioQuery {
                    name: "g1_neighbors",
                    query: neighbors,
                    params: |seed| start_params(seed, 1),
                    about: "single hop: hub ~1.5k edges, normal ~4",
                },
                ScenarioQuery {
                    name: "g2_two_hop",
                    query: two_hop,
                    params: |seed| start_params(seed, 2),
                    about: "two hops, deduplicated destination set",
                },
                ScenarioQuery {
                    name: "g3_three_hop_count",
                    query: three_hop_count,
                    params: |seed| start_params(seed, 3),
                    about: "three-hop reach folded to Count",
                },
                ScenarioQuery {
                    name: "g4_mutual",
                    query: mutual,
                    params: |_| {
                        vec![
                            vec![Value::U64(0)],
                            vec![Value::U64(1)],
                            vec![Value::U64(2)],
                            vec![Value::U64(4)],
                        ]
                    },
                    about: "reciprocal-edge 2-cycle over the full graph",
                },
                ScenarioQuery {
                    name: "g5_triangles_from",
                    query: triangles_from,
                    params: |seed| start_params(seed, 5),
                    about: "3-cycle through a start node, counted",
                },
                ScenarioQuery {
                    name: "g6_weighted_hop",
                    query: weighted_hop,
                    params: weighted_hop_params,
                    about: "hop + weight range + target-score range",
                },
            ]
        },
    }
}
