use bumbledb::{CmpOp, Comparison, FieldId, Term, Value};

use crate::gen::{self, GenConfig, Rng, Sizes};
use crate::querygen::construct::extref_of;
use crate::querygen::dress_posting::dress_posting;
use crate::querygen::{Builder, DRESS_PCT};
use crate::schema::ids;

/// Any of the six operators, uniformly (integer dressing — every legal
/// (op, integer-type) cell of the coverage matrix must be reachable).
pub(super) fn any_op(rng: &mut Rng) -> CmpOp {
    match rng.range(6) {
        0 => CmpOp::Eq,
        1 => CmpOp::Ne,
        2 => CmpOp::Lt,
        3 => CmpOp::Le,
        4 => CmpOp::Gt,
        _ => CmpOp::Ge,
    }
}

/// An i64 predicate on the field (any operator): literal or param, 50/50.
pub(super) fn i64_dress(b: &mut Builder, rng: &mut Rng, atom: usize, field: FieldId, lo: i64, hi: i64) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = any_op(rng);
    let width = u64::try_from(hi - lo).expect("ordered window");
    let rhs = if rng.chance(1, 2) {
        Term::Literal(Value::I64(
            lo + i64::try_from(rng.range(width.max(1))).expect("fits"),
        ))
    } else {
        Term::Param(b.fresh_param())
    };
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(var),
        rhs,
    });
}

/// An `Eq`/`Ne` predicate against an enum-ordinal literal.
fn enum_cmp(b: &mut Builder, rng: &mut Rng, atom: usize, field: FieldId, variants: u64) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = if rng.chance(1, 2) {
        CmpOp::Eq
    } else {
        CmpOp::Ne
    };
    let ordinal = u8::try_from(rng.range(variants)).expect("small");
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(var),
        rhs: Term::Literal(Value::Enum(ordinal)),
    });
}

/// Filter dressing ([`DRESS_PCT`]% of queries, 1–3 predicates): i64 range
/// ops on amount/at, Eq/Ne on memo (hit, miss, or param), Eq on
/// enums/bools, and same-typed var-vs-var — per the dressed atom's
/// relation.
pub(super) fn dress(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, sizes: &Sizes) {
    if !rng.chance(DRESS_PCT, 100) {
        return;
    }
    let count = 1 + rng.range(3);
    for _ in 0..count {
        let dressable: Vec<usize> = b
            .atoms
            .iter()
            .enumerate()
            .filter(|(_, atom)| !atom.bindings.is_empty())
            .map(|(index, _)| index)
            .collect();
        let atom = dressable[usize::try_from(rng.range(dressable.len() as u64)).expect("small")];
        match b.atoms[atom].relation {
            ids::POSTING => dress_posting(b, rng, atom, sizes),
            ids::ACCOUNT => {
                if rng.chance(1, 2) {
                    enum_cmp(b, rng, atom, ids::account::STATUS, 3);
                } else {
                    i64_dress(
                        b,
                        rng,
                        atom,
                        ids::account::OPENED_AT,
                        gen::AT_BASE - (1 << 30),
                        gen::AT_BASE,
                    );
                }
            }
            ids::INSTRUMENT => enum_cmp(b, rng, atom, ids::instrument::KIND, 4),
            ids::HOLDER => enum_cmp(b, rng, atom, ids::holder::REGION, 4),
            ids::TRANSFER => {
                if rng.chance(1, 2) {
                    // Bytes Eq/Ne on extref: the hit literal is the
                    // *actual* extref of a seeded row (recomputed via
                    // gen::row — the corpus is a pure function of the
                    // config); the miss is a fresh 16-byte value.
                    let Some(var) = b.var_at(atom, ids::transfer::EXTREF) else {
                        continue;
                    };
                    let op = if rng.chance(1, 2) {
                        CmpOp::Eq
                    } else {
                        CmpOp::Ne
                    };
                    let rhs = match rng.range(3) {
                        0 => {
                            b.bytes_hit = true;
                            Term::Literal(extref_of(cfg, sizes, rng.range(sizes.transfers)))
                        }
                        1 => {
                            b.miss = true;
                            b.bytes_miss = true;
                            let mut raw = Vec::with_capacity(16);
                            for _ in 0..2 {
                                raw.extend_from_slice(&rng.u64().to_le_bytes());
                            }
                            Term::Literal(Value::Bytes(raw.into()))
                        }
                        _ => Term::Param(b.fresh_param()),
                    };
                    b.predicates.push(Comparison {
                        op,
                        lhs: Term::Var(var),
                        rhs,
                    });
                } else {
                    let span = i64::try_from(sizes.transfers).expect("fits") * gen::AT_STEP * 2;
                    i64_dress(
                        b,
                        rng,
                        atom,
                        ids::transfer::AT,
                        gen::AT_BASE,
                        gen::AT_BASE + span,
                    );
                }
            }
            _ => {}
        }
    }
}
