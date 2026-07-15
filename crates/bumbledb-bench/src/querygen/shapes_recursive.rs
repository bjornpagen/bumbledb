//! The recursive-shape arm (the shipping law's generator row,
//! `docs/architecture/60-validation.md` § differential and property
//! tests):
//! seeded random `Program`s over the org tree — `OrgParent(child,
//! parent)` read as edges, `Org` as the node vocabulary. **Closure
//! sizes are bounded by construction** (the cost-bound rule's sibling):
//! the corpus org relation IS a binary tree (`child = i + 1`,
//! `parent = child / 2`), so every ancestor set is a root path of depth
//! `log₂ orgs` and every generated fixpoint stays inside
//! `orgs × log₂ orgs` tuples. Predicate counts are bounded at 2–3 and
//! recursive atoms per rule at 1–2 — programs stay query-shaped, like
//! everything the caps defend.
//!
//! Six variants, one per coverage-contract row (asserted ≥ 1 per run by
//! the querygen coverage test, which also runs every program through
//! the ENGINE's fixpoint driver against the naive fixpoint and — where
//! the `WITH RECURSIVE` gate admits it — through `SQLite`, comparing
//! answer sets):
//! linear self-recursion, a mutual pair, a non-linear rule, negation of
//! a lower stratum, a fold over a recursive predicate from a higher
//! stratum, and the empty-Δ-at-round-1 boundary (constructed: the
//! reachable set below a node whose children are leaves — round one
//! derives nothing, by the tree's own shape).
//!
//! **The budget-trip row is ACTIVE and constructed, never hoped for**
//! ([`RecursiveCoverage::budget_trip`]): the coverage test takes a
//! drawn linear closure, tightens the prepared query's fixpoint budget
//! to zero rounds (`PreparedQuery::set_fixpoint_budget`), and asserts
//! the typed `Error::FixpointBudgetExceeded` — then widens the budget
//! and asserts the same prepared query executes clean (the snapshot
//! stays usable; `MeasureOfRay`'s error model).
//!
//! Entropy rides the ordinary generator seam — one [`Rng`] in, draws
//! by range, `corpus_gen::rng` untouched.

use bumbledb::{
    AggOp, Atom, AtomSource, FieldId, FindTerm, HeadTerm, PredId, PredicateDef, Program, Rule,
    Term, Value, VarId,
};

use crate::corpus_gen::{GenConfig, Rng};
use crate::querygen::target::{Domains, ids};

/// Which recursive-shape variant a program is — the generator's intent;
/// every structural row is re-derived from the program itself by the
/// coverage tally ([`recursive_coverage`]), and the one corpus-content
/// row (the empty first Δ) is dynamically verified by the coverage
/// test against the naive fixpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecursiveVariant {
    /// Linear self-recursion: the ancestor closure, one recursive atom.
    Linear,
    /// A mutual pair: even/odd ancestor-path length, one SCC of two.
    Mutual,
    /// A non-linear rule: `p(x, z) | p(x, y), p(y, z)` — two recursive
    /// atoms.
    NonLinear,
    /// Negation OF a lower stratum: nodes outside the finished closure.
    Negation,
    /// A fold over a recursive predicate from a strictly higher
    /// stratum: `Count` per node over the finished closure.
    Fold,
    /// The empty-Δ-at-round-1 boundary: the reachable set below a node
    /// whose children are leaves — the base round IS the fixpoint.
    EmptyDelta,
}

