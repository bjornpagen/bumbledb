//! The random-query arm (docs/architecture/60-validation.md § the
//! fuzzing charter): structurally-free [`Query`] IR for the fuzz lanes'
//! validation-totality oracle. Unlike `crate::querygen` (valid **by
//! construction** — the illegal cells of its grammar are unemittable),
//! this arm deliberately reaches invalid shapes — empty rule sets,
//! dangling relation/field ids, head/rule misalignment, unsafe negation,
//! aggregate abuse, vacuous masks, param scalar/set confusion, hostile
//! condition nesting — alongside valid ones, and the ENGINE judges. The
//! generator owns no validity logic: id draws bias toward the querygen
//! target theory's spans so mutated inputs collide meaningfully instead
//! of drifting into gibberish, but nothing here re-implements the
//! validation roster (refusal: a generator that knows the rules can only
//! confirm them).

use bumbledb::{
    AggOp, AllenMask, Atom, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, MaskTerm, ParamId,
    Query, RelationId, Rule, Term, Value, VarId,
};

use super::Rng;
use crate::querygen::target;

/// The target theory's relation span (`crate::querygen::target` declares
/// 13) — an anchoring constant, not a validity rule: draws overshoot it
/// so `UnknownRelation` stays a draw away.
const RELATION_SPAN: u64 = 13;

/// A generous field span (the widest target relation has 8 fields);
/// overshooting reaches `UnknownField`.
const FIELD_SPAN: u64 = 8;

/// Small dense pools: collisions (self-joins, repeated vars, shared
/// params) arise freely.
const VAR_SPAN: u64 = 6;
const PARAM_SPAN: u64 = 4;

/// A structurally-free query: every shape the IR type can spell is
/// reachable by some byte string — zero rules, misaligned heads, deep
/// condition spines. Valid and invalid programs both arise; the verdict
/// is the engine's.
pub fn random_query(rng: &mut Rng) -> Query {
    // Half the draws start from a coherent single-atom core so the
    // ACCEPTED verdict class stays reachable (a fully free draw almost
    // never aligns finds, bindings, and types); the mutations below keep
    // every rejection reachable from that side too.
    if rng.chance(1, 2) {
        return plausible(rng);
    }
    // Mostly small programs; the occasional wide one reaches
    // `TooManyRules` (the cap is 16).
    let rule_count = if rng.chance(1, 32) {
        17 + rng.range(4)
    } else {
        rng.range(4) // zero rules: the empty union is not a query
    };
    let rules: Vec<Rule> = (0..rule_count).map(|_| random_rule(rng)).collect();
    // Mostly the first rule's own head shape (deeper roster lines need
    // the program shape to pass); the free draw keeps the misalignment
    // rejections reachable.
    let head = match rules.first() {
        Some(rule) if rng.chance(7, 8) => rule.head(),
        _ => (0..rng.range(4))
            .map(|_| random_find(rng).head_term())
            .collect(),
    };
    Query { head, rules }
}

/// The acceptance-biased draw: one atom over a real target relation,
/// distinct fields bound to dense variables, the variables projected —
/// then free mutations half the time. The anchor is the schema's field
/// counts and nothing else (theorygen's vocabulary-anchoring precedent);
/// the roster stays the engine's.
fn plausible(rng: &mut Rng) -> Query {
    let schema = target::schema();
    let rel = RelationId(
        u32::try_from(rng.range(u64::from(target::TARGET_RELATIONS))).expect("id fits u32"),
    );
    let field_count = u64::try_from(schema.relation(rel).fields().len()).expect("fits u64");
    let vars = 1 + rng.range(field_count.min(3));
    let start = rng.range(field_count);
    let bindings: Vec<(FieldId, Term)> = (0..vars)
        .map(|i| {
            (
                FieldId(u16::try_from((start + i) % field_count).expect("field id fits u16")),
                Term::Var(VarId(u16::try_from(i).expect("var id fits u16"))),
            )
        })
        .collect();
    let mut rule = Rule {
        finds: (0..vars)
            .map(|i| FindTerm::Var(VarId(u16::try_from(i).expect("var id fits u16"))))
            .collect(),
        atoms: vec![Atom {
            relation: rel,
            bindings,
        }],
        negated: vec![],
        conditions: vec![],
    };
    // Free mutations — each one a coin, so the core reaches conditions,
    // negation, extra finds, and arbitrary term substitution (and through
    // them, most of the per-rule roster) without aiming at any line.
    if rng.chance(1, 4) {
        rule.conditions.push(random_tree(rng, 2));
    }
    if rng.chance(1, 8) {
        rule.negated.push(random_atom(rng));
    }
    if rng.chance(1, 8) {
        rule.finds.push(random_find(rng));
    }
    if rng.chance(1, 8) {
        let atom = &mut rule.atoms[0];
        let slot = usize::try_from(
            rng.range(u64::try_from(atom.bindings.len()).expect("binding count fits u64")),
        )
        .expect("slot fits usize");
        atom.bindings[slot].1 = random_term(rng);
    }
    Query::single(rule)
}

