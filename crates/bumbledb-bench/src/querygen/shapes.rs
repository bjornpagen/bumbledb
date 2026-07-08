use bumbledb::{AggOp, CmpOp, Comparison, FieldId, FindTerm, Term, VarId};

use crate::gen::Rng;
use crate::querygen::{Builder, GUARDABLE, REPEAT_VAR_PCT, SATELLITES};
use crate::schema::ids;

/// One atom, serial id bound to a param, 1–2 vars projected.
pub(super) fn guard(b: &mut Builder, rng: &mut Rng) {
    let idx = usize::try_from(rng.range(GUARDABLE.len() as u64)).expect("small");
    let (relation, id, fields) = GUARDABLE[idx];
    let atom = b.atom(relation);
    let param = b.fresh_param();
    b.bind(atom, id, Term::Param(param));
    let take = 1 + usize::try_from(rng.range(2)).expect("small");
    let start = usize::try_from(rng.range(fields.len() as u64)).expect("small");
    for k in 0..take.min(fields.len()) {
        let field = fields[(start + k) % fields.len()];
        let var = b.bind_var(atom, field);
        b.find_var(var);
    }
}

/// Posting joined to 1–3 of {Account, Instrument, Transfer} on its FK
/// fields, projecting amount plus each satellite's payload.
pub(super) fn star(b: &mut Builder, rng: &mut Rng) {
    let posting = b.atom(ids::POSTING);
    let amount = b.bind_var(posting, ids::posting::AMOUNT);
    b.find_var(amount);
    let take = 1 + usize::try_from(rng.range(3)).expect("small");
    let start = usize::try_from(rng.range(SATELLITES.len() as u64)).expect("small");
    for k in 0..take {
        let (fk, relation, payload) = SATELLITES[(start + k) % SATELLITES.len()];
        let join = b.bind_var(posting, fk);
        let satellite = b.atom(relation);
        b.bind(satellite, FieldId(0), Term::Var(join));
        let projected = b.bind_var(satellite, payload);
        b.find_var(projected);
    }
    repeat_var(b, rng, posting);
}

/// Holder ← Account ← Posting (2–3 hops), projecting the ends.
pub(super) fn chain(b: &mut Builder, rng: &mut Rng) {
    let posting = b.atom(ids::POSTING);
    let amount = b.bind_var(posting, ids::posting::AMOUNT);
    b.find_var(amount);
    let account_join = b.bind_var(posting, ids::posting::ACCOUNT);
    let account = b.atom(ids::ACCOUNT);
    b.bind(account, ids::account::ID, Term::Var(account_join));
    if rng.chance(1, 2) {
        // Three hops: through to Holder, projecting its name.
        let holder_join = b.bind_var(account, ids::account::HOLDER);
        let holder = b.atom(ids::HOLDER);
        b.bind(holder, ids::holder::ID, Term::Var(holder_join));
        let name = b.bind_var(holder, ids::holder::NAME);
        b.find_var(name);
    } else {
        let opened = b.bind_var(account, ids::account::OPENED_AT);
        b.find_var(opened);
    }
    repeat_var(b, rng, posting);
}

/// Two Posting occurrences equated on `transfer`, projecting both
/// amounts — and, half the time, a cross-atom ordered residual between
/// them (`x < y` and friends): the randomized twin of the spread
/// family, exercising residual placement and survivor compaction.
pub(super) fn self_join(b: &mut Builder, rng: &mut Rng) {
    let first = b.atom(ids::POSTING);
    let transfer = b.bind_var(first, ids::posting::TRANSFER);
    let x = b.bind_var(first, ids::posting::AMOUNT);
    let second = b.atom(ids::POSTING);
    b.bind(second, ids::posting::TRANSFER, Term::Var(transfer));
    let y = b.bind_var(second, ids::posting::AMOUNT);
    b.find_var(x);
    b.find_var(y);
    if rng.chance(1, 2) {
        b.predicates.push(Comparison {
            op: order_op(rng),
            lhs: Term::Var(x),
            rhs: Term::Var(y),
        });
    }
    repeat_var(b, rng, first);
}

