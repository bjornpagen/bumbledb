//! The program roster (20-query-ir.md § engine recursion): the well-formedness
//! screen, the strata judge's typed refusals, and the signature
//! fixpoint's `Idb` typing — one reject test per roster item, plus the
//! degenerate acceptance and the legal-stratification controls (which
//! must validate WHOLE, proving the strata judge refuses nothing
//! legal; the R1 execution fence was deleted when the fixpoint driver
//! landed — recursive programs are now accepted end to end).

use super::*;
use crate::ir::{AggOp, AtomSource, HeadTerm, MAX_PREDICATES, PredId, PredicateDef, Program};

fn idb(pred: u16, bindings: Vec<(u16, Term)>) -> crate::ir::Atom {
    crate::ir::Atom {
        source: AtomSource::Idb(PredId(pred)),
        bindings: bindings.into_iter().map(|(f, t)| (FieldId(f), t)).collect(),
    }
}

fn rule(finds: Vec<FindTerm>, atoms: Vec<crate::ir::Atom>) -> Rule {
    Rule {
        finds,
        atoms,
        negated: vec![],
        conditions: vec![],
    }
}

/// `p0(x) | Account(id: x)` — the all-`Edb` base case, sealing `p0` as
/// `(u64)`.
fn base_predicate() -> PredicateDef {
    PredicateDef {
        head: vec![HeadTerm::Var],
        rules: vec![rule(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(ACCOUNT, vec![(0, var(0))])],
        )],
    }
}

/// The linear closure shape: `p0(x) | Account(id: x)` and
/// `p0(x) | p0(x)` — well-formed, stratified, safe; validates whole.
fn recursive_program() -> Program {
    Program {
        predicates: vec![PredicateDef {
            head: vec![HeadTerm::Var],
            rules: vec![
                rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![atom(ACCOUNT, vec![(0, var(0))])],
                ),
                rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![idb(0, vec![(0, var(0))])],
                ),
            ],
        }],
        output: PredId(0),
    }
}

fn expect_program_err(program: &Program) -> ValidationError {
    validate_program(&schema(), program).expect_err("must reject")
}

// --- the degenerate embedding ------------------------------------------

#[test]
fn a_degenerate_program_validates_as_its_query() {
    // The one-predicate no-`Idb` program IS today's query
    // (`lean/Bumbledb/Exec/Fixpoint.lean: degenerate_embedding`): both
    // paths seal the same predicate signature, and the trivial stratum
    // witness assigns the one predicate stratum 0.
    let query = simple(
        vec![FindTerm::Var(VarId(0))],
        vec![atom(ACCOUNT, vec![(0, var(0))])],
    );
    let program = Program::from(query.clone());
    let witness = validate_program(&schema(), &program).expect("degenerate program validates");
    let query_witness = validate(&schema(), &query).expect("query validates");
    assert_eq!(witness.output(), PredId(0));
    assert_eq!(witness.strata(), &[0]);
    assert_eq!(
        witness.output_witness().predicate(),
        query_witness.predicate(),
        "one signature derivation, both paths"
    );
}

// --- the shape roster ----------------------------------------------------

#[test]
fn rejects_too_many_predicates() {
    let program = Program {
        predicates: (0..=MAX_PREDICATES).map(|_| base_predicate()).collect(),
        output: PredId(0),
    };
    assert_eq!(
        expect_program_err(&program),
        ValidationError::TooManyPredicates {
            count: MAX_PREDICATES + 1
        }
    );
}

#[test]
fn rejects_an_unknown_output_predicate() {
    let program = Program {
        predicates: vec![base_predicate()],
        output: PredId(3),
    };
    assert_eq!(
        expect_program_err(&program),
        ValidationError::UnknownOutputPredicate { pred: PredId(3) }
    );
}

// --- the well-formedness screen (Program.WellFormed) ---------------------

#[test]
fn rejects_an_idb_atom_naming_no_predicate() {
    let mut program = recursive_program();
    program.predicates[0].rules[1].atoms[0].source = AtomSource::Idb(PredId(5));
    assert_eq!(
        expect_program_err(&program),
        ValidationError::UnknownPredicate {
            atom: 0,
            pred: PredId(5)
        }
    );
}