fn random_rule(rng: &mut Rng) -> Rule {
    Rule {
        finds: (0..rng.range(4)).map(|_| random_find(rng)).collect(),
        atoms: (0..rng.range(3)).map(|_| random_atom(rng)).collect(),
        negated: (0..rng.range(2)).map(|_| random_atom(rng)).collect(),
        conditions: (0..rng.range(3)).map(|_| random_tree(rng, 3)).collect(),
    }
}

fn random_find(rng: &mut Rng) -> FindTerm {
    match rng.range(4) {
        0 | 1 => FindTerm::Var(var(rng)),
        2 => FindTerm::Aggregate {
            op: random_agg(rng),
            // `None` under a valued op and `Some` under `Count` are both
            // draws — `AggregateWithoutVariable` / `CountWithVariable`.
            over: if rng.chance(3, 4) {
                Some(var(rng))
            } else {
                None
            },
        },
        _ => {
            if rng.chance(1, 2) {
                FindTerm::Duration(var(rng))
            } else {
                FindTerm::AggregateDuration {
                    op: random_agg(rng),
                    over: var(rng),
                }
            }
        }
    }
}

fn random_agg(rng: &mut Rng) -> AggOp {
    match rng.range(8) {
        0 => AggOp::Sum,
        1 => AggOp::Min,
        2 => AggOp::Max,
        3 => AggOp::Count,
        4 => AggOp::CountDistinct,
        5 => AggOp::ArgMax { key: var(rng) },
        6 => AggOp::ArgMin { key: var(rng) },
        _ => AggOp::Pack,
    }
}

fn random_atom(rng: &mut Rng) -> Atom {
    Atom {
        relation: relation(rng),
        bindings: (0..rng.range(4))
            .map(|_| (field(rng), random_term(rng)))
            .collect(),
    }
}

fn random_term(rng: &mut Rng) -> Term {
    match rng.range(8) {
        0..=2 => Term::Var(var(rng)),
        3 => Term::Param(param(rng)),
        4 => Term::ParamSet(param(rng)),
        // `Duration` in a binding position is a typed rejection — a draw,
        // not a mode.
        5 => Term::Duration(var(rng)),
        _ => Term::Literal(random_value(rng)),
    }
}

/// A literal of any shape the value sum can spell — type mismatches and
/// ceiling points remain hostile draws; empty intervals are unspellable.
fn random_value(rng: &mut Rng) -> Value {
    match rng.range(8) {
        0 => Value::Bool(rng.chance(1, 2)),
        1 | 2 => Value::U64(rng.range(16)),
        3 => Value::I64(signed(rng)),
        4 => Value::String(Box::from(&b"Fee"[..])),
        5 => {
            Value::FixedBytes(vec![0xA5; usize::try_from(rng.range(4) * 16).expect("small")].into())
        }
        6 => {
            let start = rng.range(8);
            let end = match rng.range(4) {
                0 => start + 1,
                1 => start + 1 + rng.range(6),
                2 => u64::MAX, // the ray end
                _ => start + 2,
            };
            Value::IntervalU64(
                bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
            )
        }
        _ => {
            let start = signed(rng);
            let end = match rng.range(3) {
                0 => start.saturating_add(1),
                1 => start.saturating_add(1 + i64::try_from(rng.range(6)).expect("small")),
                _ => i64::MAX,
            };
            Value::IntervalI64(
                bumbledb::Interval::<i64>::new(start, end).expect("nonempty interval"),
            )
        }
    }
}

