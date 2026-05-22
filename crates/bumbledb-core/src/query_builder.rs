//! Schema-aware builder for language-neutral typed query IR.

use std::collections::BTreeMap;

use crate::query_ir::{
    AggregateFunction, ComparisonOperator, Literal, TypedClause, TypedComparison,
    TypedFieldBinding, TypedFindTerm, TypedInput, TypedLiteral, TypedOperand, TypedQuery,
    TypedRelationAtom, TypedTerm, TypedVariable,
};
use crate::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

/// Query-builder result type.
pub type QueryBuildResult<T> = std::result::Result<T, QueryBuildError>;

/// Errors produced while constructing typed query IR directly.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum QueryBuildError {
    /// Unknown relation name.
    #[error("unknown relation {relation}")]
    UnknownRelation { relation: String },

    /// Unknown field name.
    #[error("unknown field {relation}.{field}")]
    UnknownField { relation: String, field: String },

    /// Variable type conflict.
    #[error("variable {variable} has incompatible types {existing} and {incoming}")]
    VariableTypeConflict {
        variable: String,
        existing: String,
        incoming: String,
    },

    /// Input parameter type conflict.
    #[error("input {input} has incompatible types {existing} and {incoming}")]
    InputTypeConflict {
        input: String,
        existing: String,
        incoming: String,
    },

    /// Literal does not fit expected type.
    #[error("literal is incompatible with expected type {expected}")]
    LiteralTypeMismatch { expected: String },

    /// Projection references an unbound variable.
    #[error("projection variable {variable} is unbound")]
    UnboundProjectionVariable { variable: String },

    /// Comparison has no typed operand to infer from.
    #[error("comparison operand is unbound")]
    UnboundComparisonOperand,

    /// Aggregate function cannot be applied to the value type.
    #[error("aggregate {function} cannot be applied to type {value_type}")]
    InvalidAggregateType {
        function: AggregateFunction,
        value_type: String,
    },

    /// Aggregate references an unbound variable.
    #[error("aggregate variable {variable} is unbound")]
    UnboundAggregateVariable { variable: String },

    /// Aggregate domain is empty.
    #[error("aggregate domain must contain at least one variable")]
    EmptyAggregateDomain,
}

/// Direct schema-aware builder for typed query IR.
#[derive(Debug)]
pub struct QueryBuilder<'schema> {
    schema: &'schema SchemaDescriptor,
    variables: Vec<TypedVariable>,
    variable_ids: BTreeMap<String, usize>,
    inputs: Vec<TypedInput>,
    input_ids: BTreeMap<String, usize>,
    find: Vec<TypedFindTerm>,
    clauses: Vec<TypedClause>,
}

impl<'schema> QueryBuilder<'schema> {
    /// Creates a new query builder for `schema`.
    pub fn new(schema: &'schema SchemaDescriptor) -> Self {
        Self {
            schema,
            variables: Vec::new(),
            variable_ids: BTreeMap::new(),
            inputs: Vec::new(),
            input_ids: BTreeMap::new(),
            find: Vec::new(),
            clauses: Vec::new(),
        }
    }

    /// Starts a relation atom using named fields.
    pub fn rel(&mut self, relation: &str) -> QueryBuildResult<RelationAtomBuilder<'_, 'schema>> {
        let (relation_id, descriptor) = self.find_relation(relation)?;
        Ok(RelationAtomBuilder {
            builder: self,
            relation_id,
            relation: descriptor,
            fields: Vec::new(),
        })
    }

    /// Starts a relation atom using named fields.
    pub fn relation(
        &mut self,
        relation: &str,
    ) -> QueryBuildResult<RelationAtomBuilder<'_, 'schema>> {
        self.rel(relation)
    }

