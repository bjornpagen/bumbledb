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

use bumbledb::{
    AllenMask, Atom, CmpOp, Comparison, FieldId, FindTerm, MaskTerm, RelationId, VarId,
};

mod builder;
mod construct;
mod coverage;
mod dress;
mod dress_posting;
pub mod interval_data;
mod negate;
mod oracle;
mod shapes;
mod shapes_chase;
mod shapes_interval;
mod shapes_sink;
pub mod target;
#[cfg(test)]
mod tests;

pub use construct::random_query;
pub use coverage::{cmp_cell_legal, coverage};
pub use oracle::{params_for, ParamDraw};

/// The shape grammar's weights (drawn by range over the sum). The five
/// original join shapes keep their proportions; the redesign's surface
/// joins the table: point membership, interval joins, the
/// adjacent-touching boundary probes, `CountDistinct` over every type,
/// and Arg-restriction.
const SHAPE_WEIGHTS: &[(Shape, u64)] = &[
    (Shape::Guard, 10),
    (Shape::Star, 15),
    (Shape::Chain, 15),
    (Shape::SelfJoin, 8),
    (Shape::Gated, 8),
    (Shape::Aggregate, 14),
    (Shape::Membership, 10),
    (Shape::IntervalJoin, 8),
    (Shape::Boundary, 4),
    (Shape::CountDistinct, 10),
    (Shape::Arg, 8),
    (Shape::ExistenceWalk, 8),
    (Shape::DuWalk, 6),
];

/// Filter dressing applies to every shape with this percent chance…
const DRESS_PCT: u64 = 60;
/// …and the repeated in-atom variable to qualifying atoms with this one.
const REPEAT_VAR_PCT: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Guard,
    Star,
    Chain,
    SelfJoin,
    Gated,
    Aggregate,
    /// Point membership against an interval field: literal, param, and
    /// var points (the var case constructs its scalar anchor first).
    Membership,
    /// `Allen` masks (composites and random singletons) and `Eq`/`Ne`
    /// between interval terms, plus the point form of `Contains`.
    IntervalJoin,
    /// The adjacent-touching boundary: query literals recomputed to touch
    /// a corpus interval exactly at its endpoint, both polarities.
    Boundary,
    /// `CountDistinct` steered across all seven types.
    CountDistinct,
    /// Arg-restriction: `ArgMax`/`ArgMin` over tie-rich and tie-free
    /// keys, key-projected and multi-carry variants.
    Arg,
    /// The chase's existence walk (`shapes_chase.rs`): the containment
    /// target joined on its full key with nothing else read from it —
    /// eliminable — plus the extra-projected-field near-miss.
    ExistenceWalk,
    /// The discriminated-union one-sided walk, both `==` directions,
    /// plus the missing-φ near-miss.
    DuWalk,
}

/// Which chase-shape variant a query is ([`Shape::ExistenceWalk`] /
/// [`Shape::DuWalk`]) — the generator's intent, which the coverage
/// contract and the engine-backed structural test hold it to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChaseVariant {
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

/// Accumulating query state: atoms, negated atoms, predicates, finds,
/// fresh id counters, the registry of variables the shapes bound
/// (group-key candidates), and each bound variable's anchoring
/// (relation, field) — the provenance the negation pass draws from.
#[allow(clippy::struct_excessive_bools)] // independent generation-fact flags.
#[derive(Default)]
struct Builder {
    atoms: Vec<Atom>,
    negated: Vec<Atom>,
    predicates: Vec<Comparison>,
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
    /// Which chase-shape variant this query is, when the shape is one.
    chase: Option<ChaseVariant>,
}

/// Generation facts the query alone cannot reveal (hit-vs-miss and the
/// boundary polarities are corpus-content properties; the chase variant
/// is the generator's intent, engine-verified in the tests).
#[allow(clippy::struct_excessive_bools)] // independent corpus-content tags.
#[derive(Debug, Clone, Copy, Default)]
struct GenTags {
    miss: bool,
    bytes_hit: bool,
    bytes_miss: bool,
    adjacent_left: bool,
    adjacent_right: bool,
    chase: Option<ChaseVariant>,
}

/// The comparison-type axis of the coverage matrix — all seven types.
pub const CMP_TYPES: [&str; 7] = ["u64", "i64", "enum", "bool", "string", "bytes", "interval"];
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
    CmpOp::Contains,
];

/// Construct counts over a generated batch — the coverage contract's
/// evidence (`60-validation.md`: the exact form the coverage test pins
/// at n = 1000). `matrix[op][type]` counts comparisons per (operator,
/// structural type).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    pub guard: u64,
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
    /// The chase variants (`shapes_chase.rs`): eliminable shapes
    /// (existence walks and both DU `==` directions) vs the near-miss
    /// refusals — the coverage contract asserts both appear per run,
    /// and the engine-backed test holds each tag to its verdict.
    pub chase_eliminable: u64,
    pub chase_extra_field: u64,
    pub chase_missing_phi: u64,
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
    pub count_distinct_types: [u64; 7],
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
    /// Membership bindings by point-term kind and by element type.
    pub membership_literal: u64,
    pub membership_param: u64,
    pub membership_var: u64,
    pub membership_u64: u64,
    pub membership_i64: u64,
    /// Interval comparisons by element type: `Allen` masks per lane,
    /// composite (≥2 basics) vs singleton mask draws, and the point form
    /// of `Contains` per lane.
    pub allen_u64: u64,
    pub allen_i64: u64,
    pub allen_composite: u64,
    pub allen_singleton: u64,
    pub contains_u64: u64,
    pub contains_i64: u64,
    /// Boundary-shape polarities (corpus-adjacent query literals).
    pub adjacent_left: u64,
    pub adjacent_right: u64,
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
    /// `Allen`/`Contains` occurrence with neither an equality join
    /// variable nor an equality selection, or a negated atom whose only
    /// bindings are memberships. Asserted **zero** — the Cartesian
    /// degenerate (`40-execution.md`) must be unemittable.
    pub spine_violations: u64,
    /// Comparison counts per `(CMP_OPS index, CMP_TYPES index)`.
    pub matrix: [[u64; 7]; 8],
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