/// A condition tree to `depth`, plus — one draw in 32 — a spine of
/// nested `And`s past [`bumbledb::MAX_CONDITION_DEPTH`] (64): hostile
/// nesting must be the typed `ConditionNestingTooDeep`, never a stack
/// exhaustion (the trust-boundary law).
fn random_tree(rng: &mut Rng, depth: u64) -> ConditionTree {
    if rng.chance(1, 32) {
        let mut spine = ConditionTree::Leaf(random_comparison(rng));
        for _ in 0..60 + rng.range(16) {
            spine = ConditionTree::And(vec![spine]);
        }
        return spine;
    }
    if depth == 0 || rng.chance(2, 5) {
        return ConditionTree::Leaf(random_comparison(rng));
    }
    let children = (0..rng.range(3))
        .map(|_| random_tree(rng, depth - 1))
        .collect();
    if rng.chance(1, 2) {
        ConditionTree::And(children) // And([]) is true — a legal draw
    } else {
        ConditionTree::Or(children) // Or([]) vanishes the rule
    }
}

fn random_comparison(rng: &mut Rng) -> Comparison {
    let op = match rng.range(8) {
        0 => CmpOp::Eq,
        1 => CmpOp::Ne,
        2 => CmpOp::Lt,
        3 => CmpOp::Le,
        4 => CmpOp::Gt,
        5 => CmpOp::Ge,
        6 => CmpOp::Allen {
            mask: random_mask(rng),
        },
        _ => CmpOp::PointIn,
    };
    Comparison {
        op,
        lhs: random_term(rng),
        rhs: random_term(rng),
    }
}

/// Any of the 2¹³ literal masks — ∅ and FULL (the vacuous rejections)
/// included — or a param mask.
fn random_mask(rng: &mut Rng) -> MaskTerm {
    if rng.chance(1, 8) {
        return MaskTerm::Param(param(rng));
    }
    let bits = u16::try_from(rng.range(1 << 13)).expect("13 bits fit u16");
    MaskTerm::Literal(AllenMask::new(bits).expect("13-bit draw is a mask"))
}

/// Mostly within the target theory's relation span, sometimes dangling.
fn relation(rng: &mut Rng) -> RelationId {
    let id = if rng.chance(7, 8) {
        rng.range(RELATION_SPAN)
    } else {
        rng.range(RELATION_SPAN + 3)
    };
    RelationId(u32::try_from(id).expect("relation id fits u32"))
}

fn field(rng: &mut Rng) -> FieldId {
    FieldId(u16::try_from(rng.range(FIELD_SPAN + 2)).expect("field id fits u16"))
}

fn var(rng: &mut Rng) -> VarId {
    VarId(u16::try_from(rng.range(VAR_SPAN)).expect("var id fits u16"))
}

fn param(rng: &mut Rng) -> ParamId {
    ParamId(u16::try_from(rng.range(PARAM_SPAN)).expect("param id fits u16"))
}

/// A small signed draw centered on zero.
fn signed(rng: &mut Rng) -> i64 {
    i64::try_from(rng.range(16)).expect("small draw fits i64") - 8
}

#[cfg(test)]
mod tests {
    use super::random_query;
    use crate::corpus_gen::Rng;
    use crate::querygen::target;

    /// The arm is deterministic in its entropy: the same byte string
    /// yields the identical query, and a different one steers away.
    #[test]
    fn the_same_bytes_yield_the_same_query() {
        let bytes: Vec<u8> = (1..=96u64)
            .flat_map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes())
            .collect();
        let first = random_query(&mut Rng::from_bytes(&bytes));
        assert_eq!(
            first,
            random_query(&mut Rng::from_bytes(&bytes)),
            "same bytes, same query"
        );
        let other: Vec<u8> = (1..=96u64)
            .flat_map(|i| i.wrapping_mul(0xC2B2_AE3D_27D4_EB4F).to_le_bytes())
            .collect();
        assert_ne!(
            first,
            random_query(&mut Rng::from_bytes(&other)),
            "bytes steer the query"
        );
    }

    /// The arm's whole point: across a modest seed sweep the engine both
    /// accepts and rejects — a generator that only produces one verdict
    /// class fuzzes nothing.
    #[test]
    fn the_arm_reaches_both_verdict_classes() {
        let dir = std::env::temp_dir().join("bumbledb-bench-irgen");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, target::Target).expect("create");
        let mut accepted = 0u32;
        let mut rejected = 0u32;
        for seed in 0..512 {
            let query = random_query(&mut Rng::new(seed));
            match db.prepare(&query) {
                Ok(_) => accepted += 1,
                Err(_) => rejected += 1,
            }
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(accepted > 0, "no accepted query in 512 seeds");
        assert!(rejected > 0, "no rejected query in 512 seeds");
        eprintln!("mix: {accepted} accepted / {rejected} rejected");
    }
}
