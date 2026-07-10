//! The OLAP rollup scenario: a star-schema fact table with dimension
//! rollups — group-by aggregates over wide scans, the regime where
//! column images and the aggregate sink carry the load and `SQLite` gets
//! its covering B-trees. Set semantics note: bumbledb folds *distinct*
//! bindings, so every fact row carries its fresh id into the binding
//! set (the true-rollup pattern the ledger's balance family pins).

use bumbledb::{
    AggOp, Atom, CmpOp, Comparison, FieldId, FindTerm, ParamId, PredicateTree, Query, Rule, Term,
    Value, VarId,
};

use super::{mix, Scenario, ScenarioQuery};
use crate::gen::Rng;

bumbledb::schema! {
    pub Olap;

    relation Store {
        id: u64 as OStoreId, fresh,
        region: enum Region { Na, Eu, Apac, Latam, Mea, Anz },
        tier: enum Tier { Flagship, Standard, Outlet },
    }
    relation Product {
        id: u64 as OProductId, fresh,
        category: enum Category {
            C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15,
        },
        brand: u64,
        price: i64,
    }
    relation Customer {
        id: u64 as OCustomerId, fresh,
        segment: enum Segment { Consumer, Smb, Enterprise, Public },
    }
    relation Sale {
        id: u64 as OSaleId, fresh,
        day: i64,
        store: u64 as OStoreId,
        product: u64 as OProductId,
        customer: u64 as OCustomerId,
        qty: i64,
        total: i64,
        promo: bool,
    }

    Sale(store) <= Store(id);
    Sale(product) <= Product(id);
    Sale(customer) <= Customer(id);
}

/// Relation ids by declaration order.
/// The validated scenario schema, memoized for the inspection surfaces
/// (DDL rendering, typing); the store is created from [`Olap`]'s
/// descriptor (`scenarios::load`).
///
/// # Panics
///
/// Never in practice: the declared scenario schema is valid.
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        Olap.descriptor()
            .validate()
            .expect("the scenario schema is valid")
    })
}

pub mod ids {
    use bumbledb::RelationId;
    pub const STORE: RelationId = RelationId(0);
    pub const PRODUCT: RelationId = RelationId(1);
    pub const CUSTOMER: RelationId = RelationId(2);
    pub const SALE: RelationId = RelationId(3);
}

pub const STORES: u64 = 200;
pub const PRODUCTS: u64 = 5_000;
pub const CUSTOMERS: u64 = 20_000;
pub const SALES: u64 = 500_000;
pub const BRANDS: u64 = 400;
/// Days span three years.
pub const DAYS: u64 = 1_095;

fn row(seed: u64, rel: bumbledb::RelationId, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(seed, rel.0, i));
    match rel {
        ids::STORE => vec![
            Value::U64(i),
            Value::Enum(u8::try_from(rng.range(6)).expect("small")),
            Value::Enum(u8::try_from(rng.range(3)).expect("small")),
        ],
        ids::PRODUCT => vec![
            Value::U64(i),
            Value::Enum(u8::try_from(rng.range(16)).expect("small")),
            Value::U64(rng.range(BRANDS)),
            Value::I64(100 + i64::try_from(rng.range(99_900)).expect("small")),
        ],
        ids::CUSTOMER => vec![
            Value::U64(i),
            Value::Enum(u8::try_from(rng.range(4)).expect("small")),
        ],
        ids::SALE => {
            // Seasonality: sales cluster toward the recent third of the
            // day span (range predicates over `day` select real slices).
            let day = if rng.chance(1, 2) {
                (DAYS * 2 / 3) + rng.range(DAYS / 3)
            } else {
                rng.range(DAYS)
            };
            let qty = 1 + i64::try_from(rng.range(20)).expect("small");
            let unit = 100 + i64::try_from(rng.range(9_900)).expect("small");
            vec![
                Value::U64(i),
                Value::I64(i64::try_from(day).expect("small")),
                Value::U64(rng.range(STORES)),
                Value::U64(rng.range(PRODUCTS)),
                Value::U64(rng.range(CUSTOMERS)),
                Value::I64(qty),
                Value::I64(qty * unit),
                Value::Bool(rng.chance(1, 5)),
            ]
        }
        other => unreachable!("no such relation {other:?}"),
    }
}

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

/// o1 — revenue by region: full-fact rollup through one dimension.
fn revenue_by_region() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::SALE,
                bindings: vec![
                    (FieldId(0), var(2)),
                    (FieldId(2), var(3)),
                    (FieldId(6), var(1)),
                ],
            },
            Atom {
                relation: ids::STORE,
                bindings: vec![(FieldId(0), var(3)), (FieldId(1), var(0))],
            },
        ],
        negated: vec![],
        predicates: vec![],
    })
}

/// o2 — category totals inside a day window: the windowed drill.
fn category_window() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::SALE,
                bindings: vec![
                    (FieldId(0), var(2)),
                    (FieldId(1), var(3)),
                    (FieldId(3), var(4)),
                    (FieldId(6), var(1)),
                ],
            },
            Atom {
                relation: ids::PRODUCT,
                bindings: vec![(FieldId(0), var(4)), (FieldId(1), var(0))],
            },
        ],
        negated: vec![],
        predicates: vec![
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(3),
                rhs: param(0),
            }),
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: var(3),
                rhs: param(1),
            }),
        ],
    })
}