    /// Adds a typed comparison predicate.
    pub fn cmp(
        &mut self,
        left: OperandRef,
        operator: ComparisonOperator,
        right: OperandRef,
    ) -> QueryBuildResult<&mut Self> {
        let left_type = self.operand_type(&left);
        let right_type = self.operand_type(&right);
        let value_type = match (left_type, right_type) {
            (Some(left), Some(right)) => {
                merge_types(&left, &right).ok_or_else(|| QueryBuildError::VariableTypeConflict {
                    variable: "comparison".to_owned(),
                    existing: type_name(&left),
                    incoming: type_name(&right),
                })?
            }
            (Some(value_type), None) | (None, Some(value_type)) => value_type,
            (None, None) => return Err(QueryBuildError::UnboundComparisonOperand),
        };

        if matches!(
            operator,
            ComparisonOperator::Lt
                | ComparisonOperator::Lte
                | ComparisonOperator::Gt
                | ComparisonOperator::Gte
        ) && !is_orderable(&value_type)
        {
            return Err(QueryBuildError::LiteralTypeMismatch {
                expected: format!("orderable type, got {}", type_name(&value_type)),
            });
        }

        let left = self.type_operand(left, &value_type)?;
        let right = self.type_operand(right, &value_type)?;
        self.clauses.push(TypedClause::Comparison(TypedComparison {
            left,
            operator,
            right,
            value_type,
        }));
        Ok(self)
    }

    /// Adds a variable projection term.
    pub fn find_var(&mut self, variable: &str) -> QueryBuildResult<&mut Self> {
        let Some(id) = self.variable_ids.get(variable).copied() else {
            return Err(QueryBuildError::UnboundProjectionVariable {
                variable: variable.to_owned(),
            });
        };
        self.find.push(TypedFindTerm::Variable { variable: id });
        Ok(self)
    }

