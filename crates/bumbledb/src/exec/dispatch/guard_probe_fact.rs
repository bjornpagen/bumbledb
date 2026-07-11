use super::fact_word::{fact_operand, FactOperand};
use super::GuardPlan;
use crate::error::Result;
use crate::image::view::{Const, FilterPredicate, ResolvedWordSource};
use crate::ir::CmpOp;
use crate::obs;
use crate::schema::Schema;
use crate::storage::env::ReadTxn;
use crate::storage::{dict, read};

/// Resolves a constant to its canonical key-segment bytes — per field, the
/// same canonical encoding [`crate::storage::keys::guard_bytes`] slices out
/// of a stored fact (`U` guards and `M` fact bytes share it, so an
/// interval constant contributes its whole 16-byte `start ‖ end` piece).
/// A `PendingIntern` that missed the dictionary resolves to the
/// never-minted sentinel id — the ensuing `U`/`M` probe then misses (empty
/// result), never an insert, never an error.
fn const_bytes(
    txn: &ReadTxn<'_>,
    value: &Const,
    params: &[Const],
    out: &mut Vec<u8>,
) -> Result<()> {
    match value {
        Const::Word(w) => out.extend_from_slice(&w.to_be_bytes()),
        Const::Byte(b) => out.push(*b),
        Const::Interval { start, end } => {
            out.extend_from_slice(&start.to_be_bytes());
            out.extend_from_slice(&end.to_be_bytes());
        }
        Const::Param(p) => {
            return const_bytes(txn, &params[usize::from(p.0)], params, out);
        }
        Const::ParamSet(_) | Const::WordSet(_) => {
            unreachable!("classification: a param-set binding never reaches the guard path")
        }
        Const::PendingIntern { tag, bytes } => {
            let id = dict::lookup(txn, *tag, bytes)?.unwrap_or(dict::SENTINEL_ID);
            out.extend_from_slice(&id.to_be_bytes());
        }
    }
    Ok(())
}

/// A filter constant in column form (for checks on the fetched fact). A
/// dictionary miss resolves to the sentinel id, so `Eq` filters fail and
/// `Ne` filters pass — per-operator miss semantics with no special cases.
/// Bytes widen to words like [`FactOperand`], so scalar comparison is one
/// word shape.
fn const_operand(txn: &ReadTxn<'_>, value: &Const, params: &[Const]) -> Result<FactOperand> {
    match value {
        Const::Word(w) => Ok(FactOperand::Word(*w)),
        Const::Byte(b) => Ok(FactOperand::Word(u64::from(*b))),
        Const::Interval { start, end } => Ok(FactOperand::Pair(*start, *end)),
        Const::Param(p) => const_operand(txn, &params[usize::from(p.0)], params),
        Const::ParamSet(_) | Const::WordSet(_) => {
            unreachable!("classification: a param-set binding never reaches the guard path")
        }
        Const::PendingIntern { tag, bytes } => Ok(FactOperand::Word(
            dict::lookup(txn, *tag, bytes)?.unwrap_or(dict::SENTINEL_ID),
        )),
    }
}

/// A membership filter's resolved point word (never var-sourced here:
/// classification routes var points to Free Join).
fn point_word(point: &ResolvedWordSource, params: &[Const]) -> u64 {
    match point {
        ResolvedWordSource::Word(word) => *word,
        ResolvedWordSource::Param(param) => match &params[usize::from(param.0)] {
            Const::Word(word) => *word,
            _ => unreachable!("validated: a point param resolves to a word"),
        },
        ResolvedWordSource::Var(_) => {
            unreachable!("classification: a var-sourced point never reaches the guard path")
        }
    }
}

/// Point membership under the half-open interval: `start ≤ p AND p < end`.
const fn contains_point(start: u64, end: u64, p: u64) -> bool {
    start <= p && p < end
}

