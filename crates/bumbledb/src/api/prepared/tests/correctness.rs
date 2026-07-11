use super::*;

/// u64 ordered comparisons and cross-atom
/// residuals — the generator's new constructs — each pinned against
/// an independent nested-loop reference, no `SQLite` in sight.
#[test]
fn u64_ranges_and_cross_atom_residuals_match_nested_loops() {
    let dir = TempDir::new("prepared-new-construct-differential");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 3, "a", 10),
        (2, 3, "b", 25),
        (3, 7, "c", 25),
        (4, 7, "d", 40),
        (5, 9, "e", -5),
        (6, 9, "f", 40),
    ];
    insert_postings(&env, &schema, rows);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Q(id) :- Posting(id, account = a), a >= 7 — a u64 ordered
    // comparison over the dense id domain.
    let range = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::U64(7)),
        })],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &range).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let mut got: Vec<u64> = (0..out.len())
        .map(|row| match out.get(row, 0) {
            ResultValue::U64(id) => id,
            other => panic!("column 0 is u64: {other:?}"),
        })
        .collect();
    got.sort_unstable();
    let mut expected: Vec<u64> = rows.iter().filter(|r| r.1 >= 7).map(|r| r.0).collect();
    expected.sort_unstable();
    assert_eq!(got, expected, "u64 ordered comparison");

    // Q(x, y) :- Posting(account = k, amount = x),
    //            Posting(account = k, amount = y), x < y — the
    // cross-atom residual, checked by nested loop.
    let spread = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            },
            Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(3), Term::Var(VarId(1))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Var(VarId(1)),
        })],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &spread).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let mut got: Vec<(i64, i64)> = (0..out.len())
        .map(|row| match (out.get(row, 0), out.get(row, 1)) {
            (ResultValue::I64(x), ResultValue::I64(y)) => (x, y),
            other => panic!("two i64 columns: {other:?}"),
        })
        .collect();
    got.sort_unstable();
    let mut expected = std::collections::BTreeSet::new();
    for p1 in rows {
        for p2 in rows {
            if p1.1 == p2.1 && p1.3 < p2.3 {
                expected.insert((p1.3, p2.3));
            }
        }
    }
    assert_eq!(
        got,
        expected.into_iter().collect::<Vec<_>>(),
        "cross-atom residual"
    );
}

/// An aggregate whose body has a node
/// binding only existential (non-projected, non-aggregated)
/// variables folds every distinct full binding — pinned against an
/// independent nested-loop reference. The plan's sink-relevance bits
/// mark every variable-binding node relevant under aggregation, so
/// no suffix skip can ever starve the fold.
#[test]
fn aggregates_fold_every_binding_of_existential_suffixes() {
    let dir = TempDir::new("prepared-agg-existential");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 7, "a", 10),
        (2, 7, "b", 10),
        (3, 7, "c", 20),
        (4, 8, "z", 5),
    ];
    insert_postings(&env, &schema, rows);

    // Q(x, Sum(y)) :- Posting(account = x, amount = y),
    //                 Posting(account = x, memo = m)
    // — m is existential; the self-join's second occurrence opens a
    // node binding only m.
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: crate::ir::AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![
            Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(3), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });

    // The nested-loop reference over distinct (x, y, m) bindings.
    let mut bindings = std::collections::BTreeSet::new();
    for p1 in rows {
        for p2 in rows {
            if p1.1 == p2.1 {
                bindings.insert((p1.1, p1.3, p2.2));
            }
        }
    }
    let mut expected = std::collections::BTreeMap::new();
    for (x, y, _) in &bindings {
        *expected.entry(*x).or_insert(0i64) += y;
    }

    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let mut got: Vec<(u64, i64)> = (0..out.len())
        .map(|row| {
            let ResultValue::U64(account) = out.get(row, 0) else {
                panic!("column 0 is u64");
            };
            let ResultValue::I64(sum) = out.get(row, 1) else {
                panic!("column 1 is i64");
            };
            (account, sum)
        })
        .collect();
    got.sort_unstable();
    assert_eq!(got, expected.into_iter().collect::<Vec<_>>());
}

/// Regression for the `Ne`-miss semantics
/// (docs/architecture/20-query-ir.md): a never-interned value under
/// `Ne` matches every stored row — the miss resolves to the sentinel
/// intern id, not to an empty result. The old blanket "miss ⇒ empty"
/// rule silently returned nothing here.
#[test]
fn ne_against_a_never_interned_string_matches_everything() {
    let dir = TempDir::new("prepared-ne-miss");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "rent", -1200), (2, 9, "food", -55)]);
    let cache = ImageCache::new(&schema);

    // Literal path: Q(amount) :- Posting(memo = m, amount), m != "ghost".
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Ne,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::String(Box::from(&b"ghost"[..]))),
        })],
    });
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 2, "no stored memo equals a never-interned value");

    // Param path: Q(amount) :- Posting(memo = m, amount), m != ?0.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Ne,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(crate::ir::ParamId(0)),
        })],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::Str("ghost")])
        .expect("execute");
    assert_eq!(out.len(), 2);
    // An interned value under Ne excludes exactly its rows.
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::Str("rent")])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::I64(-55));
}

#[test]
fn results_decode_intern_ids_to_original_bytes() {
    let dir = TempDir::new("prepared-decode");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "a rather long memo text", 10)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("execute");
    assert_eq!(
        out.get(0, 0),
        ResultValue::String("a rather long memo text")
    );
}
