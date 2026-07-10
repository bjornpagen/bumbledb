//! Canonical schema serialization and the blake3 fingerprint (docs/architecture/10-data-model.md).
//!
//! The fingerprint inputs are enumerated exhaustively in
//! `docs/architecture/10-data-model.md`; that list is the contract
//! ([`canonical_bytes`] reproduces it). Every string and list is
//! length-prefixed (u32 LE) so no two schemas can alias to one byte stream;
//! relation, field, and statement ids are pinned by declaration/materialized
//! order and therefore covered without being hashed separately.
//!
//! [`Resolved`](super::Resolved) data (target keys, key permutations,
//! interval positions) and the sealed `==` pairing
//! ([`Statement::mirror`](super::Statement::mirror)) are **not** hashed:
//! the acceptance gate computes both as deterministic functions of the
//! hashed inputs, the same way materialized order leaves "statement ids …
//! pinned by the fingerprint without being hashed separately"
//! (`docs/architecture/10-data-model.md`).

use super::{
    FieldId, Generation, IntervalElement, RelationId, Schema, Side, StatementDescriptor, ValueType,
};
use crate::encoding::encode_literal;
use crate::value::Value;

/// Bumped whenever the canonical serialization format itself changes. `v1`:
/// the statement redesign — a different format even for schemas that would
/// serialize identically under `v0` (none do).
const FORMAT_VERSION_LABEL: &[u8] = b"bumbledb-schema-v1";

/// Deterministic schema identity: blake3 of the canonical bytes. Stored at
/// database creation; open compares fingerprints and mismatches are hard
/// failures (docs/architecture/50-storage.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaFingerprint(pub [u8; 32]);

