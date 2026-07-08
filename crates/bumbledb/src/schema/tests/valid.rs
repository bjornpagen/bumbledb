use super::*;

#[test]
fn valid_schema_constructs_with_auto_uniques() {
    let schema = ledger_slice().validate().expect("valid schema");
    let holder = schema.relation(RelationId(0));
    // The serial field auto-materialized an ordinary, visible unique.
    assert_eq!(holder.constraints().len(), 1);
    assert_eq!(
        holder.constraint(ConstraintId(0)),
        &ConstraintDescriptor::Unique {
            name: "id".into(),
            fields: Box::new([FieldId(0)]),
        }
    );
    assert_eq!(holder.unique_constraints(), &[ConstraintId(0)]);
    // ...and it is FK-targeted by Account's FK (the Restrict scan set).
    assert_eq!(holder.fk_targeted(), &[ConstraintId(0)]);

    let account = schema.relation(RelationId(1));
    assert_eq!(account.constraints().len(), 2); // auto-unique + declared FK
    assert_eq!(account.fk_targeted(), &[]);
    // Layout: id 8 + holder 8 + status 1, dense.
    assert_eq!(account.layout().fact_width(), 17);
}

#[test]
fn structural_enum_equality_is_the_identity() {
    // Same ordered variant list, different declaring contexts: equal type.
    assert_eq!(enum_type(&["A", "B"]), enum_type(&["A", "B"]));
    // Different order: different type (ordinal encoding differs).
    assert_ne!(enum_type(&["A", "B"]), enum_type(&["B", "A"]));
}

#[test]
fn fk_may_target_structurally_equal_enum_key() {
    // FK compatibility is positional structural equality — an enum key
    // unifies iff the variant lists match exactly.
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "T".into(),
                fields: vec![field("kind", enum_type(&["X", "Y"]))],
                constraints: vec![ConstraintDescriptor::Unique {
                    name: "kind".into(),
                    fields: Box::new([FieldId(0)]),
                }],
            },
            RelationDescriptor {
                name: "S".into(),
                fields: vec![field("kind", enum_type(&["X", "Y"]))],
                constraints: vec![ConstraintDescriptor::ForeignKey {
                    name: "s_kind".into(),
                    fields: Box::new([FieldId(0)]),
                    target_relation: RelationId(0),
                    target_constraint: ConstraintId(0),
                }],
            },
        ],
    };
    schema.validate().expect("structural enums unify");
}

#[test]
fn nullary_relation_constructs() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Flag".into(),
            fields: vec![],
            constraints: vec![],
        }],
    }
    .validate()
    .expect("nullary relations are legal");
    assert_eq!(schema.relation(RelationId(0)).layout().fact_width(), 0);
}

#[test]
fn accepts_enum_with_exactly_256_variants() {
    let names: Vec<String> = (0..256).map(|i| format!("V{i}")).collect();
    let decl = one_relation(
        vec![field(
            "e",
            enum_type(&names.iter().map(String::as_str).collect::<Vec<_>>()),
        )],
        vec![],
    );
    decl.validate().expect("256 variants fit one byte");
}
