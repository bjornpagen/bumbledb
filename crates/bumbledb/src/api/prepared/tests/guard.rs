use super::*;

/// PRD 11 (docs/perf/): the guard fast lane — hit, miss, and a
/// param-type error, with an interned find exercising the resolving
/// column beside the word blits.
#[test]
fn guard_fast_lane_hits_misses_and_type_errors() {
    let dir = TempDir::new("prepared-guard-lane");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "memo-a", 41), (2, 8, "memo-b", 42)]);
    // Q(account, memo, amount) :- Posting(id = ?0, account, memo, amount).
    let query = Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(2))),
            ],
        }],
        predicates: vec![],
    };
    let txn = env.read_txn().expect("txn");
    let cache = crate::image::cache::ImageCache::new();
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
    assert!(
        prepared.guard_finds.is_some(),
        "plain-variable guard takes the fast lane"
    );
    let mut out = ResultBuffer::new();
    // Hit: every cell decoded straight from the fact.
    prepared
        .execute(&txn, &cache, &[crate::ir::Value::U64(2)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::U64(8));
    assert_eq!(out.get(0, 1), ResultValue::String("memo-b"));
    assert_eq!(out.get(0, 2), ResultValue::I64(42));
    // Miss: clean empty buffer.
    prepared
        .execute(&txn, &cache, &[crate::ir::Value::U64(999)], &mut out)
        .expect("miss is empty, not an error");
    assert_eq!(out.len(), 0);
    // Param-type error: typed, before any probe.
    let err = prepared
        .execute(&txn, &cache, &[crate::ir::Value::Bool(true)], &mut out)
        .expect_err("type mismatch");
    assert!(matches!(err, Error::ParamTypeMismatch { .. }), "{err:?}");
}

/// The guard lane is stats-free end to end (docs/silicon/13): a
/// guard prepare + execute builds NO image — and the lazy distinct
/// counts live on images, so no image means no stats walk, ever.
/// This is the isolation gate in its strongest form.
#[test]
fn a_guard_prepare_and_execute_build_no_image() {
    let dir = TempDir::new("prepared-guard-statsfree");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "memo-a", 41)]);
    let query = Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(2))),
            ],
        }],
        predicates: vec![],
    };
    let txn = env.read_txn().expect("txn");
    let cache = crate::image::cache::ImageCache::new();
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
    assert!(prepared.guard_finds.is_some(), "the fast lane classified");
    let mut out = ResultBuffer::new();
    prepared
        .execute(&txn, &cache, &[crate::ir::Value::U64(1)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    #[cfg(feature = "trace")]
    assert_eq!(
        cache.resident(),
        (0, 0),
        "a guard execute must not build images (and so never walks stats)"
    );
}

#[test]
fn guard_probe_queries_flow_through_the_same_surface() {
    let dir = TempDir::new("prepared-guard");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(5, 7, "found", 42)]);
    let cache = ImageCache::new();
    // Q(amount) :- Posting(id = 5, amount) — the serial key: guard probe.
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(5))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        predicates: vec![],
    };
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(matches!(prepared.plan, ExecPlan::GuardProbe(_)));
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::I64(42));

    // EXPLAIN reports the classification alongside the rows.
    let (rows, report) = prepared.explain(&txn, &cache, &[]).expect("explain");
    assert_eq!(rows.len(), 1);
    assert!(report.contains("guard probe"));
}
