use bumbledb::{AllenMask, Atom, CmpOp, Comparison, FieldId, FindTerm, RelationId, VarId};

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
pub mod target;
#[cfg(test)]
mod tests;
pub mod writes;

pub use construct::{random_cq_query, random_query};
pub use contradict::contradiction_query;
pub use coverage::{cmp_cell_legal, coverage};
pub use oracle::{ParamDraw, params_for};
pub use shapes_recursive::{
    RecursiveCoverage, RecursiveVariant, random_reach_query, recursive_coverage,
};

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
    (Shape::ExistenceWalk, 8),
    (Shape::DuWalk, 6),
    (Shape::Rules, 10),
    (Shape::Measure, 8),
    (Shape::ClosedJoin, 8),
    (Shape::GroundFold, 7),
    (Shape::Pack, 7),
    (Shape::ScalarFloat, 12),
];

const DRESS_PCT: u64 = 60;

const REPEAT_VAR_PCT: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    KeyProbe,
    Star,
    Chain,
    SelfJoin,
    Gated,
    Aggregate,

    Membership,

    IntervalJoin,

    Boundary,

    ExistenceWalk,

    DuWalk,

    Rules,

    ClosedJoin,

    GroundFold,

    Pack,

    Measure,

    ScalarFloat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClosedVariant {
    Join,

    JoinSelected,

    HandleLiteral,

    HandleSet,

    Fold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroundVariant {
    Walk,

    WalkExtraField,

    DuHeader,

    DuChild,

    DuMissingPhi,
}

fn leaf(tree: &bumbledb::ConditionTree) -> &Comparison {
    match tree {
        bumbledb::ConditionTree::Leaf(comparison) => comparison,
        bumbledb::ConditionTree::And(_) | bumbledb::ConditionTree::Or(_) => {
            unreachable!("the generator emits flat conjunctions only")
        }
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent booleans mirror the external configuration"
)]
#[derive(Default)]
struct Builder {
    atoms: Vec<Atom>,
    negated: Vec<Atom>,
    conditions: Vec<Comparison>,
    finds: Vec<FindTerm>,
    next_var: u16,
    next_param: u16,
    bound: Vec<VarId>,

    anchors: Vec<(VarId, RelationId, FieldId)>,

    miss: bool,

    bytes_hit: bool,
    bytes_miss: bool,

    adjacent_left: bool,
    adjacent_right: bool,

    ladder: [bool; 4],

    random_mask: bool,

    ground: Option<GroundVariant>,

    closed: Option<ClosedVariant>,
}

impl Builder {
    fn saw_rung(&mut self, rung: interval_data::Rung) {
        self.ladder[match rung {
            interval_data::Rung::Equal => 0,
            interval_data::Rung::Adjacent => 1,
            interval_data::Rung::Nested => 2,
            interval_data::Rung::Ray => 3,
        }] = true;
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent booleans mirror the external configuration"
)]
#[derive(Debug, Clone, Copy, Default)]
struct GenTags {
    miss: bool,
    bytes_hit: bool,
    bytes_miss: bool,
    adjacent_left: bool,
    adjacent_right: bool,
    ladder: [bool; 4],
    random_mask: bool,
    ground: Option<GroundVariant>,
    rules: Option<RulesVariant>,
    closed: Option<ClosedVariant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RulesVariant {
    Disjoint,

    Overlap,

    Aggregate,
}

pub const CMP_TYPES: [&str; 7] = ["u64", "i64", "bool", "string", "bytes", "interval", "f64"];

pub const CMP_OPS: [CmpOp; 8] = [
    CmpOp::Eq,
    CmpOp::Ne,
    CmpOp::Lt,
    CmpOp::Le,
    CmpOp::Gt,
    CmpOp::Ge,
    CmpOp::Allen {
        mask: AllenMask::INTERSECTS,
    },
    CmpOp::PointIn,
];

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    pub scalar_float: u64,
    pub key_probe: u64,
    pub star: u64,
    pub chain: u64,
    pub self_join: u64,
    pub gated: u64,
    pub aggregate: u64,
    pub membership: u64,
    pub interval_join: u64,
    pub boundary: u64,
    pub existence_walk: u64,
    pub du_walk: u64,
    pub rules: u64,
    pub closed_join: u64,
    pub ground_fold: u64,
    pub pack: u64,

    pub measure: u64,

    pub closed_join_plain: u64,
    pub closed_join_selected: u64,
    pub closed_handle_literal: u64,
    pub closed_handle_set: u64,

    pub ground_eliminable: u64,
    pub ground_extra_field: u64,
    pub ground_missing_phi: u64,
    pub du_header_falls: u64,
    pub du_child_falls: u64,
    pub gates: u64,
    pub misses: u64,
    pub params: u64,

    pub param_sets: u64,
    pub repeated_vars: u64,
    pub agg_sum: u64,
    pub agg_min: u64,
    pub agg_max: u64,
    pub agg_count: u64,

    pub agg_u64: u64,

    pub multi_aggregate: u64,

    pub membership_literal: u64,
    pub membership_param: u64,
    pub membership_var: u64,
    pub membership_u64: u64,
    pub membership_i64: u64,

    pub allen_u64: u64,
    pub allen_i64: u64,
    pub allen_composite: u64,
    pub allen_singleton: u64,
    pub allen_random_mask: u64,
    pub allen_basics: [u64; 13],
    pub point_in_u64: u64,
    pub point_in_i64: u64,

    pub adjacent_left: u64,
    pub adjacent_right: u64,

    pub ladder: [u64; 4],

    pub rules_arms: [u64; 3],
    pub rules_disjoint: u64,
    pub rules_overlap: u64,
    pub rules_aggregate: u64,

    /// the multiply-witnessed relations (rejection must not depend on
    pub negations: u64,
    pub negation_key_covered: u64,
    pub negation_open: u64,
    pub negation_literal: u64,
    pub negation_param: u64,
    pub negation_set: u64,
    pub negation_membership: u64,
    pub negation_gate: u64,
    pub negation_multi_witness: u64,

    pub neg_and_aggregate: u64,
    pub set_and_negation: u64,
    pub membership_and_allen: u64,
    pub mask_and_negation: u64,

    pub cross_residuals: u64,

    pub wide_scalar: u64,
    pub wide_interval: u64,

    pub bytes_hits: u64,
    pub bytes_misses: u64,

    pub spine_violations: u64,

    pub matrix: [[u64; 7]; 8],
}

const PARAM_DRAWS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DrawKind {
    Hit,
    Boundary,
    Miss,
}
