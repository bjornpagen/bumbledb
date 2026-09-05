use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    Atom, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, ParamId, Query, Rule, Term, Value,
    VarId,
};

use super::{Scenario, ScenarioQuery, Surface, Twin, mix};
use crate::corpus_gen::Rng;
use crate::fixture::var;

bumbledb::schema! {
    pub Points;

    relation Bucket {
        id: u64 as PBucketId,
        class: u64 as PClassId,
    }
    relation Doc {
        id: u64 as PDocId,
        key: str,
        bucket: u64 as PBucketId,
        size: i64,
        payload: bytes<32>,
    }

    closed relation Class as PClassId = { Hot, Warm, Cold, Frozen };

    // Declared id keys first (E-NO-RESERVE): the retired fresh auto-keys
    // are ordinary declared statements now, at the head so the later
    // declared statement ids keep their historical slots.
    Bucket(id) -> Bucket;
    Doc(id)    -> Doc;

    Bucket(class) <= Class(id);
    Doc(key) -> Doc;
    Doc(bucket) <= Bucket(id);
}

/// # Panics
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

#[cfg(test)]
const BUCKETS_SMOKE: u64 = 16;
#[cfg(test)]
const DOCS_SMOKE: u64 = 512;

fn bucket_row(i: u64) -> Vec<Value> {
    vec![Value::U64(i), Value::U64(i % 4)]
}

fn doc_row(seed: u64, i: u64) -> Vec<Value> {
    doc_row_sized(seed, i, BUCKETS)
}

fn doc_row_sized(seed: u64, i: u64, buckets: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(seed, ids::DOC.0, i));
    let mut payload = Vec::with_capacity(32);
    for _ in 0..4 {
        payload.extend_from_slice(&rng.u64().to_le_bytes());
    }
    vec![
        Value::U64(i),
        Value::String(format!("doc/{i:08x}").into()),
        Value::U64(rng.range(buckets)),
        Value::I64(i64::try_from(rng.range(1_000_000)).expect("small")),
        Value::FixedBytes(payload.into()),
    ]
}

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

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
    let key = |i: u64| Value::String(format!("doc/{i:08x}").into());
    vec![
        vec![key(rng.range(DOCS))],
        vec![key(rng.range(DOCS))],
        vec![key(rng.range(DOCS))],
        vec![Value::String("doc/never-a-key".into())],
    ]
}

/// # Panics
fn doc_key_statement(schema: &bumbledb::Schema) -> bumbledb::StatementId {
    schema
        .keys()
        .iter()
        .find(|statement| statement.relation == ids::DOC && *statement.projection == [FieldId(1)])
        .expect("the Doc(key) -> Doc law is declared")
        .id
}

fn keyed_get_params(seed: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 903, 5));
    let key = |i: u64| Value::String(format!("doc/{i:08x}").into());
    vec![
        vec![key(rng.range(DOCS))],
        vec![key(rng.range(DOCS))],
        vec![key(rng.range(DOCS))],
        vec![Value::String("doc/never-a-key".into())],
    ]
}

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
    vec![
        vec![Value::U64(0), Value::U64(64)],
        vec![Value::U64(1), Value::U64(64)],
        vec![Value::U64(2), Value::U64(256)],
        vec![Value::U64(3), Value::U64(0)],
    ]
}

fn size_band() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Count],
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
                    surface: Surface::Query(by_id),
                    params: |seed| id_params(seed, 1),
                    about: "fresh-id point: key probe vs B-tree descent",
                    twin: Twin::Canonical,
                    cap: None,
                },
                ScenarioQuery {
                    name: "p2_by_key",
                    surface: Surface::Query(by_key),
                    params: key_params,
                    about: "keyed string point: dictionary + determinant index",
                    twin: Twin::Canonical,
                    cap: None,
                },
                ScenarioQuery {
                    name: "p3_bucket_fetch",
                    surface: Surface::Query(bucket_fetch),
                    params: bucket_params,
                    about: "small fan-out through a dimension + id ceiling",
                    twin: Twin::Canonical,
                    cap: None,
                },
                ScenarioQuery {
                    name: "p4_size_band",
                    surface: Surface::Query(size_band),
                    params: size_band_params,
                    about: "secondary range folded to Count",
                    twin: Twin::Canonical,
                    cap: None,
                },
                ScenarioQuery {
                    name: "p5_keyed_get",
                    surface: Surface::KeyedGet {
                        relation: ids::DOC,
                        key: doc_key_statement,
                    },
                    params: keyed_get_params,
                    about: "keyed get (0.5.0): the point read through Doc(key) -> Doc — determinant probe, no query machinery",
                    twin: Twin::Canonical,
                    cap: None,
                },
            ]
        },
    }
}