fn v(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn fv(id: u16) -> FindTerm {
    FindTerm::Var(VarId(id))
}

/// `OrgParent(child = vars[0], parent = vars[1])` — the edge atom.
fn edge(child: Term, parent: Term) -> Atom {
    Atom {
        source: AtomSource::Edb(ids::ORG_PARENT),
        bindings: vec![
            (ids::org_parent::CHILD, child),
            (ids::org_parent::PARENT, parent),
        ],
    }
}

/// An `Idb` atom over positional head columns.
fn idb(pred: u16, bindings: &[(u16, Term)]) -> Atom {
    Atom {
        source: AtomSource::Idb(PredId(pred)),
        bindings: bindings
            .iter()
            .map(|(field, term)| (FieldId(*field), term.clone()))
            .collect(),
    }
}

fn projection(finds: Vec<FindTerm>, atoms: Vec<Atom>, negated: Vec<Atom>) -> Rule {
    Rule {
        finds,
        atoms,
        negated,
        conditions: vec![],
    }
}

/// The ancestor closure predicate: `p{self}(x, a) | OrgParent(x, a);
/// p{self}(x, a) | OrgParent(x, y), p{self}(y, a)` — linear, one
/// recursive atom, projection-shaped.
fn closure_predicate(this: u16) -> PredicateDef {
    PredicateDef {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![
            projection(vec![fv(0), fv(1)], vec![edge(v(0), v(1))], vec![]),
            projection(
                vec![fv(0), fv(2)],
                vec![edge(v(0), v(1)), idb(this, &[(0, v(1)), (1, v(2))])],
                vec![],
            ),
        ],
    }
}

/// A drawn org id — closure selections stay in-domain, so answers are
/// real subsets.
fn org_literal(rng: &mut Rng, domains: &Domains) -> Term {
    Term::Literal(Value::U64(rng.range(domains.orgs)))
}

/// One random recursive program and its variant tag. Predicate counts
/// 2–3, recursive atoms per rule 1–2, closure sizes bounded by the org
/// tree (module doc).
pub fn random_program(rng: &mut Rng, cfg: GenConfig) -> (Program, RecursiveVariant) {
    let domains = Domains::of(cfg.scale);
    let variant = match rng.range(6) {
        0 => RecursiveVariant::Linear,
        1 => RecursiveVariant::Mutual,
        2 => RecursiveVariant::NonLinear,
        3 => RecursiveVariant::Negation,
        4 => RecursiveVariant::Fold,
        _ => RecursiveVariant::EmptyDelta,
    };
    let program = match variant {
        RecursiveVariant::Linear => linear(rng, &domains),
        RecursiveVariant::Mutual => mutual(rng),
        RecursiveVariant::NonLinear => non_linear(rng, &domains),
        RecursiveVariant::Negation => negation(rng),
        RecursiveVariant::Fold => fold(rng),
        RecursiveVariant::EmptyDelta => empty_delta(rng, &domains),
    };
    (program, variant)
}

/// Linear self-recursion, 2–3 predicates: the closure, a selecting
/// consumer (`p1(x) | p0(x, a = lit)` — descendants of a drawn org),
/// and — half the time — a third predicate joining the vocabulary
/// (`p2(x) | Org(id = x), p1(x)`).
fn linear(rng: &mut Rng, domains: &Domains) -> Program {
    let ancestor = org_literal(rng, domains);
    let mut predicates = vec![
        closure_predicate(0),
        PredicateDef {
            head: vec![HeadTerm::Var],
            rules: vec![projection(
                vec![fv(0)],
                vec![idb(0, &[(0, v(0)), (1, ancestor)])],
                vec![],
            )],
        },
    ];
    let mut output = PredId(1);
    if rng.chance(1, 2) {
        predicates.push(PredicateDef {
            head: vec![HeadTerm::Var],
            rules: vec![projection(
                vec![fv(0)],
                vec![
                    Atom {
                        source: AtomSource::Edb(ids::ORG),
                        bindings: vec![(ids::org::ID, v(0))],
                    },
                    idb(1, &[(0, v(0))]),
                ],
                vec![],
            )],
        });
        output = PredId(2);
    }
    Program { predicates, output }
}

/// The mutual pair — one SCC of two predicates iterating jointly:
/// `even(x, a) | OrgParent(x, y), odd(y, a)` beside
/// `odd(x, a) | OrgParent(x, a); odd(x, a) | OrgParent(x, y),
/// even(y, a)`; the output side is drawn.
fn mutual(rng: &mut Rng) -> Program {
    let even = PredicateDef {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![projection(
            vec![fv(0), fv(2)],
            vec![edge(v(0), v(1)), idb(1, &[(0, v(1)), (1, v(2))])],
            vec![],
        )],
    };
    let odd = PredicateDef {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![
            projection(vec![fv(0), fv(1)], vec![edge(v(0), v(1))], vec![]),
            projection(
                vec![fv(0), fv(2)],
                vec![edge(v(0), v(1)), idb(0, &[(0, v(1)), (1, v(2))])],
                vec![],
            ),
        ],
    };
    Program {
        predicates: vec![even, odd],
        output: PredId(u16::from(rng.chance(1, 2))),
    }
}

/// The non-linear closure — two recursive atoms in one rule:
/// `p0(x, z) | p0(x, y), p0(y, z)` beside the edge base, plus the
/// selecting consumer.
fn non_linear(rng: &mut Rng, domains: &Domains) -> Program {
    let ancestor = org_literal(rng, domains);
    Program {
        predicates: vec![
            PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![
                    projection(vec![fv(0), fv(1)], vec![edge(v(0), v(1))], vec![]),
                    projection(
                        vec![fv(0), fv(2)],
                        vec![
                            idb(0, &[(0, v(0)), (1, v(1))]),
                            idb(0, &[(0, v(1)), (1, v(2))]),
                        ],
                        vec![],
                    ),
                ],
            },
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![projection(
                    vec![fv(0)],
                    vec![idb(0, &[(0, v(0)), (1, ancestor)])],
                    vec![],
                )],
            },
        ],
        output: PredId(1),
    }
}