/// Appends the canonical serialization of the schema to `out` — one linear
/// pass over the fingerprint inputs, exhaustively
/// (`docs/architecture/10-data-model.md` § Schema):
///
/// - an encoding-format version label;
/// - relations in declaration order — for each: name and fields in
///   declaration order (name, structural type description — including the
///   full ordered variant list for enums and the element type for intervals
///   — and generation flag);
/// - the dependency statements in **materialized order** — for each: the
///   judgment form (Functionality = 0, Containment = 1) and its sides as
///   (relation id, projection field-id list in statement order, selection
///   list as (field id, literal value) pairs in statement order).
fn canonical_bytes(schema: &Schema, out: &mut Vec<u8>) {
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
    }
    put_len(out, schema.statements().len());
    for statement in schema.statements() {
        match &statement.descriptor {
            StatementDescriptor::Functionality {
                relation,
                projection,
            } => {
                out.push(0);
                put_relation_id(out, *relation);
                put_len(out, projection.len());
                for field in projection {
                    put_field_id(out, *field);
                }
            }
            StatementDescriptor::Containment { source, target } => {
                out.push(1);
                put_side(out, source);
                put_side(out, target);
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

fn put_relation_id(out: &mut Vec<u8>, id: RelationId) {
    out.extend_from_slice(&id.0.to_le_bytes());
}

fn put_field_id(out: &mut Vec<u8>, id: FieldId) {
    out.extend_from_slice(&id.0.to_le_bytes());
}

fn put_side(out: &mut Vec<u8>, side: &Side) {
    put_relation_id(out, side.relation);
    put_len(out, side.projection.len());
    for field in &side.projection {
        put_field_id(out, *field);
    }
    put_len(out, side.selection.len());
    for (field, literal) in &side.selection {
        put_field_id(out, *field);
        put_literal(out, literal);
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
        ValueType::Interval { element } => {
            out.push(6);
            out.push(match element {
                IntervalElement::U64 => 0,
                IntervalElement::I64 => 1,
            });
        }
    }
}

/// A selection literal in the canonical per-type value encoding
/// ([`encode_literal`], the one definition site shared with the commit
/// judgment) — never a `Debug` or ad-hoc format. No variant tag: the
/// selected field's type is already in the stream (relations serialize
/// before statements), so the literal's shape is a function of bytes already
/// hashed and no two schemas can alias here. String/Bytes literals hash
/// their raw bytes, length-prefixed — the fact encoding's intern id is
/// per-database state, not schema identity.
fn put_literal(out: &mut Vec<u8>, literal: &Value) {
    match literal {
        Value::String(bytes) | Value::Bytes(bytes) => put_bytes(out, bytes),
        encoded => encode_literal(encoded, out),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{RelationDescriptor, SchemaDescriptor, StatementId};
    use super::*;
    use crate::schema::tests::{containment, enum_type, fd, field, serial_field, side, side_where};

    fn schema_of(descriptor: SchemaDescriptor) -> Schema {
        descriptor.validate().expect("valid fixture")
    }

    /// The mutation fixture: two relations, an enum, two serials (each
    /// materializing an auto-Functionality), a declared key, and a
    /// containment with a selection.
    fn base() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "Holder".into(),
                    fields: vec![serial_field("id"), field("name", ValueType::String)],
                },
                RelationDescriptor {
                    name: "Account".into(),
                    fields: vec![
                        serial_field("id"),
                        field("holder", ValueType::U64),
                        field("status", enum_type(&["Active", "Closed"])),
                    ],
                },
            ],
            statements: vec![
                fd(RelationId(1), &[FieldId(1)]),
                containment(
                    side_where(
                        RelationId(1),
                        &[FieldId(1)],
                        vec![(FieldId(2), Value::Enum(0))],
                    ),
                    side(RelationId(0), &[FieldId(0)]),
                ),
            ],
        }
    }

    fn base_fingerprint() -> SchemaFingerprint {
        fingerprint(&schema_of(base()))
    }

    fn hex_of(fp: &SchemaFingerprint) -> String {
        fp.0.iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                use std::fmt::Write;
                write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
                hex
            })
    }

    #[test]
    fn golden_fingerprint_pins_the_hash() {
        // Pinned: the canonical serialization (and therefore blake3 of it)
        // must not drift while the format label stays `v1`. `base()` covers
        // every literal-adjacent input: enums, serial auto-keys, a declared
        // key, and a containment with a selection literal.
        assert_eq!(
            hex_of(&base_fingerprint()),
            "b7e792d16e7b1582fcaca3d3f591fc210bff4d5bbc6a922b46fb24c5eee4c25f"
        );
    }

    #[test]
    fn mirror_links_never_reach_the_fingerprint() {
        // Identity golden: the sealed `==` pairing (`Statement::mirror`)
        // is derived from hashed inputs exactly like `Resolved`, so a
        // schema carrying a mirrored pair hashes only its descriptors —
        // pinned so the field can never leak into `canonical_bytes`.
        let mut decl = base();
        decl.statements.push(containment(
            side(RelationId(0), &[FieldId(0)]),
            side_where(
                RelationId(1),
                &[FieldId(1)],
                vec![(FieldId(2), Value::Enum(0))],
            ),
        ));
        let schema = schema_of(decl);
        // The fixture genuinely seals a pair (materialized ids 3 and 4:
        // two serial auto-keys and the declared FD precede them).
        assert_eq!(
            schema.statement(StatementId(3)).mirror,
            Some(StatementId(4))
        );
        assert_eq!(
            hex_of(&fingerprint(&schema)),
            "e262a1a960e148f5d1371a9763481fb9b65c24f677bc2347950ad1bd29b8a073"
        );
    }

    #[test]
    fn identical_declarations_yield_identical_fingerprints() {
        // Stability: two independently constructed identical descriptors —
        // relations *and* statements — produce byte-identical fingerprints.
        assert_eq!(base_fingerprint(), fingerprint(&schema_of(base())));
    }

    #[test]
    fn reordering_two_fields_changes_the_fingerprint() {
        // Standalone: base()'s statements pin fields by id, so swapping
        // fields there would change which fields the statements name, not
        // just declaration order.
        let of_fields = |names: [&str; 2]| {
            fingerprint(&schema_of(SchemaDescriptor {
                relations: vec![RelationDescriptor {
                    name: "R".into(),
                    fields: names
                        .iter()
                        .map(|name| field(name, ValueType::U64))
                        .collect(),
                }],
                statements: vec![],
            }))
        };
        assert_ne!(of_fields(["a", "b"]), of_fields(["b", "a"]));
    }

    #[test]
    fn renaming_a_field_changes_the_fingerprint() {
        let mut decl = base();
        decl.relations[0].fields[1].name = "full_name".into();
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn adding_an_enum_variant_changes_the_fingerprint() {
        let mut decl = base();
        decl.relations[1].fields[2].value_type = enum_type(&["Active", "Closed", "Frozen"]);
        // Closed domains are closed: adding a variant is a full ETL rebuild.
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn reordering_enum_variants_changes_the_fingerprint() {
        let mut decl = base();
        decl.relations[1].fields[2].value_type = enum_type(&["Closed", "Active"]);
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn toggling_serial_generation_changes_the_fingerprint() {
        let mut decl = base();
        // `Account.id`, not `Holder.id`: the containment's target key is
        // Holder's serial auto-Functionality, which must stay materialized.
        decl.relations[1].fields[0].generation = Generation::None;
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn reordering_two_statements_changes_the_fingerprint() {
        let mut decl = base();
        // Declaration order is materialized order for declared statements;
        // both orders validate (target-key resolution searches the whole
        // list), so only the order differs.
        decl.statements.swap(0, 1);
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn swapping_containment_sides_changes_the_fingerprint() {
        let mut decl = base();
        // `Holder(id) <= Account(holder | status = 0)`: still valid — the
        // new target projection {holder} resolves to the declared key.
        decl.statements[1] = containment(
            side(RelationId(0), &[FieldId(0)]),
            side_where(
                RelationId(1),
                &[FieldId(1)],
                vec![(FieldId(2), Value::Enum(0))],
            ),
        );
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn changing_a_selection_literal_changes_the_fingerprint() {
        let mut decl = base();
        decl.statements[1] = containment(
            side_where(
                RelationId(1),
                &[FieldId(1)],
                vec![(FieldId(2), Value::Enum(1))],
            ),
            side(RelationId(0), &[FieldId(0)]),
        );
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn reordering_a_projection_changes_the_fingerprint() {
        // X is ordered (the order defines the guard key): the same field
        // *set* in the other written order is a different schema.
        let of_projection = |fields: [u16; 2]| {
            fingerprint(&schema_of(SchemaDescriptor {
                relations: vec![RelationDescriptor {
                    name: "R".into(),
                    fields: vec![field("a", ValueType::U64), field("b", ValueType::U64)],
                }],
                statements: vec![fd(RelationId(0), &fields.map(FieldId))],
            }))
        };
        assert_ne!(of_projection([0, 1]), of_projection([1, 0]));
    }

    #[test]
    fn changing_an_interval_element_changes_the_fingerprint() {
        let of_element = |element| {
            fingerprint(&schema_of(SchemaDescriptor {
                relations: vec![RelationDescriptor {
                    name: "R".into(),
                    fields: vec![field("during", ValueType::Interval { element })],
                }],
                statements: vec![],
            }))
        };
        assert_ne!(
            of_element(IntervalElement::U64),
            of_element(IntervalElement::I64)
        );
    }

    #[test]
    fn golden_bytes_pin_the_canonical_serialization() {
        // One relation R { x: u64 serial }, whose serial materializes one
        // auto-Functionality. This golden is the anti-drift anchor: if it
        // breaks, the format version label must be bumped and every stored
        // fingerprint invalidated (full ETL).
        let schema = schema_of(SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "R".into(),
                fields: vec![serial_field("x")],
            }],
            statements: vec![],
        });
        let mut bytes = Vec::new();
        canonical_bytes(&schema, &mut bytes);

        let mut expected: Vec<u8> = Vec::new();
        expected.extend_from_slice(&18u32.to_le_bytes());
        expected.extend_from_slice(b"bumbledb-schema-v1");
        expected.extend_from_slice(&1u32.to_le_bytes()); // relation count
        expected.extend_from_slice(&1u32.to_le_bytes()); // name len
        expected.extend_from_slice(b"R");
        expected.extend_from_slice(&1u32.to_le_bytes()); // field count
        expected.extend_from_slice(&1u32.to_le_bytes()); // field name len
        expected.extend_from_slice(b"x");
        expected.push(2); // ValueType::U64 tag
        expected.push(1); // Generation::Serial tag
        expected.extend_from_slice(&1u32.to_le_bytes()); // statement count
        expected.push(0); // Functionality form tag
        expected.extend_from_slice(&0u32.to_le_bytes()); // relation id
        expected.extend_from_slice(&1u32.to_le_bytes()); // projection len
        expected.extend_from_slice(&0u16.to_le_bytes()); // field id
        assert_eq!(bytes, expected);
    }

    #[test]
    fn golden_bytes_pin_the_statement_serialization() {
        // `Account(holder | status = Closed) <= Holder(id)` over a declared
        // key: pins the Containment form — side layout, selection pairs,
        // and the canonical enum-ordinal literal encoding.
        let schema = schema_of(SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "Holder".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    name: "Account".into(),
                    fields: vec![
                        field("holder", ValueType::U64),
                        field("status", enum_type(&["Active", "Closed"])),
                    ],
                },
            ],
            statements: vec![
                fd(RelationId(0), &[FieldId(0)]),
                containment(
                    side_where(
                        RelationId(1),
                        &[FieldId(0)],
                        vec![(FieldId(1), Value::Enum(1))],
                    ),
                    side(RelationId(0), &[FieldId(0)]),
                ),
            ],
        });
        let mut bytes = Vec::new();
        canonical_bytes(&schema, &mut bytes);

        let mut expected: Vec<u8> = Vec::new();
        expected.extend_from_slice(&18u32.to_le_bytes());
        expected.extend_from_slice(b"bumbledb-schema-v1");
        expected.extend_from_slice(&2u32.to_le_bytes()); // relation count
        expected.extend_from_slice(&6u32.to_le_bytes());
        expected.extend_from_slice(b"Holder");
        expected.extend_from_slice(&1u32.to_le_bytes()); // field count
        expected.extend_from_slice(&2u32.to_le_bytes());
        expected.extend_from_slice(b"id");
        expected.push(2); // ValueType::U64 tag
        expected.push(0); // Generation::None tag
        expected.extend_from_slice(&7u32.to_le_bytes());
        expected.extend_from_slice(b"Account");
        expected.extend_from_slice(&2u32.to_le_bytes()); // field count
        expected.extend_from_slice(&6u32.to_le_bytes());
        expected.extend_from_slice(b"holder");
        expected.push(2); // ValueType::U64 tag
        expected.push(0); // Generation::None tag
        expected.extend_from_slice(&6u32.to_le_bytes());
        expected.extend_from_slice(b"status");
        expected.push(1); // ValueType::Enum tag
        expected.extend_from_slice(&2u32.to_le_bytes()); // variant count
        expected.extend_from_slice(&6u32.to_le_bytes());
        expected.extend_from_slice(b"Active");
        expected.extend_from_slice(&6u32.to_le_bytes());
        expected.extend_from_slice(b"Closed");
        expected.push(0); // Generation::None tag
        expected.extend_from_slice(&2u32.to_le_bytes()); // statement count
        expected.push(0); // Functionality form tag
        expected.extend_from_slice(&0u32.to_le_bytes()); // relation id
        expected.extend_from_slice(&1u32.to_le_bytes()); // projection len
        expected.extend_from_slice(&0u16.to_le_bytes()); // field id
        expected.push(1); // Containment form tag
        expected.extend_from_slice(&1u32.to_le_bytes()); // source relation id
        expected.extend_from_slice(&1u32.to_le_bytes()); // projection len
        expected.extend_from_slice(&0u16.to_le_bytes()); // field id
        expected.extend_from_slice(&1u32.to_le_bytes()); // selection len
        expected.extend_from_slice(&1u16.to_le_bytes()); // selected field id
        expected.push(1); // enum ordinal literal
        expected.extend_from_slice(&0u32.to_le_bytes()); // target relation id
        expected.extend_from_slice(&1u32.to_le_bytes()); // projection len
        expected.extend_from_slice(&0u16.to_le_bytes()); // field id
        expected.extend_from_slice(&0u32.to_le_bytes()); // selection len
        assert_eq!(bytes, expected);
    }

    #[test]
    fn length_prefixes_prevent_name_aliasing() {
        // Without length prefixes, ("AB" + "C") and ("A" + "BC") would
        // concatenate to identical streams.
        let of_names = |relation: &str, field_name: &str| {
            schema_of(SchemaDescriptor {
                relations: vec![RelationDescriptor {
                    name: relation.into(),
                    fields: vec![field(field_name, ValueType::U64)],
                }],
                statements: vec![],
            })
        };
        let one = of_names("AB", "C");
        let two = of_names("A", "BC");
        let (mut a, mut b) = (Vec::new(), Vec::new());
        canonical_bytes(&one, &mut a);
        canonical_bytes(&two, &mut b);
        assert_ne!(a, b);
        assert_ne!(fingerprint(&one), fingerprint(&two));
    }
}