#[cfg(test)]
fn scenario_smoke() -> Scenario {
    #[expect(
        clippy::type_complexity,
        reason = "the tuple shape directly represents parallel protocol streams"
    )]
    fn rows_smoke(seed: u64) -> Vec<(bumbledb::RelationId, Box<dyn Iterator<Item = Vec<Value>>>)> {
        vec![
            (ids::BUCKET, Box::new((0..BUCKETS_SMOKE).map(bucket_row))),
            (
                ids::DOC,
                Box::new((0..DOCS_SMOKE).map(move |i| doc_row_sized(seed, i, BUCKETS_SMOKE))),
            ),
        ]
    }
    fn keyed_get_params_smoke(seed: u64) -> Vec<Vec<Value>> {
        let mut rng = Rng::new(mix(seed, 903, 5));
        let key = |i: u64| Value::String(format!("doc/{i:08x}").into());
        vec![
            vec![key(rng.range(DOCS_SMOKE))],
            vec![key(rng.range(DOCS_SMOKE))],
            vec![key(rng.range(DOCS_SMOKE))],
            vec![Value::String("doc/never-a-key".into())],
        ]
    }
    Scenario {
        name: "points",
        about: "keyed-get smoke twin",
        schema,
        descriptor: || bumbledb::Theory::descriptor(Points),
        rows: rows_smoke,
        extra_indexes: &[],
        queries: || {
            vec![ScenarioQuery {
                name: "p5_keyed_get",
                surface: Surface::KeyedGet {
                    relation: ids::DOC,
                    key: doc_key_statement,
                },
                params: keyed_get_params_smoke,
                about: "keyed get (0.5.0), smoke scale",
                twin: Twin::Canonical,
                cap: None,
            }]
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_get_smoke_gate_agrees() {
        let dir = std::env::temp_dir().join("bumbledb-points-keyed-get-smoke");
        let _ = std::fs::remove_dir_all(&dir);
        crate::scenarios::gate_scenario(&dir, &scenario_smoke(), 7)
            .expect("p5 agrees with SQLite at smoke scale");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keyed_get_returns_the_exact_fact() {
        let dir = std::env::temp_dir().join("bumbledb-points-keyed-get-exact");
        let _ = std::fs::remove_dir_all(&dir);
        let db = bumbledb::Db::create(&dir, bumbledb::Theory::descriptor(Points))
            .expect("create")
            .expect("accepted");
        let seed = 7;
        db.write(|tx| {
            tx.insert_dyn(ids::BUCKET, (0..BUCKETS_SMOKE).map(bucket_row))
                .map(bumbledb::MutationReport::changed)
        })
        .expect("buckets")
        .unwrap();
        db.write(|tx| {
            tx.insert_dyn(
                ids::DOC,
                (0..DOCS_SMOKE).map(|i| doc_row_sized(seed, i, BUCKETS_SMOKE)),
            )
            .map(bumbledb::MutationReport::changed)
        })
        .expect("docs")
        .unwrap();
        let statement = doc_key_statement(schema());
        for i in [0u64, 3, DOCS_SMOKE - 1] {
            let key = Value::String(format!("doc/{i:08x}").into());
            let fact = db
                .read(|snap| snap.get_dyn(ids::DOC, statement, std::slice::from_ref(&key)))
                .expect("get_dyn")
                .expect("a loaded key is a hit");
            assert_eq!(fact, doc_row_sized(seed, i, BUCKETS_SMOKE));
        }
        let miss = Value::String("doc/never-a-key".into());
        let absent = db
            .read(|snap| snap.get_dyn(ids::DOC, statement, std::slice::from_ref(&miss)))
            .expect("get_dyn");
        assert!(absent.is_none(), "a never-interned key proves the miss");
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
