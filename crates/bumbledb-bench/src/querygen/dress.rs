use bumbledb::{CmpOp, Comparison, FieldId, RelationId, Term, Value};

use crate::gen::{GenConfig, Rng};
use crate::querygen::dress_posting::dress_posting;
use crate::querygen::target::{self, ids, Domains};
use crate::querygen::{interval_data, Builder, DRESS_PCT};

/// Any of the six word-comparison operators, uniformly — applied ONLY
/// to the two integer types by its callers (the order-op legality
/// cells).
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

pub(super) fn eq_ne(rng: &mut Rng) -> CmpOp {
    if rng.chance(1, 2) {
        CmpOp::Eq
    } else {
        CmpOp::Ne
    }
}

/// An i64 predicate on the field (any operator): literal, param, or —
/// under `Eq` — a param set.
pub(super) fn i64_dress(
    b: &mut Builder,
    rng: &mut Rng,
    atom: usize,
    field: FieldId,
    lo: i64,
    hi: i64,
) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = any_op(rng);
    let width = u64::try_from(hi - lo).expect("ordered window");
    let rhs = if op == CmpOp::Eq && rng.chance(1, 4) {
        Term::ParamSet(b.fresh_param())
    } else if rng.chance(1, 2) {
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

/// A u64 predicate on a dense-id field (any operator): the literal or
/// param draws in-domain so ordered comparisons select real slices;
/// under `Eq`, sometimes a param set.
pub(super) fn u64_dress(b: &mut Builder, rng: &mut Rng, atom: usize, field: FieldId, domain: u64) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = any_op(rng);
    let rhs = if op == CmpOp::Eq && rng.chance(1, 4) {
        Term::ParamSet(b.fresh_param())
    } else if rng.chance(1, 2) {
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

/// An `Eq`/`Ne` predicate against a closed-vocabulary row id (a plain
/// u64 field contained in a closed relation): literal, param, or —
/// under `Eq` — a param set.
fn vocab_cmp(b: &mut Builder, rng: &mut Rng, atom: usize, field: FieldId, rows: u64) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = eq_ne(rng);
    let rhs = if op == CmpOp::Eq && rng.chance(1, 5) {
        Term::ParamSet(b.fresh_param())
    } else if rng.chance(1, 4) {
        Term::Param(b.fresh_param())
    } else {
        Term::Literal(Value::U64(rng.range(rows)))
    };
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(var),
        rhs,
    });
}

/// An `Eq`/`Ne` string predicate: in-vocabulary hit, out-of-vocabulary
/// miss, param, or — under `Eq` — a param set.
pub(super) fn string_cmp(
    b: &mut Builder,
    rng: &mut Rng,
    atom: usize,
    relation: RelationId,
    field: FieldId,
) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = eq_ne(rng);
    let rhs = if op == CmpOp::Eq && rng.chance(1, 5) {
        Term::ParamSet(b.fresh_param())
    } else {
        match rng.range(3) {
            0 => Term::Literal(Value::String(
                target::string_hit(relation, field, rng).into_bytes().into(),
            )),
            1 => {
                b.miss = true;
                Term::Literal(Value::String(
                    format!("missing-{}", rng.u64()).into_bytes().into(),
                ))
            }
            _ => Term::Param(b.fresh_param()),
        }
    };
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(var),
        rhs,
    });
}

/// The i64 window `Posting.at` (and `JournalEntry.created_at`) draws
/// from, per scale.
pub(super) fn at_window(domains: &Domains) -> (i64, i64) {
    let span = i64::try_from(domains.postings).expect("fits") * target::AT_STEP;
    (target::AT_BASE, target::AT_BASE + span)
}

/// An interval literal for value-equality dressing, drawn off the
/// boundary-shape ladder (equal literals hit; adjacent/nested/ray
/// literals are exact-construction misses), rung-tagged for the
/// coverage contract.
fn window_literal_u64(b: &mut Builder, rng: &mut Rng, cfg: GenConfig) -> Value {
    let ((start, end), drawn) = interval_data::ladder_u64(cfg.seed, rng.range(64), rng);
    b.saw_rung(drawn);
    Value::IntervalU64(start, end)
}

fn active_literal_i64(b: &mut Builder, rng: &mut Rng, cfg: GenConfig) -> Value {
    let ((start, end), drawn) = interval_data::ladder_i64(cfg.seed, rng.range(64), rng);
    b.saw_rung(drawn);
    Value::IntervalI64(start, end)
}

