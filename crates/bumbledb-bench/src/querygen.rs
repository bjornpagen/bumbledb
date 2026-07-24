//! The randomized query generator (docs/architecture/60-validation.md
//! § differential and property tests): seeded random valid queries over
//! the target ledger schema — the fuel for `verify`'s randomized half.
//!
//! Construction is correct **by construction**: fresh dense `VarId`s,
//! dense `ParamId`s allocated at their use site, literals typed from the
//! schema walk, and every comparison operator applied only where its
//! type-legality cell is legal — the illegal cells of the (operator,
//! type) matrix are *unemittable*, not filtered after. The engine's
//! `validate` is the assertion, not the filter: a generated query
//! failing validation is a generator bug.
//!
//! The target schema is the [`target`] seam: the generator's grammar is
//! schema-specific by design, and everything schema-shaped it consumes
//! (relation/field ids, domains, vocabulary, the deterministic corpus
//! value functions) comes from that one module — a schema change lands
//! there without touching the grammar.
//!
//! The **recursive-shape arm** ([`random_program`],
//! `shapes_recursive.rs`) is its own entry beside [`random_query`], not
//! a [`Shape`] row: it emits `Program`s that prepare through
//! `db.prepare` and execute under the fixpoint driver, so its
//! differential runs engine-vs-naive on every program and
//! naive-vs-`SQLite` where expressible (plus the Lean conformance arm)
//! — the shipping law's estate
//! (`docs/architecture/60-validation.md` § the two oracles), all
//! oracles live.

use bumbledb::{
    AllenMask, Atom, CmpOp, Comparison, FieldId, FindTerm, MaskTerm, RelationId, VarId,
};

mod builder;
mod construct;
mod contradict;
mod coverage;
mod dress;
mod dress_posting;
pub mod interval_data;
mod negate;
mod oracle;
mod shapes;
mod shapes_closed;
mod shapes_ground;
mod shapes_interval;
mod shapes_recursive;
mod shapes_rules;
mod shapes_sink;
pub mod target;
#[cfg(test)]
mod tests;
pub mod writes;

pub use construct::random_query;
pub use contradict::contradiction_query;
pub use coverage::{cmp_cell_legal, coverage};
pub use oracle::{ParamDraw, params_for};
pub use shapes_recursive::{
    RecursiveCoverage, RecursiveVariant, random_program, recursive_coverage,
};

/// The shape grammar's weights (drawn by range over the sum). The five
/// original join shapes keep their proportions; the redesign's surface
/// joins the table: point membership, interval joins, the
/// adjacent-touching boundary probes, `CountDistinct` over every type,
/// and Arg-restriction.
const SHAPE_WEIGHTS: &[(Shape, u64)] = &[
    (Shape::KeyProbe, 10),
    (Shape::Star, 15),
    (Shape::Chain, 15),
    (Shape::SelfJoin, 8),
    (Shape::Gated, 8),
    (Shape::Aggregate, 14),
    (Shape::Membership, 10),
    (Shape::IntervalJoin, 10),
    (Shape::Boundary, 6),
    (Shape::CountDistinct, 10),
    (Shape::Arg, 8),
    (Shape::ExistenceWalk, 8),
    (Shape::DuWalk, 6),
    (Shape::Rules, 10),
    (Shape::Measure, 8),
    (Shape::ClosedJoin, 8),
    (Shape::GroundFold, 7),
    (Shape::Pack, 7),
];

/// Filter dressing applies to every shape with this percent chance…
const DRESS_PCT: u64 = 60;
/// …and the repeated in-atom variable to qualifying atoms with this one.
const REPEAT_VAR_PCT: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    KeyProbe,
    Star,
    Chain,
    SelfJoin,
    Gated,
    Aggregate,
    /// Point membership against an interval field: literal, param, and
    /// var points (the var case constructs its scalar anchor first).
    Membership,
    /// `Allen` masks (composites and random singletons) and `Eq`/`Ne`
    /// between interval terms, plus the `PointIn` predicate.
    IntervalJoin,
    /// The adjacent-touching boundary: query literals recomputed to touch
    /// a corpus interval exactly at its endpoint, both polarities.
    Boundary,
    /// `CountDistinct` steered across all six types.
    CountDistinct,
    /// Arg-restriction: `ArgMax`/`ArgMin` over tie-rich and tie-free
    /// keys, key-projected and multi-carry variants.
    Arg,
    /// The grounding's existence walk (`shapes_ground.rs`): the containment
    /// target joined on its full key with nothing else read from it —
    /// eliminable — plus the extra-projected-field near-miss.
    ExistenceWalk,
    /// The discriminated-union one-sided walk, both `==` directions,
    /// plus the missing-φ near-miss.
    DuWalk,
    /// Multi-rule programs (`shapes_rules.rs`): rule counts 2–4,
    /// overlapping and provably-disjoint arm sets (DU-arm unions),
    /// duplicate head answers across
    /// rules, and the rules ∧ aggregate union fold.
    Rules,
    /// The measure over the U64 window lane: `Duration` in a find
    /// position, in a predicate, and folded — total here (the lane's
    /// sentinel end sits below the ray); ray-bearing measure parity is
    /// the verify naive lane's.
    Measure,
    /// Closed relations in the drawable atom pool (`shapes_closed.rs`):
    /// joins against the vocabularies with/without payload projections
    /// and payload-column selections, plus handle literals and handle
    /// param sets on referencing fields.
    ClosedJoin,
    /// The fold-shaped pattern PRD 07 targets, under its own family
    /// knob: a closed atom whose only escaping variable is the join id.
    GroundFold,
    /// The coalescing fold over the Mandate claims (`AggOp::Pack`):
    /// grouped (account or closed-org key) and global, composed with
    /// the shared dressing/param/negation machinery. `SQLite` cannot
    /// spell it — the verify lane routes Pack draws to the naive leg by
    /// the typed expressibility gate (finding 025: the grammar escapes
    /// the ⊆ SQL-expressible cap).
    Pack,
}