/// Evaluates one residual filter on the fetched fact's bytes — the same
/// word compositions the view evaluator runs over image columns
/// (`image::view::apply`), sourced from [`fact_operand`] instead.
fn fact_matches(
    txn: &ReadTxn<'_>,
    schema: &Schema,
    plan: &GuardPlan,
    fact: &[u8],
    filter: &FilterPredicate,
    params: &[Const],
) -> Result<bool> {
    let operand = |field| fact_operand(schema, plan.relation, fact, field);
    let pair = |field| match operand(field) {
        FactOperand::Pair(start, end) => (start, end),
        FactOperand::Word(_) => unreachable!("validated: interval predicates read interval fields"),
    };
    let word = |field| match operand(field) {
        FactOperand::Word(word) => word,
        FactOperand::Pair(..) => unreachable!("validated: point operands are scalar fields"),
    };
    Ok(match filter {
        FilterPredicate::Compare { field, op, value } => {
            match (operand(*field), const_operand(txn, value, params)?) {
                (FactOperand::Word(w), FactOperand::Word(c)) => op.compare(&w, &c),
                // Interval-vs-interval-constant: value equality only
                // (interval-pair *predicates* are the Allen kinds below).
                (FactOperand::Pair(s, e), FactOperand::Pair(start, end)) => match op {
                    CmpOp::Eq => s == start && e == end,
                    _ => unreachable!("validated: interval constants compare under Eq only"),
                },
                _ => unreachable!("validated: filter constants match their field's shape"),
            }
        }
        FilterPredicate::FieldsCompare { left, right, op } => {
            match (operand(*left), operand(*right)) {
                (FactOperand::Word(a), FactOperand::Word(b)) => op.compare(&a, &b),
                // Interval fields compare pairwise; validation admits
                // Eq/Ne only.
                (FactOperand::Pair(a_s, a_e), FactOperand::Pair(b_s, b_e)) => match op {
                    CmpOp::Eq => a_s == b_s && a_e == b_e,
                    CmpOp::Ne => a_s != b_s || a_e != b_e,
                    _ => unreachable!("validated: no order comparison over intervals"),
                },
                _ => unreachable!("same-fact comparison joins same-typed fields"),
            }
        }
        FilterPredicate::PointIn { field, point } => {
            let (start, end) = pair(*field);
            contains_point(start, end, point_word(point, params))
        }
        FilterPredicate::AnyPointIn { .. } => {
            unreachable!("classification: a param-set binding never reaches the guard path")
        }
        // The Allen kinds: classify-then-test, exactly as the view
        // evaluator runs them (`image::view::apply`) — encoded words
        // preserve value order, so classification over fact words equals
        // classification over values.
        FilterPredicate::FieldsAllen { left, right, mask } => {
            let (l_start, l_end) = pair(*left);
            let (r_start, r_end) = pair(*right);
            crate::image::view::mask_of(*mask, params).contains(crate::allen::classify_bounds(
                &l_start, &l_end, &r_start, &r_end,
            ))
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let (f_start, f_end) = pair(*field);
            let FactOperand::Pair(start, end) = const_operand(txn, other, params)? else {
                unreachable!("validated: the Allen constant side is an interval")
            };
            crate::image::view::mask_of(*mask, params).contains(crate::allen::classify_bounds(
                &f_start, &f_end, &start, &end,
            ))
        }
        FilterPredicate::FieldsContainPoint { interval, point } => {
            let (start, end) = pair(*interval);
            contains_point(start, end, word(*point))
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let FactOperand::Pair(start, end) = const_operand(txn, outer, params)? else {
                unreachable!("validated: the outer side is an interval constant")
            };
            match operand(*field) {
                FactOperand::Word(w) => contains_point(start, end, w),
                FactOperand::Pair(..) => {
                    unreachable!("validated: within-comparands are scalar words")
                }
            }
        }
        // Measure filters disqualify guard classification (`classify`):
        // their evaluation is fallible and filter-ordered — the filtered
        // view's job, never the guard's.
        FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. } => {
            unreachable!("classify refused measure filters")
        }
    })
}

/// The probe half of the guard: key bytes from constants, one `U` get
/// through the matched key statement (or the full-fact `M` get), one `F`
/// fetch, remaining filters on the fact bytes. `None` = miss or a failed
/// filter — an empty result, never an error.
///
/// # Errors
///
/// `Lmdb`/`Corruption` from the storage reads.
pub(crate) fn guard_probe_fact<'t>(
    plan: &GuardPlan,
    txn: &'t ReadTxn<'_>,
    schema: &Schema,
    params: &[Const],
    key_scratch: &mut Vec<u8>,
) -> Result<Option<&'t [u8]>> {
    // Build the key bytes in the caller's reused scratch — the statement's
    // projection order for a `U` guard, full canonical fact bytes for `M`.
    // A dictionary miss lands the sentinel id in the key, and the probe
    // below misses.
    key_scratch.clear();
    for (_, value) in &plan.key {
        const_bytes(txn, value, params, key_scratch)?;
    }

    let mut probe_span = obs::span(obs::names::GUARD_PROBE, obs::Category::Execute);
    let row_id = match plan.statement {
        Some(statement) => read::guard_row(txn, plan.relation, statement, key_scratch)?,
        None => read::fact_row(txn, plan.relation, key_scratch)?,
    };
    probe_span.set_args(u64::from(row_id.is_some()), 0);
    let Some(row_id) = row_id else {
        return Ok(None); // miss: empty result
    };
    let fact = read::fetch(txn, schema, plan.relation, row_id)?;

    // Remaining filters run on the fact bytes.
    for filter in &plan.remaining_filters {
        if !fact_matches(txn, schema, plan, fact, filter, params)? {
            return Ok(None);
        }
    }
    Ok(Some(fact))
}
