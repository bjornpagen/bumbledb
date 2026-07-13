//! Canonical schema encoding and the blake3 fingerprint (docs/architecture/10-data-model.md).
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

/// Bumped whenever the canonical encoding format itself changes. `v1`:
/// the statement redesign. `v2`: closed relations — every relation gains a
/// closedness tag byte (so ordinary and closed relations can never alias
/// one byte stream), and a closed relation's ground axioms hash after its
/// fields.
const FORMAT_VERSION_LABEL: &[u8] = b"bumbledb-schema-v2";

/// Deterministic schema identity: blake3 of the canonical bytes. Stored at
/// database creation; open compares fingerprints and mismatches are hard
/// failures (docs/architecture/50-storage.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaFingerprint(pub [u8; 32]);

/// Appends the canonical encoding of the schema to `out` — one linear
/// pass over the fingerprint inputs, exhaustively
/// (`docs/architecture/10-data-model.md` § Schema):
///
/// - an encoding-format version label;
/// - relations in declaration order — for each: name and fields in
///   declaration order (name, structural type description — including the
///   width for fixed bytes and the element type for intervals — and
///   generation flag), then the closedness tag (ordinary = 0;
///   closed = 1 followed by the ground axioms in declaration order — for
///   each: handle bytes, then the row's canonical fact bytes, each
///   length-prefixed like everything else);
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
                Generation::Fresh => 1,
            });
        }
        // Closedness is theory identity: ground axioms hash in declaration
        // order — the sealed pre-encoded fact bytes, whose per-value shape
        // is a function of the field types already in the stream. The tag
        // keeps ordinary and closed relations from aliasing.
        match relation.extension() {
            None => out.push(0),
            Some(rows) => {
                out.push(1);
                put_len(out, rows.len());
                for row in rows {
                    put_bytes(out, row.handle.as_bytes());
                    put_bytes(out, &row.fact);
                }
            }
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
    // Tag 1 is the deleted enum type's tombstone; it is never reused —
    // a reissued tag would collide theories across the vocabulary cut.
    match value_type {
        ValueType::Bool => out.push(0),
        ValueType::U64 => out.push(2),
        ValueType::I64 => out.push(3),
        ValueType::String => out.push(4),
        // The length is hashed: a width change is a new theory
        // (`docs/architecture/10-data-model.md`).
        ValueType::FixedBytes { len } => {
            out.push(5);
            out.extend_from_slice(&len.to_le_bytes());
        }
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
/// selected field's type is already in the stream (relations encode
/// before statements), so the literal's shape is a function of bytes already
/// hashed and no two schemas can alias here. String literals hash
/// their raw bytes, length-prefixed — the fact encoding's intern id is
/// per-database state, not schema identity. `FixedBytes` literals are
/// self-encoding (their canonical bytes ARE the value, word-padded), so
/// they take the shared encoder like every other literal.
fn put_literal(out: &mut Vec<u8>, literal: &Value) {
    match literal {
        Value::String(bytes) => put_bytes(out, bytes),
        encoded => encode_literal(encoded, out),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{RelationDescriptor, SchemaDescriptor, StatementId};
    use super::*;
    use crate::schema::tests::{containment, fd, field, fresh_field, side, side_where};

    fn schema_of(descriptor: SchemaDescriptor) -> Schema {
        descriptor.validate().expect("valid fixture")
    }

    /// The mutation fixture: two relations, two fresh ids (each
    /// materializing an auto-Functionality), a declared key, and a
    /// containment with a selection.
    fn base() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Holder".into(),
                    fields: vec![fresh_field("id"), field("name", ValueType::String)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Account".into(),
                    fields: vec![
                        fresh_field("id"),
                        field("holder", ValueType::U64),
                        field("status", ValueType::U64),
                    ],
                },
            ],
            statements: vec![
                fd(RelationId(1), &[FieldId(1)]),
                containment(
                    side_where(
                        RelationId(1),
                        &[FieldId(1)],
                        vec![(FieldId(2), Value::U64(0))],
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
        // Pinned: the canonical encoding (and therefore blake3 of it)
        // must not drift while the format label stays `v2`. `base()` covers
        // every literal-adjacent input: fresh auto-keys, a declared key,
        // and a containment with a selection literal.
        assert_eq!(
            hex_of(&base_fingerprint()),
            "06681953fa8e913841b80ab76ab4be3093e8b4f5df242a2fbe83e8a884958079"
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
                vec![(FieldId(2), Value::U64(0))],
            ),
        ));
        let schema = schema_of(decl);
        // The fixture genuinely seals a pair (materialized ids 3 and 4:
        // two fresh auto-keys and the declared FD precede them).
        assert_eq!(
            schema.statement(StatementId(3)).mirror,
            Some(StatementId(4))
        );
        assert_eq!(
            hex_of(&fingerprint(&schema)),
            "d3e7542befb8a131f37a0272726faba24ebb0a31d077abc4a22b21e65aad3a73"
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
                    extension: None,
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
    fn changing_a_field_type_changes_the_fingerprint() {
        // `Holder.name`: no statement binds it, so the mutated
        // declaration stays valid and only the type description moves.
        let mut decl = base();
        decl.relations[0].fields[1].value_type = ValueType::I64;
        assert_ne!(base_fingerprint(), fingerprint(&schema_of(decl)));
    }

    #[test]
    fn toggling_fresh_generation_changes_the_fingerprint() {
        let mut decl = base();
        // `Account.id`, not `Holder.id`: the containment's target key is
        // Holder's fresh auto-Functionality, which must stay materialized.
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
                vec![(FieldId(2), Value::U64(0))],
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
                vec![(FieldId(2), Value::U64(1))],
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
                    extension: None,
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
                    extension: None,
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
    fn golden_bytes_pin_the_canonical_encoding() {
        // One relation R { x: u64 fresh }, whose fresh materializes one
        // auto-Functionality. This golden is the anti-drift anchor: if it
        // breaks, the format version label must be bumped and every stored
        // fingerprint invalidated (full ETL).
        let schema = schema_of(SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "R".into(),
                fields: vec![fresh_field("x")],
            }],
            statements: vec![],
        });
        let mut bytes = Vec::new();
        canonical_bytes(&schema, &mut bytes);

        let mut expected: Vec<u8> = Vec::new();
        expected.extend_from_slice(&18u32.to_le_bytes());
        expected.extend_from_slice(b"bumbledb-schema-v2");
        expected.extend_from_slice(&1u32.to_le_bytes()); // relation count
        expected.extend_from_slice(&1u32.to_le_bytes()); // name len
        expected.extend_from_slice(b"R");
        expected.extend_from_slice(&1u32.to_le_bytes()); // field count
        expected.extend_from_slice(&1u32.to_le_bytes()); // field name len
        expected.extend_from_slice(b"x");
        expected.push(2); // ValueType::U64 tag
        expected.push(1); // Generation::Fresh tag
        expected.push(0); // ordinary: no extension
        expected.extend_from_slice(&1u32.to_le_bytes()); // statement count
        expected.push(0); // Functionality form tag
        expected.extend_from_slice(&0u32.to_le_bytes()); // relation id
        expected.extend_from_slice(&1u32.to_le_bytes()); // projection len
        expected.extend_from_slice(&0u16.to_le_bytes()); // field id
        assert_eq!(bytes, expected);
    }

    #[test]
    fn golden_bytes_pin_the_statement_encoding() {
        // `Account(holder | status = 1) <= Holder(id)` over a declared
        // key: pins the Containment form — side layout, selection pairs,
        // and the canonical selection-literal encoding.
        let schema = schema_of(SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Holder".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Account".into(),
                    fields: vec![
                        field("holder", ValueType::U64),
                        field("status", ValueType::U64),
                    ],
                },
            ],
            statements: vec![
                fd(RelationId(0), &[FieldId(0)]),
                containment(
                    side_where(
                        RelationId(1),
                        &[FieldId(0)],
                        vec![(FieldId(1), Value::U64(1))],
                    ),
                    side(RelationId(0), &[FieldId(0)]),
                ),
            ],
        });
        let mut bytes = Vec::new();
        canonical_bytes(&schema, &mut bytes);

        let mut expected: Vec<u8> = Vec::new();
        expected.extend_from_slice(&18u32.to_le_bytes());
        expected.extend_from_slice(b"bumbledb-schema-v2");
        expected.extend_from_slice(&2u32.to_le_bytes()); // relation count
        expected.extend_from_slice(&6u32.to_le_bytes());
        expected.extend_from_slice(b"Holder");
        expected.extend_from_slice(&1u32.to_le_bytes()); // field count
        expected.extend_from_slice(&2u32.to_le_bytes());
        expected.extend_from_slice(b"id");
        expected.push(2); // ValueType::U64 tag
        expected.push(0); // Generation::None tag
        expected.push(0); // ordinary: no extension
        expected.extend_from_slice(&7u32.to_le_bytes());
        expected.extend_from_slice(b"Account");
        expected.extend_from_slice(&2u32.to_le_bytes()); // field count
        expected.extend_from_slice(&6u32.to_le_bytes());
        expected.extend_from_slice(b"holder");
        expected.push(2); // ValueType::U64 tag
        expected.push(0); // Generation::None tag
        expected.extend_from_slice(&6u32.to_le_bytes());
        expected.extend_from_slice(b"status");
        expected.push(2); // ValueType::U64 tag
        expected.push(0); // Generation::None tag
        expected.push(0); // ordinary: no extension
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
        expected.extend_from_slice(&1u64.to_be_bytes()); // u64 literal, canonical encoding
        expected.extend_from_slice(&0u32.to_le_bytes()); // target relation id
        expected.extend_from_slice(&1u32.to_le_bytes()); // projection len
        expected.extend_from_slice(&0u16.to_le_bytes()); // field id
        expected.extend_from_slice(&0u32.to_le_bytes()); // selection len
        assert_eq!(bytes, expected);
    }

    /// Currency { `minor_units`: u64 } = { Usd(2), Eur(2) } — the closed
    /// mutation fixture.
    fn closed_base() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![crate::schema::tests::closed(
                "Currency",
                vec![field("minor_units", ValueType::U64)],
                vec![
                    crate::schema::tests::row("Usd", vec![Value::U64(2)]),
                    crate::schema::tests::row("Eur", vec![Value::U64(2)]),
                ],
            )],
            statements: vec![],
        }
    }

    #[test]
    fn identical_closed_declarations_yield_identical_fingerprints() {
        // The invariance test, extended to ground axioms: the sealed
        // pre-encoded rows (like `Resolved` and `mirror`) are deterministic
        // functions of the hashed declaration, so two independently built
        // identical closed declarations hash identically.
        assert_eq!(
            fingerprint(&schema_of(closed_base())),
            fingerprint(&schema_of(closed_base()))
        );
    }

    #[test]
    fn reordering_extension_rows_changes_the_fingerprint() {
        // Row order is identity: handles are declaration-order ids.
        let mut decl = closed_base();
        let rows = decl.relations[0].extension.as_mut().expect("closed");
        rows.swap(0, 1);
        assert_ne!(
            fingerprint(&schema_of(closed_base())),
            fingerprint(&schema_of(decl))
        );
    }

    #[test]
    fn changing_an_extension_value_changes_the_fingerprint() {
        // Intrinsic values are theory identity — changing one is a new
        // theory (the intrinsic-vs-policy law, `10-data-model.md`).
        let mut decl = closed_base();
        decl.relations[0].extension.as_mut().expect("closed")[1] =
            crate::schema::tests::row("Eur", vec![Value::U64(3)]);
        assert_ne!(
            fingerprint(&schema_of(closed_base())),
            fingerprint(&schema_of(decl))
        );
    }

    #[test]
    fn renaming_a_handle_changes_the_fingerprint() {
        let mut decl = closed_base();
        decl.relations[0].extension.as_mut().expect("closed")[0] =
            crate::schema::tests::row("Chf", vec![Value::U64(2)]);
        assert_ne!(
            fingerprint(&schema_of(closed_base())),
            fingerprint(&schema_of(decl))
        );
    }

    #[test]
    fn golden_bytes_pin_the_extension_encoding() {
        // `closed_base()` pins the closedness tag, the synthetic id field,
        // the pre-encoded row fact bytes (id ‖ values), and the closed
        // auto-key's materialization.
        let schema = schema_of(closed_base());
        let mut bytes = Vec::new();
        canonical_bytes(&schema, &mut bytes);

        let mut expected: Vec<u8> = Vec::new();
        expected.extend_from_slice(&18u32.to_le_bytes());
        expected.extend_from_slice(b"bumbledb-schema-v2");
        expected.extend_from_slice(&1u32.to_le_bytes()); // relation count
        expected.extend_from_slice(&8u32.to_le_bytes());
        expected.extend_from_slice(b"Currency");
        expected.extend_from_slice(&2u32.to_le_bytes()); // field count: synthetic id + 1
        expected.extend_from_slice(&2u32.to_le_bytes());
        expected.extend_from_slice(b"id");
        expected.push(2); // ValueType::U64 tag
        expected.push(0); // Generation::None tag
        expected.extend_from_slice(&11u32.to_le_bytes());
        expected.extend_from_slice(b"minor_units");
        expected.push(2); // ValueType::U64 tag
        expected.push(0); // Generation::None tag
        expected.push(1); // closed
        expected.extend_from_slice(&2u32.to_le_bytes()); // row count
        expected.extend_from_slice(&3u32.to_le_bytes());
        expected.extend_from_slice(b"Usd");
        expected.extend_from_slice(&16u32.to_le_bytes()); // fact len
        expected.extend_from_slice(&0u64.to_be_bytes()); // id 0
        expected.extend_from_slice(&2u64.to_be_bytes()); // minor_units 2
        expected.extend_from_slice(&3u32.to_le_bytes());
        expected.extend_from_slice(b"Eur");
        expected.extend_from_slice(&16u32.to_le_bytes()); // fact len
        expected.extend_from_slice(&1u64.to_be_bytes()); // id 1
        expected.extend_from_slice(&2u64.to_be_bytes()); // minor_units 2
        expected.extend_from_slice(&1u32.to_le_bytes()); // statement count
        expected.push(0); // Functionality form tag (the closed auto-key)
        expected.extend_from_slice(&0u32.to_le_bytes()); // relation id
        expected.extend_from_slice(&1u32.to_le_bytes()); // projection len
        expected.extend_from_slice(&0u16.to_le_bytes()); // field id: the synthetic id
        assert_eq!(bytes, expected);
    }

    #[test]
    fn length_prefixes_prevent_name_aliasing() {
        // Without length prefixes, ("AB" + "C") and ("A" + "BC") would
        // concatenate to identical streams.
        let of_names = |relation: &str, field_name: &str| {
            schema_of(SchemaDescriptor {
                relations: vec![RelationDescriptor {
                    extension: None,
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
