//! Canonical schema serialization and the blake3 fingerprint (docs/architecture/10-data-model.md).
//!
//! The fingerprint inputs are enumerated exhaustively in
//! `docs/architecture/10-data-model.md`; that list is the contract. Every
//! string and list is length-prefixed (u32 LE) so no two schemas can alias
//! to one byte stream; ids are pinned by declaration order and therefore
//! covered without being hashed separately.

use super::{ConstraintDescriptor, Generation, Schema, ValueType};

/// Bumped whenever the canonical serialization format itself changes.
const FORMAT_VERSION_LABEL: &[u8] = b"bumbledb-schema-v0";

/// Deterministic schema identity: blake3 of the canonical bytes. Stored at
/// database creation; open compares fingerprints and mismatches are hard
/// failures (docs/architecture/40-storage.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaFingerprint(pub [u8; 32]);

/// Appends the canonical serialization of the schema to `out`.
pub fn canonical_bytes(schema: &Schema, out: &mut Vec<u8>) {
    put_bytes(out, FORMAT_VERSION_LABEL);
    put_len(out, schema.relations().len());
    for relation in schema.relations() {
        put_bytes(out, relation.name().as_bytes());
        put_len(out, relation.fields().len());
        for field in relation.fields() {
            put_bytes(out, field.name.as_bytes());
            put_value_type(out, &field.value_type);
            out.push(match field.generation {
                Generation::None => 0,
                Generation::Serial => 1,
            });
        }
        // Auto-materialized serial uniques are ordinary constraints in the
        // descriptor (docs/architecture/10-data-model.md), so they are serialized with no special case —
        // which is the point.
        put_len(out, relation.constraints().len());
        for constraint in relation.constraints() {
            match constraint {
                ConstraintDescriptor::Unique { name, fields } => {
                    out.push(0);
                    put_bytes(out, name.as_bytes());
                    put_field_ids(out, fields);
                }
                ConstraintDescriptor::ForeignKey {
                    name,
                    fields,
                    target_relation,
                    target_constraint,
                } => {
                    out.push(1);
                    put_bytes(out, name.as_bytes());
                    put_field_ids(out, fields);
                    // Targets serialize as names (10-data-model's input
                    // list); the ids are equivalent but the doc is the
                    // contract.
                    let target_rel = schema.relation(*target_relation);
                    put_bytes(out, target_rel.name().as_bytes());
                    put_bytes(
                        out,
                        target_rel.constraint(*target_constraint).name().as_bytes(),
                    );
                }
            }
        }
    }
}

/// Computes the schema fingerprint: blake3 of [`canonical_bytes`].
#[must_use]
pub fn fingerprint(schema: &Schema) -> SchemaFingerprint {
    let mut bytes = Vec::new();
    canonical_bytes(schema, &mut bytes);
    SchemaFingerprint(*blake3::hash(&bytes).as_bytes())
}

fn put_len(out: &mut Vec<u8>, len: usize) {
    let len = u32::try_from(len).expect("validated schema: list lengths fit u32");
    out.extend_from_slice(&len.to_le_bytes());
}

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    put_len(out, bytes.len());
    out.extend_from_slice(bytes);
}

fn put_field_ids(out: &mut Vec<u8>, fields: &[super::FieldId]) {
    put_len(out, fields.len());
    for field in fields {
        out.extend_from_slice(&field.0.to_le_bytes());
    }
}