/// The repeated in-atom variable ([`REPEAT_VAR_PCT`]% of qualifying
/// Posting atoms): `at` rebound to the `amount` variable — two same-typed
/// (i64) fields of one atom carrying one variable.
fn repeat_var(b: &mut Builder, rng: &mut Rng, posting: usize) {
    if !rng.chance(REPEAT_VAR_PCT, 100) {
        return;
    }
    let amount = b.atoms[posting]
        .bindings
        .iter()
        .find_map(|(f, t)| (*f == ids::posting::AMOUNT).then(|| t.clone()));
    let at_free = !b.atoms[posting]
        .bindings
        .iter()
        .any(|(f, _)| *f == ids::posting::AT);
    if let (Some(term @ Term::Var(_)), true) = (amount, at_free) {
        b.bind(posting, ids::posting::AT, term);
    }
}

/// Any join shape re-projected as group-by + one aggregate (sometimes
/// two); group key = 0–2 of the shape's bound variables. Aggregate
/// targets cover both integer types: i64 (amount/at) and u64 (the
/// posting's account id — Sum over it is provably bounded: the fold is
/// over distinct bindings, so any group's sum is at most
/// postings × accounts ≤ 10⁷ × 5 × 10⁴ = 5 × 10¹¹ ≪ 2⁶³ at every scale,
/// satisfying the Sum-range rule). A fifth of the time the posting's
/// bool field joins the group-key candidates.
pub(super) fn aggregate(b: &mut Builder, rng: &mut Rng) {
    if rng.chance(1, 2) {
        star(b, rng);
    } else {
        chain(b, rng);
    }
    let amount = b
        .var_at(0, ids::posting::AMOUNT)
        .expect("shape binds amount");
    let at = b.var_at(0, ids::posting::AT).expect("var or fresh");
    if rng.chance(1, 5) {
        // A bool group-key candidate (registered by bind_var).
        let _ = b.var_at(0, ids::posting::RECONCILED);
    }
    let (op, over) = match rng.range(7) {
        0 => (AggOp::Sum, Some(amount)),
        1 => (AggOp::Count, None),
        2 => (AggOp::Min, Some(at)),
        3 => (AggOp::Max, Some(amount)),
        // The u64 targets (account: dense ids, bounded sums).
        4 => (AggOp::Sum, b.var_at(0, ids::posting::ACCOUNT)),
        5 => (AggOp::Min, b.var_at(0, ids::posting::ACCOUNT)),
        _ => (AggOp::Max, b.var_at(0, ids::posting::ACCOUNT)),
    };
    let candidates: Vec<VarId> = b
        .bound
        .iter()
        .copied()
        .filter(|var| Some(*var) != over)
        .collect();
    let group = usize::try_from(rng.range(3))
        .expect("small")
        .min(candidates.len());
    let start = if candidates.is_empty() {
        0
    } else {
        usize::try_from(rng.range(candidates.len() as u64)).expect("small")
    };
    b.finds.clear();
    let mut key: Vec<VarId> = (0..group)
        .map(|k| candidates[(start + k) % candidates.len()])
        .collect();
    key.sort_unstable();
    key.dedup();
    let in_key = |var: Option<VarId>| var.is_some_and(|v| key.contains(&v));
    for var in &key {
        b.find_var(*var);
    }
    b.finds.push(FindTerm::Aggregate { op, over });
    // Multi-aggregate finds, a quarter of the time: Count beside any
    // valued aggregate (always distinct), or Sum(amount) beside Count
    // when amount stays off the group key.
    if rng.chance(1, 4) {
        let amount_term = FindTerm::Aggregate {
            op: AggOp::Sum,
            over: Some(amount),
        };
        if op == AggOp::Count {
            if !in_key(Some(amount)) {
                b.finds.push(amount_term);
            }
        } else {
            b.finds.push(FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            });
        }
    }
}

/// One order operator, uniformly.
fn order_op(rng: &mut Rng) -> CmpOp {
    match rng.range(4) {
        0 => CmpOp::Lt,
        1 => CmpOp::Le,
        2 => CmpOp::Gt,
        _ => CmpOp::Ge,
    }
}
