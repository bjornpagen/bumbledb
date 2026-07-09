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
//!
//! **The equality-spine cost bound** (`docs/architecture/60-validation.md`
//! § the generator contract; `40-execution.md` names the degenerate): a
//! var-point membership binding or a cross-atom `Overlaps`/`Contains`
//! join whose interval occurrence shares **no** equality variable with
//! the rest of the query is a Cartesian with a filter — O(bindings × n).
//! Every such construct here is built on a spine: the Mandate lane joins
//! through its account/org group key, and every Transfer occurrence in a
//! var-point or var-vs-var construct carries an equality selection
//! ([`pin_transfer`] — Transfers have no scalar join key). The unbounded
//! shape is unemittable, not filtered after.

use bumbledb::{CmpOp, Comparison, Term, Value, VarId};

use crate::gen::{GenConfig, Rng};
use crate::querygen::interval_data;
use crate::querygen::target::{self, ids, Domains};
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

/// The cost-bound rule's equality selection for a Transfer occurrence
/// (Transfers carry no scalar join key, so a var-point or var-vs-var
/// interval construct over one must pin it): the serial id bound to a
/// param, or the extref bound to a recomputed in-vocabulary literal.
/// Returns a projected payload var so the occurrence contributes to the
/// find set.
fn pin_transfer(
    b: &mut Builder,
    rng: &mut Rng,
    cfg: GenConfig,
    domains: &Domains,
    transfer: usize,
) -> VarId {
    if rng.chance(1, 2) {
        let param = b.fresh_param();
        b.bind(transfer, ids::transfer::ID, Term::Param(param));
        let extref = b.bind_var(transfer, ids::transfer::EXTREF);
        b.find_var(extref);
        extref
    } else {
        b.bytes_hit = true;
        b.bind(
            transfer,
            ids::transfer::EXTREF,
            Term::Literal(target::extref(cfg, rng.range(domains.transfers))),
        );
        let id = b.bind_var(transfer, ids::transfer::ID);
        b.find_var(id);
        id
    }
}

/// Point membership against an interval field. The point term is a
/// literal, a param, or a variable — and the var and param cases
/// **construct** their scalar anchor (a Posting scalar binding) first,
/// deliberately: a point term with no enumerable domain is invalid by
/// the roster, so the anchor is never left to chance. Var-point
/// occurrences ride the equality spine (module doc).
pub(super) fn membership(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    if rng.chance(3, 5) {
        membership_i64(b, rng, cfg, domains);
    } else {
        membership_u64(b, rng, cfg, domains);
    }
}