    /// Adds a domain-count aggregate projection term.
    pub fn find_count_domain<I, S>(&mut self, domain: I) -> QueryBuildResult<&mut Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let domain = self.domain_variables(domain)?;
        let variable = *domain
            .first()
            .ok_or(QueryBuildError::EmptyAggregateDomain)?;
        self.find.push(TypedFindTerm::Aggregate {
            function: AggregateFunction::CountDomain,
            variable,
            domain,
            value_type: ValueType::U64,
        });
        Ok(self)
    }

    /// Adds a distinct-value count aggregate projection term.
    pub fn find_count_distinct(&mut self, variable: &str) -> QueryBuildResult<&mut Self> {
        let variable = self.aggregate_variable(variable)?;
        self.find.push(TypedFindTerm::Aggregate {
            function: AggregateFunction::CountDistinct,
            variable,
            domain: vec![variable],
            value_type: ValueType::U64,
        });
        Ok(self)
    }

    /// Adds a sum aggregate over an explicit set domain.
    pub fn find_sum_over<I, S>(&mut self, variable: &str, domain: I) -> QueryBuildResult<&mut Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.find_domain_aggregate(AggregateFunction::Sum, variable, domain)
    }

    /// Adds a minimum aggregate over an explicit set domain.
    pub fn find_min_over<I, S>(&mut self, variable: &str, domain: I) -> QueryBuildResult<&mut Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.find_domain_aggregate(AggregateFunction::Min, variable, domain)
    }

    /// Adds a maximum aggregate over an explicit set domain.
    pub fn find_max_over<I, S>(&mut self, variable: &str, domain: I) -> QueryBuildResult<&mut Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.find_domain_aggregate(AggregateFunction::Max, variable, domain)
    }

    /// Finishes construction and returns typed query IR.
    pub fn finish(&mut self) -> QueryBuildResult<TypedQuery> {
        Ok(TypedQuery {
            variables: std::mem::take(&mut self.variables),
            inputs: std::mem::take(&mut self.inputs),
            find: std::mem::take(&mut self.find),
            clauses: std::mem::take(&mut self.clauses),
        })
    }

    fn find_relation(&self, name: &str) -> QueryBuildResult<(usize, &'schema RelationDescriptor)> {
        self.schema
            .relations
            .iter()
            .enumerate()
            .find(|(_, relation)| relation.name == name)
            .ok_or_else(|| QueryBuildError::UnknownRelation {
                relation: name.to_owned(),
            })
    }

    fn bind_variable(&mut self, name: &str, incoming: ValueType) -> QueryBuildResult<usize> {
        if let Some(id) = self.variable_ids.get(name).copied() {
            let existing = self.variables[id].value_type.clone();
            let Some(merged) = merge_types(&existing, &incoming) else {
                return Err(QueryBuildError::VariableTypeConflict {
                    variable: name.to_owned(),
                    existing: type_name(&existing),
                    incoming: type_name(&incoming),
                });
            };
            self.variables[id].value_type = merged;
            Ok(id)
        } else {
            let id = self.variables.len();
            self.variable_ids.insert(name.to_owned(), id);
            self.variables.push(TypedVariable {
                id,
                name: name.to_owned(),
                value_type: incoming,
            });
            Ok(id)
        }
    }

    fn aggregate_variable(&self, variable: &str) -> QueryBuildResult<usize> {
        self.variable_ids.get(variable).copied().ok_or_else(|| {
            QueryBuildError::UnboundAggregateVariable {
                variable: variable.to_owned(),
            }
        })
    }

    fn domain_variables<I, S>(&self, domain: I) -> QueryBuildResult<Vec<usize>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut variables = Vec::new();
        for variable in domain {
            variables.push(self.aggregate_variable(variable.as_ref())?);
        }
        if variables.is_empty() {
            return Err(QueryBuildError::EmptyAggregateDomain);
        }
        Ok(variables)
    }

    fn find_domain_aggregate<I, S>(
        &mut self,
        function: AggregateFunction,
        variable: &str,
        domain: I,
    ) -> QueryBuildResult<&mut Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let variable = self.aggregate_variable(variable)?;
        let domain = self.domain_variables(domain)?;
        let value_type = self.variables[variable].value_type.clone();
        if !aggregate_supports(function, &value_type) {
            return Err(QueryBuildError::InvalidAggregateType {
                function,
                value_type: type_name(&value_type),
            });
        }
        self.find.push(TypedFindTerm::Aggregate {
            function,
            variable,
            domain,
            value_type,
        });
        Ok(self)
    }

    fn bind_input(&mut self, name: &str, incoming: ValueType) -> QueryBuildResult<usize> {
        if let Some(id) = self.input_ids.get(name).copied() {
            let existing = self.inputs[id].value_type.clone();
            let Some(merged) = merge_types(&existing, &incoming) else {
                return Err(QueryBuildError::InputTypeConflict {
                    input: name.to_owned(),
                    existing: type_name(&existing),
                    incoming: type_name(&incoming),
                });
            };
            self.inputs[id].value_type = merged;
            Ok(id)
        } else {
            let id = self.inputs.len();
            self.input_ids.insert(name.to_owned(), id);
            self.inputs.push(TypedInput {
                id,
                name: name.to_owned(),
                value_type: incoming,
            });
            Ok(id)
        }
    }

    fn type_literal(
        &self,
        literal: Literal,
        expected: &ValueType,
    ) -> QueryBuildResult<TypedLiteral> {
        if literal_fits_type(self.schema, &literal, expected) {
            Ok(TypedLiteral {
                literal,
                value_type: expected.clone(),
            })
        } else {
            Err(QueryBuildError::LiteralTypeMismatch {
                expected: type_name(expected),
            })
        }
    }

    fn operand_type(&self, operand: &OperandRef) -> Option<ValueType> {
        match operand {
            OperandRef::Variable(name) => self
                .variable_ids
                .get(name)
                .map(|id| self.variables[*id].value_type.clone()),
            OperandRef::Input(name) => self
                .input_ids
                .get(name)
                .map(|id| self.inputs[*id].value_type.clone()),
            OperandRef::Literal(_) => None,
        }
    }

    fn type_operand(
        &mut self,
        operand: OperandRef,
        expected: &ValueType,
    ) -> QueryBuildResult<TypedOperand> {
        match operand {
            OperandRef::Variable(name) => Ok(TypedOperand::Variable(
                self.bind_variable(&name, expected.clone())?,
            )),
            OperandRef::Input(name) => Ok(TypedOperand::Input(
                self.bind_input(&name, expected.clone())?,
            )),
            OperandRef::Literal(literal) => {
                Ok(TypedOperand::Literal(self.type_literal(literal, expected)?))
            }
        }
    }
}

