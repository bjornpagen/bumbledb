use bumbledb_core::query_ir::{
    TypedComparison, TypedFindTerm, TypedInput, TypedLiteral, TypedVariable,
};
use bumbledb_core::schema::ValueType;

/// Stable relation-atom occurrence ID in normalized clause order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AtomOccurrenceId(pub(crate) usize);

/// Query normalized for formal Free Join planning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalizedQuery {
    /// Typed variables retained for projection, execution sinks, and future folds.
    pub(crate) variables: Vec<TypedVariable>,
    /// Typed runtime inputs.
    pub(crate) inputs: Vec<TypedInput>,
    /// Projection terms.
    pub(crate) find: Vec<TypedFindTerm>,
    /// Relation atom occurrences in stable clause order.
    pub(crate) atoms: Vec<AtomOccurrence>,
    /// Residual typed comparisons.
    pub(crate) comparisons: Vec<TypedComparison>,
}

/// One normalized occurrence of a relation atom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AtomOccurrence {
    /// Stable occurrence ID.
    pub(crate) id: AtomOccurrenceId,
    /// Base relation descriptor ID.
    pub(crate) relation_id: usize,
    /// Source relation name.
    pub(crate) relation: String,
    /// Full field view in schema field order.
    pub(crate) fields: Vec<NormalizedFieldBinding>,
    /// Variables in first-seen schema field order for this atom occurrence.
    pub(crate) variable_tuple: Vec<usize>,
    /// Atom-local equality filters visible to later planning.
    pub(crate) source_predicates: Vec<SourcePredicate>,
}

/// One field in a normalized full relation-field view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalizedFieldBinding {
    /// Field descriptor ID.
    pub(crate) field_id: usize,
    /// Field name.
    pub(crate) field: String,
    /// Field type.
    pub(crate) value_type: ValueType,
    /// Normalized term for this field.
    pub(crate) term: NormalizedTerm,
}

/// Normalized field term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NormalizedTerm {
    /// Query variable ID.
    Variable(usize),
    /// Runtime input ID.
    Input(usize),
    /// Typed literal.
    Literal(TypedLiteral),
    /// Explicit wildcard.
    Wildcard,
    /// Field omitted from the source atom.
    Omitted,
}

/// Atom-local predicate for planning and future pushdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourcePredicate {
    /// Field equals a runtime input.
    InputEq { field_id: usize, input: usize },
    /// Field equals a typed literal.
    LiteralEq {
        field_id: usize,
        literal: TypedLiteral,
    },
}
