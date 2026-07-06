//! The performance tripwires (docs/architecture/50-validation.md): every fix in the perf
//! suite, encoded as a structural regression test over the pinned S
//! corpus — trace-event counts and counter work bounds, never wall
//! clock. If a finding from the 2026-07-03 report silently returns,
//! one of these fails by name.
//!
//! This module is test-only enforcement; it compiles no production
//! code.

#[cfg(test)]
mod tests {
    use crate::corpus;
    use crate::families;
    use crate::gen::{GenConfig, Scale, Sizes, MEMO_VOCAB, UNIQUE_MEMO_DEN};
    use crate::schema::schema;
    use bumbledb::Db;

    const CFG: GenConfig = GenConfig {
        seed: 1,
        scale: Scale::S,
    };

    fn corpus_db(tag: &str) -> (std::path::PathBuf, Db<'static>) {
        let dir = std::env::temp_dir().join(format!("bumbledb-tripwires-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = Db::create(&dir, schema()).expect("create");
        corpus::load_bumbledb(&db, CFG).expect("load");
        (dir, db)
    }

    /// Finding 1 (the access path), forever: after one full param
    /// rotation, **no read family ever rebuilds a view again within a
    /// generation** — selections probe, residual bindings ride the LRU,
    /// and the guard family never touches views at all.
    #[cfg(feature = "obs")]
    #[test]
    fn no_read_family_rescans_after_one_rotation() {
        use bumbledb::obs;

        let (dir, db) = corpus_db("rescan");
        for family in families::all() {
            let query = (family.query)();
            let mut prepared = db.prepare(&query).expect("prepare");
            let sets = (family.params)(&CFG);
            // One full warm rotation: every binding built once.
            for params in &sets {
                db.read(|snap| snap.execute_collect(&mut prepared, params).map(|_| ()))
                    .expect("warm");
            }
            // Two further rotations: zero view builds, ever.
            for cycle in 0..2 {
                for (set_idx, params) in sets.iter().enumerate() {
                    obs::start_capture();
                    db.read(|snap| snap.execute_collect(&mut prepared, params).map(|_| ()))
                        .expect("execute");
                    let events = obs::finish_capture();
                    let builds = events
                        .iter()
                        .filter(|e| e.name == obs::names::VIEW_BUILD)
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

    /// Finding 2 (per-row intern resolution), forever: a traced warm
    /// sample resolves each distinct string once — `fk_walk`'s single
    /// holder name costs one lookup for its ~200 rows, skew's single
    /// label one for its ~50k.
    #[cfg(feature = "obs")]
    #[test]
    fn finalize_resolution_stays_collapsed() {
        use bumbledb::obs;

        let (dir, db) = corpus_db("resolve");
        let resolves_of = |name: &str, set_idx: usize| -> (usize, usize) {
            let family = families::all()
                .iter()
                .find(|f| f.name == name)
                .expect("registered");
            let mut prepared = db.prepare(&(family.query)()).expect("prepare");
            let sets = (family.params)(&CFG);
            for params in &sets {
                db.read(|snap| snap.execute_collect(&mut prepared, params).map(|_| ()))
                    .expect("warm");
            }
            obs::start_capture();
            let out = db
                .read(|snap| snap.execute_collect(&mut prepared, &sets[set_idx]))
                .expect("execute");
            let events = obs::finish_capture();
            let resolves = events
                .iter()
                .filter(|e| e.name == obs::names::DICT_RESOLVE)
                .count();
            (resolves, out.len())
        };

        // fk_walk set 0 = one cold account = one holder name.
        let (resolves, rows) = resolves_of("fk_walk", 0);
        assert!(rows > 1, "a real result set");
        assert_eq!(resolves, 1, "one distinct name, one resolution");

        // skew set 0 = the hot tag = one label across ~50k rows.
        let (resolves, rows) = resolves_of("skew", 0);
        assert!(rows > 10_000, "the hot label is hot");
        assert_eq!(resolves, 1, "one distinct label, one resolution");
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Findings 1+3 (scan work and cover choice), forever: profiled work
    /// per family is bounded by the corpus's logical selectivities —
    /// never O(relation) for selective families, never O(map capacity)
    /// anywhere. Bounds carry their derivations; counters are
    /// deterministic over the pinned corpus.
    #[test]
    fn read_work_is_bounded_by_selectivity() {
        let (dir, db) = corpus_db("work");
        let sizes = Sizes::of(CFG.scale);
        let typical = |name: &str| -> usize {
            match name {
                "balance" => 1,
                "skew" => 2,
                _ => 0,
            }
        };
        for family in families::all() {
            let query = (family.query)();
            let mut prepared = db.prepare(&query).expect("prepare");
            let sets = (family.params)(&CFG);
            for params in &sets {
                db.read(|snap| snap.execute_collect(&mut prepared, params).map(|_| ()))
                    .expect("warm");
            }
            let (out, stats) = db
                .read(|snap| snap.profile(&mut prepared, &sets[typical(family.name)]))
                .expect("profile");
            let drawn: u64 = stats.nodes.iter().map(|n| n.batch_entries).sum();
            // Derivations over the pinned corpus (postings = 100_000,
            // accounts = 500, holders = 125, instruments = 512):
            let bound = match family.name {
                // Guard probe: no join nodes at all.
                "point" => 0,
                // One cold account: ~postings/accounts = 200 postings,
                // plus the account and holder probes. 4x margin.
                "fk_walk" => 4 * (sizes.postings / sizes.accounts + 2),
                // ~2% suffix x open share, three relations walked by
                // cover: bounded by 3x the window's postings.
                "chain" => 3 * (sizes.postings * 8 / 100),
                // The pure scan family: one pass over postings.
                "range" => 2 * sizes.postings,
                // One light holder: ~4 accounts x ~200 postings + keys.
                "balance" => 4 * (4 * (sizes.postings / sizes.accounts) + 8),
                // The full fold: every posting once + the instruments.
                "stats" => sizes.postings + 2 * sizes.instruments,
                // One vocabulary memo: ~postings/vocab + uniq share.
                "string" => {
                    8 * (sizes.postings / MEMO_VOCAB + sizes.postings / UNIQUE_MEMO_DEN / 64)
                }
                // One uniform tag: ~2 accounts x ~200 postings + probes.
                "skew" => 8 * (2 * (sizes.postings / sizes.accounts) + 8),
                // Param-less self-join on transfer: the cover draws every
                // posting once (~postings (t, x) keys) and the second
                // occurrence ~2-3 per surviving entry (measured 4.0x
                // postings at S) — 6x margin.
                "spread" => 6 * sizes.postings,
                // The cyclic self-join: the measured S plan iterates one
                // full occurrence and probes the closing edges — bounded
                // by a small multiple of postings (the cold ~1% window
                // keeps the surviving suffix tiny).
                "triangle" => 8 * sizes.postings,
                other => unreachable!("unregistered family {other}"),
            };
            eprintln!(
                "tripwire {}: drawn {drawn}, bound {bound}, rows {}",
                family.name,
                out.len()
            );
            assert!(
                drawn <= bound,
                "{}: drew {drawn} entries, bound {bound} — a scan or a \
                 capacity walk came back ({stats:?})",
                family.name
            );
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