/// Builder for one relation atom.
#[derive(Debug)]
pub struct RelationAtomBuilder<'builder, 'schema> {
    builder: &'builder mut QueryBuilder<'schema>,
    relation_id: usize,
    relation: &'schema RelationDescriptor,
    fields: Vec<TypedFieldBinding>,
}

impl<'builder, 'schema> RelationAtomBuilder<'builder, 'schema> {
    /// Binds `field` to a query variable.
    pub fn var(mut self, field: &str, variable: &str) -> QueryBuildResult<Self> {
        self.bind(field, |builder, value_type| {
            Ok(TypedTerm::Variable(
                builder.bind_variable(variable, value_type.clone())?,
            ))
        })?;
        Ok(self)
    }

    /// Binds `field` to an input parameter.
    pub fn input(mut self, field: &str, input: &str) -> QueryBuildResult<Self> {
        self.bind(field, |builder, value_type| {
            Ok(TypedTerm::Input(
                builder.bind_input(input, value_type.clone())?,
            ))
        })?;
        Ok(self)
    }

    /// Binds `field` to a wildcard.
    pub fn wildcard(mut self, field: &str) -> QueryBuildResult<Self> {
        self.bind(field, |_builder, _value_type| Ok(TypedTerm::Wildcard))?;
        Ok(self)
    }

    /// Binds `field` to a literal.
    pub fn literal(mut self, field: &str, literal: Literal) -> QueryBuildResult<Self> {
        self.bind(field, |builder, value_type| {
            Ok(TypedTerm::Literal(
                builder.type_literal(literal.clone(), value_type)?,
            ))
        })?;
        Ok(self)
    }

    /// Binds `field` to an integer literal.
    pub fn integer(self, field: &str, value: i128) -> QueryBuildResult<Self> {
        self.literal(field, Literal::Integer(value))
    }

    /// Binds `field` to a string literal.
    pub fn string(self, field: &str, value: impl Into<String>) -> QueryBuildResult<Self> {
        self.literal(field, Literal::String(value.into()))
    }

    /// Binds `field` to a bool literal.
    pub fn bool(self, field: &str, value: bool) -> QueryBuildResult<Self> {
        self.literal(field, Literal::Bool(value))
    }

    /// Finishes this relation atom and returns to the parent query builder.
    pub fn done(self) -> &'builder mut QueryBuilder<'schema> {
        self.builder
            .clauses
            .push(TypedClause::Relation(TypedRelationAtom {
                relation_id: self.relation_id,
                relation: self.relation.name.clone(),
                fields: self.fields,
            }));
        self.builder
    }

    fn bind(
        &mut self,
        field_name: &str,
        term: impl FnOnce(&mut QueryBuilder<'schema>, &ValueType) -> QueryBuildResult<TypedTerm>,
    ) -> QueryBuildResult<()> {
        let (field_id, field) = self.field(field_name)?;
        let value_type = field.value_type.clone();
        let term = term(self.builder, &value_type)?;
        self.fields.push(TypedFieldBinding {
            field_id,
            field: field.name.clone(),
            value_type,
            term,
        });
        Ok(())
    }

    fn field(&self, name: &str) -> QueryBuildResult<(usize, &'schema FieldDescriptor)> {
        self.relation
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == name)
            .ok_or_else(|| QueryBuildError::UnknownField {
                relation: self.relation.name.clone(),
                field: name.to_owned(),
            })
    }
}