fn day_windows(_: u64) -> Vec<Vec<Value>> {
    let d = |x: u64| Value::I64(i64::try_from(x).expect("small"));
    vec![
        vec![d(DAYS - 30), d(DAYS)],
        vec![d(DAYS - 90), d(DAYS)],
        vec![d(0), d(DAYS / 3)],
        vec![d(DAYS), d(DAYS)],
    ]
}

/// o3 — promo split: the 2-group full-scan fold.
fn promo_split() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: ids::SALE,
            bindings: vec![
                (FieldId(0), var(2)),
                (FieldId(6), var(1)),
                (FieldId(7), var(0)),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    })
}

/// o4 — segment × category: the two-dimension rollup (64 groups).
fn segment_category() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::SALE,
                bindings: vec![
                    (FieldId(0), var(2)),
                    (FieldId(3), var(3)),
                    (FieldId(4), var(4)),
                ],
            },
            Atom {
                relation: ids::CUSTOMER,
                bindings: vec![(FieldId(0), var(4)), (FieldId(1), var(0))],
            },
            Atom {
                relation: ids::PRODUCT,
                bindings: vec![(FieldId(0), var(3)), (FieldId(1), var(1))],
            },
        ],
        negated: vec![],
        predicates: vec![],
    })
}

/// o5 — per-store extremes: Min/Max over the whole fact table, 200
/// groups.
fn store_extremes() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Min,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Max,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: ids::SALE,
            bindings: vec![
                (FieldId(0), var(2)),
                (FieldId(2), var(0)),
                (FieldId(6), var(1)),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    })
}

/// o6 — brand drill: selective dimension point + day range, summed.
fn brand_drill() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Sum,
            over: Some(VarId(0)),
        }],
        atoms: vec![
            Atom {
                relation: ids::SALE,
                bindings: vec![
                    (FieldId(0), var(1)),
                    (FieldId(1), var(2)),
                    (FieldId(3), var(3)),
                    (FieldId(5), var(0)),
                ],
            },
            Atom {
                relation: ids::PRODUCT,
                bindings: vec![(FieldId(0), var(3)), (FieldId(2), param(0))],
            },
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: var(2),
            rhs: param(1),
        })],
    })
}

fn brand_drill_params(seed: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 902, 6));
    let d = |x: u64| Value::I64(i64::try_from(x).expect("small"));
    vec![
        vec![Value::U64(rng.range(BRANDS)), d(DAYS - 90)],
        vec![Value::U64(rng.range(BRANDS)), d(0)],
        vec![Value::U64(rng.range(BRANDS)), d(DAYS * 2 / 3)],
        vec![Value::U64(BRANDS + 1_000_000), d(0)],
    ]
}

/// The scenario registration.
#[must_use]
pub fn scenario() -> Scenario {
    Scenario {
        name: "olap",
        about: "star-schema rollups: group-by aggregates over the fact table",
        schema,
        descriptor: || bumbledb::Theory::descriptor(Olap),
        rows: |seed| {
            vec![
                (
                    ids::STORE,
                    Box::new((0..STORES).map(move |i| row(seed, ids::STORE, i))),
                ),
                (
                    ids::PRODUCT,
                    Box::new((0..PRODUCTS).map(move |i| row(seed, ids::PRODUCT, i))),
                ),
                (
                    ids::CUSTOMER,
                    Box::new((0..CUSTOMERS).map(move |i| row(seed, ids::CUSTOMER, i))),
                ),
                (
                    ids::SALE,
                    Box::new((0..SALES).map(move |i| row(seed, ids::SALE, i))),
                ),
            ]
        },
        extra_indexes: &[
            "CREATE INDEX ix_sale_day ON \"Sale\"(\"day\")",
            "CREATE INDEX ix_product_brand ON \"Product\"(\"brand\")",
            "CREATE INDEX ix_product_category ON \"Product\"(\"category\")",
            "CREATE INDEX ix_customer_segment ON \"Customer\"(\"segment\")",
            "CREATE INDEX ix_store_region ON \"Store\"(\"region\")",
        ],
        queries: || {
            vec![
                ScenarioQuery {
                    name: "o1_revenue_by_region",
                    query: revenue_by_region,
                    params: |_| vec![vec![]],
                    about: "full-fact Sum through one dimension, 6 groups",
                },
                ScenarioQuery {
                    name: "o2_category_window",
                    query: category_window,
                    params: day_windows,
                    about: "Sum+Count by category inside day windows",
                },
                ScenarioQuery {
                    name: "o3_promo_split",
                    query: promo_split,
                    params: |_| vec![vec![]],
                    about: "bool group key, full-scan fold",
                },
                ScenarioQuery {
                    name: "o4_segment_category",
                    query: segment_category,
                    params: |_| vec![vec![]],
                    about: "two-dimension rollup, 64 groups, 3-way join",
                },
                ScenarioQuery {
                    name: "o5_store_extremes",
                    query: store_extremes,
                    params: |_| vec![vec![]],
                    about: "Min+Max per store, 200 groups",
                },
                ScenarioQuery {
                    name: "o6_brand_drill",
                    query: brand_drill,
                    params: brand_drill_params,
                    about: "selective brand point + day range, one Sum",
                },
            ]
        },
    }
}
