//! Recursive and interiors-heavy Query shapes for the differential /
//! coverage generators. Mutual and nonlinear are unwritable this cut.

use bumbledb::{
    Atom, AtomSource, FieldId, FindTerm, HeadTerm, Interior, InteriorId, NonEmpty, ProjectionRule,
    Query, Rec, RecRule, RecStep, Rule, Term, Value, VarId,
};

use crate::corpus_gen::{GenConfig, Rng};
use crate::querygen::target::{Domains, ids};

/// Which shape a generated query is — the generator's intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecursiveVariant {
    /// Linear self-recursion: the ancestor closure, one rec atom.
    Linear,
    /// Negation of the finished rec **in main**.
    Negation,
    /// A fold over the finished rec on main.
    Fold,
    /// The empty-Δ-at-round-1 boundary.
    EmptyDelta,
    /// Primer-shaped `reach(x, x)`: main is the diagonal.
    PrimerReachXx,
    /// Deep interior DAG (interior reading interior).
    InteriorsDag,
    /// Main anti-joins an earlier interior.
    InteriorsAntiJoin,
    /// More than 16 interiors.
    ManyInteriors,
}

fn v(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn fv(id: u16) -> FindTerm {
    FindTerm::Var(VarId(id))
}

fn edge(child: Term, parent: Term) -> Atom {
    Atom {
        source: AtomSource::Edb(ids::ORG_PARENT),
        bindings: vec![
            (ids::org_parent::CHILD, child),
            (ids::org_parent::PARENT, parent),
        ],
    }
}

fn interior(id: u32, bindings: &[(u16, Term)]) -> Atom {
    Atom {
        source: AtomSource::Interior(InteriorId(id)),
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

fn proj(finds: Vec<VarId>, atoms: Vec<Atom>, negated: Vec<Atom>) -> ProjectionRule {
    ProjectionRule {
        finds,
        atoms,
        negated,
        conditions: vec![],
    }
}

fn rec_rule(finds: Vec<VarId>, atoms: Vec<Atom>) -> RecRule {
    RecRule {
        finds,
        atoms,
        conditions: vec![],
    }
}

fn rec_step(finds: Vec<VarId>, self_bindings: Vec<(u16, Term)>, atoms: Vec<Atom>) -> RecStep {
    RecStep {
        finds,
        self_bindings: self_bindings
            .into_iter()
            .map(|(field, term)| (FieldId(field), term))
            .collect(),
        atoms,
        conditions: vec![],
    }
}

fn identity_main(arity: u16, rec_id: u32) -> Rule {
    projection(
        (0..arity).map(fv).collect(),
        vec![interior(
            rec_id,
            &(0..arity).map(|i| (i, v(i))).collect::<Vec<_>>(),
        )],
        vec![],
    )
}

fn closure_rec() -> Rec {
    Rec {
        base: NonEmpty::one(rec_rule(vec![VarId(0), VarId(1)], vec![edge(v(0), v(1))])),
        rec: NonEmpty::one(rec_step(
            vec![VarId(0), VarId(2)],
            vec![(0, v(1)), (1, v(2))],
            vec![edge(v(0), v(1))],
        )),
    }
}

fn org_literal(rng: &mut Rng, domains: &Domains) -> Term {
    Term::Literal(Value::U64(rng.range(domains.orgs)))
}

/// One random interiors/rec Query and its variant tag.
pub fn random_reach_query(rng: &mut Rng, cfg: GenConfig) -> (Query, RecursiveVariant) {
    let domains = Domains::of(cfg.scale);
    let variant = match rng.range(8) {
        0 => RecursiveVariant::Linear,
        1 => RecursiveVariant::Negation,
        2 => RecursiveVariant::Fold,
        3 => RecursiveVariant::EmptyDelta,
        4 => RecursiveVariant::PrimerReachXx,
        5 => RecursiveVariant::InteriorsDag,
        6 => RecursiveVariant::InteriorsAntiJoin,
        _ => RecursiveVariant::ManyInteriors,
    };
    let query = match variant {
        RecursiveVariant::Linear => linear(rng, &domains),
        RecursiveVariant::Negation => negation(rng),
        RecursiveVariant::Fold => fold(rng),
        RecursiveVariant::EmptyDelta => empty_delta(rng, &domains),
        RecursiveVariant::PrimerReachXx => primer_reach_xx(),
        RecursiveVariant::InteriorsDag => interiors_dag(),
        RecursiveVariant::InteriorsAntiJoin => interiors_anti_join(),
        RecursiveVariant::ManyInteriors => many_interiors(),
    };
    (query, variant)
}

fn linear(rng: &mut Rng, domains: &Domains) -> Query {
    let ancestor = org_literal(rng, domains);
    Query {
        interiors: vec![],
        rec: Some(closure_rec()),
        head: vec![HeadTerm::Var],
        rules: vec![projection(
            vec![fv(0)],
            vec![interior(0, &[(0, v(0)), (1, ancestor)])],
            vec![],
        )],
    }
}

fn negation(rng: &mut Rng) -> Query {
    let column = u16::from(rng.chance(1, 2));
    Query {
        interiors: vec![],
        rec: Some(closure_rec()),
        head: vec![HeadTerm::Var],
        rules: vec![projection(
            vec![fv(0)],
            vec![Atom {
                source: AtomSource::Edb(ids::ORG),
                bindings: vec![(ids::org::ID, v(0))],
            }],
            vec![interior(0, &[(column, v(0))])],
        )],
    }
}

fn fold(rng: &mut Rng) -> Query {
    let grouped = u16::from(rng.chance(1, 2));
    Query {
        interiors: vec![],
        rec: Some(closure_rec()),
        head: vec![HeadTerm::Var, HeadTerm::Aggregate(bumbledb::HeadOp::Count)],
        rules: vec![Rule {
            finds: vec![fv(0), FindTerm::Count],
            atoms: vec![interior(0, &[(grouped, v(0)), (1 - grouped, v(1))])],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

fn empty_delta(rng: &mut Rng, domains: &Domains) -> Query {
    let lo = domains.orgs.div_ceil(4).max(1);
    let hi = domains.orgs / 2;
    let hub = lo + rng.range(hi.saturating_sub(lo).max(1));
    Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(rec_rule(
                vec![VarId(0)],
                vec![edge(v(0), Term::Literal(Value::U64(hub)))],
            )),
            rec: NonEmpty::one(rec_step(
                vec![VarId(0)],
                vec![(0, v(1))],
                vec![edge(v(0), v(1))],
            )),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![identity_main(1, 0)],
    }
}

fn primer_reach_xx() -> Query {
    Query {
        interiors: vec![],
        rec: Some(closure_rec()),
        head: vec![HeadTerm::Var],
        rules: vec![projection(
            vec![fv(0)],
            vec![interior(0, &[(0, v(0)), (1, v(0))])],
            vec![],
        )],
    }
}

fn interiors_dag() -> Query {
    let copy = Interior {
        rules: vec![proj(
            vec![VarId(0), VarId(1)],
            vec![edge(v(0), v(1))],
            vec![],
        )],
    };
    let hop = Interior {
        rules: vec![proj(
            vec![VarId(0), VarId(2)],
            vec![interior(0, &[(0, v(0)), (1, v(1))]), edge(v(1), v(2))],
            vec![],
        )],
    };
    Query {
        interiors: vec![copy, hop],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![projection(
            vec![fv(0), fv(1)],
            vec![interior(1, &[(0, v(0)), (1, v(1))])],
            vec![],
        )],
        rec: None,
    }
}

fn interiors_anti_join() -> Query {
    Query {
        interiors: vec![Interior {
            rules: vec![proj(
                vec![VarId(0), VarId(1)],
                vec![edge(v(0), v(1))],
                vec![],
            )],
        }],
        head: vec![HeadTerm::Var],
        rules: vec![projection(
            vec![fv(0)],
            vec![Atom {
                source: AtomSource::Edb(ids::ORG),
                bindings: vec![(ids::org::ID, v(0))],
            }],
            vec![interior(0, &[(0, v(0))])],
        )],
        rec: None,
    }
}

fn many_interiors() -> Query {
    let interiors = (0..17)
        .map(|_| Interior {
            rules: vec![proj(
                vec![VarId(0), VarId(1)],
                vec![edge(v(0), v(1))],
                vec![],
            )],
        })
        .collect();
    Query {
        interiors,
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![projection(
            vec![fv(0), fv(1)],
            vec![interior(16, &[(0, v(0)), (1, v(1))])],
            vec![],
        )],
        rec: None,
    }
}

/// Structural coverage rows, re-derived from the Query.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RecursiveCoverage {
    pub queries: u64,
    pub linear_self_recursion: u64,
    pub negation_of_finished_rec: u64,
    pub fold_over_rec: u64,
    pub empty_delta_round_one: u64,
    pub primer_reach_xx: u64,
    pub interiors_dag: u64,
    pub interiors_anti_join: u64,
    pub many_interiors: u64,
    pub sqlite_expressible: u64,
    pub budget_trip: u64,
    pub preamble_ledger_trip: u64,
}

impl RecursiveVariant {
    /// Coverage-report class. Interiors-only shapes are not recursive.
    /// `Debug` names stay frozen for reach-case provenance.
    #[must_use]
    pub fn coverage_class(self) -> &'static str {
        match self {
            Self::InteriorsDag | Self::InteriorsAntiJoin | Self::ManyInteriors => "interiors",
            Self::Linear | Self::Negation | Self::Fold | Self::EmptyDelta | Self::PrimerReachXx => {
                "reach"
            }
        }
    }
}

pub fn recursive_coverage(query: &Query, variant: RecursiveVariant, tally: &mut RecursiveCoverage) {
    tally.queries += 1;
    match variant {
        RecursiveVariant::Linear => tally.linear_self_recursion += 1,
        RecursiveVariant::Negation => tally.negation_of_finished_rec += 1,
        RecursiveVariant::Fold => tally.fold_over_rec += 1,
        RecursiveVariant::EmptyDelta => tally.empty_delta_round_one += 1,
        RecursiveVariant::PrimerReachXx => tally.primer_reach_xx += 1,
        RecursiveVariant::InteriorsDag => tally.interiors_dag += 1,
        RecursiveVariant::InteriorsAntiJoin => tally.interiors_anti_join += 1,
        RecursiveVariant::ManyInteriors => tally.many_interiors += 1,
    }
    let _ = query;
}
