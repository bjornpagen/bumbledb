use bumbledb::{CmpOp, Comparison, Term, Value};

use crate::corpus_gen::Rng;
use crate::querygen::Builder;
use crate::querygen::dress::{at_window, eq_ne, i64_dress, string_cmp, u64_dress};
use crate::querygen::target::{AMOUNT_LEVELS, AMOUNT_STEP, Domains, ids};

/// One dressing predicate on a Posting atom.
pub(super) fn dress_posting(b: &mut Builder, rng: &mut Rng, atom: usize, domains: &Domains) {
    match rng.range(6) {
        // The quantized amount window (the corpus draws its 8 levels
        // inside it, so range predicates select real subsets).
        0 => i64_dress(
            b,
            rng,
            atom,
            ids::posting::AMOUNT,
            -(AMOUNT_LEVELS / 2) * AMOUNT_STEP,
            (AMOUNT_LEVELS / 2) * AMOUNT_STEP,
        ),
        1 => {
            let (lo, hi) = at_window(domains);
            i64_dress(b, rng, atom, ids::posting::AT, lo, hi);
        }
        // Eq/Ne on memo: in-vocabulary hit, out-of-vocabulary miss (the
        // miss path), param, or set.
        2 => string_cmp(b, rng, atom, ids::POSTING, ids::posting::MEMO),
        3 => {
            let Some(var) = b.var_at(atom, ids::posting::RECONCILED) else {
                return;
            };
            b.predicates.push(Comparison {
                op: eq_ne(rng),
                lhs: Term::Var(var),
                rhs: Term::Literal(Value::Bool(rng.chance(1, 2))),
            });
        }
        // U64 dressing on a dense-id reference field: ordered
        // comparisons (and Eq/Ne) over real id slices.
        4 => {
            let (field, domain) = match rng.range(3) {
                0 => (ids::posting::ENTRY, domains.entries),
                1 => (ids::posting::ACCOUNT, domains.accounts),
                _ => (ids::posting::INSTRUMENT, domains.instruments),
            };
            u64_dress(b, rng, atom, field, domain);
        }
        // Same-atom var-vs-var: amount vs at, the same-typed (i64)
        // pair. Skipped when the repeated-var pass fused them (a
        // self-comparison is invalid by the roster).
        _ => {
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
