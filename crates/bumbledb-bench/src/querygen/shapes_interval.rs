//! The interval-surface shapes: point membership (literal, param, and
//! var points), interval joins (`Allen` masks — the composites, random
//! singleton basics, and the `Eq`/`Ne` derived facts — plus `PointIn`'s
//! point form), and the adjacent-touching boundary probes whose literals
//! are recomputed from the corpus interval generator — the query touches
//! a corpus interval at exactly its endpoint, both polarities.
//!
//! Interval operators are constructed **only here**, and only over
//! interval-typed terms; order operators never touch these shapes: the
//! illegal (operator, type) matrix cells are unemittable by
//! construction. Masks are literal: the translator does not render param
//! masks — owed to whichever family first needs one.
//!
//! **The equality-spine cost bound** (`docs/architecture/60-validation.md`
//! § the generator contract; `40-execution.md` names the degenerate): a
//! var-point membership binding or a cross-atom `Allen`/`PointIn`
//! join whose interval occurrence shares **no** equality variable with
//! the rest of the query is a Cartesian with a filter — O(bindings × n).
//! Every such construct here is built on a spine: the Mandate lane joins
//! through its account/org group key, and every Transfer occurrence in a
//! var-point or var-vs-var construct carries an equality selection
//! ([`pin_transfer`] — Transfers have no scalar join key). The unbounded
//! shape is unemittable, not filtered after.

use bumbledb::{AllenMask, Basic, CmpOp, Comparison, MaskTerm, Term, Value, VarId};

/// An `Allen` op with a literal mask — the shapes' one constructor.
fn allen(mask: AllenMask) -> CmpOp {
    CmpOp::Allen {
        mask: MaskTerm::Literal(mask),
    }
}

/// A uniformly drawn singleton basic's mask.
fn singleton_mask(rng: &mut Rng) -> AllenMask {
    AllenMask::new(Basic::ALL[usize::try_from(rng.range(13)).expect("small")].bit())
        .expect("a basic's bit is in range")
}

/// A random mask: any nonempty, non-full 13-bit subset — the coordinate
/// system's whole space, not just the named composites (the vacuous
/// EMPTY and FULL masks are roster rejections, exercised by the verify
/// error-parity lane, so the generator never emits them).
///
/// Total by repair, never by rejection sampling: the fuzzer arm's
/// exhausted `Rng::Bytes` yields a CONSTANT zero tail, so a redraw
/// loop on a rejected draw spins forever (the ops target's second
/// finding — a generator hang, not an engine one). EMPTY gains the
/// lowest bit, FULL drops one drawn bit; every other draw is itself.
fn random_mask(rng: &mut Rng) -> AllenMask {
    let bits = match u16::try_from(rng.range(1 << 13)).expect("13 bits") {
        0 => 1,
        0x1FFF => 0x1FFF & !(1 << rng.range(13)),
        drawn => drawn,
    };
    let mask = AllenMask::new(bits).expect("13 bits are in range");
    assert!(!mask.is_empty() && !mask.is_full(), "the repair is total");
    mask
}

use crate::corpus_gen::{GenConfig, Rng};
use crate::querygen::Builder;
use crate::querygen::interval_data;
use crate::querygen::target::{self, Domains, ids};

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

/// An interval literal off the boundary-shape ladder
/// ([`interval_data::ladder_i64`] — equal/adjacent/nested/ray,
/// systematized for every interval literal draw), rung-tagged for the
/// coverage contract.
fn i64_interval(b: &mut Builder, rng: &mut Rng, cfg: GenConfig) -> Value {
    let ((start, end), drawn) = interval_data::ladder_i64(cfg.seed, rng.range(GROUP_POOL), rng);
    b.saw_rung(drawn);
    Value::IntervalI64(bumbledb::Interval::<i64>::new(start, end).expect("nonempty interval"))
}

