//! The dual-run chase differential (`docs/architecture/40-execution.md`
//! § the chase; the naive model is the semantics oracle): each
//! eliminable fixture runs through the engine twice — rewrite on and
//! off, via the engine's `chase-off` test-support switch — and three-way
//! compares with the model, under the projection sink and the aggregate
//! sink. The profile surface proves the runs are not vacuously equal:
//! the chase-on plan carries exactly one `Role::Eliminated` mark naming
//! the fixture's fallen relation; the chase-off plan carries none.

use std::path::{Path, PathBuf};

use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};
use bumbledb::{
    with_chase_disabled, AggOp, Atom, Db, FindTerm, Query, RelationId, Rule, Term, Value, VarId,
};

use crate::differential::{engine_query, Rows};
use crate::naive::{Delta, NaiveDb};

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn fresh(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
    }
}

fn side(relation: u32, projection: u16, selection: &[(u16, Value)]) -> Side {
    Side {
        relation: RelationId(relation),
        projection: Box::new([FieldId(projection)]),
        selection: selection
            .iter()
            .map(|(f, v)| (FieldId(*f), v.clone()))
            .collect(),
    }
}

fn atom(relation: u32, bindings: &[(u16, Term)]) -> Atom {
    Atom {
        relation: RelationId(relation),
        bindings: bindings
            .iter()
            .map(|(f, t)| (FieldId(*f), t.clone()))
            .collect(),
    }
}

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bumbledb-chase-{tag}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create test dir");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One store pair over a fixture: the engine store and the model,
/// loaded with the same single-commit delta (containment clusters
/// insert together — judged on the final state, both sides).
fn stores(
    dir: &Path,
    descriptor: &SchemaDescriptor,
    inserts: Vec<(RelationId, Vec<Value>)>,
) -> (Db<SchemaDescriptor>, NaiveDb) {
    let db = Db::create(dir, descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(descriptor);
    let delta = Delta {
        deletes: vec![],
        inserts,
    };
    naive.apply(&delta).expect("the fixture data commits");
    db.write(|tx| {
        for (rel, fact) in &delta.inserts {
            tx.insert_dyn(*rel, fact)?;
        }
        Ok(())
    })
    .expect("the fixture data commits");
    (db, naive)
}

/// The eliminated occurrences of the query's prepared plan, through the
/// public profile surface (ANALYZE executes; the empty param list is
/// every fixture query's).
fn eliminated(db: &Db<SchemaDescriptor>, query: &Query) -> Vec<bumbledb::EliminatedOccurrence> {
    let mut prepared = db.prepare(query).expect("fixture queries validate");
    let (_, mut stats) = db
        .read(|snap| snap.profile(&mut prepared, &[]))
        .expect("profile executes");
    stats.rules.swap_remove(0).eliminated
}

/// The dual run: chase-on, chase-off, and the model must produce one
/// result set — with the marks asserted so neither equality is vacuous
/// (on eliminates exactly `fallen`; off eliminates nothing).
fn three_way(db: &Db<SchemaDescriptor>, naive: &NaiveDb, query: &Query, fallen: &str) {
    let on = engine_query(db, query, &[]);
    let off = with_chase_disabled(|| engine_query(db, query, &[]));
    let model = Rows::Ok(naive.query(query, &[]).expect("the model executes"));
    assert_eq!(on, off, "chase-on and chase-off disagree ({fallen})");
    assert_eq!(on, model, "engine and model disagree ({fallen})");
    let Rows::Ok(rows) = &on else {
        unreachable!("fixture queries never overflow")
    };
    assert!(!rows.is_empty(), "the fixture produces rows ({fallen})");
    let marks = eliminated(db, query);
    assert_eq!(marks.len(), 1, "one mark expected ({fallen})");
    assert_eq!(marks[0].relation, fallen, "the wrong side fell");
    assert!(
        with_chase_disabled(|| eliminated(db, query)).is_empty(),
        "the off switch keeps every occurrence joining ({fallen})"
    );
}

/// Posting(id fresh, account u64, amount i64); Account(id fresh,
/// holder u64); Posting(account) <= Account(id) — statement 2 after the
/// two fresh auto-keys.
fn walk_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Posting".into(),
                fields: vec![
                    fresh("id"),
                    field("account", ValueType::U64),
                    field("amount", ValueType::I64),
                ],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![fresh("id"), field("holder", ValueType::U64)],
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: side(0, 1, &[]),
            target: side(1, 0, &[]),
        }],
    }
}

/// Accounts 1..=3 and postings with duplicate (account, amount) pairs —
/// the aggregate fold must count distinct posting bindings either way.
fn walk_inserts() -> Vec<(RelationId, Vec<Value>)> {
    let mut inserts: Vec<(RelationId, Vec<Value>)> = (1u64..=3)
        .map(|id| (RelationId(1), vec![Value::U64(id), Value::U64(id * 7)]))
        .collect();
    for (id, account, amount) in [
        (1u64, 1u64, 10i64),
        (2, 1, 10),
        (3, 1, -5),
        (4, 2, 40),
        (5, 2, 25),
        (6, 3, 7),
    ] {
        inserts.push((
            RelationId(0),
            vec![Value::U64(id), Value::U64(account), Value::I64(amount)],
        ));
    }
    inserts
}

