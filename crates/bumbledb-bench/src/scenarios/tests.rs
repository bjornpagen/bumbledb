use super::*;
use crate::translate::translate;

/// Every scenario query validates, prepares, and (per its [`Twin`])
/// translates against its own schema (no corpus needed), its param sets
/// are seeded deterministic with at least one set, and the twin
/// invariants hold: `Canonical`/`Tuned` queries MUST translate, a
/// `Tuned`/`Hand` rendering must be nonempty, and `Hand` is legal ONLY
/// where the translator refuses.
#[test]
fn every_scenario_query_prepares_and_translates() {
    for scenario in all() {
        let dir = std::env::temp_dir().join(format!("bumbledb-scenario-check-{}", scenario.name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let schema = (scenario.schema)();
        let db = Db::create(&dir, (scenario.descriptor)()).expect("create");
        for sq in (scenario.queries)() {
            db.prepare(&(sq.query)())
                .unwrap_or_else(|e| panic!("{}/{}: validation: {e:?}", scenario.name, sq.name));
            match sq.twin {
                Twin::Canonical => {
                    translate(&(sq.query)(), schema, &[]).unwrap_or_else(|e| {
                        panic!("{}/{}: translation: {e}", scenario.name, sq.name)
                    });
                }
                Twin::Tuned(tuned) => {
                    translate(&(sq.query)(), schema, &[]).unwrap_or_else(|e| {
                        panic!("{}/{}: translation: {e}", scenario.name, sq.name)
                    });
                    assert!(
                        !tuned().sql.is_empty(),
                        "{}/{}: the tuned rendering must be nonempty",
                        scenario.name,
                        sq.name
                    );
                }
                Twin::Hand(hand) => {
                    assert!(
                        translate(&(sq.query)(), schema, &[]).is_err(),
                        "{}/{}: Hand is legal only where the translator refuses",
                        scenario.name,
                        sq.name
                    );
                    assert!(
                        !hand().sql.is_empty(),
                        "{}/{}: the hand rendering must be nonempty",
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