/// Negation of a lower stratum: `p1(x) | Org(id = x), ¬p0(c = x)` — the
/// negated column (ancestor side or descendant side) is drawn.
fn negation(rng: &mut Rng) -> Program {
    let column = u16::from(rng.chance(1, 2));
    Program {
        predicates: vec![
            closure_predicate(0),
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![projection(
                    vec![fv(0)],
                    vec![Atom {
                        source: AtomSource::Edb(ids::ORG),
                        bindings: vec![(ids::org::ID, v(0))],
                    }],
                    vec![idb(0, &[(column, v(0))])],
                )],
            },
        ],
        output: PredId(1),
    }
}

/// A fold over the finished closure from a strictly higher stratum:
/// `p1(x, Count) | p0(x, a)` — ancestor counts per node (or, drawn, the
/// descendant counts through the other column).
fn fold(rng: &mut Rng) -> Program {
    let grouped = u16::from(rng.chance(1, 2));
    Program {
        predicates: vec![
            closure_predicate(0),
            PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Aggregate(bumbledb::HeadOp::Count)],
                rules: vec![Rule {
                    finds: vec![
                        fv(0),
                        FindTerm::Aggregate {
                            op: AggOp::Count,
                            over: None,
                        },
                    ],
                    atoms: vec![idb(0, &[(grouped, v(0)), (1 - grouped, v(1))])],
                    negated: vec![],
                    conditions: vec![],
                }],
            },
        ],
        output: PredId(1),
    }
}

/// The empty-Δ-at-round-1 boundary, constructed: the reachable set
/// below a node whose children are LEAVES of the org tree (`child =
/// i + 1, parent = child / 2` — a node `p` with `2p < orgs ≤ 4p` has
/// children and no grandchildren), so the recursive round derives
/// nothing and the base round is the fixpoint.
fn empty_delta(rng: &mut Rng, domains: &Domains) -> Program {
    let lo = domains.orgs.div_ceil(4).max(1);
    let hi = domains.orgs / 2;
    let hub = lo + rng.range(hi.saturating_sub(lo).max(1));
    Program {
        predicates: vec![
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![
                    projection(
                        vec![fv(0)],
                        vec![edge(v(0), Term::Literal(Value::U64(hub)))],
                        vec![],
                    ),
                    projection(
                        vec![fv(0)],
                        vec![edge(v(0), v(1)), idb(0, &[(0, v(1))])],
                        vec![],
                    ),
                ],
            },
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![projection(vec![fv(0)], vec![idb(0, &[(0, v(0))])], vec![])],
            },
        ],
        output: PredId(1),
    }
}

