//! The interval-surface shapes: point membership (literal, param, and
//! var points), interval joins (`Overlaps`/`Contains`/`Eq`/`Ne`), and
//! the adjacent-touching boundary probes whose literals are recomputed
//! from the corpus interval generator — the query touches a corpus
//! interval at exactly its endpoint, both polarities.
//!
//! Interval operators are constructed **only here**, and only over
//! interval-typed terms; order operators never touch these shapes: the
//! illegal (operator, type) matrix cells are unemittable by
//! construction.

use bumbledb::{CmpOp, Comparison, Term, Value, VarId};

use crate::gen::{GenConfig, Rng};
use crate::querygen::interval_data;
use crate::querygen::target::{ids, Domains};
use crate::querygen::Builder;

/// The collision-group pool query literals draw from — small enough
/// that every drawn group exists at every scale.
const GROUP_POOL: u64 = 64;

/// The width of a query literal constructed to touch a corpus interval.
const TOUCH_WIDTH: u64 = 64;

/// A midpoint of the group's parent interval (`k = 2`) — an in-data
/// point for membership probes.
fn i64_point(cfg: GenConfig, rng: &mut Rng) -> i64 {
    let (start, end) = interval_data::group_i64(cfg.seed, rng.range(GROUP_POOL), 2);
    start + (end - start) / 2
}

fn u64_point(cfg: GenConfig, rng: &mut Rng) -> u64 {
    let (start, end) = interval_data::group_u64(cfg.seed, rng.range(GROUP_POOL), 2);
    start + (end - start) / 2
}

/// An in-data interval literal (any shape of a drawn group).
fn i64_interval(cfg: GenConfig, rng: &mut Rng) -> Value {
    let (start, end) = interval_data::group_i64(
        cfg.seed,
        rng.range(GROUP_POOL),
        rng.range(interval_data::PER_GROUP),
    );
    Value::IntervalI64(start, end)
}

fn u64_interval(cfg: GenConfig, rng: &mut Rng) -> Value {
    let (start, end) = interval_data::group_u64(
        cfg.seed,
        rng.range(GROUP_POOL),
        rng.range(interval_data::PER_GROUP),
    );
    Value::IntervalU64(start, end)
}

/// Point membership against an interval field. The point term is a
/// literal, a param, or a variable — and the var and param cases
/// **construct** their scalar anchor (a Posting scalar binding) first,
/// deliberately: a point term with no enumerable domain is invalid by
/// the roster, so the anchor is never left to chance.
pub(super) fn membership(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    if rng.chance(3, 5) {
        membership_i64(b, rng, cfg, domains);
    } else {
        membership_u64(b, rng, cfg);
    }
}

/// Mandate-at-instant over the I64 element lane:
/// `Posting(account = a, at = t), Mandate(account = a, active ∋ t)` and
/// its param/literal-point variants.
fn membership_i64(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    let org;
    match rng.range(3) {
        // Var point: `at` is the scalar anchor, constructed here.
        0 => {
            let posting = b.atom(ids::POSTING);
            let account = b.bind_var(posting, ids::posting::ACCOUNT);
            let at = b.bind_var(posting, ids::posting::AT);
            let mandate = b.atom(ids::MANDATE);
            b.bind(mandate, ids::mandate::ACCOUNT, Term::Var(account));
            org = b.bind_var(mandate, ids::mandate::ORG);
            b.bind(mandate, ids::mandate::ACTIVE, Term::Var(at));
            b.find_var(account);
        }
        // Param point: the param's scalar anchor is a Posting.at
        // binding — mandates covering a probed instant.
        1 => {
            let posting = b.atom(ids::POSTING);
            let account = b.bind_var(posting, ids::posting::ACCOUNT);
            let point = b.fresh_param();
            b.bind(posting, ids::posting::AT, Term::Param(point));
            let mandate = b.atom(ids::MANDATE);
            b.bind(mandate, ids::mandate::ACCOUNT, Term::Var(account));
            org = b.bind_var(mandate, ids::mandate::ORG);
            b.bind(mandate, ids::mandate::ACTIVE, Term::Param(point));
            b.find_var(account);
        }
        // Literal point: unambiguous (element-typed literal at an
        // interval field IS membership); the account pin is a literal
        // or — the account-set flavor — a param set.
        _ => {
            let mandate = b.atom(ids::MANDATE);
            let account_term = if rng.chance(2, 5) {
                Term::ParamSet(b.fresh_param())
            } else {
                Term::Literal(Value::U64(rng.range(domains.accounts.max(1))))
            };
            b.bind(mandate, ids::mandate::ACCOUNT, account_term);
            org = b.bind_var(mandate, ids::mandate::ORG);
            b.bind(
                mandate,
                ids::mandate::ACTIVE,
                Term::Literal(Value::I64(i64_point(cfg, rng))),
            );
        }
    }
    b.find_var(org);
    // The membership ∧ Overlaps composition (one of the three the
    // contract asserts per run): a second Mandate occurrence joined on
    // org whose interval must overlap an in-data literal.
    if rng.chance(7, 20) {
        let second = b.atom(ids::MANDATE);
        b.bind(second, ids::mandate::ORG, Term::Var(org));
        let active = b.bind_var(second, ids::mandate::ACTIVE);
        b.predicates.push(Comparison {
            op: CmpOp::Overlaps,
            lhs: Term::Var(active),
            rhs: Term::Literal(i64_interval(cfg, rng)),
        });
    }
}

