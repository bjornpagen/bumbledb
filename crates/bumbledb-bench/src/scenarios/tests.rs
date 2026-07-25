use super::*;
use crate::translate::translate;

/// Every scenario query validates, prepares, and (per its [`Twin`])
/// translates against its own schema (no corpus needed), its param sets
/// are seeded deterministic with at least one set, and the twin
/// invariants hold: `Canonical`/`Tuned` queries MUST translate, a
/// `Tuned`/`Hand` rendering must be nonempty, and `Hand` is legal ONLY
/// where the translator refuses. A keyed-get surface must resolve its
/// key statement ON its relation and is canonical-only (the derived
/// point SELECT is `SQLite`'s best shot — no tuned/hand lane exists).
#[test]
fn every_scenario_query_prepares_and_translates() {
    for scenario in all() {
        let dir = std::env::temp_dir().join(format!("bumbledb-scenario-check-{}", scenario.name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let schema = (scenario.schema)();
        let db = Db::create(&dir, (scenario.descriptor)()).expect("create");
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

/// The prepared-query arm of the per-query check: validation + the
/// [`Twin`] translation invariants.
fn check_query(
    scenario: &str,
    sq: &ScenarioQuery,
    query: fn() -> Query,
    schema: &Schema,
    db: &Db<SchemaDescriptor>,
) {
    db.prepare(&query())
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

/// The traced scenario path (`--trace`): every timed query lands the
/// warm+cold pair as BOTH a Chrome `.json` and a collapsed `.folded`
/// beside it, and the report carries the warm flame table. One light
/// scenario, tiny protocol — a smoke test, not a measurement.
#[cfg(feature = "obs")]
#[test]
fn traced_scenarios_land_the_warm_cold_pair_and_flame() {
    use crate::harness::Protocol;
    let root = std::env::temp_dir().join("bumbledb-scenario-trace-smoke");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    let modes = super::QueryModes {
        trace_root: Some(root.clone()),
        alloc: false,
    };
    let only = vec!["points".to_owned()];
    let proto = Protocol {
        warmups: 1,
        samples: 2,
    };
    let (_markdown, reports) = super::run(&root, 7, proto, Some(only.as_slice()), &modes)
        .expect("traced scenario run");
    assert!(!reports.is_empty(), "points scenario has queries");
    for r in &reports {
        assert!(r.flame.is_some(), "{}/{}: warm flame", r.scenario, r.name);
        let dir = root.join("trace").join("scenarios").join(r.scenario);
        for half in ["warm", "cold"] {
            let json = dir.join(format!("{}.{half}.json", r.name));
            let text = std::fs::read_to_string(&json)
                .unwrap_or_else(|e| panic!("{}: {e}", json.display()));
            assert!(
                text.starts_with("[\n") && text.ends_with("\n]\n"),
                "{} parses as a Chrome array",
                json.display()
            );
            let folded = dir.join(format!("{}.{half}.folded", r.name));
            let f = std::fs::read_to_string(&folded)
                .unwrap_or_else(|e| panic!("{}: {e}", folded.display()));
            for line in f.lines() {
                let count = line.rsplit(' ').next().expect("a self-ns tail");
                assert!(count.parse::<u64>().is_ok(), "folded self-ns: {line}");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The alloc pass (`--alloc`): a SEPARATE pass from `--trace`, scoped per
/// query — every timed query gets its own reading. No trace artifacts
/// are written in this pass.
#[cfg(feature = "obs")]
#[test]
fn the_alloc_pass_scopes_a_reading_per_query() {
    use crate::harness::Protocol;
    let root = std::env::temp_dir().join("bumbledb-scenario-alloc-smoke");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    let modes = super::QueryModes {
        trace_root: None,
        alloc: true,
    };
    let only = vec!["points".to_owned()];
    let proto = Protocol {
        warmups: 1,
        samples: 2,
    };
    let (_markdown, reports) = super::run(&root, 7, proto, Some(only.as_slice()), &modes)
        .expect("alloc scenario run");
    assert!(!reports.is_empty());
    for r in &reports {
        assert!(r.alloc.is_some(), "{}/{}: per-query alloc", r.scenario, r.name);
        assert!(r.flame.is_none(), "the alloc pass writes no traces");
    }
    assert!(
        !root.join("trace").exists(),
        "the alloc pass writes no trace tree"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Scenario corpora are pure functions of the seed: the first row of
/// every relation reproduces.
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
