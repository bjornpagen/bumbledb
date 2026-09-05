use bumbledb::Query;

use crate::corpus_gen::{GenConfig, Rng};
use crate::querygen::dress::dress;
use crate::querygen::negate::negate;
use crate::querygen::shapes::{aggregate, chain, key_probe, self_join, star};
use crate::querygen::shapes_closed::{closed_join, ground_fold};
use crate::querygen::shapes_ground::{du_walk, existence_walk};
use crate::querygen::shapes_interval::{boundary, interval_join, measure, membership, pack};
use crate::querygen::shapes_rules::rules;
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
            match rng.range(4) {
                0 => key_probe(&mut b, rng),
                1 => star(&mut b, rng),
                2 => chain(&mut b, rng),
                _ => aggregate(&mut b, rng),
            }

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
        Shape::ExistenceWalk => existence_walk(&mut b, rng),
        Shape::DuWalk => du_walk(&mut b, rng),
        Shape::ClosedJoin => closed_join(&mut b, rng),
        Shape::GroundFold => ground_fold(&mut b, rng),
        Shape::Pack => pack(&mut b, rng),
        Shape::Measure => measure(&mut b, rng, cfg, domains),
        Shape::ScalarFloat => scalar_float(&mut b, rng),
        Shape::Rules => unreachable!("multi-rule queries assemble their own query"),
    }

    // would flip an eliminable shape to a refusal (or blur the counted

    if !matches!(
        shape,
        Shape::ExistenceWalk
            | Shape::DuWalk
            | Shape::ClosedJoin
            | Shape::GroundFold
            | Shape::ScalarFloat
    ) {
        dress(&mut b, rng, cfg, domains);

        negate(&mut b, rng);
    }
    b
}

fn scalar_float(b: &mut Builder, rng: &mut Rng) {
    use bumbledb::{CmpOp, Comparison, F64, FieldId, Term, Value};
    let atom = b.add_atom(ids::FLOAT_VALUE);
    let id = b.bind_var(atom, FieldId(0));
    let value = b.bind_var(atom, FieldId(1));
    let ops = [
        CmpOp::Eq,
        CmpOp::Ne,
        CmpOp::Lt,
        CmpOp::Le,
        CmpOp::Gt,
        CmpOp::Ge,
    ];
    let op = ops[usize::try_from(rng.range(ops.len() as u64)).expect("six comparisons")];
    let rhs = if op == CmpOp::Eq && rng.chance(1, 3) {
        Term::ParamSet(b.fresh_param())
    } else if rng.chance(1, 2) {
        Term::Param(b.fresh_param())
    } else {
        let bits = super::target::FLOAT_BITS[usize::try_from(
            rng.range(super::target::FLOAT_BITS.len() as u64),
        )
        .expect("small vocabulary")];
        Term::Literal(Value::F64(F64::from_bits(bits)))
    };
    b.conditions.push(Comparison {
        op,
        lhs: Term::Var(value),
        rhs,
    });
    b.find_var(id);
    b.find_var(value);
}

pub(super) fn random_query_tagged(rng: &mut Rng, cfg: GenConfig) -> (Query, Shape, GenTags) {
    let domains = Domains::of(cfg.scale);
    let shape = shape_of(rng);
    if shape == Shape::Rules {
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

#[must_use]
pub fn random_cq_query(rng: &mut Rng, cfg: GenConfig) -> Query {
    random_query_tagged(rng, cfg).0
}

#[must_use]
pub fn random_query(rng: &mut Rng, cfg: GenConfig) -> Query {
    match QueryClass::draw(rng) {
        QueryClass::Cq => random_cq_query(rng, cfg),
        QueryClass::Derived => super::random_reach_query(rng, cfg).0,
    }
}

enum QueryClass {
    Cq,
    Derived,
}

impl QueryClass {
    fn draw(rng: &mut Rng) -> Self {
        if rng.range(8) == 0 {
            Self::Derived
        } else {
            Self::Cq
        }
    }
}