/// Which closed-relation class a query is ([`Shape::ClosedJoin`] /
/// [`Shape::GroundFold`]) — the generator's intent, counted by the
/// closed-class self-test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClosedVariant {
    /// A join through the closed atom's id (payload projected or not).
    Join,
    /// The join under a payload-column selection (the ψ shape).
    JoinSelected,
    /// A handle literal on a referencing field.
    HandleLiteral,
    /// A handle param set on a referencing field.
    HandleSet,
    /// The fold shape (dead payload variable included half the time).
    Fold,
}

/// Which grounding-shape variant a query is ([`Shape::ExistenceWalk`] /
/// [`Shape::DuWalk`]) — the generator's intent, which the coverage
/// contract and the engine-backed structural test hold it to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroundVariant {
    /// Eliminable existence walk (projection or aggregate sink).
    Walk,
    /// Near-miss: one extra projected target field — must refuse.
    WalkExtraField,
    /// DU one-sided, the header falls (child-to-header direction).
    DuHeader,
    /// DU one-sided, the child falls (header-to-child direction).
    DuChild,
    /// Near-miss: φ missing from the header occurrence — must refuse.
    DuMissingPhi,
}

/// The generator's queries carry flat conjunctions — every predicate
/// tree is a leaf ([`Builder::into_query`] wraps them); its readers
/// (coverage, oracles, tests) unwrap through here. The tree grammar's
/// OR shapes are the DNF property suite's territory
/// (`naive/tests/dnf.rs`), never the generator's.
fn leaf(tree: &bumbledb::ConditionTree) -> &Comparison {
    match tree {
        bumbledb::ConditionTree::Leaf(comparison) => comparison,
        bumbledb::ConditionTree::And(_) | bumbledb::ConditionTree::Or(_) => {
            unreachable!("the generator emits flat conjunctions only")
        }
    }
}

/// Accumulating query state: atoms, negated atoms, predicates, finds,
/// fresh id counters, the registry of variables the shapes bound
/// (group-key candidates), and each bound variable's anchoring
/// (relation, field) — the provenance the negation pass draws from.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent booleans mirror the external configuration"
)] // independent generation-fact flags.
#[derive(Default)]
struct Builder {
    atoms: Vec<Atom>,
    negated: Vec<Atom>,
    conditions: Vec<Comparison>,
    finds: Vec<FindTerm>,
    next_var: u16,
    next_param: u16,
    bound: Vec<VarId>,
    /// Every `bind_var`'s (var, relation, field) — negation templates and
    /// membership anchors select by provenance, never by hope.
    anchors: Vec<(VarId, RelationId, FieldId)>,
    /// Whether dressing emitted an out-of-vocabulary string or bytes
    /// literal.
    miss: bool,
    /// Whether dressing emitted an in-vocabulary bytes literal (a
    /// recomputed extref) / an out-of-vocabulary one.
    bytes_hit: bool,
    bytes_miss: bool,
    /// Boundary-shape polarity: the query literal touches a corpus
    /// interval at the corpus interval's start / at its end.
    adjacent_left: bool,
    adjacent_right: bool,
    /// Boundary-shape ladder rungs drawn for this query's interval
    /// literals ([`interval_data::Rung`] — systematized for every
    /// interval literal draw).
    ladder: [bool; 4],
    /// Whether an `Allen` predicate carries a random (unnamed) mask.
    random_mask: bool,
    /// Whether an `Allen` predicate carries a bind-time mask param
    /// (`MaskTerm::Param` — finding 086).
    mask_param: bool,
    /// Which grounding-shape variant this query is, when the shape is one.
    ground: Option<GroundVariant>,
    /// Which closed-relation class this query is, when the shape is one.
    closed: Option<ClosedVariant>,
}

