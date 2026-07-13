//! The rewrites target (the crucible packet (git ecec1dc3)): the dual-PIPELINE
//! differential. "Rewrites are semantics-preserving" — the grounding
//! (occurrence elimination + prepare-time folding) and the statically-
//! empty condition fold are where "looks right" and "is right" diverge
//! silently: a wrong fold produces plausible wrong answers, not crashes.
//! Every iteration runs the same query × draw through the rewritten
//! pipeline and the rewrite-free one and demands identical result sets.
//!
//! NOT a dual build (recorded in the PRD's Results): cargo refuses a
//! second renamed dependency on the same package, and feature
//! unification would union a rename into one build anyway. The
//! `ground-off`/`fold-off` features never were compile-time pass removal
//! — they gate PUBLIC access to the passes' thread-local off switches
//! (`with_grounding_disabled`/`with_fold_disabled`), so ONE binary carries
//! both pipelines and flips them per execution, exactly the bench
//! crate's dual-run differential idiom.
//!
//! Non-vacuity is counted, not assumed: scalar-only draws profile the
//! rewritten plan and tally iterations where a rewrite actually fired
//! (an eliminated/folded occurrence or a dead rule), logged
//! periodically.

use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::{with_grounding_disabled, with_fold_disabled};
use bumbledb_bench::corpus_gen::Rng;
use bumbledb_bench::families;
use bumbledb_bench::querygen;

use crate::world::{self, WORLD_SEEDS, with_world};

/// Dual-run tallies: total compared draws, and draws whose rewritten
/// plan provably rewrote something (the equality was not vacuous).
static COMPARED: AtomicU64 = AtomicU64::new(0);
static REWRITE_FIRED: AtomicU64 = AtomicU64::new(0);

const LOG_EVERY: u64 = 10_000;

/// The rewrites runner: one fuzz iteration.
pub fn run(data: &[u8]) {
    let mut rng = Rng::from_bytes(data);
    let index = usize::try_from(rng.range(WORLD_SEEDS.len() as u64)).expect("index fits usize");
    let cfg = world::config(index);
    let query = querygen::random_query(&mut rng, cfg);
    let draws = querygen::params_for(&query, &mut rng, cfg);
    with_world(index, |world| {
        for draw in draws {
            let params = world::positional(&draw);
            // The rewritten pipeline: grounding and fold on (the default).
            let rewritten = world::execute(&world.db, &query, &params);
            // The rewrite-free pipeline: both switches thrown for the
            // prepare inside — the plan is built with the grounding skipped
            // and the fold skipped, then executed on the same store.
            let rewrite_free = with_grounding_disabled(|| {
                with_fold_disabled(|| world::execute(&world.db, &query, &params))
            });
            // THE oracle: identical result sets (typed runtime errors
            // compared whole — error parity is part of the semantics).
            assert_eq!(
                rewritten.verdict, rewrite_free.verdict,
                "the rewritten and rewrite-free pipelines disagree: {query:#?}\n\
                 params: {params:#?}"
            );
            tally(world, &query, &draw);
        }
    });
}

/// The non-vacuity tally: for scalar-only draws (the profile surface is
/// scalar-params-only), count whether the rewritten plan carries any
/// eliminated/folded occurrence or dead rule.
fn tally(world: &world::World, query: &bumbledb::Query, draw: &querygen::ParamDraw) {
    let compared = COMPARED.fetch_add(1, Ordering::Relaxed) + 1;
    if draw.sets.is_empty() {
        let mut prepared = world
            .db
            .prepare(query)
            .expect("a compared query re-prepares");
        let params = world::positional(draw);
        let scalars = families::scalar_values(&params);
        let (_, stats) = world
            .db
            .read(|snap| snap.profile(&mut prepared, &scalars))
            .expect("a compared query profiles");
        let fired = !stats.dead.is_empty()
            || stats
                .rules
                .iter()
                .any(|rule| !rule.eliminated.is_empty() || !rule.folded.is_empty());
        if fired {
            REWRITE_FIRED.fetch_add(1, Ordering::Relaxed);
        }
    }
    if compared.is_multiple_of(LOG_EVERY) {
        eprintln!(
            "rewrites target: {compared} draws compared, {} with a rewrite provably fired",
            REWRITE_FIRED.load(Ordering::Relaxed)
        );
    }
}