/// The existence walk through both sinks: `Q(pid, m) :- Posting(id =
/// pid, account = x, amount = m), Account(id = x)` and the per-account
/// `Sum(m)` — the Account occurrence falls, results identical three
/// ways.
#[test]
fn the_existence_walk_agrees_three_ways_on_both_sinks() {
    let dir = TempDir::new("walk");
    let descriptor = walk_descriptor();
    let (db, naive) = stores(dir.path(), &descriptor, walk_inserts());
    let atoms = vec![
        atom(0, &[(0, var(0)), (1, var(1)), (2, var(2))]),
        atom(1, &[(0, var(1))]),
    ];
    let projection = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: atoms.clone(),
        negated: vec![],
        predicates: vec![],
    });
    let aggregate = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(1)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(2)),
            },
        ],
        atoms,
        negated: vec![],
        predicates: vec![],
    });
    three_way(&db, &naive, &projection, "Account");
    three_way(&db, &naive, &aggregate, "Account");
}

/// Grading(id fresh, kind enum{Det, Custom}); Det(grading u64, rate
/// i64) with the declared key Det(grading) -> Det and the pair
/// `Grading(id | kind == Det) == Det(grading)` as its two containments
/// — statements 1, 2, 3 after Grading's auto-key.
fn du_descriptor() -> SchemaDescriptor {
    let kind = ValueType::Enum {
        variants: ["Det", "Custom"].iter().map(|v| Box::from(*v)).collect(),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Grading".into(),
                fields: vec![fresh("id"), field("kind", kind)],
            },
            RelationDescriptor {
                name: "Det".into(),
                fields: vec![
                    field("grading", ValueType::U64),
                    field("rate", ValueType::I64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(0, 0, &[(1, Value::Enum(0))]),
                target: side(1, 0, &[]),
            },
            StatementDescriptor::Containment {
                source: side(1, 0, &[]),
                target: side(0, 0, &[(1, Value::Enum(0))]),
            },
        ],
    }
}

/// Two Det gradings (with their arm rows) and one Custom.
fn du_inserts() -> Vec<(RelationId, Vec<Value>)> {
    vec![
        (RelationId(0), vec![Value::U64(1), Value::Enum(0)]),
        (RelationId(0), vec![Value::U64(2), Value::Enum(0)]),
        (RelationId(0), vec![Value::U64(3), Value::Enum(1)]),
        (RelationId(1), vec![Value::U64(1), Value::I64(25)]),
        (RelationId(1), vec![Value::U64(2), Value::I64(40)]),
    ]
}

fn du_atoms() -> (Atom, Atom) {
    (
        atom(0, &[(0, var(0)), (1, Term::Literal(Value::Enum(0)))]),
        atom(1, &[(0, var(0)), (1, var(1))]),
    )
}

/// The DU one-sided walk, header direction, both sinks: `Q(g, rate) :-
/// Det(grading = g, rate), Grading(id = g, kind == Det)` and the global
/// `Sum(rate)` — the header falls.
#[test]
fn the_du_header_direction_agrees_three_ways_on_both_sinks() {
    let dir = TempDir::new("du-header");
    let descriptor = du_descriptor();
    let (db, naive) = stores(dir.path(), &descriptor, du_inserts());
    let (header, child) = du_atoms();
    let atoms = vec![child, header];
    let projection = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: atoms.clone(),
        negated: vec![],
        predicates: vec![],
    });
    let aggregate = Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Sum,
            over: Some(VarId(1)),
        }],
        atoms,
        negated: vec![],
        predicates: vec![],
    });
    three_way(&db, &naive, &projection, "Grading");
    three_way(&db, &naive, &aggregate, "Grading");
}

/// The DU one-sided walk, child direction, both sinks: `Q(g) :-
/// Grading(id = g, kind == Det), Det(grading = g)` and the grouped
/// count — the child falls (its `rate` stays unread; the statement scan
/// order fells the child before the header's turn, and support
/// acyclicity keeps the header standing).
#[test]
fn the_du_child_direction_agrees_three_ways_on_both_sinks() {
    let dir = TempDir::new("du-child");
    let descriptor = du_descriptor();
    let (db, naive) = stores(dir.path(), &descriptor, du_inserts());
    let (header, child) = du_atoms();
    let atoms = vec![header, child];
    let projection = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: atoms.clone(),
        negated: vec![],
        predicates: vec![],
    });
    let aggregate = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms,
        negated: vec![],
        predicates: vec![],
    });
    three_way(&db, &naive, &projection, "Det");
    three_way(&db, &naive, &aggregate, "Det");
}

/// The missing-φ near-miss refuses on the real pipeline, and the
/// unrewritten plan still agrees with the model — the refusal's own
/// differential. `Q(g, k) :- Grading(id = g, kind = k), Det(grading =
/// g)`: the header's `kind` is a projected variable, not the literal φ,
/// so the child may not fall (its certificate needs σφ membership) and
/// the header may not either (it produces output).
#[test]
fn the_missing_phi_near_miss_refuses_and_still_agrees() {
    let dir = TempDir::new("du-missing-phi");
    let descriptor = du_descriptor();
    let (db, naive) = stores(dir.path(), &descriptor, du_inserts());
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: vec![
            atom(0, &[(0, var(0)), (1, var(2))]),
            atom(1, &[(0, var(0)), (1, var(1))]),
        ],
        negated: vec![],
        predicates: vec![],
    });
    assert!(
        eliminated(&db, &query).is_empty(),
        "without φ the chase must refuse"
    );
    let engine = engine_query(&db, &query, &[]);
    let model = Rows::Ok(naive.query(&query, &[]).expect("the model executes"));
    assert_eq!(engine, model, "engine and model disagree on the near-miss");
}