impl Builder {
    /// Records one ladder-rung draw ([`interval_data::Rung`]).
    fn saw_rung(&mut self, rung: interval_data::Rung) {
        self.ladder[match rung {
            interval_data::Rung::Equal => 0,
            interval_data::Rung::Adjacent => 1,
            interval_data::Rung::Nested => 2,
            interval_data::Rung::Ray => 3,
        }] = true;
    }
}

/// Generation facts the query alone cannot reveal (hit-vs-miss and the
/// boundary polarities are corpus-content properties; the grounding variant
/// is the generator's intent, engine-verified in the tests).
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent booleans mirror the external configuration"
)] // independent corpus-content tags.
#[derive(Debug, Clone, Copy, Default)]
struct GenTags {
    miss: bool,
    bytes_hit: bool,
    bytes_miss: bool,
    adjacent_left: bool,
    adjacent_right: bool,
    ladder: [bool; 4],
    random_mask: bool,
    mask_param: bool,
    ground: Option<GroundVariant>,
    rules: Option<RulesVariant>,
    closed: Option<ClosedVariant>,
}

/// Which multi-rule variant a [`Shape::Rules`] query is
/// (`shapes_rules.rs`) — the generator's intent, held to its band by the
/// coverage contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RulesVariant {
    /// Provably-disjoint arms: one relation, distinct vocabulary selections on
    /// the discriminant field.
    Disjoint,
    /// Overlapping arms with duplicate head answers across rules (the
    /// union's teeth) — including the DU twin (`JournalEntry` import
    /// arm vs `ImportBatch`, equal denotations by the `==` statement).
    Overlap,
    /// The multi-rule aggregate head (the union fold).
    Aggregate,
}

/// The comparison-type axis of the coverage matrix — all six types.
pub const CMP_TYPES: [&str; 6] = ["u64", "i64", "bool", "string", "bytes", "interval"];
/// The operator axis, in `CmpOp` order — all eight operators (the Allen
/// row counts every mask; the representative here is only a row label).
pub const CMP_OPS: [CmpOp; 8] = [
    CmpOp::Eq,
    CmpOp::Ne,
    CmpOp::Lt,
    CmpOp::Le,
    CmpOp::Gt,
    CmpOp::Ge,
    CmpOp::Allen {
        mask: MaskTerm::Literal(AllenMask::INTERSECTS),
    },
    CmpOp::PointIn,
];

