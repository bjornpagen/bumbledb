//! The performance tripwires (docs/architecture/60-validation.md): the
//! perf suite's fixes, encoded as structural regression tests over the
//! pinned S corpus — trace-event counts, plan-shape observables, and
//! counter work bounds, never wall clock. If a repaired finding silently
//! returns, one of these fails by name.
//!
//! This module is test-only enforcement; it compiles no production
//! code.

#[cfg(test)]
mod tests {
    use crate::corpus;
    use crate::corpus_gen::{GenConfig, Scale, Sizes};
    use crate::families::{self, has_sets, param_args, scalar_values};
    use crate::schema::Ledger;
    use bumbledb::Db;

    const CFG: GenConfig = GenConfig {
        seed: 1,
        scale: Scale::S,
    };

    fn corpus_db(tag: &str) -> (std::path::PathBuf, Db<Ledger>) {
        let dir = std::env::temp_dir().join(format!("bumbledb-tripwires-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = Db::create(&dir, Ledger).expect("create");
        corpus::load_bumbledb(&db, CFG).expect("load");
        (dir, db)
    }

    /// The selection machinery engages for the param-set family
    /// (docs/architecture/40-execution.md § selection levels — param
    /// sets ride the selection trie): warm executions emit
    /// `select_probe` events, and the membership families never emit a
    /// `key_probe`.
    #[cfg(feature = "obs")]
    #[test]
    fn selection_levels_engage_for_the_param_set_family() {
        use bumbledb::obs;

        let (dir, db) = corpus_db("selections");
        let events_of = |name: &str, event: &str| -> usize {
            let family = families::all()
                .iter()
                .find(|f| f.name == name)
                .expect("registered");
            let mut prepared = db.prepare(&(family.query)()).expect("prepare");
            let sets = (family.params)(&CFG);
            for params in &sets {
                let args = param_args(params);
                db.read(|snap| snap.execute_collect_args(&mut prepared, &args).map(|_| ()))
                    .expect("warm");
            }
            obs::start_capture();
            let args = param_args(&sets[0]);
            db.read(|snap| snap.execute_collect_args(&mut prepared, &args).map(|_| ()))
                .expect("execute");
            obs::finish_capture()
                .iter()
                .filter(|e| e.name == event)
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

    /// The calendar aggregate family's plan regime, pinned (structural, no wall
    /// clock — docs/architecture/60-validation.md § the calendar
    /// benchmark):
    /// - `claim_hours` binds the claim key (`source`), so the fold's
    ///   distinct-bindings elision engages (the `balance` regime);
    #[test]
    fn calendar_family_regimes_are_pinned() {
        use crate::calendar::{Scheduling, families as cal};
        let dir = std::env::temp_dir().join("bumbledb-tripwires-calendar");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, Scheduling).expect("create");
        let prepared = |name: &str| {
            let family = cal::all()
                .iter()
                .find(|f| f.name == name)
                .expect("registered");
            db.prepare(&(family.query)()).expect("prepares")
        };
        assert!(
            prepared("claim_hours").distinct_bindings(),
            "the source binding covers the claim key — the fold elides its seen set"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The aggregate families' fold regimes, pinned: balance binds the
    /// posting fresh — distinct bindings proven, the seen-set elided.
    /// stats binds no key coverage **by design** (collapsing duplicate
    /// (currency, amount, at, account) bindings is the family's set
    /// semantics), so its dedup pass is semantically required. A planner
    /// change that flips either regime is a semantics bug, not a tuning
    /// change.
    #[test]
    fn aggregate_family_fold_regimes_are_pinned() {
        let dir = std::env::temp_dir().join("bumbledb-tripwires-elide");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = Db::create(&dir, Ledger).expect("create");
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
    /// rotation, **no read family ever rebuilds a view again within a
    /// generation** — selections probe, residual bindings ride the LRU,
    /// and the key-probe family never touches views at all.
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
                let args = param_args(params);
                db.read(|snap| snap.execute_collect_args(&mut prepared, &args).map(|_| ()))
                    .expect("warm");
            }
            // Two further rotations: zero view builds, ever.
            for cycle in 0..2 {
                for (set_idx, params) in sets.iter().enumerate() {
                    let args = param_args(params);
                    obs::start_capture();
                    db.read(|snap| snap.execute_collect_args(&mut prepared, &args).map(|_| ()))
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
    /// sample resolves each distinct string once — `containment_walk`'s single
    /// holder name costs one lookup for its ~200 rows.
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
        for params in &sets {
            let args = param_args(params);
            db.read(|snap| snap.execute_collect_args(&mut prepared, &args).map(|_| ()))
                .expect("warm");
        }
        obs::start_capture();
        let args = param_args(&sets[0]);
        let out = db
            .read(|snap| snap.execute_collect_args(&mut prepared, &args))
            .expect("execute");
        let events = obs::finish_capture();
        let resolves = events
            .iter()
            .filter(|e| e.name == obs::names::DICT_RESOLVE)
            .count();
        assert!(out.len() > 1, "a real result set");
        assert_eq!(resolves, 1, "one distinct name, one resolution");
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Findings 1+3 (scan work and cover choice), forever: profiled work
    /// per family is bounded by the corpus's logical selectivities —
    /// never O(relation) for selective families, never O(map capacity)
    /// anywhere. Bounds carry their derivations; counters are
    /// deterministic over the pinned corpus. The param-set family has no
    /// scalar profile path (set selectivity is an execution fact) — its
    /// structural assertion is the selection-level tripwire above.
    #[test]
    fn read_work_is_bounded_by_selectivity() {
        let (dir, db) = corpus_db("work");
        let sizes = Sizes::of(CFG.scale);
        let typical = |name: &str| -> usize {
            match name {
                "balance" | "skew" => 1,
                _ => 0,
            }
        };
        for family in families::all() {
            let sets = (family.params)(&CFG);
            if has_sets(&sets) {
                continue;
            }
            let query = (family.query)();
            let mut prepared = db.prepare(&query).expect("prepare");
            for params in &sets {
                let args = param_args(params);
                db.read(|snap| snap.execute_collect_args(&mut prepared, &args).map(|_| ()))
                    .expect("warm");
            }
            let (out, stats) = db
                .read(|snap| {
                    snap.profile(&mut prepared, &scalar_values(&sets[typical(family.name)]))
                })
                .expect("profile");
            let drawn: u64 = stats.rules[0].nodes.iter().map(|n| n.batch_entries).sum();
            // Derivations over the pinned corpus (postings = 100_000,
            // entries = 50_000, accounts = 500, holders = 125,
            // instruments = 512, orgs = 64, mandates = 2_000):
            let bound = match family.name {
                // Key probe: no join nodes at all.
                "point" => 0,
                // One cold account: ~postings/accounts = 200 postings,
                // plus the account and holder probes. 4x margin.
                "containment_walk" => 4 * (sizes.postings / sizes.accounts + 2),
                // ~2% suffix x 1/3 currency share, three relations
                // walked by cover: bounded by 3x the window's postings.
                "chain" => 3 * (sizes.postings * 8 / 100),
                // The pure scan family: one pass over postings.
                "range" => 2 * sizes.postings,
                // One light holder: ~4 accounts x ~200 postings + keys.
                "balance" => 4 * (4 * (sizes.postings / sizes.accounts) + 8),
                // The full fold: every posting once + the accounts.
                "stats" => 2 * (sizes.postings + sizes.accounts),
                // One symbol: ~postings/instruments postings + probes.
                "string" => 8 * (sizes.postings / sizes.instruments + 8),
                // One uniform tag: ~40% of second tag slots — bounded by
                // the full PostingTag pass plus the matched postings.
                "skew" => 2 * (sizes.posting_tags + sizes.postings),
                // Param-less self-join on entry: the cover draws every
                // posting once and the second occurrence ~2-3 per
                // surviving entry — 6x margin.
                "spread" => 6 * sizes.postings,
                // The cyclic self-join: the measured plan iterates one
                // full occurrence and probes the closing edges — bounded
                // by a small multiple of postings (the cold ~1% window
                // keeps the surviving suffix tiny).
                "triangle" => 8 * sizes.postings,
                // One account's postings + its anti-probes.
                "postings_without_tag" => 16 * (sizes.postings / sizes.accounts + 16),
                // The full Arg restriction: every posting once, plus the
                // per-account extremes.
                "latest_posting_per_account" => 4 * sizes.postings,
                // One (account, at) posting point + the account's ~4
                // mandate segments; generous margin for the cover's
                // account-postings walk.
                "mandate_at_instant" => 16 * (sizes.postings / sizes.accounts + 64),
                // Chain's ~2% suffix walked through four nodes (no
                // currency pin; entry/account/holder key probes per
                // surviving posting) — bounded by 4x the window.
                "deep_chain" => 4 * (sizes.postings * 8 / 100),
                // One org's ~mandates/orgs segments squared (the
                // overlap join), plus probes.
                "mandate_overlap" => {
                    let per_org = sizes.mandates / sizes.orgs + 8;
                    8 * per_org * per_org
                }
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
