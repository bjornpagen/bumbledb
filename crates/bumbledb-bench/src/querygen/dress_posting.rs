use bumbledb::{CmpOp, Comparison, FieldId, Term, Value};

use crate::gen::{self, Rng, Sizes};
use crate::querygen::dress::{any_op, i64_dress};
use crate::querygen::Builder;
use crate::schema::ids;

/// A u64 predicate on a dense-id field (any operator): the literal or
/// param draws in-domain, so ordered comparisons select real slices.
fn u64_dress(b: &mut Builder, rng: &mut Rng, atom: usize, field: FieldId, domain: u64) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = any_op(rng);
    let rhs = if rng.chance(1, 2) {
        Term::Literal(Value::U64(rng.range(domain.max(1))))
    } else {
        Term::Param(b.fresh_param())
    };
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(var),
        rhs,
    });
}

/// The i64 windows the corpus draws from, per field (dressing literals
/// land inside them so range predicates select real subsets).
pub(super) fn posting_at_window(sizes: &Sizes) -> (i64, i64) {
    let span = i64::try_from(sizes.postings).expect("fits") * gen::AT_STEP;
    (gen::AT_BASE, gen::AT_BASE + span)
}

/// One dressing predicate on a Posting atom.
pub(super) fn dress_posting(b: &mut Builder, rng: &mut Rng, atom: usize, sizes: &Sizes) {
    match rng.range(6) {
        0 => i64_dress(b, rng, atom, ids::posting::AMOUNT, -5_000_000, 5_000_000),
        5 => {
            // U64 dressing on a dense-id FK field: ordered comparisons
            // (and Eq/Ne) over real id slices.
            let (field, domain) = match rng.range(3) {
                0 => (ids::posting::ACCOUNT, sizes.accounts),
                1 => (ids::posting::INSTRUMENT, sizes.instruments),
                _ => (ids::posting::TRANSFER, sizes.transfers),
            };
            u64_dress(b, rng, atom, field, domain);
        }
        1 => {
            let (lo, hi) = posting_at_window(sizes);
            i64_dress(b, rng, atom, ids::posting::AT, lo, hi);
        }
        2 => {
            // Eq/Ne on memo: in-vocabulary literal, out-of-vocabulary
            // literal (the miss path), or a param — equal weight.
            let Some(var) = b.var_at(atom, ids::posting::MEMO) else {
                return;
            };
            let op = if rng.chance(1, 2) {
                CmpOp::Eq
            } else {
                CmpOp::Ne
            };
            let rhs = match rng.range(3) {
                0 => Term::Literal(Value::String(
                    format!("m{}", rng.range(gen::MEMO_VOCAB))
                        .into_bytes()
                        .into(),
                )),
                1 => {
                    b.miss = true;
                    Term::Literal(Value::String(
                        format!("missing-{}", rng.u64()).into_bytes().into(),
                    ))
                }
                _ => Term::Param(b.fresh_param()),
            };
            b.predicates.push(Comparison {
                op,
                lhs: Term::Var(var),
                rhs,
            });
        }
        3 => {
            let Some(var) = b.var_at(atom, ids::posting::RECONCILED) else {
                return;
            };
            let op = if rng.chance(1, 2) {
                CmpOp::Eq
            } else {
                CmpOp::Ne
            };
            b.predicates.push(Comparison {
                op,
                lhs: Term::Var(var),
                rhs: Term::Literal(Value::Bool(rng.chance(1, 2))),
            });
        }
        _ => {
            // Same-atom var-vs-var: amount vs at, the same-typed (i64)
            // pair. Skipped when the repeated-var pass fused them (a
            // self-comparison is invalid by the roster).
            let (Some(amount), Some(at)) = (
                b.var_at(atom, ids::posting::AMOUNT),
                b.var_at(atom, ids::posting::AT),
            ) else {
                return;
            };
            if amount == at {
                return;
            }
            let op = match rng.range(6) {
                0 => CmpOp::Eq,
                1 => CmpOp::Ne,
                2 => CmpOp::Lt,
                3 => CmpOp::Le,
                4 => CmpOp::Gt,
                _ => CmpOp::Ge,
            };
            b.predicates.push(Comparison {
                op,
                lhs: Term::Var(amount),
                rhs: Term::Var(at),
            });
        }
    }
}