/// Window membership over the U64 element lane.
fn membership_u64(b: &mut Builder, rng: &mut Rng, cfg: GenConfig) {
    match rng.range(3) {
        // Var point: the account id anchors as the scalar domain.
        0 => {
            let posting = b.atom(ids::POSTING);
            let account = b.bind_var(posting, ids::posting::ACCOUNT);
            let transfer = b.atom(ids::TRANSFER);
            let id = b.bind_var(transfer, ids::transfer::ID);
            b.bind(transfer, ids::transfer::WINDOW, Term::Var(account));
            b.find_var(id);
            b.find_var(account);
        }
        // Param point, anchored at the Posting.account scalar binding.
        1 => {
            let point = b.fresh_param();
            let posting = b.atom(ids::POSTING);
            b.bind(posting, ids::posting::ACCOUNT, Term::Param(point));
            let transfer = b.atom(ids::TRANSFER);
            let extref = b.bind_var(transfer, ids::transfer::EXTREF);
            b.bind(transfer, ids::transfer::WINDOW, Term::Param(point));
            b.find_var(extref);
        }
        // Literal point.
        _ => {
            let transfer = b.atom(ids::TRANSFER);
            let extref = b.bind_var(transfer, ids::transfer::EXTREF);
            b.bind(
                transfer,
                ids::transfer::WINDOW,
                Term::Literal(Value::U64(u64_point(cfg, rng))),
            );
            b.find_var(extref);
        }
    }
    // The composition, U64 lane: a second window overlapping a literal.
    if rng.chance(7, 20) {
        let second = b.atom(ids::TRANSFER);
        let id = b.bind_var(second, ids::transfer::ID);
        let window = b.bind_var(second, ids::transfer::WINDOW);
        b.find_var(id);
        b.predicates.push(Comparison {
            op: CmpOp::Overlaps,
            lhs: Term::Var(window),
            rhs: Term::Literal(u64_interval(cfg, rng)),
        });
    }
}

/// An interval-vs-interval (or interval-vs-literal) comparison over one
/// element lane: `Overlaps`, `Contains` (both the same-element interval
/// and the element-typed right side), and interval `Eq`/`Ne` — the
/// (Eq/Ne, interval) matrix cells.
pub(super) fn interval_join(b: &mut Builder, rng: &mut Rng, cfg: GenConfig) {
    if rng.chance(1, 2) {
        // I64 lane: two Mandate occurrences joined on account.
        let first = b.atom(ids::MANDATE);
        let account = b.bind_var(first, ids::mandate::ACCOUNT);
        let lhs = b.bind_var(first, ids::mandate::ACTIVE);
        let second = b.atom(ids::MANDATE);
        b.bind(second, ids::mandate::ACCOUNT, Term::Var(account));
        let org = b.bind_var(second, ids::mandate::ORG);
        let rhs = b.bind_var(second, ids::mandate::ACTIVE);
        b.find_var(account);
        b.find_var(org);
        let literal = i64_interval(cfg, rng);
        let element = Value::I64(i64_point(cfg, rng));
        push_interval_cmp(b, rng, lhs, rhs, literal, element);
    } else {
        // U64 lane: two Transfer windows.
        let first = b.atom(ids::TRANSFER);
        let x = b.bind_var(first, ids::transfer::ID);
        let lhs = b.bind_var(first, ids::transfer::WINDOW);
        let second = b.atom(ids::TRANSFER);
        let y = b.bind_var(second, ids::transfer::ID);
        let rhs = b.bind_var(second, ids::transfer::WINDOW);
        b.find_var(x);
        b.find_var(y);
        let literal = u64_interval(cfg, rng);
        let element = Value::U64(u64_point(cfg, rng));
        push_interval_cmp(b, rng, lhs, rhs, literal, element);
    }
}

