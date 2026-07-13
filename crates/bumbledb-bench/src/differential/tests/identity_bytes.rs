//! The identity-bytes differential criteria (10-data-model, the
//! `bytes<N>` cut): round-trip, guard-key FD enforcement, and
//! containment over a **bytes<32> key** — engine vs the naive model —
//! plus `CountDistinct` and group-by over bytes<N> at widths
//! 8/16/32/64. The corpus digests are adversarial by construction:
//! shared prefixes (whole zero words), single-byte deltas between
//! neighbors, the all-zeros digest, and the pad-boundary widths 7/9/63
//! stored alongside for round-trip.

use bumbledb::schema::{
    FieldId, RelationDescriptor, SchemaDescriptor, Side, StatementDescriptor, ValueType,
};
use bumbledb::{
    AggOp, Atom, CmpOp, Comparison, Db, FindTerm, PredicateTree, Query, RelationId, Rule, Term,
    Value, VarId,
};

use crate::differential::{run, Op};
use crate::fixture::{field, var, TempDir};
use crate::naive::{Delta, NaiveDb};

const BLOB: RelationId = RelationId(0);
const REF: RelationId = RelationId(1);

/// Blob(hash bytes<32> KEY, d8, d16, d64, d7, d9, d63, weight u64);
/// Ref(hash bytes<32>) <= Blob(hash). Materialized order:
/// 0 Blob(hash) -> Blob, 1 Ref(hash) <= Blob(hash).
fn schema() -> SchemaDescriptor {
    let digest = |name: &str, len: u16| field(name, ValueType::FixedBytes { len });
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Blob".into(),
                fields: vec![
                    digest("hash", 32),
                    digest("d8", 8),
                    digest("d16", 16),
                    digest("d64", 64),
                    digest("d7", 7),
                    digest("d9", 9),
                    digest("d63", 63),
                    field("weight", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Ref".into(),
                fields: vec![digest("hash", 32)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: BLOB,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: Side {
                    relation: REF,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: BLOB,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
        ],
    }
}

/// An adversarial width-`len` digest: all-zero but for the trailing
/// bytes of `k` — every digest shares the maximal prefix, neighbors are
/// single-byte deltas, `k = 0` is all-zeros.
fn digest(len: usize, k: u64) -> Value {
    let mut raw = vec![0u8; len];
    let tail = k.to_be_bytes();
    let n = len.min(8);
    raw[len - n..].copy_from_slice(&tail[8 - n..]);
    Value::FixedBytes(raw.into())
}

/// One Blob row keyed by digest(32, k): the width-W columns draw from
/// vocabularies of W-dependent size so distinct counts and group keys
/// fold real duplicates.
fn blob(k: u64) -> Vec<Value> {
    vec![
        digest(32, k),
        digest(8, k % 3),
        digest(16, k % 5),
        digest(64, k % 7),
        digest(7, k % 2),
        digest(9, k % 4),
        digest(63, k % 6),
        Value::U64(k * 10),
    ]
}

/// splitmix64, local like every differential stream's rng.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The write ops: consistent pairs (commit), duplicate bytes<32> keys
/// (guard aborts — the same guard bytes under a second fact), lone Refs
/// (source-side containment aborts), key deletes stranding a Ref
/// (target-side aborts), and pair demolitions (commit). The generator
/// keeps a naive mirror so deletes name real facts.
fn write_ops(rng: &mut Rng) -> Vec<Delta> {
    let mut mirror = NaiveDb::new(&schema());
    let mut deltas = Vec::new();
    for _ in 0..160 {
        let delta = match rng.below(10) {
            // A keyed blob alone: commits unless the hash guard is taken.
            0..=3 => {
                let k = rng.below(24);
                Delta {
                    deletes: vec![],
                    inserts: vec![(BLOB, blob(k))],
                }
            }
            // A blob with a second blob under the SAME bytes<32> key but
            // different payload: the guard put-conflict — both oracles
            // must abort on statement 0.
            4 => {
                let k = rng.below(24);
                let mut other = blob(k);
                other[7] = Value::U64(9_999);
                Delta {
                    deletes: vec![],
                    inserts: vec![(BLOB, blob(k)), (BLOB, other)],
                }
            }
            // A Ref with its Blob: commits.
            5 | 6 => {
                let k = rng.below(24);
                Delta {
                    deletes: vec![],
                    inserts: vec![(BLOB, blob(k)), (REF, vec![digest(32, k)])],
                }
            }
            // A lone Ref: aborts source-side unless its Blob stands.
            7 => Delta {
                deletes: vec![],
                inserts: vec![(REF, vec![digest(32, rng.below(24))])],
            },
            // Delete a Blob: aborts target-side when a Ref survives.
            8 => {
                let k = rng.below(24);
                Delta {
                    deletes: vec![(BLOB, blob(k))],
                    inserts: vec![],
                }
            }
            // Demolish a pair: commits when both stand.
            _ => {
                let k = rng.below(24);
                Delta {
                    deletes: vec![(REF, vec![digest(32, k)]), (BLOB, blob(k))],
                    inserts: vec![],
                }
            }
        };
        let _ = mirror.apply(&delta);
        deltas.push(delta);
    }
    deltas
}

fn blob_atom(bindings: Vec<(u16, Term)>) -> Atom {
    Atom {
        relation: BLOB,
        bindings: bindings
            .into_iter()
            .map(|(field, term)| (FieldId(field), term))
            .collect(),
    }
}

fn plain(finds: Vec<FindTerm>, atoms: Vec<Atom>, predicates: Vec<PredicateTree>) -> Query {
    Query::single(Rule {
        finds,
        atoms,
        negated: vec![],
        predicates,
    })
}

/// The query block, replayed after every writes-prefix: round-trip
/// projections of every width (7/8/9/16/32/63/64 — the pad boundaries),
/// bytes<32> Eq hits and adversarial misses, a membership set, a
/// bytes<32> join (Ref ⋈ Blob on hash), and the criteria pair —
/// group-by over bytes<N> and `CountDistinct` at widths 8/16/32/64.
#[allow(clippy::too_many_lines)] // the criteria block, one query per line item
fn queries() -> Vec<Op> {
    let mut ops = Vec::new();
    // Round-trip: project the whole row back (all seven digest widths).
    ops.push(Op::Query {
        query: plain(
            (0..8).map(|v| FindTerm::Var(VarId(v))).collect(),
            vec![blob_atom((0..8u16).map(|f| (f, var(f))).collect())],
            vec![],
        ),
        params: vec![],
    });
    // bytes<32> Eq: a hit (digest 3), the all-zeros digest, and an
    // adversarial single-byte-delta miss.
    for key in [digest(32, 3), digest(32, 0), {
        let Value::FixedBytes(mut raw) = digest(32, 3) else {
            unreachable!()
        };
        raw[0] = 0xA5;
        Value::FixedBytes(raw)
    }] {
        ops.push(Op::Query {
            query: plain(
                vec![FindTerm::Var(VarId(0))],
                vec![blob_atom(vec![(0, Term::Literal(key)), (7, var(0))])],
                vec![],
            ),
            params: vec![],
        });
    }
    // Ne over a width-16 digest, and a membership set of bytes<16>.
    ops.push(Op::Query {
        query: plain(
            vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            vec![blob_atom(vec![(2, var(0)), (7, var(1))])],
            vec![PredicateTree::Leaf(Comparison {
                op: CmpOp::Ne,
                lhs: var(0),
                rhs: Term::Literal(digest(16, 1)),
            })],
        ),
        params: vec![],
    });
    ops.push(Op::Query {
        query: plain(
            vec![FindTerm::Var(VarId(0))],
            vec![blob_atom(vec![(2, var(0)), (7, var(1))])],
            vec![PredicateTree::Leaf(Comparison {
                op: CmpOp::Eq,
                lhs: var(0),
                rhs: Term::ParamSet(bumbledb::ParamId(0)),
            })],
        ),
        params: vec![crate::naive::query::ParamValue::Set(vec![
            digest(16, 0),
            digest(16, 2),
            digest(16, 4),
        ])],
    });
    // The bytes<32> join: Ref(hash = h), Blob(hash = h, weight = w).
    ops.push(Op::Query {
        query: plain(
            vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            vec![
                Atom {
                    relation: REF,
                    bindings: vec![(FieldId(0), var(0))],
                },
                blob_atom(vec![(0, var(0)), (7, var(1))]),
            ],
            vec![],
        ),
        params: vec![],
    });
    // The criteria pair, per width 8/16/32/64: group-by the digest
    // (finds: [digest, Count]) and CountDistinct over it (global).
    for field in [1u16, 2, 0, 3] {
        ops.push(Op::Query {
            query: plain(
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Count,
                        over: None,
                    },
                ],
                vec![blob_atom(vec![(field, var(0)), (7, var(1))])],
                vec![],
            ),
            params: vec![],
        });
        ops.push(Op::Query {
            query: plain(
                vec![FindTerm::Aggregate {
                    op: AggOp::CountDistinct,
                    over: Some(VarId(0)),
                }],
                vec![blob_atom(vec![(field, var(0)), (7, var(1))])],
                vec![],
            ),
            params: vec![],
        });
        // And grouped CountDistinct: distinct digests per weight-parity
        // bucket... the schema has no parity column, so group by d8
        // instead when counting a different width — cross-width group
        // keys exercise multi-word group keys beside multi-word inputs.
        if field != 1 {
            ops.push(Op::Query {
                query: plain(
                    vec![
                        FindTerm::Var(VarId(2)),
                        FindTerm::Aggregate {
                            op: AggOp::CountDistinct,
                            over: Some(VarId(0)),
                        },
                    ],
                    vec![blob_atom(vec![(field, var(0)), (1, var(2)), (7, var(1))])],
                    vec![],
                ),
                params: vec![],
            });
        }
    }
    ops
}

/// The seeded stream: writes interleaved with the query block, engine vs
/// naive — every verdict (guard aborts on the bytes<32> key, both
/// containment directions) and every result set must agree.
#[test]
fn identity_bytes_agree_with_the_naive_model() {
    let dir = TempDir::new("differential");
    let descriptor = schema();
    let db = Db::create(dir.path(), descriptor).expect("create");
    let mut naive = NaiveDb::new(&schema());

    let mut rng = Rng(0x1D_B17E5);
    let writes = write_ops(&mut rng);
    let mut ops: Vec<Op> = Vec::new();
    for chunk in writes.chunks(40) {
        ops.extend(chunk.iter().cloned().map(Op::Write));
        ops.extend(queries());
    }

    let summary = match run(&db, &mut naive, &ops) {
        Ok(summary) => summary,
        Err(divergence) => panic!("divergence: {divergence:?}"),
    };
    // The stream genuinely exercised both verdicts and the queries.
    assert!(summary.commits >= 20, "commits: {}", summary.commits);
    assert!(summary.aborts >= 20, "aborts: {}", summary.aborts);
    assert!(summary.queries >= 60, "queries: {}", summary.queries);
}
