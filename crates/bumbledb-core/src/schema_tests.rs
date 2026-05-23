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
    changed_field_type.relations[1].fields[4].value_type = ValueType::I64;
    assert_ne!(schema.fingerprint(), changed_field_type.fingerprint());

    let mut changed_index = ledger_schema();
    changed_index.relations[1].fields[5].indexing.range = false;
    assert_ne!(schema.fingerprint(), changed_index.fingerprint());

    let mut changed_constraint = ledger_schema();
    changed_constraint.relations[0].constraints.clear();
    assert_ne!(schema.fingerprint(), changed_constraint.fingerprint());

    let mut changed_explicit_index = ledger_schema();
    changed_explicit_index.relations[0].indexes.clear();
    assert_ne!(schema.fingerprint(), changed_explicit_index.fingerprint());
}

#[test]
fn computes_access_layouts() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let layouts = ledger_schema().access_layouts(511)?;

    let account_fact_set = find_layout(&layouts, "Account", "fact_set")?;
    assert_eq!(account_fact_set.kind, IndexKind::FactSet);
    assert_eq!(
        account_fact_set.leading_fields,
        ["id", "holder", "currency"]
    );
    assert_eq!(field_names(account_fact_set), ["id", "holder", "currency"]);

    let posting_at = find_layout(&layouts, "Posting", "by_at")?;
    assert_eq!(posting_at.kind, IndexKind::Range);
    assert_eq!(posting_at.leading_fields, ["at"]);

    let holder_unique = find_layout(&layouts, "Holder", "unique_name")?;
    assert_eq!(holder_unique.kind, IndexKind::Unique);
    assert_eq!(holder_unique.leading_fields, ["name"]);
    assert_eq!(field_names(holder_unique), ["name"]);

    let account_holder_fk = find_layout(&layouts, "Account", "by_fk_holder")?;
    assert_eq!(account_holder_fk.kind, IndexKind::ForeignKey);
    assert_eq!(account_holder_fk.leading_fields, ["holder"]);
    assert_eq!(field_names(account_holder_fk), ["holder"]);

    let account_currency = find_layout(&layouts, "Account", "by_currency")?;
    assert_eq!(account_currency.kind, IndexKind::Equality);
    assert_eq!(account_currency.leading_fields, ["currency", "id"]);
    assert_eq!(field_names(account_currency), ["currency", "id"]);

    for layout in &layouts {
        assert_eq!(field_names(layout), layout.leading_fields);
        assert_eq!(
            layout.encoded_len,
            INDEX_KEY_OVERHEAD_BYTES
                + layout
                    .components
                    .iter()
                    .map(|component| component.encoded_width)
                    .sum::<usize>()
                + FACT_ID_BYTES
        );
    }

    assert!(
        layouts
            .iter()
            .all(|layout| !layout.needs_runtime_type_tags())
    );
    Ok(())
}

#[test]
fn fact_set_layout_is_first_even_when_declared_later()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema = SchemaDescriptor::new(
        "Ordering",
        vec![
            RelationDescriptor::new(
                "Account",
                vec![
                    FieldDescriptor::new("id", serial_type("AccountId", "Account")),
                    FieldDescriptor::new("code", ValueType::U64),
                ],
            )
            .with_constraint(ConstraintDescriptor::unique("code", ["code"]))
            .with_unique("id", ["id"]),
        ],
    );

    let layouts = schema.access_layouts(511)?;
    assert_eq!(layouts[0].index_name, "fact_set");
    assert_eq!(layouts[0].kind, IndexKind::FactSet);
    assert_eq!(layouts[1].index_name, "unique_code");
    Ok(())
}

#[test]
fn string_and_bytes_fields_use_interned_placeholders()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema = ledger_schema();
    let layouts = schema.access_layouts(511)?;
    let holder_unique = find_layout(&layouts, "Holder", "unique_name")?;
    let name = holder_unique
        .components
        .iter()
        .find(|component| component.field_name == "name")
        .ok_or_else(|| std::io::Error::other("missing Holder.name component"))?;
    assert!(name.value_type.is_interned_placeholder());
    assert_eq!(name.encoded_width, 8);

    let source_fact_set = find_layout(&layouts, "SourceDocument", "fact_set")?;
    let payload = source_fact_set
        .components
        .iter()
        .find(|component| component.field_name == "payload")
        .ok_or_else(|| std::io::Error::other("missing SourceDocument.payload component"))?;
    assert!(payload.value_type.is_interned_placeholder());
    assert_eq!(payload.encoded_width, 8);

    let account_fact_set = find_layout(&layouts, "Account", "fact_set")?;
    let currency = account_fact_set
        .components
        .iter()
        .find(|component| component.field_name == "currency")
        .ok_or_else(|| std::io::Error::other("missing Account.currency component"))?;
    assert_eq!(currency.encoded_width, 1);
    Ok(())
}

#[test]
fn rejects_oversized_index_layouts() {
    let schema = SchemaDescriptor::new(
        "TooWide",
        vec![
            RelationDescriptor::new(
                "Wide",
                (0..80)
                    .map(|index| {
                        FieldDescriptor::new(format!("f{index}"), ValueType::Decimal { scale: 0 })
                    })
                    .collect(),
            )
            .with_unique("id", ["f0"]),
        ],
    );

    assert!(matches!(
        schema.access_layouts(511),
        Err(SchemaError::KeyLayoutTooLarge { .. })
    ));
}

#[test]
fn rejects_duplicate_explicit_index_fields() {
    let schema = SchemaDescriptor::new(
        "DuplicateIndexFields",
        vec![
            RelationDescriptor::new(
                "Account",
                vec![
                    FieldDescriptor::new("id", serial_type("AccountId", "Account")),
                    FieldDescriptor::new(
                        "currency",
                        ValueType::Enum {
                            name: "Currency".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality(
                "bad_currency",
                ["currency", "currency"],
            )),
        ],
    );

    assert!(matches!(
        schema.access_layouts(511),
        Err(SchemaError::DuplicateIndexField { field, .. }) if field == "currency"
    ));
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
fn validation_rejects_duplicate_index_names() {
    let mut schema = valid_schema();
    schema.relations[0]
        .indexes
        .push(IndexDescriptor::equality("by_code_exact", ["code"]));
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::DuplicateIndex { relation, index })
            if relation == "Parent" && index == "by_code_exact"
    ));
}

#[test]
fn validation_rejects_reserved_generated_index_names() {
    let mut schema = valid_schema();
    schema.relations[0]
        .indexes
        .push(IndexDescriptor::equality("unique_code", ["code", "id"]));
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::ReservedIndexName { relation, index })
            if relation == "Parent" && index == "unique_code"
    ));
}

#[test]
fn validation_rejects_non_orderable_range_index() {
    let mut schema = valid_schema();
    schema.relations[0].fields[1] = FieldDescriptor::new("code", ValueType::String).range_indexed();
    assert!(matches!(
        schema.validate(),
        Err(SchemaError::InvalidIndex { relation, index, .. })
            if relation == "Parent" && index == "by_code"
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