#[test]
fn rejects_a_negated_phantom_read() {
    // The screen's whole point (`lean/Bumbledb/Query/Syntax.lean`, the
    // module doc's gap record): a NEGATED phantom read would be
    // vacuously satisfied, and the stratification witness alone never
    // refuses it.
    let mut program = recursive_program();
    program.predicates[0].rules[0].negated = vec![idb(9, vec![(0, var(0))])];
    assert_eq!(
        expect_program_err(&program),
        ValidationError::UnknownPredicate {
            atom: 1,
            pred: PredId(9)
        }
    );
}

#[test]
fn rejects_an_idb_binding_beyond_the_target_arity() {
    let mut program = recursive_program();
    program.predicates[0].rules[1].atoms[0]
        .bindings
        .push((FieldId(4), var(1)));
    assert_eq!(
        expect_program_err(&program),
        ValidationError::PredicateColumnOutOfRange {
            atom: 0,
            field: FieldId(4)
        }
    );
}

// --- the strata judge's refusals ------------------------------------------

#[test]
fn rejects_negation_through_a_cycle() {
    let program = Program {
        predicates: vec![PredicateDef {
            head: vec![HeadTerm::Var],
            rules: vec![
                rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![atom(ACCOUNT, vec![(0, var(0))])],
                ),
                Rule {
                    finds: vec![FindTerm::Var(VarId(0))],
                    atoms: vec![atom(ACCOUNT, vec![(0, var(0))])],
                    negated: vec![idb(0, vec![(0, var(0))])],
                    conditions: vec![],
                },
            ],
        }],
        output: PredId(0),
    };
    assert_eq!(
        expect_program_err(&program),
        ValidationError::NegationThroughCycle {
            pred: PredId(0),
            via: PredId(0)
        }
    );
}

#[test]
fn rejects_aggregation_through_a_cycle() {
    let program = Program {
        predicates: vec![PredicateDef {
            head: vec![HeadTerm::Var, HeadTerm::Aggregate(crate::ir::HeadOp::Count)],
            rules: vec![rule(
                vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Count,
                        over: None,
                    },
                ],
                vec![atom(ACCOUNT, vec![(0, var(0))]), idb(0, vec![(0, var(0))])],
            )],
        }],
        output: PredId(0),
    };
    assert_eq!(
        expect_program_err(&program),
        ValidationError::AggregationThroughCycle {
            pred: PredId(0),
            via: PredId(0)
        }
    );
}

#[test]
fn rejects_a_measure_in_a_recursive_head() {
    let program = Program {
        predicates: vec![PredicateDef {
            head: vec![HeadTerm::Var],
            rules: vec![
                rule(
                    vec![FindTerm::Measure(VarId(0))],
                    vec![atom(ACCOUNT, vec![(VALIDITY, var(0))])],
                ),
                rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![idb(0, vec![(0, var(0))])],
                ),
            ],
        }],
        output: PredId(0),
    };
    assert_eq!(
        expect_program_err(&program),
        ValidationError::MeasureInRecursiveHead { pred: PredId(0) }
    );
}

#[test]
fn a_measure_head_over_a_lower_stratum_is_legal() {
    // The legality control for the measure refusal: the measure over a
    // LOWER stratum from a non-recursive head stays legal — it
    // evaluates after the fixpoint is a set — so the program validates
    // whole.
    let program = Program {
        predicates: vec![
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![atom(ACCOUNT, vec![(VALIDITY, var(0))])],
                )],
            },
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![rule(
                    vec![FindTerm::Measure(VarId(0))],
                    vec![idb(0, vec![(0, var(0))])],
                )],
            },
        ],
        output: PredId(1),
    };
    let witness = validate_program(&schema(), &program).expect("legal: the fixpoint is a set");
    assert_eq!(witness.strata(), &[0, 1]);
}

