use bumbledb::{AggOp, CmpOp, Comparison, FieldId, FindTerm, RelationId, Term, VarId};

use crate::corpus_gen::Rng;
use crate::querygen::target::ids;
use crate::querygen::{Builder, REPEAT_VAR_PCT};

/// Key-probe-capable relations: (relation, fresh-id field, projectable fields).
const KEY_PROBE_RELATIONS: &[(RelationId, FieldId, &[FieldId])] = &[
    (ids::HOLDER, ids::holder::ID, &[ids::holder::NAME]),
    (
        ids::ACCOUNT,
        ids::account::ID,
        &[ids::account::HOLDER, ids::account::CURRENCY],
    ),
    (
        ids::INSTRUMENT,
        ids::instrument::ID,
        &[ids::instrument::SYMBOL],
    ),
    (
        ids::JOURNAL_ENTRY,
        ids::journal_entry::ID,
        &[ids::journal_entry::SOURCE, ids::journal_entry::CREATED_AT],
    ),
    (
        ids::POSTING,
        ids::posting::ID,
        &[
            ids::posting::ENTRY,
            ids::posting::ACCOUNT,
            ids::posting::AMOUNT,
            ids::posting::AT,
            ids::posting::MEMO,
        ],
    ),
    (ids::ORG, ids::org::ID, &[ids::org::NAME]),
    (
        ids::TRANSFER,
        ids::transfer::ID,
        &[ids::transfer::EXTREF, ids::transfer::WINDOW],
    ),
];

/// Star satellites: (Posting reference field, relation, projected
/// payload field) — each satellite joins on its fresh id (field 0).
const SATELLITES: &[(FieldId, RelationId, FieldId)] = &[
    (
        ids::posting::ENTRY,
        ids::JOURNAL_ENTRY,
        ids::journal_entry::SOURCE,
    ),
    (ids::posting::ACCOUNT, ids::ACCOUNT, ids::account::CURRENCY),
    (
        ids::posting::INSTRUMENT,
        ids::INSTRUMENT,
        ids::instrument::SYMBOL,
    ),
];

/// One atom, fresh id bound to a param — or, a fifth of the time, a
/// param **set** (the point-lookup-over-a-set family) — with 1–2 vars
/// projected.
pub(super) fn key_probe(b: &mut Builder, rng: &mut Rng) {
    let idx = usize::try_from(rng.range(KEY_PROBE_RELATIONS.len() as u64)).expect("small");
    let (relation, id, fields) = KEY_PROBE_RELATIONS[idx];
    let atom = b.add_atom(relation);
    let param = b.fresh_param();
    let term = if rng.chance(1, 5) {
        Term::ParamSet(param)
    } else {
        Term::Param(param)
    };
    b.bind(atom, id, term);
    let take = (1 + usize::try_from(rng.range(2)).expect("small")).min(fields.len());
    let start = usize::try_from(rng.range(fields.len() as u64)).expect("small");
    for k in 0..take {
        let field = fields[(start + k) % fields.len()];
        let var = b.bind_var(atom, field);
        b.find_var(var);
    }
}

/// Posting joined to 1–3 of `JournalEntry`/`Account`/`Instrument` on
/// its reference fields, projecting amount plus each satellite's
/// payload.
pub(super) fn star(b: &mut Builder, rng: &mut Rng) {
    let posting = b.add_atom(ids::POSTING);
    let amount = b.bind_var(posting, ids::posting::AMOUNT);
    b.find_var(amount);
    let take = 1 + usize::try_from(rng.range(3)).expect("small");
    let start = usize::try_from(rng.range(SATELLITES.len() as u64)).expect("small");
    for k in 0..take {
        let (edge, relation, payload) = SATELLITES[(start + k) % SATELLITES.len()];
        let join = b.bind_var(posting, edge);
        let satellite = b.add_atom(relation);
        b.bind(satellite, FieldId(0), Term::Var(join));
        let projected = b.bind_var(satellite, payload);
        b.find_var(projected);
    }
    // The wide-scalar projection (a quarter of stars): every remaining
    // Posting field joins the find list, pushing the projected word
    // count past 8 — the executor's hoist paths are width-unbounded by
    // construction (docs/architecture/40-execution.md, scan-fold
    // pushdown), and the differential oracle keeps that class covered.
    // Before `repeat_var`, so `at` is a fresh variable, never the
    // already-projected amount: 9–11 projected words, all scalar.
    if rng.chance(1, 4) {
        for field in [
            ids::posting::ID,
            ids::posting::ENTRY,
            ids::posting::ACCOUNT,
            ids::posting::INSTRUMENT,
            ids::posting::AT,
            ids::posting::MEMO,
            ids::posting::RECONCILED,
        ] {
            let var = b
                .var_at(posting, field)
                .expect("star binds posting fields to variables only");
            b.find_var(var);
        }
    }
    repeat_var(b, rng, posting);
}

/// Holder ← Account ← Posting (2–3 hops), projecting the ends.
pub(super) fn chain(b: &mut Builder, rng: &mut Rng) {
    let posting = b.add_atom(ids::POSTING);
    let amount = b.bind_var(posting, ids::posting::AMOUNT);
    b.find_var(amount);
    let account_join = b.bind_var(posting, ids::posting::ACCOUNT);
    let account = b.add_atom(ids::ACCOUNT);
    b.bind(account, ids::account::ID, Term::Var(account_join));
    if rng.chance(1, 2) {
        // Three hops: through to Holder, projecting its name.
        let holder_join = b.bind_var(account, ids::account::HOLDER);
        let holder = b.add_atom(ids::HOLDER);
        b.bind(holder, ids::holder::ID, Term::Var(holder_join));
        let name = b.bind_var(holder, ids::holder::NAME);
        b.find_var(name);
    } else {
        let currency = b.bind_var(account, ids::account::CURRENCY);
        b.find_var(currency);
    }
    repeat_var(b, rng, posting);
}

/// Two Posting occurrences equated on `entry`, projecting both amounts
/// — and, half the time, a cross-atom ordered residual between them
/// (`x < y` and friends): residual placement and survivor compaction.
pub(super) fn self_join(b: &mut Builder, rng: &mut Rng) {
    let first = b.add_atom(ids::POSTING);
    let entry = b.bind_var(first, ids::posting::ENTRY);
    let x = b.bind_var(first, ids::posting::AMOUNT);
    let second = b.add_atom(ids::POSTING);
    b.bind(second, ids::posting::ENTRY, Term::Var(entry));
    let y = b.bind_var(second, ids::posting::AMOUNT);
    b.find_var(x);
    b.find_var(y);
    if rng.chance(1, 2) {
        b.conditions.push(Comparison {
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

/// Any join shape re-projected as group-by + one fold aggregate
/// (sometimes two); group key = 0–2 of the shape's bound variables.
/// Aggregate targets cover both integer types: i64 (amount/at) and u64
/// (the posting's account id — Sum over it is provably bounded: the fold
/// is over distinct bindings, so any group's sum is at most
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

/// One order operator, uniformly — applied ONLY to integer-typed
/// variable pairs by every caller: the (order op, non-integer) matrix
/// cells are unemittable because no other construction site exists.
pub(super) fn order_op(rng: &mut Rng) -> CmpOp {
    match rng.range(4) {
        0 => CmpOp::Lt,
        1 => CmpOp::Le,
        2 => CmpOp::Gt,
        _ => CmpOp::Ge,
    }
}