fn u64_interval(b: &mut Builder, rng: &mut Rng, cfg: GenConfig) -> Value {
    let ((start, end), drawn) = interval_data::ladder_u64(cfg.seed, rng.range(GROUP_POOL), rng);
    b.saw_rung(drawn);
    Value::IntervalU64(bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"))
}

/// The cost-bound rule's equality selection for a Transfer occurrence
/// (Transfers carry no scalar join key, so a var-point or var-vs-var
/// interval construct over one must pin it): the fresh id bound to a
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
            let posting = b.add_atom(ids::POSTING);
            let account = b.bind_var(posting, ids::posting::ACCOUNT);
            let at = b.bind_var(posting, ids::posting::AT);
            let mandate = b.add_atom(ids::MANDATE);
            b.bind(mandate, ids::mandate::ACCOUNT, Term::Var(account));
            org = b.bind_var(mandate, ids::mandate::ORG);
            b.bind(mandate, ids::mandate::ACTIVE, Term::Var(at));
            b.find_var(account);
        }
        // Param point: the param's scalar anchor is a Posting.at
        // binding — mandates covering a probed instant.
        1 => {
            let posting = b.add_atom(ids::POSTING);
            let account = b.bind_var(posting, ids::posting::ACCOUNT);
            let point = b.fresh_param();
            b.bind(posting, ids::posting::AT, Term::Param(point));
            let mandate = b.add_atom(ids::MANDATE);
            b.bind(mandate, ids::mandate::ACCOUNT, Term::Var(account));
            org = b.bind_var(mandate, ids::mandate::ORG);
            b.bind(mandate, ids::mandate::ACTIVE, Term::Param(point));
            b.find_var(account);
        }
        // Literal point: unambiguous (element-typed literal at an
        // interval field IS membership); the account pin is a literal
        // or — the account-set flavor — a param set.
        _ => {
            let mandate = b.add_atom(ids::MANDATE);
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
    // The membership ∧ Allen composition (one of the three the contract
    // asserts per run): a second Mandate occurrence joined on org (the
    // spine) whose interval must intersect an in-data literal.
    if rng.chance(7, 20) {
        let second = b.add_atom(ids::MANDATE);
        b.bind(second, ids::mandate::ORG, Term::Var(org));
        let active = b.bind_var(second, ids::mandate::ACTIVE);
        let rhs = Term::Literal(i64_interval(b, rng, cfg));
        b.conditions.push(Comparison {
            op: allen(AllenMask::INTERSECTS),
            lhs: Term::Var(active),
            rhs,
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
            let posting = b.add_atom(ids::POSTING);
            let account = b.bind_var(posting, ids::posting::ACCOUNT);
            let transfer = b.add_atom(ids::TRANSFER);
            let _payload = pin_transfer(b, rng, cfg, domains, transfer);
            b.bind(transfer, ids::transfer::WINDOW, Term::Var(account));
            b.find_var(account);
        }
        // Param point, anchored at the Posting.account scalar binding
        // (no var rides the membership — the rule does not apply).
        1 => {
            let point = b.fresh_param();
            let posting = b.add_atom(ids::POSTING);
            b.bind(posting, ids::posting::ACCOUNT, Term::Param(point));
            let transfer = b.add_atom(ids::TRANSFER);
            let extref = b.bind_var(transfer, ids::transfer::EXTREF);
            b.bind(transfer, ids::transfer::WINDOW, Term::Param(point));
            b.find_var(extref);
        }
        // Literal point.
        _ => {
            let transfer = b.add_atom(ids::TRANSFER);
            let extref = b.bind_var(transfer, ids::transfer::EXTREF);
            b.bind(
                transfer,
                ids::transfer::WINDOW,
                Term::Literal(Value::U64(u64_point(cfg, rng))),
            );
            b.find_var(extref);
        }
    }
    // The composition, U64 lane: a second window intersecting a literal.
    // The occurrence is pinned — with no scalar join key it would
    // otherwise cross-product against the membership part.
    if rng.chance(7, 20) {
        let second = b.add_atom(ids::TRANSFER);
        let _payload = pin_transfer(b, rng, cfg, domains, second);
        let window = b.bind_var(second, ids::transfer::WINDOW);
        let rhs = Term::Literal(u64_interval(b, rng, cfg));
        b.conditions.push(Comparison {
            op: allen(AllenMask::INTERSECTS),
            lhs: Term::Var(window),
            rhs,
        });
    }
}

/// The right-hand side of an interval comparison.
#[derive(Clone, Copy)]
enum Right {
    /// A second interval occurrence (a cross-atom join — spine-bound).
    Var,
    /// An in-data interval literal (a filter).
    Literal,
    /// An element-typed literal (point membership as a predicate).
    Element,
}

/// An interval-vs-interval (or interval-vs-literal) comparison over one
/// element lane: `Allen` masks — the workload composites (`INTERSECTS`,
/// `COVERS`, `DISJOINT`), a random singleton basic per draw (all 13
/// reachable over the run), **random masks** (any nonempty proper
/// subset of the 13 — the coordinate space itself), and the `Eq`/`Ne`
/// derived facts (the (Eq/Ne, interval) matrix cells) — plus
/// `PointIn`. Var-vs-var joins build the second occurrence
/// **on the spine**: the Mandate lane equality-joins on account; the
/// Transfer lane pins each occurrence. Var-vs-literal filters build no
/// second occurrence at all; their literals draw from the
/// boundary-shape ladder.
pub(super) fn interval_join(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    let draw = rng.range(14);
    let (op, right) = match draw {
        0 | 1 => (allen(AllenMask::INTERSECTS), Right::Var),
        2 => (allen(AllenMask::INTERSECTS), Right::Literal),
        3 | 4 => (allen(AllenMask::COVERS), Right::Var),
        5 => (allen(AllenMask::COVERED_BY), Right::Literal),
        6 => (allen(AllenMask::DISJOINT), Right::Literal),
        // A random singleton basic — every classify branch is reachable.
        7 => (allen(singleton_mask(rng)), Right::Var),
        8 => (allen(singleton_mask(rng)), Right::Literal),
        // PointIn: point membership as a predicate.
        9 => (CmpOp::PointIn, Right::Element),
        10 => (CmpOp::Eq, Right::Var),
        11 => (CmpOp::Ne, Right::Var),
        // Random masks, both operand shapes.
        12 => {
            b.random_mask = true;
            (allen(random_mask(rng)), Right::Var)
        }
        _ => {
            b.random_mask = true;
            (allen(random_mask(rng)), Right::Literal)
        }
    };
    let (lhs, rhs) = if rng.chance(1, 2) {
        // I64 lane: Mandate occurrences joined on account (the spine).
        if matches!(right, Right::Var) && rng.chance(1, 3) {
            wide_mandate_join(b)
        } else {
            mandate_join(b, rng, cfg, right)
        }
    } else {
        // U64 lane: every occurrence pinned (no scalar join key exists).
        let first = b.add_atom(ids::TRANSFER);
        let _payload = pin_transfer(b, rng, cfg, domains, first);
        let lhs = b.bind_var(first, ids::transfer::WINDOW);
        let rhs = match right {
            Right::Var => {
                let second = b.add_atom(ids::TRANSFER);
                let _payload = pin_transfer(b, rng, cfg, domains, second);
                let window = b.bind_var(second, ids::transfer::WINDOW);
                Term::Var(window)
            }
            Right::Literal => Term::Literal(u64_interval(b, rng, cfg)),
            Right::Element => Term::Literal(Value::U64(u64_point(cfg, rng))),
        };
        (lhs, rhs)
    };
    b.conditions.push(Comparison {
        op,
        lhs: Term::Var(lhs),
        rhs,
    });
}

/// [`interval_join`]'s I64 lane: Mandate occurrences joined on account
/// (the spine).
fn mandate_join(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, right: Right) -> (VarId, Term) {
    let first = b.add_atom(ids::MANDATE);
    let account = b.bind_var(first, ids::mandate::ACCOUNT);
    let lhs = b.bind_var(first, ids::mandate::ACTIVE);
    let org = b.bind_var(first, ids::mandate::ORG);
    b.find_var(account);
    b.find_var(org);
    let rhs = match right {
        Right::Var => {
            let second = b.add_atom(ids::MANDATE);
            b.bind(second, ids::mandate::ACCOUNT, Term::Var(account));
            let active = b.bind_var(second, ids::mandate::ACTIVE);
            Term::Var(active)
        }
        Right::Literal => Term::Literal(i64_interval(b, rng, cfg)),
        Right::Element => Term::Literal(Value::I64(i64_point(cfg, rng))),
    };
    (lhs, rhs)
}

/// The wide interval projection (the ≥4-interval-find, >8-word class):
/// four Mandate occurrences, each pinned to ONE account param — the
/// eq-selection spine; a shared-var four-way join would be
/// accounts × `PER_GROUP`⁴ — and each projecting its `active`, so the
/// find list carries four interval finds (8 words) plus the org var.
/// The executor's hoist paths are width-unbounded by construction
/// (docs/architecture/40-execution.md, scan-fold pushdown); the
/// differential oracle keeps this class covered.
fn wide_mandate_join(b: &mut Builder) -> (VarId, Term) {
    let account = b.fresh_param();
    let first = b.add_atom(ids::MANDATE);
    b.bind(first, ids::mandate::ACCOUNT, Term::Param(account));
    let org = b.bind_var(first, ids::mandate::ORG);
    let lhs = b.bind_var(first, ids::mandate::ACTIVE);
    b.find_var(org);
    b.find_var(lhs);
    let mut rhs = lhs;
    for _ in 0..3 {
        let occurrence = b.add_atom(ids::MANDATE);
        b.bind(occurrence, ids::mandate::ACCOUNT, Term::Param(account));
        let active = b.bind_var(occurrence, ids::mandate::ACTIVE);
        b.find_var(active);
        rhs = active;
    }
    (lhs, Term::Var(rhs))
}

/// The adjacent-touching boundary probe: the query literal is
/// recomputed from the corpus interval generator to touch a corpus
/// interval **exactly** at an endpoint — `[a,b) [b,c)` as data and
/// query — in both polarities (the literal ends at a corpus start;
/// the literal starts at a corpus end). Half the probes are
/// `Allen(INTERSECTS)` (adjacency must NOT intersect — *meets* shares
/// no point); half are `PointIn` with the touch point itself
/// (`b ∉ [a,b)`, `b ∈ [b,c)`). Single-occurrence filters: the accepted
/// O(n) scan, not the Cartesian shape.
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
            Value::IntervalI64(
                bumbledb::Interval::<i64>::new(s0 - width, s0).expect("nonempty interval"),
            )
        } else {
            Value::IntervalI64(
                bumbledb::Interval::<i64>::new(e1, e1 + width).expect("nonempty interval"),
            )
        };
        let point = Value::I64(if left { s0 } else { e1 });
        let mandate = b.add_atom(ids::MANDATE);
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
            Value::IntervalU64(
                bumbledb::Interval::<u64>::new(s0 - TOUCH_WIDTH, s0).expect("nonempty interval"),
            )
        } else {
            Value::IntervalU64(
                bumbledb::Interval::<u64>::new(e1, e1 + TOUCH_WIDTH).expect("nonempty interval"),
            )
        };
        let point = Value::U64(if left { s0 } else { e1 });
        let transfer = b.add_atom(ids::TRANSFER);
        let id = b.bind_var(transfer, ids::transfer::ID);
        let window = b.bind_var(transfer, ids::transfer::WINDOW);
        b.find_var(id);
        b.find_var(window);
        push_boundary_cmp(b, rng, window, literal, point);
    }
}