/// Filter dressing ([`DRESS_PCT`]% of queries, 1–3 predicates), per the
/// dressed atom's relation: integer range ops, string/bytes hits and
/// misses, vocabulary and bool equalities, interval-value `Eq`/`Ne` against
/// in-data literals, and same-typed var-vs-var.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one arm per dressed relation, in id order
pub(super) fn dress(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, domains: &Domains) {
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
            ids::POSTING => dress_posting(b, rng, atom, domains),
            ids::ACCOUNT => {
                if rng.chance(1, 2) {
                    vocab_cmp(b, rng, atom, ids::account::CURRENCY, 3);
                } else {
                    u64_dress(b, rng, atom, ids::account::HOLDER, domains.holders);
                }
            }
            ids::JOURNAL_ENTRY => {
                if rng.chance(1, 2) {
                    vocab_cmp(b, rng, atom, ids::journal_entry::SOURCE, 3);
                } else {
                    let (lo, hi) = at_window(domains);
                    i64_dress(b, rng, atom, ids::journal_entry::CREATED_AT, lo, hi);
                }
            }
            ids::HOLDER => string_cmp(b, rng, atom, ids::HOLDER, ids::holder::NAME),
            ids::ORG => string_cmp(b, rng, atom, ids::ORG, ids::org::NAME),
            ids::INSTRUMENT => string_cmp(b, rng, atom, ids::INSTRUMENT, ids::instrument::SYMBOL),
            ids::TRANSFER => {
                if rng.chance(1, 3) {
                    // Interval value equality: Eq/Ne against an in-data
                    // window literal — the (Eq/Ne, interval) cells. A
                    // membership *point* var bound here is element-typed
                    // and must not be compared against interval values.
                    let Some(var) = b.var_at(atom, ids::transfer::WINDOW) else {
                        continue;
                    };
                    if !b.interval_valued(var) {
                        continue;
                    }
                    let rhs = Term::Literal(window_literal_u64(b, rng, cfg));
                    b.predicates.push(Comparison {
                        op: eq_ne(rng),
                        lhs: Term::Var(var),
                        rhs,
                    });
                } else if rng.chance(1, 2) {
                    // bytes<32> Eq/Ne on extref: the hit literal is the
                    // *actual* extref of a seeded row (recomputed — the
                    // corpus is a pure function of the config); the miss
                    // is adversarial — a single-byte delta of a real
                    // extref (the corpus pins byte 0 to zero, so the
                    // flipped digest exists nowhere).
                    let Some(var) = b.var_at(atom, ids::transfer::EXTREF) else {
                        continue;
                    };
                    let op = eq_ne(rng);
                    let rhs = match rng.range(3) {
                        0 => {
                            b.bytes_hit = true;
                            Term::Literal(target::extref(cfg, rng.range(domains.transfers)))
                        }
                        1 => {
                            b.miss = true;
                            b.bytes_miss = true;
                            let Value::FixedBytes(mut raw) =
                                target::extref(cfg, rng.range(domains.transfers))
                            else {
                                unreachable!("extref is bytes<32>")
                            };
                            raw[0] = 0xA5;
                            Term::Literal(Value::FixedBytes(raw))
                        }
                        _ => Term::Param(b.fresh_param()),
                    };
                    b.predicates.push(Comparison {
                        op,
                        lhs: Term::Var(var),
                        rhs,
                    });
                } else {
                    // A pad-boundary digest tag (widths 7/8/9/16/63/64):
                    // Eq/Ne against a vocabulary hit, an adversarial
                    // single-byte-delta miss, a param, or — under Eq —
                    // a param set of digests.
                    let which = usize::try_from(rng.range(target::DIGEST_WIDTHS.len() as u64))
                        .expect("small");
                    let width = target::DIGEST_WIDTHS[which];
                    let Some(var) = b.var_at(atom, ids::transfer::TAGS[which]) else {
                        continue;
                    };
                    let op = eq_ne(rng);
                    let rhs = if op == CmpOp::Eq && rng.chance(1, 4) {
                        Term::ParamSet(b.fresh_param())
                    } else {
                        match rng.range(3) {
                            0 => {
                                b.bytes_hit = true;
                                Term::Literal(target::digest_vocab_value(
                                    width,
                                    rng.range(target::DIGEST_VOCAB),
                                ))
                            }
                            1 => {
                                b.miss = true;
                                b.bytes_miss = true;
                                let Value::FixedBytes(mut raw) = target::digest_vocab_value(
                                    width,
                                    rng.range(target::DIGEST_VOCAB),
                                ) else {
                                    unreachable!("digests are bytes<N>")
                                };
                                raw[0] = 0xA5;
                                Term::Literal(Value::FixedBytes(raw))
                            }
                            _ => Term::Param(b.fresh_param()),
                        }
                    };
                    b.predicates.push(Comparison {
                        op,
                        lhs: Term::Var(var),
                        rhs,
                    });
                }
            }
            ids::MANDATE => {
                // Interval value equality on the I64 element lane —
                // skipped when the bound term is a membership point
                // (element-typed, not an interval value).
                let Some(var) = b.var_at(atom, ids::mandate::ACTIVE) else {
                    continue;
                };
                if !b.interval_valued(var) {
                    continue;
                }
                let rhs = Term::Literal(active_literal_i64(b, rng, cfg));
                b.predicates.push(Comparison {
                    op: eq_ne(rng),
                    lhs: Term::Var(var),
                    rhs,
                });
            }
            _ => {}
        }
    }
}