#[test]
fn rejects_a_measure_in_an_interior_head() {
    // The fanout's witness, locked: a NON-recursive interior predicate
    // projecting a measure — `p0(x, |v|) | Account(id: x, validity: v)`
    // with the output `p1` reading `Idb(0)`. The Lean cut cannot
    // represent the shape (`lean/Bumbledb/Query/Syntax.lean: PRule` has
    // `finds : List VarId`), so the engine refuses it rather than
    // executing a class with zero oracle coverage — the measure-half of
    // the executable-class item, beside `AggregateInteriorPredicate`.
    // The measure at the OUTPUT head stays legal (the control above).
    let program = Program {
        predicates: vec![
            PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![rule(
                    vec![FindTerm::Var(VarId(0)), FindTerm::Measure(VarId(1))],
                    vec![atom(ACCOUNT, vec![(0, var(0)), (VALIDITY, var(1))])],
                )],
            },
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![idb(0, vec![(0, var(0))])],
                )],
            },
        ],
        output: PredId(1),
    };
    assert_eq!(
        expect_program_err(&program),
        ValidationError::MeasureInteriorPredicate { pred: PredId(0) }
    );
}

// --- the signature fixpoint -------------------------------------------------

#[test]
fn rejects_a_signature_that_never_seals() {
    // `p0(x) | p0(x)` — safe, stratified, well-formed, and untypable:
    // no stored column ever names `x`'s type.
    let program = Program {
        predicates: vec![PredicateDef {
            head: vec![HeadTerm::Var],
            rules: vec![rule(
                vec![FindTerm::Var(VarId(0))],
                vec![idb(0, vec![(0, var(0))])],
            )],
        }],
        output: PredId(0),
    };
    assert_eq!(
        expect_program_err(&program),
        ValidationError::UnresolvedPredicateSignature { pred: PredId(0) }
    );
}

#[test]
fn idb_anchors_resolve_against_the_sealed_columns() {
    // `p0` seals as `(u64)` from `Account.id`; `p1` binds `p0`'s column
    // 0 to a variable it also binds at `Posting.amount` (i64) — the
    // `Idb` anchor is a REAL anchor, so the conflict is typed.
    let program = Program {
        predicates: vec![
            base_predicate(),
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![idb(0, vec![(0, var(0))]), atom(POSTING, vec![(2, var(0))])],
                )],
            },
        ],
        output: PredId(1),
    };
    assert_eq!(
        expect_program_err(&program),
        ValidationError::VariableTypeConflict { var: VarId(0) }
    );
}

#[test]
fn recursive_rules_align_against_the_sealing_rules_row() {
    // The base rule seals `p0` as `(u64)`; the recursive rule projects
    // an i64 (`Posting.amount`) at the same position — the per-predicate
    // head-type alignment, pinned by the sealing rule.
    let program = Program {
        predicates: vec![PredicateDef {
            head: vec![HeadTerm::Var],
            rules: vec![
                rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![atom(ACCOUNT, vec![(0, var(0))])],
                ),
                rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![
                        idb(0, vec![(0, var(1))]),
                        atom(POSTING, vec![(2, var(0)), (1, var(1))]),
                    ],
                ),
            ],
        }],
        output: PredId(0),
    };
    assert_eq!(
        expect_program_err(&program),
        ValidationError::HeadTypeMismatch {
            rule: 1,
            position: 0
        }
    );
}

#[test]
fn an_interval_predicate_column_reads_bivalently() {
    // `p0` seals `(interval<u64>)` from `Account.validity`; `p1` binds
    // that column to a u64-anchored variable — the membership reading
    // (an interval-typed predicate column participates in point
    // membership exactly as an interval field does), so the program
    // validates whole.
    let program = Program {
        predicates: vec![
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![atom(ACCOUNT, vec![(VALIDITY, var(0))])],
                )],
            },
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![atom(POSTING, vec![(1, var(0))]), idb(0, vec![(0, var(0))])],
                )],
            },
        ],
        output: PredId(1),
    };
    let witness = validate_program(&schema(), &program).expect("legal: the membership typing rule");
    assert_eq!(witness.strata(), &[0, 1]);
}

