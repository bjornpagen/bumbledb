use bumbledb::{Query, Value};

use crate::gen::{self, GenConfig, Rng, Sizes};
use crate::querygen::dress::dress;
use crate::querygen::shapes::{aggregate, chain, guard, self_join, star};
use crate::querygen::{Builder, GenTags, Shape, SHAPE_WEIGHTS};
use crate::schema::ids;

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

fn build(rng: &mut Rng, shape: Shape, cfg: GenConfig, sizes: &Sizes) -> Builder {
    let mut b = Builder::default();
    match shape {
        Shape::Guard => guard(&mut b, rng),
        Shape::Star => star(&mut b, rng),
        Shape::Chain => chain(&mut b, rng),
        Shape::SelfJoin => self_join(&mut b, rng),
        Shape::Gated => {
            match rng.range(5) {
                0 => guard(&mut b, rng),
                1 => star(&mut b, rng),
                2 => chain(&mut b, rng),
                3 => aggregate(&mut b, rng),
                _ => self_join(&mut b, rng),
            }
            // The zero-binding nonemptiness gate, over either non-empty
            // relation (falsity is the empty-store pass's job; diversity
            // here is about relation shape).
            b.atom(if rng.chance(1, 2) {
                ids::TAG
            } else {
                ids::TAG_NOTE
            });
        }
        Shape::Aggregate => aggregate(&mut b, rng),
    }
    dress(&mut b, rng, cfg, sizes);
    b
}

pub(super) fn random_query_tagged(rng: &mut Rng, cfg: GenConfig) -> (Query, Shape, GenTags) {
    let sizes = Sizes::of(cfg.scale);
    let shape = shape_of(rng);
    let b = build(rng, shape, cfg, &sizes);
    let tags = GenTags {
        miss: b.miss,
        bytes_hit: b.bytes_hit,
        bytes_miss: b.bytes_miss,
    };
    (b.into_query(), shape, tags)
}

/// The seeded extref of one Transfer row — corpus rows are a pure
/// function of the config, so in-vocabulary Bytes literals recompute.
pub(super) fn extref_of(cfg: GenConfig, sizes: &Sizes, row: u64) -> Value {
    gen::row(&cfg, sizes, ids::TRANSFER, row)
        .into_iter()
        .nth(usize::from(ids::transfer::EXTREF.0))
        .expect("transfer rows carry extref")
}

/// One seeded random valid query over the ledger schema. The schema is
/// the ledger (the grammar is schema-specific by design); the config
/// bounds dressing literals (and recomputes in-vocabulary Bytes hits)
/// so predicates select real subsets.
#[must_use]
pub fn random_query(rng: &mut Rng, cfg: GenConfig) -> Query {
    random_query_tagged(rng, cfg).0
}