/// Comparison operand reference for direct query construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperandRef {
    /// Query variable by name.
    Variable(String),
    /// Input parameter by name.
    Input(String),
    /// Untyped literal to be resolved from the other operand.
    Literal(Literal),
}

impl OperandRef {
    /// Creates a variable operand.
    pub fn var(name: impl Into<String>) -> Self {
        Self::Variable(name.into())
    }

    /// Creates an input operand.
    pub fn input(name: impl Into<String>) -> Self {
        Self::Input(name.into())
    }

    /// Creates a literal operand.
    pub fn literal(literal: Literal) -> Self {
        Self::Literal(literal)
    }

    /// Creates an integer literal operand.
    pub fn integer(value: i128) -> Self {
        Self::Literal(Literal::Integer(value))
    }

    /// Creates a string literal operand.
    pub fn string(value: impl Into<String>) -> Self {
        Self::Literal(Literal::String(value.into()))
    }

    /// Creates a bool literal operand.
    pub fn bool(value: bool) -> Self {
        Self::Literal(Literal::Bool(value))
    }
}

fn merge_types(existing: &ValueType, incoming: &ValueType) -> Option<ValueType> {
    if existing == incoming {
        return Some(existing.clone());
    }
    None
}

fn literal_fits_type(schema: &SchemaDescriptor, literal: &Literal, expected: &ValueType) -> bool {
    match (literal, expected) {
        (Literal::Bool(_), ValueType::Bool) => true,
        (Literal::String(_), ValueType::String) => true,
        (Literal::Integer(value), ValueType::Enum { name }) => {
            *value >= 0
                && *value <= u8::MAX as i128
                && schema.enum_contains_code(name, *value as u8)
        }
        (Literal::Integer(value), ValueType::U64) => *value >= 0 && *value <= u64::MAX as i128,
        (Literal::Integer(value), ValueType::Serial { .. }) => {
            *value >= 0 && *value <= u64::MAX as i128
        }
        (Literal::Integer(value), ValueType::I64 | ValueType::TimestampMicros) => {
            *value >= i64::MIN as i128 && *value <= i64::MAX as i128
        }
        (Literal::Integer(_), ValueType::Decimal { .. }) => true,
        _ => false,
    }
}

fn aggregate_supports(function: AggregateFunction, value_type: &ValueType) -> bool {
    match function {
        AggregateFunction::CountDomain | AggregateFunction::CountDistinct => true,
        AggregateFunction::Sum => matches!(
            value_type,
            ValueType::U64 | ValueType::I64 | ValueType::Decimal { .. }
        ),
        AggregateFunction::Min | AggregateFunction::Max => is_orderable(value_type),
    }
}

fn is_orderable(value_type: &ValueType) -> bool {
    matches!(
        value_type,
        ValueType::U64
            | ValueType::I64
            | ValueType::TimestampMicros
            | ValueType::Decimal { .. }
            | ValueType::Serial { .. }
    )
}