// --- the legal recursive shapes (the deleted fence's controls) ---------------

#[test]
fn a_wellformed_recursive_program_validates_whole() {
    // Linear self-recursion passes the WHOLE roster — the screen, the
    // strata judge, the signature fixpoint — and seals; the execution
    // fence died with the fixpoint driver (`api/prepared/fixpoint.rs`).
    let witness =
        validate_program(&schema(), &recursive_program()).expect("recursion is executable");
    assert_eq!(witness.strata(), &[0]);
}

#[test]
fn mutual_recursion_validates_into_one_stratum() {
    // `p0 ↔ p1` — one SCC, iterated jointly under the driver's round
    // loop; nothing in the roster refuses the shape.
    let mutual = |other: u16| PredicateDef {
        head: vec![HeadTerm::Var],
        rules: vec![
            rule(
                vec![FindTerm::Var(VarId(0))],
                vec![atom(ACCOUNT, vec![(0, var(0))])],
            ),
            rule(
                vec![FindTerm::Var(VarId(0))],
                vec![idb(other, vec![(0, var(0))])],
            ),
        ],
    };
    let program = Program {
        predicates: vec![mutual(1), mutual(0)],
        output: PredId(0),
    };
    let witness = validate_program(&schema(), &program).expect("mutual recursion is ordinary");
    assert_eq!(
        witness.strata()[0],
        witness.strata()[1],
        "one SCC, one stratum"
    );
}

#[test]
fn negation_of_a_lower_stratum_passes_the_strata_judge() {
    // `p1` negates the FINISHED `p0` — legal stratified negation
    // (`lean/Bumbledb/Exec/Fixpoint.lean: stratumOp_mono` is exactly
    // why), never `NegationThroughCycle`.
    let program = Program {
        predicates: vec![
            base_predicate(),
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![Rule {
                    finds: vec![FindTerm::Var(VarId(0))],
                    atoms: vec![atom(POSTING, vec![(1, var(0))])],
                    negated: vec![idb(0, vec![(0, var(0))])],
                    conditions: vec![],
                }],
            },
        ],
        output: PredId(1),
    };
    let witness = validate_program(&schema(), &program).expect("stratified negation is legal");
    assert!(
        witness.strata()[0] < witness.strata()[1],
        "the negated target sits strictly lower"
    );
}

#[test]
fn an_idb_carrying_query_refuses_at_the_query_boundary() {
    // A bare `Query` has no predicate address space
    // (`IdbSignatures::EMPTY`), and a `ValidatedQuery` cannot carry a
    // fixpoint — recursion's surface is the program boundary
    // (`Db::prepare_program`), so the `Idb` atom refuses with the
    // screen's own vocabulary.
    let query = Query {
        head: vec![HeadTerm::Var],
        rules: vec![
            Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![atom(ACCOUNT, vec![(0, var(0))])],
                negated: vec![],
                conditions: vec![],
            },
            Rule {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![idb(0, vec![(0, var(0))])],
                negated: vec![],
                conditions: vec![],
            },
        ],
    };
    assert_eq!(
        expect_err(&query),
        ValidationError::UnknownPredicate {
            atom: 0,
            pred: PredId(0)
        }
    );
}

#[test]
fn program_wide_params_unify_across_predicates() {
    // Params are program-global: `?0` anchored u64 in `p0` and i64 in
    // `p1` is one typed conflict.
    let program = Program {
        predicates: vec![
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![atom(
                        ACCOUNT,
                        vec![(0, var(0)), (2, Term::Param(ParamId(0)))],
                    )],
                )],
            },
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![rule(
                    vec![FindTerm::Var(VarId(0))],
                    vec![
                        idb(0, vec![(0, var(0))]),
                        atom(POSTING, vec![(2, Term::Param(ParamId(0))), (1, var(1))]),
                    ],
                )],
            },
        ],
        output: PredId(1),
    };
    assert_eq!(
        expect_program_err(&program),
        ValidationError::ParamTypeConflict { param: ParamId(0) }
    );
}
