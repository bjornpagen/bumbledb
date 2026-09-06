use super::*;
use crate::translate::translate;

#[test]
fn native_profile_gates_prepared_and_keyed_reads_before_sampling() {
    let root = std::env::temp_dir().join(format!("bumbledb-native-oracle-{}", std::process::id()));
    std::fs::create_dir(&root).expect("fresh test root");
    let scenario = Scenario {
        rows: |_| {
            vec![
                (
                    points::ids::BUCKET,
                    Box::new(vec![vec![Value::U64(0), Value::U64(0)]].into_iter()),
                ),
                (
                    points::ids::DOC,
                    Box::new(
                        vec![vec![
                            Value::U64(0),
                            Value::String("found".into()),
                            Value::U64(0),
                            Value::I64(42),
                            Value::FixedBytes(vec![7; 32].into()),
                        ]]
                        .into_iter(),
                    ),
                ),
            ]
        },
        ..points::scenario()
    };
    let stores = super::load::load(&root, &scenario, 1).expect("two-row oracle corpus");
    for name in ["p1_by_id", "p5_keyed_get"] {
        let mut query = (scenario.queries)()
            .into_iter()
            .find(|q| q.name == name)
            .unwrap();
        query.params = if name == "p1_by_id" {
            |_| vec![vec![Value::U64(0)], vec![Value::U64(1)]]
        } else {
            |_| {
                vec![
                    vec![Value::String("found".into())],
                    vec![Value::String("missing".into())],
                ]
            }
        };
        let args = crate::cli::ProfileArgs {
            corpus: crate::cli::CorpusArgs::default(),
            family: name.to_owned(),
            seconds: 0,
            out: None,
        };
        let result = super::run_query::profile(&stores, &scenario, &query, &args).unwrap();
        assert_eq!(
            (result.draws, result.cycles, result.rows_per_cycle),
            (2, 1, 1)
        );
        stores.conn.execute("UPDATE Doc SET size = 43", []).unwrap();
        let error = super::run_query::profile(&stores, &scenario, &query, &args).unwrap_err();
        assert!(error.contains("ENGINES DISAGREE"), "{error}");
        stores.conn.execute("UPDATE Doc SET size = 42", []).unwrap();
    }
    drop(stores);
    std::fs::remove_dir_all(root).expect("remove this test's corpus");
}

#[test]
fn every_scenario_query_prepares_and_translates() {
    for scenario in all() {
        let dir = std::env::temp_dir().join(format!("bumbledb-scenario-check-{}", scenario.name));
        let _ = std::fs::remove_dir_all(&dir);
        let schema = (scenario.schema)();
        let db = Db::create(&dir, (scenario.descriptor)(), crate::harness::bench_work())
            .expect("create")
            .expect("accepted");
        for sq in (scenario.queries)() {
            match &sq.surface {
                Surface::Query(query) => check_query(scenario.name, &sq, *query, schema, &db),
                Surface::KeyedGet { relation, key } => {
                    let statement = key(schema);
                    let bumbledb::schema::StatementView::Key(_, key_statement) =
                        schema.statement(statement)
                    else {
                        panic!(
                            "{}/{}: {statement:?} is not a key statement",
                            scenario.name, sq.name
                        );
                    };
                    assert_eq!(
                        key_statement.relation, *relation,
                        "{}/{}: the key statement lives on the queried relation",
                        scenario.name, sq.name
                    );
                    assert!(
                        matches!(sq.twin, Twin::Canonical),
                        "{}/{}: a keyed-get twin is canonical-only",
                        scenario.name,
                        sq.name
                    );
                    let rendered = crate::translate::keyed_get(schema, *relation, key_statement);
                    assert!(
                        !rendered.sql.is_empty() && !rendered.params.is_empty(),
                        "{}/{}: the keyed-get point SELECT must render",
                        scenario.name,
                        sq.name
                    );
                }
            }
            let a = (sq.params)(1);
            let b = (sq.params)(1);
            assert_eq!(a, b, "{}/{}: params must be seeded", scenario.name, sq.name);
            assert!(
                !a.is_empty(),
                "{}/{}: at least one param set",
                scenario.name,
                sq.name
            );
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

fn check_query(
    scenario: &str,
    sq: &ScenarioQuery,
    query: fn() -> Query,
    schema: &Schema,
    db: &Db<SchemaDescriptor>,
) {
    db.prepare(&query(), crate::harness::bench_work())
        .unwrap_or_else(|e| panic!("{scenario}/{}: validation: {e:?}", sq.name));
    match sq.twin {
        Twin::Canonical => {
            translate(&query(), schema, &[])
                .unwrap_or_else(|e| panic!("{scenario}/{}: translation: {e}", sq.name));
        }
        Twin::Tuned(tuned) => {
            translate(&query(), schema, &[])
                .unwrap_or_else(|e| panic!("{scenario}/{}: translation: {e}", sq.name));
            assert!(
                !tuned().sql.is_empty(),
                "{scenario}/{}: the tuned rendering must be nonempty",
                sq.name
            );
        }
        Twin::Hand(hand) => {
            assert!(
                translate(&query(), schema, &[]).is_err(),
                "{scenario}/{}: Hand is legal only where the translator refuses",
                sq.name
            );
            assert!(
                !hand().sql.is_empty(),
                "{scenario}/{}: the hand rendering must be nonempty",
                sq.name
            );
        }
    }
}

#[cfg(feature = "alloc-counter")]
#[test]
fn the_alloc_pass_scopes_a_reading_per_query() {
    use crate::harness::Protocol;
    let root = std::env::temp_dir().join("bumbledb-scenario-alloc-smoke");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    let modes = super::QueryModes { alloc: true };
    let only = vec!["points".to_owned()];
    let proto = Protocol {
        warmups: 1,
        samples: 2,
    };
    let (_markdown, reports) =
        super::run(&root, 7, proto, Some(only.as_slice()), &modes).expect("alloc scenario run");
    assert!(!reports.is_empty());
    for r in &reports {
        assert!(
            r.alloc.is_some(),
            "{}/{}: per-query alloc",
            r.scenario,
            r.name
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn scenario_rows_are_deterministic() {
    for scenario in all() {
        let first = |seed: u64| -> Vec<Vec<Value>> {
            (scenario.rows)(seed)
                .into_iter()
                .filter_map(|(_, mut rows)| rows.next())
                .collect()
        };
        assert_eq!(first(7), first(7), "{}: rows must be seeded", scenario.name);
    }
}
