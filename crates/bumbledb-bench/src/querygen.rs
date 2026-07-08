//! The randomized query generator (docs/architecture/50-validation.md): seeded random
//! valid queries over the ledger schema — the fuel for `verify`'s
//! randomized half.
//!
//! Construction is correct **by construction**: fresh dense `VarId`s,
//! dense `ParamId`s allocated at their use site, and literals typed from
//! the schema walk. The engine's `validate` is the assertion, not the
//! filter — a generated query failing validation is a generator bug.

use bumbledb::{Atom, CmpOp, Comparison, FieldId, FindTerm, RelationId, VarId};

use crate::schema::ids;

mod builder;
mod construct;
mod coverage;
mod dress;
mod dress_posting;
mod oracle;
mod shapes;
#[cfg(test)]
mod tests;

pub use construct::random_query;
pub use coverage::{cmp_cell_legal, coverage};
pub use oracle::params_for;

/// The shape grammar's weights (drawn by range over the sum — the PRD's
/// percentages, normative):
/// guard 10, star 20, chain 20, self-join 10, gated 10, aggregate 20.
const SHAPE_WEIGHTS: &[(Shape, u64)] = &[
    (Shape::Guard, 10),
    (Shape::Star, 20),
    (Shape::Chain, 20),
    (Shape::SelfJoin, 10),
    (Shape::Gated, 10),
    (Shape::Aggregate, 20),
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
}

/// Accumulating query state: atoms, predicates, finds, fresh id counters,
/// and the registry of variables the shapes bound (group-key candidates).
#[derive(Default)]
struct Builder {
    atoms: Vec<Atom>,
    predicates: Vec<Comparison>,
    finds: Vec<FindTerm>,
    next_var: u16,
    next_param: u16,
    bound: Vec<VarId>,
    /// Whether dressing emitted an out-of-vocabulary string or bytes
    /// literal.
    miss: bool,
    /// Whether dressing emitted an in-vocabulary bytes literal (a
    /// recomputed extref) / an out-of-vocabulary one.
    bytes_hit: bool,
    bytes_miss: bool,
}

/// Guardable relations: (relation, serial-id field, projectable fields).
const GUARDABLE: &[(RelationId, FieldId, &[FieldId])] = &[
    (ids::CURRENCY, ids::currency::ID, &[ids::currency::CODE]),
    (
        ids::HOLDER,
        ids::holder::ID,
        &[ids::holder::NAME, ids::holder::REGION],
    ),
    (
        ids::INSTRUMENT,
        ids::instrument::ID,
        &[
            ids::instrument::SYMBOL,
            ids::instrument::CURRENCY,
            ids::instrument::KIND,
        ],
    ),
    (
        ids::ACCOUNT,
        ids::account::ID,
        &[
            ids::account::HOLDER,
            ids::account::STATUS,
            ids::account::OPENED_AT,
        ],
    ),
    (
        ids::TRANSFER,
        ids::transfer::ID,
        &[ids::transfer::AT, ids::transfer::EXTREF],
    ),
    (
        ids::POSTING,
        ids::posting::ID,
        &[
            ids::posting::ACCOUNT,
            ids::posting::AMOUNT,
            ids::posting::AT,
            ids::posting::MEMO,
        ],
    ),
    (ids::TAG, ids::tag::ID, &[ids::tag::LABEL]),
];

/// Star satellites: (posting FK field, relation, projected payload field).
const SATELLITES: &[(FieldId, RelationId, FieldId)] = &[
    (ids::posting::ACCOUNT, ids::ACCOUNT, ids::account::STATUS),
    (
        ids::posting::INSTRUMENT,
        ids::INSTRUMENT,
        ids::instrument::KIND,
    ),
    (ids::posting::TRANSFER, ids::TRANSFER, ids::transfer::AT),
];

/// Generation facts the query alone cannot reveal (hit-vs-miss is a
/// corpus-content property).
#[derive(Debug, Clone, Copy, Default)]
struct GenTags {
    miss: bool,
    bytes_hit: bool,
    bytes_miss: bool,
}

/// The comparison-type axis of the coverage matrix.
pub const CMP_TYPES: [&str; 6] = ["u64", "i64", "enum", "bool", "string", "bytes"];
/// The operator axis, in `CmpOp` order.
pub const CMP_OPS: [CmpOp; 6] = [
    CmpOp::Eq,
    CmpOp::Ne,
    CmpOp::Lt,
    CmpOp::Le,
    CmpOp::Gt,
    CmpOp::Ge,
];

/// Construct counts over a generated batch — the coverage contract's
/// evidence. `matrix[op][type]` counts comparisons per (operator,
/// structural type): the asserted form of 50-validation's "every
/// comparison op on every legal type".
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    pub guard: u64,
    pub star: u64,
    pub chain: u64,
    pub self_join: u64,
    pub gated: u64,
    pub aggregate: u64,
    pub gates: u64,
    pub misses: u64,
    pub params: u64,
    pub repeated_vars: u64,
    pub agg_sum: u64,
    pub agg_min: u64,
    pub agg_max: u64,
    pub agg_count: u64,
    /// Aggregates whose input variable is u64-typed.
    pub agg_u64: u64,
    /// Aggregate-bearing find lists with more than one aggregate.
    pub multi_aggregate: u64,
    /// Var-vs-var comparisons whose variables bind in different atoms.
    pub cross_residuals: u64,
    /// In-vocabulary / out-of-vocabulary bytes literals.
    pub bytes_hits: u64,
    pub bytes_misses: u64,
    /// Comparison counts per `(CMP_OPS index, CMP_TYPES index)`.
    pub matrix: [[u64; 6]; 6],
}

/// Which set each of the four generated param vectors is.
const PARAM_SETS: usize = 4;

/// Which of the four sets is being filled.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SetKind {
    Hit,
    Boundary,
    Miss,
}
