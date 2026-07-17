//! The point-lookup scenario: a key-value-shaped store where every
//! query touches a handful of rows. This is `SQLite`'s home turf — one
//! B-tree descent per point — and the regime where bumbledb’s determinant index
//! probe and per-execution overhead (snapshot open, bind, memo check)
//! either hold up or drown the win. String keys stress the interning
//! dictionary on every lookup.

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    AggOp, Atom, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, ParamId, Query, Rule, Term,
    Value, VarId,
};

use super::{Scenario, ScenarioQuery, mix};
use crate::corpus_gen::Rng;
use crate::fixture::var;

bumbledb::schema! {
    pub Points;

    relation Bucket {
        id: u64 as PBucketId, fresh,
        class: u64 as PClassId,
    }
    relation Doc {
        id: u64 as PDocId, fresh,
        key: str,
        bucket: u64 as PBucketId,
        size: i64,
        payload: bytes<32>,
    }

    closed relation Class as PClassId = { Hot, Warm, Cold, Frozen };

    Bucket(class) <= Class(id);
    Doc(key) -> Doc;
    Doc(bucket) <= Bucket(id);
}

/// Relation ids by declaration order.
/// The validated scenario schema, memoized for the inspection surfaces
/// (DDL rendering, typing); the store is created from [`Points`]'s
/// descriptor (`scenarios::load`).
///
/// # Panics
///
/// Never in practice: the declared scenario schema is valid.
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        Points
            .descriptor()
            .validate()
            .expect("the scenario schema is valid")
    })
}

pub mod ids {
    use bumbledb::RelationId;
    pub const BUCKET: RelationId = RelationId(0);
    pub const DOC: RelationId = RelationId(1);
    pub const CLASS: RelationId = RelationId(2);
}

pub const BUCKETS: u64 = 4_096;
pub const DOCS: u64 = 300_000;

fn bucket_row(i: u64) -> Vec<Value> {
    vec![Value::U64(i), Value::U64(i % 4)]
}

fn doc_row(seed: u64, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(seed, ids::DOC.0, i));
    let mut payload = Vec::with_capacity(32);
    for _ in 0..4 {
        payload.extend_from_slice(&rng.u64().to_le_bytes());
    }
    vec![
        Value::U64(i),
        Value::String(format!("doc/{i:08x}").into_bytes().into()),
        Value::U64(rng.range(BUCKETS)),
        Value::I64(i64::try_from(rng.range(1_000_000)).expect("small")),
        // Identity-shaped: a random 32-byte payload digest, inline.
        Value::FixedBytes(payload.into()),
    ]
}

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

/// p1 — point by fresh id (the key probe vs one B-tree descent).
fn by_id() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::DOC),
            bindings: vec![
                (FieldId(0), param(0)),
                (FieldId(3), var(0)),
                (FieldId(2), var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn id_params(seed: u64, salt: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 903, salt));
    vec![
        vec![Value::U64(rng.range(DOCS))],
        vec![Value::U64(rng.range(DOCS))],
        vec![Value::U64(rng.range(DOCS))],
        vec![Value::U64(DOCS + 1_000_000)],
    ]
}

/// p2 — point by string key (interning on every execution).
fn by_key() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::DOC),
            bindings: vec![
                (FieldId(1), param(0)),
                (FieldId(0), var(0)),
                (FieldId(3), var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn key_params(seed: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 903, 2));
    let key = |i: u64| Value::String(format!("doc/{i:08x}").into_bytes().into());
    vec![
        vec![key(rng.range(DOCS))],
        vec![key(rng.range(DOCS))],
        vec![key(rng.range(DOCS))],
        vec![Value::String(b"doc/never-a-key".to_vec().into())],
    ]
}

/// p3 — bucket fetch through the containment edge: ~73 docs per bucket.
fn bucket_fetch() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(ids::DOC),
                bindings: vec![(FieldId(2), var(1)), (FieldId(0), var(0))],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(ids::BUCKET),
                bindings: vec![(FieldId(0), var(1)), (FieldId(1), param(0))],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: var(1),
            rhs: param(1),
        })],
    })
}

fn bucket_params(_: u64) -> Vec<Vec<Value>> {
    // Class enum + a bucket-id ceiling: small slices of the dimension.
    vec![
        vec![Value::U64(0), Value::U64(64)],
        vec![Value::U64(1), Value::U64(64)],
        vec![Value::U64(2), Value::U64(256)],
        vec![Value::U64(3), Value::U64(0)],
    ]
}

/// p4 — size-band count: secondary-range aggregation, no join.
fn size_band() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(ids::DOC),
            bindings: vec![(FieldId(0), var(0)), (FieldId(3), var(1))],
        }],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(1),
                rhs: param(0),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: var(1),
                rhs: param(1),
            }),
        ],
    })
}

fn size_band_params(_: u64) -> Vec<Vec<Value>> {
    vec![
        vec![Value::I64(0), Value::I64(10_000)],
        vec![Value::I64(500_000), Value::I64(510_000)],
        vec![Value::I64(0), Value::I64(1_000_000)],
        vec![Value::I64(999_999), Value::I64(999_999)],
    ]
}

/// The scenario registration.
#[must_use]
pub fn scenario() -> Scenario {
    Scenario {
        name: "points",
        about: "key-value regime: point lookups, tiny fetches, per-query overhead",
        schema,
        descriptor: || bumbledb::Theory::descriptor(Points),
        rows: |seed| {
            vec![
                (ids::BUCKET, Box::new((0..BUCKETS).map(bucket_row))),
                (ids::DOC, Box::new((0..DOCS).map(move |i| doc_row(seed, i)))),
            ]
        },
        extra_indexes: &[
            "CREATE INDEX ix_doc_size ON \"Doc\"(\"size\")",
            "CREATE INDEX ix_bucket_class ON \"Bucket\"(\"class\")",
        ],
        queries: || {
            vec![
                ScenarioQuery {
                    name: "p1_by_id",
                    query: by_id,
                    params: |seed| id_params(seed, 1),
                    about: "fresh-id point: key probe vs B-tree descent",
                },
                ScenarioQuery {
                    name: "p2_by_key",
                    query: by_key,
                    params: key_params,
                    about: "keyed string point: dictionary + determinant index",
                },
                ScenarioQuery {
                    name: "p3_bucket_fetch",
                    query: bucket_fetch,
                    params: bucket_params,
                    about: "small fan-out through a dimension + id ceiling",
                },
                ScenarioQuery {
                    name: "p4_size_band",
                    query: size_band,
                    params: size_band_params,
                    about: "secondary range folded to Count",
                },
            ]
        },
    }
}
