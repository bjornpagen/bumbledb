use super::*;

#[test]
fn valid_schema_constructs_with_statement_indices() {
    let schema = ledger_slice().validate().expect("valid schema");
    let holder = schema.relation(RelationId(0));
    // The serial fields auto-materialized ordinary, visible Functionality
    // statements; the declared Containment follows them.
    assert_eq!(holder.keys(), &[StatementId(0)]);
    assert_eq!(holder.outgoing(), &[]);
    // ...and Holder is the declared Containment's target (the delete-side
    // reverse-edge scan set).
    assert_eq!(holder.incoming(), &[StatementId(2)]);

    let account = schema.relation(RelationId(1));
    assert_eq!(account.keys(), &[StatementId(1)]);
    assert_eq!(account.outgoing(), &[StatementId(2)]);
    assert_eq!(account.incoming(), &[]);
    // Layout: id 8 + holder 8 + status 1, dense.
    assert_eq!(account.layout().fact_width(), 17);
}

/// The PRD 02 materialization-order pin: two relations with one serial
/// field each plus two declared statements — auto-FDs take ids 0 and 1
/// (relation declaration order, then field order), declared statements
/// take 2 and 3 (declaration order).
#[test]
fn statement_ids_are_auto_fds_first_then_declared_order() {
    let mut decl = ledger_slice();
    decl.statements.push(StatementDescriptor::Functionality {
        relation: RelationId(1),
        projection: Box::new([FieldId(1), FieldId(2)]),
    });
    let materialized = decl.materialized_statements();
    assert_eq!(
        materialized,
        vec![
            // id 0: Holder's serial auto-FD.
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
            },
            // id 1: Account's serial auto-FD.
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
            },
            // id 2: the declared Containment.
            StatementDescriptor::Containment {
                source: side(RelationId(1), &[FieldId(1)]),
                target: side(RelationId(0), &[FieldId(0)]),
            },
            // id 3: the declared Functionality.
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(1), FieldId(2)]),
            },
        ]
    );
    // The sealed schema holds the same list; StatementId = index into it.
    let schema = decl.validate().expect("valid schema");
    let sealed: Vec<&StatementDescriptor> =
        schema.statements().iter().map(|s| &s.descriptor).collect();
    assert_eq!(sealed, materialized.iter().collect::<Vec<_>>());
    assert_eq!(
        schema.relation(RelationId(1)).keys(),
        &[StatementId(1), StatementId(3)]
    );
}

#[test]
fn structural_enum_equality_is_the_identity() {
    // Same ordered variant list, different declaring contexts: equal type.
    assert_eq!(enum_type(&["A", "B"]), enum_type(&["A", "B"]));
    // Different order: different type (ordinal encoding differs).
    assert_ne!(enum_type(&["A", "B"]), enum_type(&["B", "A"]));
}

#[test]
fn nullary_relation_constructs() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Flag".into(),
            fields: vec![],
        }],
        statements: vec![],
    }
    .validate()
    .expect("nullary relations are legal");
    assert_eq!(schema.relation(RelationId(0)).layout().fact_width(), 0);
}

#[test]
fn accepts_enum_with_exactly_256_variants() {
    let names: Vec<String> = (0..256).map(|i| format!("V{i}")).collect();
    let decl = one_relation(vec![field(
        "e",
        enum_type(&names.iter().map(String::as_str).collect::<Vec<_>>()),
    )]);
    decl.validate().expect("256 variants fit one byte");
}
