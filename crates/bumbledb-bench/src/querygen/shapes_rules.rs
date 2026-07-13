//! The multi-rule shapes: programs of 2–4 rules over one head
//! (`docs/architecture/20-query-ir.md`, the rules shape — disjunction as
//! data at the top). Three variants, each holding one obligation:
//!
//! - **Disjoint arms**: one relation, every arm selecting a distinct
//!   vocabulary row id on the discriminant field — provably disjoint,
//!   exercised against the oracles' plain set union.
//! - **Overlapping arms**: nested selections over one relation whose
//!   arm denotations share rows — duplicate head rows across rules, the
//!   union's teeth — including the **DU twin**: the `JournalEntry`
//!   import arm vs `ImportBatch`, equal denotations by the corpus's
//!   `==` statement, total duplication.
//! - **The union fold**: a multi-rule aggregate head (`Sum`/`Count`/
//!   `CountDistinct`) — the fold over the union of head-projected rows.
//!
//! Rules bind literals only (no params): variables are rule-scoped and
//! restart per arm; the head aligns positionally by construction. Like
//! the grounding shapes, these are their own deliberate dressing — a random
//! predicate landing on one arm would not break anything, but the
//! variants' bands are the point, so nothing is appended.

use bumbledb::{
    AggOp, Atom, CmpOp, Comparison, ConditionTree, FindTerm, Query, Rule, Term, Value, VarId,
};

use crate::corpus_gen::Rng;
use crate::querygen::RulesVariant;
use crate::querygen::target::{self, Domains, ids};

/// One multi-rule query and its variant tag.
pub(super) fn rules(rng: &mut Rng, domains: &Domains) -> (Query, RulesVariant) {
    let variant = match rng.range(3) {
        0 => RulesVariant::Disjoint,
        1 => RulesVariant::Overlap,
        _ => RulesVariant::Aggregate,
    };
    let query = match variant {
        RulesVariant::Disjoint => disjoint_arms(rng),
        RulesVariant::Overlap => {
            if rng.chance(1, 3) {
                du_twin()
            } else {
                overlapping_arms(rng, domains)
            }
        }
        RulesVariant::Aggregate => union_fold(rng, domains),
    };
    (query, variant)
}

fn assemble(rules: Vec<Rule>) -> Query {
    Query {
        head: rules[0].head(),
        rules,
    }
}

/// `JournalEntry` arms with distinct `source` selections — disjoint by
/// the vocabulary row id, 2–3 arms (three rows exist), head
/// `[id, created_at]`.
fn disjoint_arms(rng: &mut Rng) -> Query {
    let arms = 2 + rng.range(2);
    let mut ordinals = [0u64, 1, 2];
    // A seeded shuffle: which sources the arms take is drawn, their
    // distinctness is constructed.
    ordinals.swap(0, usize::try_from(rng.range(3)).expect("small"));
    ordinals.swap(1, 1 + usize::try_from(rng.range(2)).expect("small"));
    let rules = ordinals[..usize::try_from(arms).expect("small")]
        .iter()
        .map(|ordinal| Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                relation: ids::JOURNAL_ENTRY,
                bindings: vec![
                    (ids::journal_entry::ID, Term::Var(VarId(0))),
                    (
                        ids::journal_entry::SOURCE,
                        Term::Literal(Value::U64(*ordinal)),
                    ),
                    (ids::journal_entry::CREATED_AT, Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        })
        .collect();
    assemble(rules)
}

/// The DU twin: the import arm of `JournalEntry` vs `ImportBatch` — the
/// corpus's `==` statement makes the two denotations equal, so every
/// head row is a cross-rule duplicate.
fn du_twin() -> Query {
    assemble(vec![
        Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: ids::JOURNAL_ENTRY,
                bindings: vec![
                    (ids::journal_entry::ID, Term::Var(VarId(0))),
                    (
                        ids::journal_entry::SOURCE,
                        Term::Literal(Value::U64(target::SOURCE_IMPORT)),
                    ),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        },
        Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: ids::IMPORT_BATCH,
                bindings: vec![(ids::import_batch::ENTRY, Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![],
        },
    ])
}

/// One `Posting` arm: `Posting(account = v0, at = v1)` under an
/// `at >=` selection.
fn posting_arm(finds: Vec<FindTerm>, floor: i64) -> Rule {
    Rule {
        finds,
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ACCOUNT, Term::Var(VarId(0))),
                (ids::posting::AT, Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::I64(floor)),
        })],
    }
}

/// Overlapping `Posting` arms, 2–4: ascending `at` floors nest the arm
/// denotations, so every later arm's head rows duplicate earlier ones —
/// the union's dedup is load-bearing, not incidental.
fn overlapping_arms(rng: &mut Rng, domains: &Domains) -> Query {
    let arms = 2 + rng.range(3);
    let span = i64::try_from(domains.postings).expect("fits") * target::AT_STEP;
    let wide_head = rng.chance(1, 2);
    let rules = (0..arms)
        .map(|arm| {
            let floor = target::AT_BASE + i64::try_from(arm).expect("small") * (span / 8);
            let finds = if wide_head {
                vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))]
            } else {
                // The account-only head folds many witnesses per row —
                // duplicates within AND across arms.
                vec![FindTerm::Var(VarId(0))]
            };
            posting_arm(finds, floor)
        })
        .collect();
    assemble(rules)
}

/// The multi-rule aggregate head: 2–3 `Posting` arms under one fold —
/// the union-fold path (per-rule head projection, one set union, then
/// the fold), `Sum`/`Count`/`CountDistinct` drawn per query.
fn union_fold(rng: &mut Rng, domains: &Domains) -> Query {
    let arms = 2 + rng.range(2);
    let span = i64::try_from(domains.postings).expect("fits") * target::AT_STEP;
    let aggregate = match rng.range(3) {
        // Sum over quantized amounts: |amount| ≤ 4 000, distinct head
        // rows ≤ postings — bounded far below 2⁶³ at every scale.
        0 => FindTerm::Aggregate {
            op: AggOp::Sum,
            over: Some(VarId(1)),
        },
        // The nullary Count across rules: the constant-filler head
        // position (the union fold's stable-arity rule).
        1 => FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        },
        _ => FindTerm::Aggregate {
            op: AggOp::CountDistinct,
            over: Some(VarId(1)),
        },
    };
    let over_amount = !matches!(
        aggregate,
        FindTerm::Aggregate {
            op: AggOp::Count,
            ..
        }
    );
    let rules = (0..arms)
        .map(|arm| {
            let floor = target::AT_BASE + i64::try_from(arm).expect("small") * (span / 6);
            let mut rule = posting_arm(vec![FindTerm::Var(VarId(0)), aggregate.clone()], floor);
            if over_amount {
                // The fold input: amount, bound beside the selection.
                rule.atoms[0]
                    .bindings
                    .push((ids::posting::AMOUNT, Term::Var(VarId(1))));
                rule.conditions.clear();
                rule.conditions.push(ConditionTree::Leaf(Comparison {
                    op: CmpOp::Ge,
                    lhs: Term::Var(VarId(2)),
                    rhs: Term::Literal(Value::I64(floor)),
                }));
                // `at` rebinds to a fresh selection variable: VarId(1)
                // is the fold input now.
                rule.atoms[0].bindings[1] = (ids::posting::AT, Term::Var(VarId(2)));
            }
            rule
        })
        .collect();
    assemble(rules)
}