/// Mandate-at-instant over the I64 element lane:
/// `Posting(account = a, at = t), Mandate(account = a, active ∋ t)` and
/// its param/literal-point variants. The spine is the shared account
/// variable (the group key real interval workloads carry).
fn membership_i64(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    let org;
    match rng.range(3) {
        // Var point: `at` is the scalar anchor, constructed here; the
        // Mandate occurrence equality-joins on account.
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
    // org (the spine) whose interval must overlap an in-data literal.
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

/// Window membership over the U64 element lane. Every Transfer
/// occurrence in a var-point construct is pinned ([`pin_transfer`]):
/// `Posting(account = v) × Transfer(window ∋ v)` without the pin is the
/// named Cartesian degenerate.
fn membership_u64(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    match rng.range(3) {
        // Var point: the account id anchors as the scalar domain; the
        // window's occurrence carries an equality selection.
        0 => {
            let posting = b.atom(ids::POSTING);
            let account = b.bind_var(posting, ids::posting::ACCOUNT);
            let transfer = b.atom(ids::TRANSFER);
            let _payload = pin_transfer(b, rng, cfg, domains, transfer);
            b.bind(transfer, ids::transfer::WINDOW, Term::Var(account));
            b.find_var(account);
        }
        // Param point, anchored at the Posting.account scalar binding
        // (no var rides the membership — the rule does not apply).
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
    // The occurrence is pinned — with no scalar join key it would
    // otherwise cross-product against the membership part.
    if rng.chance(7, 20) {
        let second = b.atom(ids::TRANSFER);
        let _payload = pin_transfer(b, rng, cfg, domains, second);
        let window = b.bind_var(second, ids::transfer::WINDOW);
        b.predicates.push(Comparison {
            op: CmpOp::Overlaps,
            lhs: Term::Var(window),
            rhs: Term::Literal(u64_interval(cfg, rng)),
        });
    }
}

/// The right-hand side of an interval comparison.
enum Right {
    /// A second interval occurrence (a cross-atom join — spine-bound).
    Var,
    /// An in-data interval literal (a filter).
    Literal,
    /// An element-typed literal (point membership as a predicate).
    Element,
}

/// An interval-vs-interval (or interval-vs-literal) comparison over one
/// element lane: `Overlaps`, `Contains` (both the same-element interval
/// and the element-typed right side), and interval `Eq`/`Ne` — the
/// (Eq/Ne, interval) matrix cells. Var-vs-var joins build the second
/// occurrence **on the spine**: the Mandate lane equality-joins on
/// account; the Transfer lane pins each occurrence. Var-vs-literal
/// filters build no second occurrence at all.
pub(super) fn interval_join(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    let (op, right) = match rng.range(10) {
        0..=2 => (CmpOp::Overlaps, Right::Var),
        3 => (CmpOp::Overlaps, Right::Literal),
        4 | 5 => (CmpOp::Contains, Right::Var),
        6 => (CmpOp::Contains, Right::Literal),
        // Contains with an element-typed right side: point membership
        // as a predicate.
        7 => (CmpOp::Contains, Right::Element),
        8 => (CmpOp::Eq, Right::Var),
        _ => (CmpOp::Ne, Right::Var),
    };
    let (lhs, rhs) = if rng.chance(1, 2) {
        // I64 lane: Mandate occurrences joined on account (the spine).
        let first = b.atom(ids::MANDATE);
        let account = b.bind_var(first, ids::mandate::ACCOUNT);
        let lhs = b.bind_var(first, ids::mandate::ACTIVE);
        let org = b.bind_var(first, ids::mandate::ORG);
        b.find_var(account);
        b.find_var(org);
        let rhs = match right {
            Right::Var => {
                let second = b.atom(ids::MANDATE);
                b.bind(second, ids::mandate::ACCOUNT, Term::Var(account));
                let active = b.bind_var(second, ids::mandate::ACTIVE);
                Term::Var(active)
            }
            Right::Literal => Term::Literal(i64_interval(cfg, rng)),
            Right::Element => Term::Literal(Value::I64(i64_point(cfg, rng))),
        };
        (lhs, rhs)
    } else {
        // U64 lane: every occurrence pinned (no scalar join key exists).
        let first = b.atom(ids::TRANSFER);
        let _payload = pin_transfer(b, rng, cfg, domains, first);
        let lhs = b.bind_var(first, ids::transfer::WINDOW);
        let rhs = match right {
            Right::Var => {
                let second = b.atom(ids::TRANSFER);
                let _payload = pin_transfer(b, rng, cfg, domains, second);
                let window = b.bind_var(second, ids::transfer::WINDOW);
                Term::Var(window)
            }
            Right::Literal => Term::Literal(u64_interval(cfg, rng)),
            Right::Element => Term::Literal(Value::U64(u64_point(cfg, rng))),
        };
        (lhs, rhs)
    };
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(lhs),
        rhs,
    });
}

/// The adjacent-touching boundary probe: the query literal is
/// recomputed from the corpus interval generator to touch a corpus
/// interval **exactly** at an endpoint — `[a,b) [b,c)` as data and
/// query — in both polarities (the literal ends at a corpus start;
/// the literal starts at a corpus end). Half the probes are `Overlaps`
/// (adjacency must NOT overlap); half are `Contains` with the touch
/// point itself (`b ∉ [a,b)`, `b ∈ [b,c)`). Single-occurrence filters:
/// the accepted O(n) scan, not the Cartesian shape.
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