/// The measure shape (`docs/architecture/20-query-ir.md` § the
/// measure), over the U64 window lane — total here: the lane's
/// sentinel end (`interval_data::U64_SENTINEL_END`) sits below the
/// element domain's `MAX`, so no window is a ray and `Duration` is
/// `(end − start)` on both oracles (ray-bearing measure parity is the
/// verify naive lane's obligation). Three construct kinds: the measure
/// in a find position, in an order predicate against a literal, and
/// folded (`Sum`/`Min`/`Max` — `Sum` under a duration bound so the
/// reachable sum stays far below 2⁶³, the generator's Sum-range duty).
pub(super) fn measure(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    let transfer = b.add_atom(ids::TRANSFER);
    match rng.range(3) {
        // Find position: `[extref, Duration(window)]` over a pinned
        // occurrence, or the open distinct-durations scan.
        0 => {
            if rng.chance(1, 2) {
                let _payload = pin_transfer(b, rng, cfg, domains, transfer);
            } else {
                let id = b.bind_var(transfer, ids::transfer::ID);
                b.find_var(id);
            }
            let window = b.bind_var(transfer, ids::transfer::WINDOW);
            b.finds.push(bumbledb::FindTerm::Duration(window));
        }
        // Predicate: `Duration(window) <op> literal` — an order
        // comparison over the measure word, the selection form.
        1 => {
            let id = b.bind_var(transfer, ids::transfer::ID);
            b.find_var(id);
            let window = b.bind_var(transfer, ids::transfer::WINDOW);
            b.conditions.push(Comparison {
                op: crate::querygen::shapes::order_op(rng),
                lhs: Term::Duration(window),
                rhs: Term::Literal(Value::U64(rng.range(3 * interval_data::GROUP_SPAN))),
            });
        }
        // Fold: a global Sum/Min/Max over the measure.
        _ => {
            let window = b.bind_var(transfer, ids::transfer::WINDOW);
            let op = match rng.range(3) {
                0 => bumbledb::AggOp::Sum,
                1 => bumbledb::AggOp::Min,
                _ => bumbledb::AggOp::Max,
            };
            if op == bumbledb::AggOp::Sum {
                // The Sum bound: durations capped at a group span, so
                // the sum tops out near transfers × 4096 ≪ 2⁶³.
                b.conditions.push(Comparison {
                    op: bumbledb::CmpOp::Le,
                    lhs: Term::Duration(window),
                    rhs: Term::Literal(Value::U64(4 * interval_data::GROUP_SPAN)),
                });
            }
            b.finds
                .push(bumbledb::FindTerm::AggregateDuration { op, over: window });
        }
    }
}

