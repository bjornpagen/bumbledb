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
        .var("at_micros", "t")?
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
                    FieldDescriptor::generated_serial("id", "HolderId", "Holder"),
                    FieldDescriptor::new("name", ValueType::String),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Account",
                vec![
                    FieldDescriptor::generated_serial("id", "AccountId", "Account"),
                    FieldDescriptor::new("holder", serial_type("HolderId", "Holder")),
                    FieldDescriptor::new(
                        "currency",
                        ValueType::Enum {
                            name: "Currency".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "holder",
                ["holder"],
                "Holder",
                "id",
            )),
            RelationDescriptor::new(
                "Posting",
                vec![
                    FieldDescriptor::generated_serial("id", "PostingId", "Posting"),
                    FieldDescriptor::new("account", serial_type("AccountId", "Account")),
                    FieldDescriptor::new("amount", ValueType::I64),
                    FieldDescriptor::new("at_micros", ValueType::I64),
                ],
            )
            .with_unique("id", ["id"])
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
