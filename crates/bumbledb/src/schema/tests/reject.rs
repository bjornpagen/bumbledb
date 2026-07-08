use super::*;
use crate::error::SchemaError;

// The statement roster (docs/architecture/30-dependencies.md § validation
// roster) is PRD 03's site; its reject corpus lands there. This module pins
// the field-level checks the placeholder validator keeps.

#[test]
fn rejects_duplicate_relation_name() {
    let decl = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "R".into(),
                fields: vec![],
            },
            RelationDescriptor {
                name: "R".into(),
                fields: vec![],
            },
        ],
        statements: vec![],
    };
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateRelationName { name: "R".into() }
    );
}

#[test]
fn rejects_duplicate_field_name() {
    let decl = one_relation(vec![field("x", ValueType::U64), field("x", ValueType::I64)]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateFieldName {
            relation: RelationId(0),
            name: "x".into()
        }
    );
}

#[test]
fn rejects_enum_without_variants() {
    let decl = one_relation(vec![field("e", enum_type(&[]))]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::EnumWithoutVariants {
            relation: RelationId(0),
            field: FieldId(0)
        }
    );
}

#[test]
fn rejects_enum_with_more_than_256_variants() {
    let names: Vec<String> = (0..257).map(|i| format!("V{i}")).collect();
    let decl = one_relation(vec![field(
        "e",
        enum_type(&names.iter().map(String::as_str).collect::<Vec<_>>()),
    )]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::EnumTooManyVariants {
            relation: RelationId(0),
            field: FieldId(0),
            count: 257
        }
    );
}

#[test]
fn rejects_duplicate_enum_variant() {
    let decl = one_relation(vec![field("e", enum_type(&["A", "A"]))]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::DuplicateEnumVariant {
            relation: RelationId(0),
            field: FieldId(0),
            variant: "A".into()
        }
    );
}

#[test]
fn rejects_serial_on_non_u64() {
    let decl = one_relation(vec![FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::I64,
        generation: Generation::Serial,
    }]);
    assert_eq!(
        decl.validate().unwrap_err(),
        SchemaError::SerialOnNonU64 {
            relation: RelationId(0),
            field: FieldId(0)
        }
    );
}

// `Schema` is unconstructible outside this module: its fields and
// `Relation`'s fields are private, and no public constructor exists —
// the only path in is `SchemaDescriptor::validate`. (Compile-time
// property; recorded here as the sealing contract.)