fn push_boundary_cmp(b: &mut Builder, rng: &mut Rng, var: VarId, literal: Value, point: Value) {
    let (op, rhs) = if rng.chance(1, 2) {
        (allen(AllenMask::INTERSECTS), Term::Literal(literal))
    } else {
        (CmpOp::PointIn, Term::Literal(point))
    };
    b.conditions.push(Comparison {
        op,
        lhs: Term::Var(var),
        rhs,
    });
}

#[cfg(test)]
mod tests {
    use super::random_mask;
    use crate::corpus_gen::Rng;

    /// The fuzz campaign's generator-hang pin (ops finding 2): an
    /// exhausted byte source draws zero forever, so `random_mask` must
    /// be total on a CONSTANT stream — rejection sampling here once
    /// spun the fuzzer for minutes per input. Both vacuous constants
    /// (EMPTY's zero draw; the all-ones FULL draw) must terminate with
    /// a legal mask.
    #[test]
    fn random_mask_is_total_on_constant_streams() {
        let empty_tail = random_mask(&mut Rng::from_bytes(&[]));
        assert!(!empty_tail.is_empty() && !empty_tail.is_full());
        let full: Vec<u8> = 0x1FFFu64.to_le_bytes().repeat(4);
        let full_tail = random_mask(&mut Rng::from_bytes(&full));
        assert!(!full_tail.is_empty() && !full_tail.is_full());
    }
}
