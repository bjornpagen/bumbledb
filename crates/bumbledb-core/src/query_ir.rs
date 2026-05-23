//! Language-neutral typed query IR.

use crate::schema::ValueType;

/// Literal value in query text or generated frontend IR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    /// Boolean literal.
    Bool(bool),
    /// Integer literal.
    Integer(i128),
    /// String literal.
    String(String),
}

/// Comparison operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// `=`.
    Eq,
    /// `!=`.
    NotEq,
    /// `<`.
    Lt,
    /// `<=`.
    Lte,
    /// `>`.
    Gt,
    /// `>=`.
    Gte,
}

/// Typed logical IR query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedQuery {
    /// Dense variables used by this query.
    pub variables: Vec<TypedVariable>,
    /// Dense inputs used by this query.
    pub inputs: Vec<TypedInput>,
    /// Projection terms.
    pub find: Vec<TypedFindTerm>,
    /// Typed clauses.
    pub clauses: Vec<TypedClause>,
}

/// Typed variable metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedVariable {
    /// Dense variable ID.
    pub id: usize,
    /// Source variable name.
    pub name: String,
    /// Inferred logical type.
    pub value_type: ValueType,
}

/// Typed input metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedInput {
    /// Dense input ID.
    pub id: usize,
    /// Source input name.
    pub name: String,
    /// Inferred logical type.
    pub value_type: ValueType,
}

/// Typed projection term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedFindTerm {
    /// Variable projection.
    Variable { variable: usize },
}

/// Typed clause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedClause {
    /// Typed relation atom.
    Relation(TypedRelationAtom),
    /// Typed comparison predicate.
    Comparison(TypedComparison),
}

/// Typed relation atom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedRelationAtom {
    /// Relation declaration ID.
    pub relation_id: usize,
    /// Relation name.
    pub relation: String,
    /// Typed field bindings.
    pub fields: Vec<TypedFieldBinding>,
}

/// Typed field binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedFieldBinding {
    /// Field declaration ID.
    pub field_id: usize,
    /// Field name.
    pub field: String,
    /// Expected field type.
    pub value_type: ValueType,
    /// Bound term.
    pub term: TypedTerm,
}

/// Typed term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedTerm {
    /// Variable ID.
    Variable(usize),
    /// Input ID.
    Input(usize),
    /// Wildcard.
    Wildcard,
    /// Typed literal.
    Literal(TypedLiteral),
}

/// Typed comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedComparison {
    /// Left operand.
    pub left: TypedOperand,
    /// Operator.
    pub operator: ComparisonOperator,
    /// Right operand.
    pub right: TypedOperand,
    /// Comparison type.
    pub value_type: ValueType,
}

/// Typed comparison operand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedOperand {
    /// Variable ID.
    Variable(usize),
    /// Input ID.
    Input(usize),
    /// Typed literal.
    Literal(TypedLiteral),
}

/// Typed literal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedLiteral {
    /// Literal value.
    pub literal: Literal,
    /// Resolved logical type.
    pub value_type: ValueType,
}
