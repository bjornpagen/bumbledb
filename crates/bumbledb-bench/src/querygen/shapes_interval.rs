use bumbledb::{AllenMask, Basic, CmpOp, Comparison, Term, Value, VarId};

fn allen(mask: AllenMask) -> CmpOp {
    CmpOp::Allen { mask }
}

fn singleton_mask(rng: &mut Rng) -> AllenMask {
    AllenMask::new(Basic::ALL[usize::try_from(rng.range(13)).expect("small")].bit())
        .expect("a basic's bit is in range")
}

pub(super) fn random_mask(rng: &mut Rng) -> AllenMask {
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

const GROUP_POOL: u64 = 64;

const TOUCH_WIDTH: u64 = 64;

fn i64_point(cfg: GenConfig, rng: &mut Rng) -> i64 {
    let (start, end) = interval_data::group_i64(cfg.seed, rng.range(GROUP_POOL), 2);
    start + (end - start) / 2
}

fn u64_point(cfg: GenConfig, rng: &mut Rng) -> u64 {
    let (start, end) = interval_data::group_u64(cfg.seed, rng.range(GROUP_POOL), 2);
    start + (end - start) / 2
}

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

pub(super) fn membership(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    if rng.chance(3, 5) {
        membership_i64(b, rng, cfg, domains);
    } else {
        membership_u64(b, rng, cfg, domains);
    }
}

fn membership_i64(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    let org;
    match rng.range(3) {

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

fn membership_u64(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    match rng.range(3) {

        0 => {
            let posting = b.add_atom(ids::POSTING);
            let account = b.bind_var(posting, ids::posting::ACCOUNT);
            let transfer = b.add_atom(ids::TRANSFER);
            let _payload = pin_transfer(b, rng, cfg, domains, transfer);
            b.bind(transfer, ids::transfer::WINDOW, Term::Var(account));
            b.find_var(account);
        }

        1 => {
            let point = b.fresh_param();
            let posting = b.add_atom(ids::POSTING);
            b.bind(posting, ids::posting::ACCOUNT, Term::Param(point));
            let transfer = b.add_atom(ids::TRANSFER);
            let extref = b.bind_var(transfer, ids::transfer::EXTREF);
            b.bind(transfer, ids::transfer::WINDOW, Term::Param(point));
            b.find_var(extref);
        }

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

#[derive(Clone, Copy)]
enum Right {

    Var,

    Literal,

    Element,
}

pub(super) fn interval_join(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    let draw = rng.range(14);
    let (op, right) = match draw {
        0 | 1 => (allen(AllenMask::INTERSECTS), Right::Var),
        2 => (allen(AllenMask::INTERSECTS), Right::Literal),
        3 | 4 => (allen(AllenMask::COVERS), Right::Var),
        5 => (allen(AllenMask::COVERED_BY), Right::Literal),
        6 => (allen(AllenMask::DISJOINT), Right::Literal),

        7 => (allen(singleton_mask(rng)), Right::Var),
        8 => (allen(singleton_mask(rng)), Right::Literal),

        9 => (CmpOp::PointIn, Right::Element),
        10 => (CmpOp::Eq, Right::Var),
        11 => (CmpOp::Ne, Right::Var),

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

        if matches!(right, Right::Var) && rng.chance(1, 3) {
            wide_mandate_join(b)
        } else {
            mandate_join(b, rng, cfg, right)
        }
    } else {

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

/// Half the probes are `Allen(INTERSECTS)` (adjacency must NOT intersect —
/// *meets* shares no point); half are `PointIn` with the touch point itself (`b
/// ∉ [a,b)`, `b ∈ [b,c)`).
pub(super) fn boundary(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    let group = rng.range(GROUP_POOL);
    let left = rng.chance(1, 2);
    if left {
        b.adjacent_left = true;
    } else {
        b.adjacent_right = true;
    }
    if rng.chance(1, 2) {

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

pub(super) fn measure(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
    let transfer = b.add_atom(ids::TRANSFER);
    match rng.range(3) {
        0 => {
            if rng.chance(1, 2) {
                let _payload = pin_transfer(b, rng, cfg, domains, transfer);
            } else {
                let id = b.bind_var(transfer, ids::transfer::ID);
                b.find_var(id);
            }
            let window = b.bind_var(transfer, ids::transfer::WINDOW);
            b.find_var(window);
        }
        1 => {
            let id = b.bind_var(transfer, ids::transfer::ID);
            b.find_var(id);
            let window = b.bind_var(transfer, ids::transfer::WINDOW);
            let _op = crate::querygen::shapes::order_op(rng);
            let _lit = rng.range(3 * interval_data::GROUP_SPAN);
            let _ = window;
        }
        _ => {
            let window = b.bind_var(transfer, ids::transfer::WINDOW);
            let _ = rng.range(3);
            b.find_var(window);
        }
    }
}

pub(super) fn pack(b: &mut Builder, rng: &mut Rng) {
    let mandate = b.add_atom(ids::MANDATE);
    let active = b.bind_var(mandate, ids::mandate::ACTIVE);
    match rng.range(3) {

        0 => {
            let account = b.bind_var(mandate, ids::mandate::ACCOUNT);
            b.find_var(account);
        }

        1 => {
            let org = b.bind_var(mandate, ids::mandate::ORG);
            b.find_var(org);
        }

        _ => {}
    }
    b.finds.push(bumbledb::FindTerm::Pack { over: active });
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

    #[test]
    fn random_mask_is_total_on_constant_streams() {
        let empty_tail = random_mask(&mut Rng::from_bytes(&[]));
        assert!(!empty_tail.is_empty() && !empty_tail.is_full());
        let full: Vec<u8> = 0x1FFFu64.to_le_bytes().repeat(4);
        let full_tail = random_mask(&mut Rng::from_bytes(&full));
        assert!(!full_tail.is_empty() && !full_tail.is_full());
    }
}
