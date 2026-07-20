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

use super::{Scenario, ScenarioQuery, Surface, Twin, mix};
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

/// The smoke corpus (tests only): the same generators over tiny counts,
/// so the tier-0 keyed-get gate exercises exactly what the night times.
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

/// One doc row with its bucket drawn from `buckets` — the full corpus
/// and the smoke twin share the generator, differing only in counts.
fn doc_row_sized(seed: u64, i: u64, buckets: u64) -> Vec<Value> {
    let mut rng = Rng::new(mix(seed, ids::DOC.0, i));
    let mut payload = Vec::with_capacity(32);
    for _ in 0..4 {
        payload.extend_from_slice(&rng.u64().to_le_bytes());
    }
    vec![
        Value::U64(i),
        Value::String(format!("doc/{i:08x}").into_bytes().into()),
        Value::U64(rng.range(buckets)),
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

/// p5 — the keyed GET surface (0.5.0's flagship): the typed point read
/// through the declared key law `Doc(key) -> Doc`, via the dynamic entry
/// the TS SDK bridge calls (`Snapshot::get_dyn` — the scenario stores
/// are `Db<SchemaDescriptor>`, so the dynamic surface is the reachable
/// twin of the macro-typed `snap.get(key)`). No query, no plan, no
/// prepared object: determinant encode → index probe → full-fact decode.
/// The statement resolves on the validated schema by relation +
/// projection — materialized order is a fact of validation, never a
/// literal in this table.
///
/// # Panics
///
/// Never: the `Doc(key) -> Doc` law is declared above.
fn doc_key_statement(schema: &bumbledb::Schema) -> bumbledb::StatementId {
    schema
        .keys()
        .iter()
        .find(|statement| statement.relation == ids::DOC && *statement.projection == [FieldId(1)])
        .expect("the Doc(key) -> Doc law is declared")
        .id
}

/// p5's params: p2's shape — 3 real keys + 1 miss — under its own draw
/// salt, so the two lanes never share a rotation by accident.
fn keyed_get_params(seed: u64) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(mix(seed, 903, 5));
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

/// The smoke twin (tests only): identical schema, the keyed-get lane's
/// registration over the tiny corpus with hit keys drawn from it — the
/// oracle-gate entry for the p5 unit smoke (zero timing).
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
        let key = |i: u64| Value::String(format!("doc/{i:08x}").into_bytes().into());
        vec![
            vec![key(rng.range(DOCS_SMOKE))],
            vec![key(rng.range(DOCS_SMOKE))],
            vec![key(rng.range(DOCS_SMOKE))],
            vec![Value::String(b"doc/never-a-key".to_vec().into())],
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

    /// The p5 smoke gate: tiny corpus, zero timing — the FULL uncapped
    /// multiset oracle (3 hits + the miss) through the exact gate seam
    /// the night run times (`gate_scenario` → the keyed-get arm).
    #[test]
    fn keyed_get_smoke_gate_agrees() {
        let dir = std::env::temp_dir().join("bumbledb-points-keyed-get-smoke");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        crate::scenarios::gate_scenario(&dir, &scenario_smoke(), 7)
            .expect("p5 agrees with SQLite at smoke scale");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The surface pinned directly against the corpus truth: a known key
    /// returns exactly its generated row (every field, declaration
    /// order); a never-interned key answers `None`.
    #[test]
    fn keyed_get_returns_the_exact_fact() {
        let dir = std::env::temp_dir().join("bumbledb-points-keyed-get-exact");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, bumbledb::Theory::descriptor(Points)).expect("create");
        let seed = 7;
        db.bulk_load_dyn(ids::BUCKET, (0..BUCKETS_SMOKE).map(bucket_row))
            .expect("buckets");
        db.bulk_load_dyn(
            ids::DOC,
            (0..DOCS_SMOKE).map(|i| doc_row_sized(seed, i, BUCKETS_SMOKE)),
        )
        .expect("docs");
        let statement = doc_key_statement(schema());
        for i in [0u64, 3, DOCS_SMOKE - 1] {
            let key = Value::String(format!("doc/{i:08x}").into_bytes().into());
            let fact = db
                .read(|snap| snap.get_dyn(ids::DOC, statement, std::slice::from_ref(&key)))
                .expect("get_dyn")
                .expect("a loaded key is a hit");
            assert_eq!(fact, doc_row_sized(seed, i, BUCKETS_SMOKE));
        }
        let miss = Value::String(b"doc/never-a-key".to_vec().into());
        let absent = db
            .read(|snap| snap.get_dyn(ids::DOC, statement, std::slice::from_ref(&miss)))
            .expect("get_dyn");
        assert!(absent.is_none(), "a never-interned key proves the miss");
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