/// The structural coverage rows, re-derived from the program itself
/// (the coverage discipline: tags carry corpus-content facts only).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RecursiveCoverage {
    /// Programs generated.
    pub programs: u64,
    /// Some rule reads its own predicate through exactly one atom.
    pub linear_self_recursion: u64,
    /// Two distinct predicates read each other (one SCC of two).
    pub mutual_pair: u64,
    /// Some rule carries two recursive atoms.
    pub non_linear_rule: u64,
    /// Some negated occurrence targets a predicate (a lower stratum —
    /// validation refused every other placement).
    pub negation_of_lower_stratum: u64,
    /// Some fold rule reads a recursive predicate (from a higher
    /// stratum — `AggregationThroughCycle` refused the rest).
    pub fold_over_recursive: u64,
    /// Programs the generator tagged as the empty-Δ boundary (the
    /// corpus-content row; the coverage test verifies it dynamically
    /// against the naive fixpoint).
    pub empty_delta_round_one: u64,
    /// Programs by predicate count − 2 (the 2–3 bound).
    pub predicate_counts: [u64; 2],
    /// The SQLite lane's routing tally — expressible vs the enumerated
    /// classes, counted, never silent.
    pub sqlite_expressible: u64,
    pub sqlite_non_linear: u64,
    pub sqlite_mutual: u64,
    pub sqlite_fold: u64,
    /// Constructed budget trips: a drawn closure under a zero-round
    /// budget raised the typed `Error::FixpointBudgetExceeded` (module
    /// doc — active, never hoped for).
    pub budget_trip: u64,
}

/// Tallies one program's structural rows.
pub fn recursive_coverage(program: &Program, tally: &mut RecursiveCoverage) {
    tally.programs += 1;
    tally.predicate_counts[program.predicates.len() - 2] += 1;
    let count = program.predicates.len();
    let mut reads = vec![vec![false; count]; count];
    let mut fold_reads: Vec<usize> = Vec::new();
    for (index, def) in program.predicates.iter().enumerate() {
        for rule in &def.rules {
            let fold = rule.finds.iter().any(|find| {
                matches!(
                    find,
                    FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. }
                )
            });
            let mut self_atoms = 0usize;
            for atom in &rule.atoms {
                let Some(pred) = atom.source.idb() else {
                    continue;
                };
                reads[index][usize::from(pred.0)] = true;
                if usize::from(pred.0) == index {
                    self_atoms += 1;
                }
                if fold {
                    fold_reads.push(usize::from(pred.0));
                }
            }
            for atom in &rule.negated {
                if let Some(pred) = atom.source.idb() {
                    reads[index][usize::from(pred.0)] = true;
                    tally.negation_of_lower_stratum += 1;
                }
            }
            match self_atoms {
                1 => tally.linear_self_recursion += 1,
                2.. => tally.non_linear_rule += 1,
                0 => {}
            }
        }
    }
    // The reachability closure: mutual pairs, and which predicates are
    // genuinely recursive (a fold's read must be one for its row).
    loop {
        let mut changed = false;
        for from in 0..count {
            for via in 0..count {
                if !reads[from][via] {
                    continue;
                }
                let via_row = reads[via].clone();
                for (to, reachable) in via_row.iter().enumerate() {
                    if *reachable && !reads[from][to] {
                        reads[from][to] = true;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    for (index, row) in reads.iter().enumerate() {
        for (target, forward) in row.iter().enumerate().skip(index + 1) {
            if *forward && reads[target][index] {
                tally.mutual_pair += 1;
            }
        }
    }
    tally.fold_over_recursive += fold_reads
        .iter()
        .filter(|target| reads[**target][**target])
        .count() as u64;
}