/// Construct counts over a generated batch — the coverage contract's
/// evidence (`60-validation.md`: the exact form the coverage test pins
/// at n = 1000). `matrix[op][type]` counts comparisons per (operator,
/// structural type).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    pub key_probe: u64,
    pub star: u64,
    pub chain: u64,
    pub self_join: u64,
    pub gated: u64,
    pub aggregate: u64,
    pub membership: u64,
    pub interval_join: u64,
    pub boundary: u64,
    pub count_distinct: u64,
    pub arg: u64,
    pub existence_walk: u64,
    pub du_walk: u64,
    pub rules: u64,
    pub measure: u64,
    pub closed_join: u64,
    pub ground_fold: u64,
    pub pack: u64,
    /// The closed-relation pattern classes (`shapes_closed.rs`): the
    /// plain join, the payload-column selection, the handle literal,
    /// and the handle param set — all four counted by the closed-class
    /// self-test (the fourth write-side class lives in [`writes`]).
    pub closed_join_plain: u64,
    pub closed_join_selected: u64,
    pub closed_handle_literal: u64,
    pub closed_handle_set: u64,
    /// The grounding variants (`shapes_ground.rs`): eliminable shapes
    /// (existence walks and both DU `==` directions) vs the near-miss
    /// refusals — the coverage contract asserts both appear per run,
    /// and the engine-backed test holds each tag to its verdict.
    pub ground_eliminable: u64,
    pub ground_extra_field: u64,
    pub ground_missing_phi: u64,
    pub du_header_falls: u64,
    pub du_child_falls: u64,
    pub gates: u64,
    pub misses: u64,
    pub params: u64,
    /// `Term::ParamSet` occurrences (bindings and `Eq` sides).
    pub param_sets: u64,
    pub repeated_vars: u64,
    pub agg_sum: u64,
    pub agg_min: u64,
    pub agg_max: u64,
    pub agg_count: u64,
    /// Aggregates whose input variable is u64-typed.
    pub agg_u64: u64,
    /// Aggregate-bearing find lists with more than one aggregate.
    pub multi_aggregate: u64,
    /// `CountDistinct` inputs per `CMP_TYPES` index — every type.
    pub count_distinct_types: [u64; 6],
    pub arg_max: u64,
    pub arg_min: u64,
    /// Arg terms carrying the key variable itself.
    pub arg_key_projected: u64,
    /// Arg queries with an empty group key (one global group).
    pub arg_global: u64,
    /// Arg keys anchored on the tie-rich field (`Posting.amount`, values
    /// quantized by the corpus) / the tie-free field (`Posting.at`,
    /// strictly monotone by construction).
    pub arg_tie_key: u64,
    pub arg_tie_free_key: u64,
    /// Arg restrictions keyed on an interval measure
    /// (`ArgKey::Measure` — ruled 2026-07-23, R5).
    pub arg_measure_key: u64,
    /// Membership bindings by point-term kind and by element type.
    pub membership_literal: u64,
    pub membership_param: u64,
    pub membership_var: u64,
    pub membership_u64: u64,
    pub membership_i64: u64,
    /// Interval comparisons by element type: `Allen` masks per lane,
    /// composite (≥2 basics) vs singleton mask draws, random (unnamed)
    /// masks, per-basic occurrence across every literal mask (all 13
    /// reachable per run), and `PointIn` per lane.
    pub allen_u64: u64,
    pub allen_i64: u64,
    pub allen_composite: u64,
    pub allen_singleton: u64,
    pub allen_random_mask: u64,
    /// `Allen` predicates whose mask is a bind-time param
    /// (`MaskTerm::Param` — finding 086: the temporal relation as an
    /// argument, its own bind-time vacuous-mask rejection).
    pub allen_mask_param: u64,
    pub allen_basics: [u64; 13],
    pub point_in_u64: u64,
    pub point_in_i64: u64,
    /// Boundary-shape polarities (corpus-adjacent query literals).
    pub adjacent_left: u64,
    pub adjacent_right: u64,
    /// Boundary-shape ladder rungs (equal/adjacent/nested/ray) drawn by
    /// the shapes' interval literals.
    pub ladder: [u64; 4],
    /// Multi-rule programs by arm count (2/3/4) and by variant; the
    /// duplicate-head DU twin counts under overlap.
    pub rules_arms: [u64; 3],
    pub rules_disjoint: u64,
    pub rules_overlap: u64,
    pub rules_aggregate: u64,
    /// The measure's construct kinds: `Duration` finds, predicates, and
    /// folds (`Sum`/`Min`/`Max` over the measure).
    pub duration_find: u64,
    pub duration_predicate: u64,
    pub duration_fold: u64,
    /// Negated atoms, and their binding-shape split: key-covered (a
    /// fresh key field bound) vs open; literal/param/set/membership
    /// bindings inside; zero-binding negated gates; open negations over
    /// the multiply-witnessed relations (rejection must not depend on
    /// witness count).
    pub negations: u64,
    pub negation_key_covered: u64,
    pub negation_open: u64,
    pub negation_literal: u64,
    pub negation_param: u64,
    pub negation_set: u64,
    pub negation_membership: u64,
    pub negation_gate: u64,
    pub negation_multi_witness: u64,
    /// The structural compositions where bugs hide — asserted ≥ 1 per
    /// run.
    pub neg_and_aggregate: u64,
    pub set_and_negation: u64,
    pub membership_and_allen: u64,
    pub mask_and_negation: u64,
    /// Var-vs-var comparisons whose variables bind in different atoms.
    pub cross_residuals: u64,
    /// Wide projections — the >8-projected-word class the executor's
    /// hoist paths must never cap (docs/architecture/40-execution.md,
    /// scan-fold pushdown): all-scalar find lists past 8 words, and
    /// find lists carrying ≥4 interval-typed finds (≥8 interval words).
    pub wide_scalar: u64,
    pub wide_interval: u64,
    /// In-vocabulary / out-of-vocabulary bytes literals.
    pub bytes_hits: u64,
    pub bytes_misses: u64,
    /// Equality-spine cost-bound violations
    /// (`docs/architecture/60-validation.md` § the generator contract):
    /// an atom carrying a var-point membership or a cross-atom
    /// `Allen`/`PointIn` occurrence with neither an equality join
    /// variable nor an equality selection, or a negated atom whose only
    /// bindings are memberships. Asserted **zero** — the Cartesian
    /// degenerate (`40-execution.md`) must be unemittable.
    pub spine_violations: u64,
    /// Comparison counts per `(CMP_OPS index, CMP_TYPES index)`.
    pub matrix: [[u64; 6]; 8],
}

/// Which set each of the four generated param draws is.
const PARAM_DRAWS: usize = 4;

/// Which of the four draws is being filled.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DrawKind {
    Hit,
    Boundary,
    Miss,
}