/// The operator roster of an interval join — every operator here is
/// typed at intervals; nothing else can reach these variables.
fn push_interval_cmp(
    b: &mut Builder,
    rng: &mut Rng,
    lhs: VarId,
    rhs: VarId,
    literal: Value,
    element: Value,
) {
    let (op, right) = match rng.range(10) {
        0..=2 => (CmpOp::Overlaps, Term::Var(rhs)),
        3 => (CmpOp::Overlaps, Term::Literal(literal)),
        4 | 5 => (CmpOp::Contains, Term::Var(rhs)),
        6 => (CmpOp::Contains, Term::Literal(literal)),
        // Contains with an element-typed right side: point membership
        // as a predicate.
        7 => (CmpOp::Contains, Term::Literal(element)),
        8 => (CmpOp::Eq, Term::Var(rhs)),
        _ => (CmpOp::Ne, Term::Var(rhs)),
    };
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(lhs),
        rhs: right,
    });
}

/// The adjacent-touching boundary probe: the query literal is
/// recomputed from the corpus interval generator to touch a corpus
/// interval **exactly** at an endpoint — `[a,b) [b,c)` as data and
/// query — in both polarities (the literal ends at a corpus start;
/// the literal starts at a corpus end). Half the probes are `Overlaps`
/// (adjacency must NOT overlap); half are `Contains` with the touch
/// point itself (`b ∉ [a,b)`, `b ∈ [b,c)`).
pub(super) fn boundary(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    let group = rng.range(GROUP_POOL);
    let left = rng.chance(1, 2);
    if left {
        b.adjacent_left = true;
    } else {
        b.adjacent_right = true;
    }
    if rng.chance(1, 2) {
        // I64 lane, pinned to the group's account so the touch is
        // against this group's intervals.
        let (s0, _) = interval_data::group_i64(cfg.seed, group, 0);
        let (_, e1) = interval_data::group_i64(cfg.seed, group, 1);
        let width = i64::try_from(TOUCH_WIDTH).expect("small");
        let literal = if left {
            Value::IntervalI64(s0 - width, s0)
        } else {
            Value::IntervalI64(e1, e1 + width)
        };
        let point = Value::I64(if left { s0 } else { e1 });
        let mandate = b.atom(ids::MANDATE);
        b.bind(
            mandate,
            ids::mandate::ACCOUNT,
            Term::Literal(Value::U64(group % domains.accounts.max(1))),
        );
        let org = b.bind_var(mandate, ids::mandate::ORG);
        let active = b.bind_var(mandate, ids::mandate::ACTIVE);
        b.find_var(org);
        b.find_var(active);
        push_boundary_cmp(b, rng, active, literal, point);
    } else {
        // U64 lane.
        let (s0, _) = interval_data::group_u64(cfg.seed, group, 0);
        let (_, e1) = interval_data::group_u64(cfg.seed, group, 1);
        let literal = if left {
            Value::IntervalU64(s0 - TOUCH_WIDTH, s0)
        } else {
            Value::IntervalU64(e1, e1 + TOUCH_WIDTH)
        };
        let point = Value::U64(if left { s0 } else { e1 });
        let transfer = b.atom(ids::TRANSFER);
        let id = b.bind_var(transfer, ids::transfer::ID);
        let window = b.bind_var(transfer, ids::transfer::WINDOW);
        b.find_var(id);
        b.find_var(window);
        push_boundary_cmp(b, rng, window, literal, point);
    }
}

fn push_boundary_cmp(b: &mut Builder, rng: &mut Rng, var: VarId, literal: Value, point: Value) {
    let (op, rhs) = if rng.chance(1, 2) {
        (CmpOp::Overlaps, Term::Literal(literal))
    } else {
        (CmpOp::Contains, Term::Literal(point))
    };
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(var),
        rhs,
    });
}
