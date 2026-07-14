use bumbledb::Query;

use crate::corpus_gen::{GenConfig, Rng};
use crate::querygen::dress::dress;
use crate::querygen::negate::negate;
use crate::querygen::shapes::{aggregate, chain, key_probe, self_join, star};
use crate::querygen::shapes_closed::{closed_join, ground_fold};
use crate::querygen::shapes_ground::{du_walk, existence_walk};
use crate::querygen::shapes_interval::{boundary, interval_join, measure, membership};
use crate::querygen::shapes_rules::rules;
use crate::querygen::shapes_sink::{arg, count_distinct};
use crate::querygen::target::{Domains, ids};
use crate::querygen::{Builder, GenTags, SHAPE_WEIGHTS, Shape};

fn shape_of(rng: &mut Rng) -> Shape {
    let total: u64 = SHAPE_WEIGHTS.iter().map(|(_, w)| w).sum();
    let mut draw = rng.range(total);
    for (shape, weight) in SHAPE_WEIGHTS {
        if draw < *weight {
            return *shape;
        }
        draw -= weight;
    }
    unreachable!("weights cover the draw")
}

fn build(rng: &mut Rng, shape: Shape, cfg: GenConfig, domains: &Domains) -> Builder {
    let mut b = Builder::default();
    match shape {
        Shape::KeyProbe => key_probe(&mut b, rng),
        Shape::Star => star(&mut b, rng),
        Shape::Chain => chain(&mut b, rng),
        Shape::SelfJoin => self_join(&mut b, rng),
        Shape::Gated => {
            match rng.range(5) {
                0 => key_probe(&mut b, rng),
                1 => star(&mut b, rng),
                2 => chain(&mut b, rng),
                3 => aggregate(&mut b, rng),
                _ => count_distinct(&mut b, rng),
            }
            // The zero-binding nonemptiness gate, drawn from more than
            // one relation (falsity is the empty-store pass's job;
            // diversity here is about relation shape) — including under
            // aggregates, per the two aggregate-bearing arms above.
            b.add_atom(match rng.range(3) {
                0 => ids::ORG,
                1 => ids::ORG_PARENT,
                _ => ids::POSTING_TAG,
            });
        }
        Shape::Aggregate => aggregate(&mut b, rng),
        Shape::Membership => membership(&mut b, rng, cfg, domains),
        Shape::IntervalJoin => interval_join(&mut b, rng, cfg, domains),
        Shape::Boundary => boundary(&mut b, rng, cfg, domains),
        Shape::CountDistinct => count_distinct(&mut b, rng),
        Shape::Arg => arg(&mut b, rng),
        Shape::ExistenceWalk => existence_walk(&mut b, rng),
        Shape::DuWalk => du_walk(&mut b, rng),
        Shape::Measure => measure(&mut b, rng, cfg, domains),
        Shape::ClosedJoin => closed_join(&mut b, rng),
        Shape::GroundFold => ground_fold(&mut b, rng),
        Shape::Rules => unreachable!("multi-rule programs assemble their own query"),
    }
    // The grounding and closed shapes are their own deliberate dressing: a
    // random predicate or negated probe landing on the target atom
    // would flip an eliminable shape to a refusal (or blur the counted
    // closed class) nondeterministically, and the coverage contract
    // asserts each variant per run (shapes_ground.rs, shapes_closed.rs).
    if !matches!(
        shape,
        Shape::ExistenceWalk | Shape::DuWalk | Shape::ClosedJoin | Shape::GroundFold
    ) {
        dress(&mut b, rng, cfg, domains);
        // Negation last: its templates draw on every anchor the shape
        // and the dressing established.
        negate(&mut b, rng);
    }
    b
}

pub(super) fn random_query_tagged(rng: &mut Rng, cfg: GenConfig) -> (Query, Shape, GenTags) {
    let domains = Domains::of(cfg.scale);
    let shape = shape_of(rng);
    if shape == Shape::Rules {
        // Multi-rule programs bypass the single-rule Builder: variables
        // are rule-scoped, so each arm carries its own scope and the
        // shape assembles the `Query` itself (dressing and negation are
        // deliberately withheld, like the grounding shapes — the variants'
        // bands are the point).
        let (query, variant) = rules(rng, &domains);
        let tags = GenTags {
            rules: Some(variant),
            ..GenTags::default()
        };
        return (query, shape, tags);
    }
    let b = build(rng, shape, cfg, &domains);
    let tags = GenTags {
        miss: b.miss,
        bytes_hit: b.bytes_hit,
        bytes_miss: b.bytes_miss,
        adjacent_left: b.adjacent_left,
        adjacent_right: b.adjacent_right,
        ladder: b.ladder,
        random_mask: b.random_mask,
        ground: b.ground,
        rules: None,
        closed: b.closed,
    };
    (b.into_query(), shape, tags)
}

/// One seeded random valid query over the target ledger schema. The
/// grammar is schema-specific by design ([`crate::querygen::target`] is
/// the seam); the config bounds dressing literals (and recomputes
/// in-vocabulary hits — Bytes extrefs, interval windows) so predicates
/// select real subsets.
#[must_use]
pub fn random_query(rng: &mut Rng, cfg: GenConfig) -> Query {
    random_query_tagged(rng, cfg).0
}
