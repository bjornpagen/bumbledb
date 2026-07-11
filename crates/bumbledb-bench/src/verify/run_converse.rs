//! The converse property lane (`docs/architecture/60-validation.md`
//! § the two oracles, the Allen families): `Allen(a, b, m) ≡
//! Allen(b, a, converse(m))` is a theorem of the coordinate system, so
//! for every generated query carrying literal-mask `Allen` predicates
//! the *converse twin* — every Allen leaf's operands swapped and its
//! mask conversed — must produce the identical result set on the
//! engine. The twin is built mechanically from the original
//! ([`converse_twin`]), so the lane quantifies over exactly the
//! generator's mask distribution: named composites, singletons, and
//! random masks, over both element lanes and every operand shape.

use bumbledb::{CmpOp, MaskTerm, PredicateTree, Query};

use crate::differential::engine_query;
use crate::gen::Rng;
use crate::querygen::{self, target};
use crate::verify::{Run, VerifyConfig, MAX_BUNDLES};

use super::run::positional;

/// How many converse pairs one verify run compares.
const CONVERSE_CASES: u32 = 100;

/// The converse twin: every literal-mask `Allen` leaf with operands
/// swapped and the mask conversed — semantics-preserving per leaf by
/// the converse theorem, so the whole query's denotation is unchanged.
/// `None` when the query carries no `Allen` predicate.
fn converse_twin(query: &Query) -> Option<Query> {
    let mut twin = query.clone();
    let mut any = false;
    for rule in &mut twin.rules {
        for tree in &mut rule.predicates {
            let PredicateTree::Leaf(comparison) = tree else {
                continue; // the generator emits flat conjunctions
            };
            if let CmpOp::Allen {
                mask: MaskTerm::Literal(mask),
            } = comparison.op
            {
                comparison.op = CmpOp::Allen {
                    mask: MaskTerm::Literal(mask.converse()),
                };
                std::mem::swap(&mut comparison.lhs, &mut comparison.rhs);
                any = true;
            }
        }
    }
    any.then_some(twin)
}

/// Draws generated queries until [`CONVERSE_CASES`] Allen-bearing ones
/// have been twinned and compared engine-vs-engine (rows and error
/// verdicts alike — a `MeasureOfRay` on one side of a converse pair
/// would be its own bug).
pub(super) fn converse_lane(run: &mut Run<'_, target::Target>, cfg: &VerifyConfig) {
    let mut rng = Rng::new(cfg.gen.seed ^ 0x0115_C09E);
    let mut compared = 0u32;
    // A generous draw budget: Allen-bearing shapes are a sizable band,
    // so the budget is slack, not a hidden skip.
    for _ in 0..CONVERSE_CASES * 20 {
        if compared >= CONVERSE_CASES || run.bundles.len() >= MAX_BUNDLES {
            break;
        }
        let query = querygen::random_query(&mut rng, cfg.gen);
        let Some(twin) = converse_twin(&query) else {
            continue;
        };
        let Some(draw) = querygen::params_for(&query, &mut rng, cfg.gen)
            .into_iter()
            .next()
        else {
            continue;
        };
        let params = positional(&draw);
        let original = engine_query(run.db, &query, &params);
        let conversed = engine_query(run.db, &twin, &params);
        run.cases += 1;
        compared += 1;
        if original != conversed {
            let bundle = run.out_dir.join(format!("mismatch-{}", run.bundles.len()));
            std::fs::create_dir_all(&bundle).expect("bundle dir");
            std::fs::write(
                bundle.join("mismatch.txt"),
                format!(
                    "converse property violated: swapping Allen operands and \
                     conversing the mask changed the result\n\
                     original:\n{query:#?}\ntwin:\n{twin:#?}\n\
                     original result:\n{original:#?}\ntwin result:\n{conversed:#?}\n"
                ),
            )
            .expect("bundle");
            eprintln!("verify: CONVERSE MISMATCH -> {}", bundle.display());
            run.bundles.push(bundle);
        }
    }
    assert!(
        compared >= CONVERSE_CASES / 2,
        "the converse lane must actually run (compared {compared})"
    );
}
