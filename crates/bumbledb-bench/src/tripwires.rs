#[cfg(test)]
mod tests {
    use crate::families;
    use crate::schema::Ledger;
    use bumbledb::Db;

    #[cfg(feature = "obs")]
    use crate::corpus;
    #[cfg(feature = "obs")]
    use crate::corpus_gen::{GenConfig, Scale};
    #[cfg(feature = "obs")]
    use crate::families::param_args;

    #[cfg(feature = "obs")]
    const CFG: GenConfig = GenConfig {
        seed: 1,
        scale: Scale::S,
    };

    #[cfg(feature = "obs")]
    fn corpus_db(tag: &str) -> (std::path::PathBuf, Db<Ledger>) {
        let dir = std::env::temp_dir().join(format!("bumbledb-tripwires-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        let db = Db::create(&dir, Ledger).expect("create").expect("accepted");
        corpus::load_bumbledb(&db, CFG).expect("load");
        (dir, db)
    }

    #[cfg(feature = "obs")]
    #[test]
    fn selection_levels_engage_for_the_param_set_family() {
        use bumbledb::obs;

        let (dir, db) = corpus_db("selections");
        let events_of = |name: &str, point: obs::TracePoint| -> usize {
            let family = families::all()
                .iter()
                .find(|f| f.name == name)
                .expect("registered");
            let mut prepared = db.prepare(&(family.query)()).expect("prepare");
            let sets = (family.params)(&CFG);
            for params in &sets {
                let args = param_args(params);
                db.read(|snap| snap.execute_collect(&mut prepared, &args).map(|_| ()))
                    .expect("warm");
            }
            obs::start_capture();
            let args = param_args(&sets[0]);
            db.read(|snap| snap.execute_collect(&mut prepared, &args).map(|_| ()))
                .expect("execute");
            obs::finish_capture()
                .iter()
                .filter(|e| e.point() == point)
                .count()
        };
        assert!(
            events_of("entries_for_account_set", obs::names::SELECT_PROBE) > 0,
            "the set binding must probe selection levels"
        );
        for membership in ["mandate_at_instant", "mandate_overlap"] {
            assert_eq!(
                events_of(membership, obs::names::KEY_PROBE),
                0,
                "{membership} must not key-probe"
            );
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn aggregate_family_fold_regimes_are_pinned() {
        let dir = std::env::temp_dir().join("bumbledb-tripwires-elide");
        let _ = std::fs::remove_dir_all(&dir);
        let db = Db::create(&dir, Ledger).expect("create").expect("accepted");
        let regime = |name: &str| {
            let family = families::all()
                .iter()
                .find(|f| f.name == name)
                .expect("registered");
            let prepared = db.prepare(&(family.query)()).expect("prepares");
            prepared.distinct_bindings()
        };
        assert!(regime("balance"), "balance elides the seen set");
        assert!(!regime("stats"), "stats' dedup is semantics");
        assert!(
            regime("latest_posting_per_account"),
            "the Arg family binds the posting fresh"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Finding 1 (the access path), forever: after one full param

    #[cfg(feature = "obs")]
    #[test]
    fn no_read_family_rescans_after_one_rotation() {
        use bumbledb::obs;

        let (dir, db) = corpus_db("rescan");
        for family in families::all() {
            let query = (family.query)();
            let mut prepared = db.prepare(&query).expect("prepare");
            let sets = (family.params)(&CFG);

            for params in &sets {
                let args = param_args(params);
                db.read(|snap| snap.execute_collect(&mut prepared, &args).map(|_| ()))
                    .expect("warm");
            }

            for cycle in 0..2 {
                for (set_idx, params) in sets.iter().enumerate() {
                    let args = param_args(params);
                    obs::start_capture();
                    db.read(|snap| snap.execute_collect(&mut prepared, &args).map(|_| ()))
                        .expect("execute");
                    let events = obs::finish_capture();
                    let builds = events
                        .iter()
                        .filter(|e| e.point() == obs::names::VIEW_BUILD)
                        .count();
                    assert_eq!(
                        builds, 0,
                        "{} set {set_idx} cycle {cycle} rebuilt a view",
                        family.name
                    );
                }
            }
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "obs")]
    #[test]
    fn finalize_resolution_stays_collapsed() {
        use bumbledb::obs;

        let (dir, db) = corpus_db("resolve");
        let family = families::all()
            .iter()
            .find(|f| f.name == "containment_walk")
            .expect("registered");
        let mut prepared = db.prepare(&(family.query)()).expect("prepare");
        let sets = (family.params)(&CFG);
        let args = param_args(&sets[0]);
        obs::start_capture();
        let out = db
            .read(|snap| snap.execute_collect(&mut prepared, &args))
            .expect("first execute");
        let cold = obs::finish_capture()
            .iter()
            .filter(|e| e.point() == obs::names::DICT_RESOLVE)
            .count();
        assert!(out.len() > 1, "a real result set");
        assert_eq!(cold, 1, "one distinct name, one descent on first touch");
        for params in &sets {
            let warm_args = param_args(params);
            db.read(|snap| snap.execute_collect(&mut prepared, &warm_args).map(|_| ()))
                .expect("warm");
        }
        obs::start_capture();
        let out = db
            .read(|snap| snap.execute_collect(&mut prepared, &args))
            .expect("re-execute");
        let warm = obs::finish_capture()
            .iter()
            .filter(|e| e.point() == obs::names::DICT_RESOLVE)
            .count();
        assert!(out.len() > 1, "a real result set");
        assert_eq!(warm, 0, "the persistent tier holds: zero descents warm");
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
