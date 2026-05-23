use super::*;

#[path = "schema/test_fixtures.rs"]
mod test_fixtures;

use test_fixtures::*;

#[test]
fn typed_ids_are_logically_distinct() {
    let account = serial_type("AccountId", "Account");
    let instrument = serial_type("InstrumentId", "Instrument");

    assert_ne!(account, instrument);
    assert_eq!(account.encoded_width(), instrument.encoded_width());
}

#[test]
fn schema_fingerprint_is_deterministic_and_sensitive() {
    let schema = ledger_schema();
    assert_eq!(schema.fingerprint(), ledger_schema().fingerprint());

    let mut changed_relation = ledger_schema();
    changed_relation.relations[0].name = "Accounts".to_owned();
    assert_ne!(schema.fingerprint(), changed_relation.fingerprint());

    let mut changed_field_name = ledger_schema();
    changed_field_name.relations[0].fields[1].name = "owner".to_owned();
    assert_ne!(schema.fingerprint(), changed_field_name.fingerprint());

    let mut changed_field_type = ledger_schema();
    changed_field_type.relations[1].fields[4].value_type = ValueType::U64;
    assert_ne!(schema.fingerprint(), changed_field_type.fingerprint());

    let mut changed_constraint = ledger_schema();
    changed_constraint.relations[0].constraints.clear();
    assert_ne!(schema.fingerprint(), changed_constraint.fingerprint());
}

#[test]
fn string_and_bytes_fields_use_interned_placeholders() {
    assert!(ValueType::String.is_interned_placeholder());
    assert!(ValueType::Bytes.is_interned_placeholder());
    assert_eq!(ValueType::String.encoded_width(), 8);
    assert_eq!(ValueType::Bytes.encoded_width(), 8);
    assert_eq!(
        ValueType::Enum {
            name: "E".to_owned()
        }
        .encoded_width(),
        1
    );
}

#[test]
fn validates_well_formed_schema() {
    assert_eq!(valid_schema().validate(), Ok(()));
}

#[test]
fn validation_rejects_duplicate_relations() {
    let mut schema = valid_schema();
    schema.relations.push(schema.relations[0].clone());
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::DuplicateRelation { relation }) if relation == "Parent"
    ));
}

#[test]
fn validation_rejects_duplicate_fields() {
    let mut schema = valid_schema();
    let duplicate = schema.relations[0].fields[0].clone();
    schema.relations[0].fields.push(duplicate);
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::DuplicateField { relation, field }) if relation == "Parent" && field == "id"
    ));
}

#[test]
fn validation_allows_relation_without_named_unique() {
    let mut schema = valid_schema();
    schema.relations[0]
        .constraints
        .retain(|constraint| !matches!(constraint, ConstraintDescriptor::Unique { .. }));
    schema.relations[1].constraints.clear();
    assert!(schema.validate().is_ok());
}

#[test]
fn validation_accepts_multiple_named_unique_constraints() {
    let mut schema = valid_schema();
    schema.relations[0]
        .constraints
        .push(ConstraintDescriptor::unique("id_code", ["id", "code"]));
    assert!(schema.validate().is_ok());
}

#[test]
fn validation_rejects_unknown_foreign_key_target() {
    let mut schema = valid_schema();
    schema.relations[1]
        .constraints
        .push(ConstraintDescriptor::foreign_key(
            "missing_parent",
            ["parent"],
            "Missing",
            "id",
        ));
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::InvalidConstraint { relation, constraint, .. })
            if relation == "Child" && constraint == "missing_parent"
    ));
}

#[test]
fn validation_rejects_duplicate_constraint_names() {
    let mut schema = valid_schema();
    schema.relations[0]
        .constraints
        .push(ConstraintDescriptor::unique("code", ["code"]));
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::DuplicateConstraint { relation, constraint })
            if relation == "Parent" && constraint == "code"
    ));
}

#[test]
fn validation_rejects_empty_unique_fields() {
    let mut schema = valid_schema();
    schema.relations[0].constraints[1] = ConstraintDescriptor::unique("code", [] as [&str; 0]);
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::InvalidConstraint { relation, constraint, .. })
            if relation == "Parent" && constraint == "code"
    ));
}

#[test]
fn validation_rejects_duplicate_enum_names() {
    let schema = valid_schema()
        .with_enum(EnumDescriptor::codes("Status", [1]))
        .with_enum(EnumDescriptor::codes("Status", [2]));
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::DuplicateEnum { enum_name }) if enum_name == "Status"
    ));
}