fn type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".to_owned(),
        ValueType::U64 => "u64".to_owned(),
        ValueType::I64 => "i64".to_owned(),
        ValueType::TimestampMicros => "timestamp".to_owned(),
        ValueType::Decimal { scale } => format!("decimal(scale={scale})"),
        ValueType::Enum { name } => name.clone(),
        ValueType::String => "string".to_owned(),
        ValueType::Bytes => "bytes".to_owned(),
        ValueType::Serial {
            type_name,
            owning_relation,
        } => format!("{type_name}@{owning_relation}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        ConstraintDescriptor, EnumDescriptor, FieldDescriptor, RelationDescriptor, SchemaDescriptor,
    };

    #[test]
    fn builds_single_relation_query() -> QueryBuildResult<()> {
        let schema = schema();
        let query = QueryBuilder::new(&schema)
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .var("currency", "currency")?
            .done()
            .find_var("account")?
            .find_var("currency")?
            .finish()?;

        assert_eq!(query.variables.len(), 2);
        assert_eq!(query.inputs.len(), 1);
        assert_eq!(query.find.len(), 2);
        assert_eq!(query.clauses.len(), 1);
        Ok(())
    }

    #[test]
    fn builds_multi_relation_join_query() -> QueryBuildResult<()> {
        let schema = schema();
        let query = QueryBuilder::new(&schema)
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done()
            .rel("Holder")?
            .var("id", "holder")?
            .var("name", "holder_name")?
            .done()
            .find_var("account")?
            .find_var("holder_name")?
            .finish()?;

        assert_eq!(query.variables.len(), 3);
        assert_eq!(query.variables[1].name, "holder");
        assert!(matches!(
            query.variables[1].value_type,
            ValueType::Serial { .. }
        ));
        Ok(())
    }

    #[test]
    fn builds_comparison_query() -> QueryBuildResult<()> {
        let schema = schema();
        let query = QueryBuilder::new(&schema)
            .rel("Posting")?
            .var("id", "posting")?
            .var("at", "t")?
            .done()
            .cmp(
                OperandRef::var("t"),
                ComparisonOperator::Gte,
                OperandRef::input("start"),
            )?
            .find_var("posting")?
            .finish()?;

        assert_eq!(query.inputs.len(), 1);
        assert_eq!(query.clauses.len(), 2);
        Ok(())
    }

    #[test]
    fn builds_aggregate_query() -> QueryBuildResult<()> {
        let schema = schema();
        let query = QueryBuilder::new(&schema)
            .rel("Posting")?
            .var("id", "posting")?
            .var("amount", "amount")?
            .done()
            .find_sum_over("amount", ["posting"])?
            .find_count_domain(["posting"])?
            .finish()?;

        assert_eq!(query.find.len(), 2);
        Ok(())
    }

    #[test]
    fn rejects_unknown_relation() {
        let schema = schema();
        let mut builder = QueryBuilder::new(&schema);
        let result = builder.rel("Missing");
        assert!(result.is_err(), "missing relation should fail");
        let Err(error) = result else { return };
        assert_eq!(
            error,
            QueryBuildError::UnknownRelation {
                relation: "Missing".to_owned()
            }
        );
    }

    #[test]
    fn rejects_unknown_field() {
        let schema = schema();
        let mut builder = QueryBuilder::new(&schema);
        let result = builder
            .rel("Account")
            .and_then(|atom| atom.var("missing", "x"));
        assert!(result.is_err(), "missing field should fail");
        let Err(error) = result else { return };
        assert_eq!(
            error,
            QueryBuildError::UnknownField {
                relation: "Account".to_owned(),
                field: "missing".to_owned()
            }
        );
    }

    #[test]
    fn rejects_variable_type_conflict() {
        let schema = schema();
        let mut builder = QueryBuilder::new(&schema);
        let result = builder
            .rel("Account")
            .and_then(|atom| atom.var("id", "x")?.var("currency", "x"));
        assert!(result.is_err(), "variable type conflict should fail");
        let Err(error) = result else { return };
        assert!(matches!(
            error,
            QueryBuildError::VariableTypeConflict { .. }
        ));
    }

    #[test]
    fn rejects_cross_serial_variable_unification() {
        let schema = schema();
        let mut builder = QueryBuilder::new(&schema);
        let result = builder
            .rel("Account")
            .and_then(|atom| atom.var("id", "x")?.done().rel("Holder"))
            .and_then(|atom| atom.var("id", "x"));
        assert!(matches!(
            result,
            Err(QueryBuildError::VariableTypeConflict { .. })
        ));
    }

    #[test]
    fn accepts_matching_serial_variable_unification() -> QueryBuildResult<()> {
        let schema = schema();
        QueryBuilder::new(&schema)
            .rel("Account")?
            .var("holder", "x")?
            .done()
            .rel("Holder")?
            .var("id", "x")?
            .done()
            .find_var("x")?
            .finish()?;
        Ok(())
    }

    #[test]
    fn rejects_input_type_conflict() {
        let schema = schema();
        let mut builder = QueryBuilder::new(&schema);
        let result = builder
            .rel("Account")
            .and_then(|atom| atom.input("id", "x")?.input("currency", "x"));
        assert!(result.is_err(), "input type conflict should fail");
        let Err(error) = result else { return };
        assert!(matches!(error, QueryBuildError::InputTypeConflict { .. }));
    }

    #[test]
    fn rejects_unbound_projection() {
        let schema = schema();
        let mut builder = QueryBuilder::new(&schema);
        let result = builder.find_var("missing");
        assert!(result.is_err(), "unbound projection should fail");
        let Err(error) = result else { return };
        assert_eq!(
            error,
            QueryBuildError::UnboundProjectionVariable {
                variable: "missing".to_owned()
            }
        );
    }

    #[test]
    fn rejects_invalid_aggregate_type() {
        let schema = schema();
        let mut builder = QueryBuilder::new(&schema);
        let result = builder
            .rel("Holder")
            .and_then(|atom| atom.var("name", "name"))
            .map(RelationAtomBuilder::done)
            .and_then(|builder| builder.find_sum_over("name", ["name"]));
        assert!(result.is_err(), "invalid aggregate type should fail");
        let Err(error) = result else { return };
        assert!(matches!(
            error,
            QueryBuildError::InvalidAggregateType { .. }
        ));
    }

    #[test]
    fn rejects_enum_literal_outside_byte_width() {
        let schema = schema();
        let mut builder = QueryBuilder::new(&schema);
        let result = builder
            .rel("Account")
            .and_then(|atom| atom.var("currency", "currency"))
            .map(RelationAtomBuilder::done)
            .and_then(|builder| {
                builder.cmp(
                    OperandRef::var("currency"),
                    ComparisonOperator::Eq,
                    OperandRef::integer(256),
                )
            });
        assert!(matches!(
            result,
            Err(QueryBuildError::LiteralTypeMismatch { .. })
        ));
    }

    fn schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "QueryBuilderDb",
            vec![
                RelationDescriptor::new(
                    "Holder",
                    vec![
                        FieldDescriptor::new("id", serial_type("HolderId", "Holder")),
                        FieldDescriptor::new("name", ValueType::String),
                    ],
                )
                .with_covering_unique("id", ["id"]),
                RelationDescriptor::new(
                    "Account",
                    vec![
                        FieldDescriptor::new("id", serial_type("AccountId", "Account")),
                        FieldDescriptor::new("holder", serial_type("HolderId", "Holder")),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_covering_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "holder",
                    ["holder"],
                    "Holder",
                    "id",
                )),
                RelationDescriptor::new(
                    "Posting",
                    vec![
                        FieldDescriptor::new("id", serial_type("PostingId", "Posting")),
                        FieldDescriptor::new("account", serial_type("AccountId", "Account")),
                        FieldDescriptor::new("amount", ValueType::Decimal { scale: 2 }),
                        FieldDescriptor::new("at", ValueType::TimestampMicros),
                    ],
                )
                .with_covering_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "account",
                    ["account"],
                    "Account",
                    "id",
                )),
            ],
        )
        .with_enum(EnumDescriptor::codes("Currency", [1, 2]))
    }

    fn serial_type(name: &str, relation: &str) -> ValueType {
        ValueType::Serial {
            type_name: name.to_owned(),
            owning_relation: relation.to_owned(),
        }
    }
}