fn put_value_type(out: &mut Vec<u8>, value_type: &ValueType) {
    match value_type {
        ValueType::Bool => out.push(0),
        ValueType::Enum { variants } => {
            out.push(1);
            put_len(out, variants.len());
            for variant in variants {
                put_bytes(out, variant.as_bytes());
            }
        }
        ValueType::U64 => out.push(2),
        ValueType::I64 => out.push(3),
        ValueType::String => out.push(4),
        ValueType::Bytes => out.push(5),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        ConstraintDescriptor, ConstraintId, FieldDescriptor, FieldId, Generation,
        RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
    };
    use super::*;

    fn schema_of(relations: Vec<RelationDescriptor>) -> Schema {
        SchemaDescriptor { relations }
            .validate()
            .expect("valid fixture")
    }

    fn field(name: &str, value_type: ValueType, generation: Generation) -> FieldDescriptor {
        FieldDescriptor {
            name: name.into(),
            value_type,
            generation,
        }
    }

    fn enum_type(variants: &[&str]) -> ValueType {
        ValueType::Enum {
            variants: variants.iter().map(|v| Box::from(*v)).collect(),
        }
    }

    /// The mutation fixture: two relations, an enum, a serial, an FK.
    fn base() -> Vec<RelationDescriptor> {
        vec![
            RelationDescriptor {
                name: "Holder".into(),
                fields: vec![
                    field("id", ValueType::U64, Generation::Serial),
                    field("name", ValueType::String, Generation::None),
                ],
                constraints: vec![],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    field("id", ValueType::U64, Generation::Serial),
                    field("holder", ValueType::U64, Generation::None),
                    field("status", enum_type(&["Active", "Closed"]), Generation::None),
                ],
                constraints: vec![
                    ConstraintDescriptor::Unique {
                        name: "holder_status".into(),
                        fields: Box::new([FieldId(1), FieldId(2)]),
                    },
                    ConstraintDescriptor::ForeignKey {
                        name: "account_holder".into(),
                        fields: Box::new([FieldId(1)]),
                        target_relation: RelationId(0),
                        target_constraint: ConstraintId(0),
                    },
                ],
            },
        ]
    }

    fn base_fingerprint() -> SchemaFingerprint {
        fingerprint(&schema_of(base()))
    }

    #[test]
    fn identical_declarations_yield_identical_fingerprints() {
        assert_eq!(base_fingerprint(), fingerprint(&schema_of(base())));
    }

    #[test]
    fn reordering_two_fields_changes_the_fingerprint() {
        let mut decl = base();
        // Whole descriptors swap, so the serial rides along with `id` and the
        // schema stays valid; only declaration order changes.
        decl[0].fields.swap(0, 1);
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn renaming_a_field_changes_the_fingerprint() {
        let mut decl = base();
        decl[0].fields[1].name = "full_name".into();
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn adding_an_enum_variant_changes_the_fingerprint() {
        let mut decl = base();
        decl[1].fields[2].value_type = enum_type(&["Active", "Closed", "Frozen"]);
        // Closed domains are closed: adding a variant is a full ETL rebuild.
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn reordering_enum_variants_changes_the_fingerprint() {
        let mut decl = base();
        decl[1].fields[2].value_type = enum_type(&["Closed", "Active"]);
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn changing_constraint_field_order_changes_the_fingerprint() {
        let mut decl = base();
        decl[1].constraints[0] = ConstraintDescriptor::Unique {
            name: "holder_status".into(),
            fields: Box::new([FieldId(2), FieldId(1)]),
        };
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn changing_an_fk_target_changes_the_fingerprint() {
        let mut decl = base();
        decl[1].constraints[1] = ConstraintDescriptor::ForeignKey {
            name: "account_holder".into(),
            fields: Box::new([FieldId(1)]),
            target_relation: RelationId(1),
            target_constraint: ConstraintId(0), // Account's own auto-unique id
        };
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn toggling_serial_generation_changes_the_fingerprint() {
        let mut decl = base();
        decl[0].fields[0].generation = Generation::None;
        // Dropping Serial also drops the auto-unique, which Account's FK
        // targets — retarget it to a declared unique to keep the schema valid.
        decl[0].constraints = vec![ConstraintDescriptor::Unique {
            name: "id".into(),
            fields: Box::new([FieldId(0)]),
        }];
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn golden_bytes_pin_the_canonical_serialization() {
        // One relation R { x: u64 serial } — the auto-unique on x is
        // serialized as an ordinary constraint. This golden is the
        // anti-drift anchor: if it breaks, the format version label must be
        // bumped and every stored fingerprint invalidated (full ETL).
        let schema = schema_of(vec![RelationDescriptor {
            name: "R".into(),
            fields: vec![field("x", ValueType::U64, Generation::Serial)],
            constraints: vec![],
        }]);
        let mut bytes = Vec::new();
        canonical_bytes(&schema, &mut bytes);

        let mut expected: Vec<u8> = Vec::new();
        expected.extend_from_slice(&18u32.to_le_bytes());
        expected.extend_from_slice(b"bumbledb-schema-v0");
        expected.extend_from_slice(&1u32.to_le_bytes()); // relation count
        expected.extend_from_slice(&1u32.to_le_bytes()); // name len
        expected.extend_from_slice(b"R");
        expected.extend_from_slice(&1u32.to_le_bytes()); // field count
        expected.extend_from_slice(&1u32.to_le_bytes()); // field name len
        expected.extend_from_slice(b"x");
        expected.push(2); // ValueType::U64 tag
        expected.push(1); // Generation::Serial tag
        expected.extend_from_slice(&1u32.to_le_bytes()); // constraint count
        expected.push(0); // Unique tag
        expected.extend_from_slice(&1u32.to_le_bytes()); // constraint name len
        expected.extend_from_slice(b"x");
        expected.extend_from_slice(&1u32.to_le_bytes()); // field id count
        expected.extend_from_slice(&0u16.to_le_bytes()); // FieldId(0)
        assert_eq!(bytes, expected);
    }

    #[test]
    fn length_prefixes_prevent_name_aliasing() {
        // Without length prefixes, ("AB" + "C") and ("A" + "BC") would
        // concatenate to identical streams.
        let one = schema_of(vec![RelationDescriptor {
            name: "AB".into(),
            fields: vec![field("C", ValueType::U64, Generation::None)],
            constraints: vec![],
        }]);
        let two = schema_of(vec![RelationDescriptor {
            name: "A".into(),
            fields: vec![field("BC", ValueType::U64, Generation::None)],
            constraints: vec![],
        }]);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        canonical_bytes(&one, &mut a);
        canonical_bytes(&two, &mut b);
        assert_ne!(a, b);
        assert_ne!(fingerprint(&one), fingerprint(&two));
    }
}