#[test]
fn validation_rejects_duplicate_enum_variants_and_codes() {
    let duplicate_variant = valid_schema().with_enum(EnumDescriptor::new(
        "Status",
        [
            EnumVariantDescriptor::new("Open", 1),
            EnumVariantDescriptor::new("Open", 2),
        ],
    ));
    assert!(matches!(
        duplicate_variant.validate(),
        Err(SchemaError::DuplicateEnumVariant { enum_name, variant })
            if enum_name == "Status" && variant == "Open"
    ));

    let duplicate_code = valid_schema().with_enum(EnumDescriptor::new(
        "Status",
        [
            EnumVariantDescriptor::new("Open", 1),
            EnumVariantDescriptor::new("Closed", 1),
        ],
    ));
    assert!(matches!(
        duplicate_code.validate(),
        Err(SchemaError::DuplicateEnumCode { enum_name, code })
            if enum_name == "Status" && code == 1
    ));
}

#[test]
fn validation_rejects_unknown_enum_domains() {
    let mut schema = valid_schema();
    schema.relations[0].fields[1] = FieldDescriptor::new(
        "code",
        ValueType::Enum {
            name: "Missing".to_owned(),
        },
    );
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::UnknownEnum { relation, field, enum_name })
            if relation == "Parent" && field == "code" && enum_name == "Missing"
    ));
}

#[test]
fn validation_accepts_compound_foreign_key() {
    assert_eq!(compound_fk_schema().validate(), Ok(()));
}

#[test]
fn validation_accepts_single_enum_foreign_key() {
    assert_eq!(enum_fk_schema().validate(), Ok(()));
}

#[test]
fn validation_accepts_compound_enum_foreign_key() {
    assert_eq!(compound_enum_fk_schema().validate(), Ok(()));
}

#[test]
fn validation_accepts_compound_serial_enum_foreign_key() {
    assert_eq!(compound_serial_enum_fk_schema().validate(), Ok(()));
}

#[test]
fn validation_rejects_foreign_key_arity_mismatch() {
    let mut schema = compound_fk_schema();
    schema.relations[1].constraints[0] =
        ConstraintDescriptor::foreign_key("parent", ["parent_a"], "Parent", "by_ab");
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::InvalidConstraint { relation, constraint, .. })
            if relation == "Child" && constraint == "parent"
    ));
}

#[test]
fn validation_rejects_unknown_target_constraint() {
    let mut schema = compound_fk_schema();
    schema.relations[1].constraints[0] =
        ConstraintDescriptor::foreign_key("parent", ["parent_a", "parent_b"], "Parent", "missing");
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::UnknownTargetConstraint { relation, constraint, .. })
            if relation == "Child" && constraint == "parent"
    ));
}

#[test]
fn fingerprint_changes_when_unique_fields_change() {
    let schema = valid_schema();
    let mut changed = valid_schema();
    changed.relations[0].constraints[0] = ConstraintDescriptor::unique("id_code", ["id", "code"]);
    assert_ne!(schema.fingerprint(), changed.fingerprint());
}

#[test]
fn fingerprint_changes_when_fk_target_constraint_changes() {
    let schema = compound_fk_schema();
    let mut changed = compound_fk_schema();
    changed.relations[0]
        .constraints
        .push(ConstraintDescriptor::unique("by_ba", ["b", "a"]));
    changed.relations[1].constraints[0] =
        ConstraintDescriptor::foreign_key("parent", ["parent_b", "parent_a"], "Parent", "by_ba");
    assert_ne!(schema.fingerprint(), changed.fingerprint());
}

#[test]
fn validation_rejects_foreign_key_type_mismatch() {
    let mut schema = compound_fk_schema();
    schema.relations[1].fields[1] = FieldDescriptor::new("parent_a", ValueType::String);
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::ForeignKeyTypeMismatch { relation, constraint, .. })
            if relation == "Child" && constraint == "parent"
    ));
}

#[test]
fn validation_rejects_enum_foreign_key_domain_mismatch() {
    let mut schema = enum_fk_schema();
    schema.relations[1].fields[1] = FieldDescriptor::new(
        "currency",
        ValueType::Enum {
            name: "Country".to_owned(),
        },
    );
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::ForeignKeyTypeMismatch {
            relation,
            constraint,
            source_field,
            target_field,
            source_type,
            target_type,
        }) if relation == "Account"
            && constraint == "currency"
            && source_field == "currency"
            && target_field == "Currency.code"
            && source_type.contains("Country")
            && target_type.contains("Currency")
    ));
}

#[test]
fn validation_rejects_compound_foreign_key_field_order_mismatch() {
    let mut schema = compound_enum_fk_schema();
    schema.relations[1].constraints[0] = ConstraintDescriptor::foreign_key(
        "policy",
        ["currency", "country"],
        "Policy",
        "by_country_currency",
    );
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::ForeignKeyTypeMismatch { relation, constraint, .. })
            if relation == "Account" && constraint == "policy"
    ));
}
