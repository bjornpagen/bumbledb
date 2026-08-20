//! Canonical schema encoding and the blake3 fingerprint (docs/architecture/10-data-model.md).
//!
//! The fingerprint inputs are enumerated exhaustively in
//! `docs/architecture/10-data-model.md`; that list is the contract
//! ([`canonical_bytes`] reproduces it). Every string and list is
//! length-prefixed (u32 LE) so no two schemas can alias to one byte stream;
//! relation, field, and statement ids are pinned by declaration/materialized
//! order and therefore covered without being hashed separately.
//!
//! Sealed enforcement data (target keys, key permutations, interval tails)
//! and the sealed `==` pairing ([`super::ContainmentStatement::mirror`])
//! are **not** hashed:
//! the acceptance gate computes both as deterministic functions of the
//! hashed inputs, the same way materialized order leaves "statement ids …
//! pinned by the fingerprint without being hashed separately"
//! (`docs/architecture/10-data-model.md`).

use super::wire::{
    ClosednessTag, EncodedHi, GenerationTag, IntervalElementTag, StatementFormTag, ValueTypeTag,
    WeightTag,
};
use super::{
    FieldId, Generation, IntervalElement, LiteralSet, RelationId, Schema, Side, StatementId,
    StatementView, ValueType, Weight,
};
use crate::encoding::encode_literal;
use bumbledb_theory::Value;

/// Bumped whenever the canonical encoding format itself changes. `v1`:
/// the statement redesign. `v2`: closed relations — every relation gains a
/// closedness tag byte (so ordinary and closed relations can never alias
/// one byte stream), and a closed relation's ground axioms hash after its
/// fields. `v3`: the dependency-vocabulary extension — every selection
/// binding hashes a literal COUNT before its literals (the disjunctive
/// set form), and the two extension statement forms took tags 2 (the
/// count-only window) and 3 (order mark). `v4`: the order purge — the
/// statement spine sum shrank (tag 3 no longer exists), so the label
/// bumps (the version-bump law; nothing deployed carries an order
/// statement). `v5`: the capacity cutover — the count-only encoding
/// retired with tag 2 and the capacity statement minted tag 4 (never 3:
/// a retired tag is never reissued — ruled 2026-07-24, C5), encoding
/// weight descriptor and kind-tagged bounds in the operator's read
/// order (C2), so every fingerprint moves with the label.
pub(super) const FORMAT_VERSION_LABEL: &[u8] = b"bumbledb-schema-v5";

/// Deterministic schema identity: blake3 of the canonical bytes. Stored at
/// database creation; open compares fingerprints and mismatches are hard
/// failures (docs/architecture/50-storage.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaFingerprint(pub [u8; 32]);

/// The 64-char lowercase hex of the 32 bytes — the fingerprint's one
/// rendering, so the mismatch and desync diagnostics
/// ([`crate::error::Error::SchemaMismatch`],
/// [`crate::error::CorruptionError::DescriptorFingerprintDesync`]) print
/// the values a host must compare, never a payload-free sentence.
impl std::fmt::Display for SchemaFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

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
///   judgment form (Functionality = 0, Containment = 1, Capacity = 4 —
///   tags 2 and 3 are retired and never reissued, C5)
///   and its body — sides as (relation id, projection field-id
///   list in statement order, selection list as (field id, literal count,
///   literal values in canonical set order) bindings in statement order);
///   a capacity statement encodes in the operator's read order (C2):
///   target side, the weight descriptor (kind tag 0 = unit / 1 = field ‖
///   field id / 2 = Duration ‖ field id), the window (`lo`'s literal
///   value, then the `hi` presence tag followed, when present, by its
///   kind tag 0 = literal ‖ u64 / 1 = target field ‖ field id /
///   2 = Duration(target field) ‖ field id), then the source side.
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
                Generation::None => GenerationTag::None.tag(),
                Generation::Fresh => GenerationTag::Fresh.tag(),
            });
        }
        // Closedness is theory identity: ground axioms hash in declaration
        // order — the sealed pre-encoded fact bytes, whose per-value shape
        // is a function of the field types already in the stream. The tag
        // keeps ordinary and closed relations from aliasing.
        match relation.body().closed_rows() {
            None => out.push(ClosednessTag::Ordinary.tag()),
            Some(rows) => {
                out.push(ClosednessTag::Closed.tag());
                put_len(out, rows.len());
                for row in rows {
                    put_bytes(out, row.handle.as_bytes());
                    put_bytes(out, &row.fact);
                }
            }
        }
    }
    let statement_count =
        schema.keys().len() + schema.containments().len() + schema.capacities().len();
    put_len(out, statement_count);
    for index in 0..statement_count {
        let id = StatementId(u16::try_from(index).expect("statement count fits u16"));
        match schema.statement(id) {
            StatementView::Key(_, statement) => {
                out.push(StatementFormTag::Functionality.tag());
                put_relation_id(out, statement.relation);
                put_len(out, statement.projection.len());
                for field in &statement.projection {
                    put_field_id(out, *field);
                }
            }
            StatementView::Containment(_, statement) => {
                out.push(StatementFormTag::Containment.tag());
                put_side(out, schema, &statement.source);
                put_side(out, schema, &statement.target);
            }
            StatementView::Capacity(_, statement) => {
                out.push(StatementFormTag::Capacity.tag());
                put_side(out, schema, &statement.target);
                match statement.weight.to_weight() {
                    Weight::Unit => out.push(WeightTag::Unit.tag()),
                    Weight::Field(field) => {
                        out.push(WeightTag::Field.tag());
                        put_field_id(out, field);
                    }
                    Weight::DurationOf(field) => {
                        out.push(WeightTag::DurationOf.tag());
                        put_field_id(out, field);
                    }
                }
                out.extend_from_slice(&statement.lo.to_le_bytes());
                EncodedHi::from_bound(statement.hi.to_bound()).write(out);
                put_side(out, schema, &statement.source);
            }
        }
    }
}

/// The canonical schema-descriptor byte string — the fingerprint's exact
/// preimage, materialized. These are THE bytes a store persists beside its
/// fingerprint (`docs/architecture/50-storage.md` § the `_meta` block):
/// one canonical encoding exists, and persisting anything else would mint
/// a second one. Readers: store creation and format-8 open-time
/// fingerprint check (`storage/env`). There is no decoder: the bytes are
/// the fingerprint preimage, not a product open path.
#[must_use]
pub(crate) fn canonical_descriptor(schema: &Schema) -> Vec<u8> {
    let mut bytes = Vec::new();
    canonical_bytes(schema, &mut bytes);
    bytes
}

/// Blake3 of a canonical descriptor byte string — the one hash the
/// fingerprint IS. Split from [`fingerprint`] so the store paths that
/// already hold the persisted bytes hash exactly what they read instead
/// of re-encoding.
#[must_use]
pub(crate) fn fingerprint_of_descriptor(bytes: &[u8]) -> SchemaFingerprint {
    SchemaFingerprint(*blake3::hash(bytes).as_bytes())
}

/// Computes the schema fingerprint: blake3 of [`canonical_bytes`].
#[must_use]
pub fn fingerprint(schema: &Schema) -> SchemaFingerprint {
    fingerprint_of_descriptor(&canonical_descriptor(schema))
}

fn put_len(out: &mut Vec<u8>, len: usize) {
    let len = u32::try_from(len).expect("schema list length fits u32");
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

fn put_side(out: &mut Vec<u8>, schema: &Schema, side: &Side) {
    put_relation_id(out, side.relation);
    put_len(out, side.projection.len());
    for field in &side.projection {
        put_field_id(out, *field);
    }
    put_len(out, side.selection.len());
    for (field, literals) in &side.selection {
        put_field_id(out, *field);
        // Literals hash at the selected FIELD's encoding — the same
        // type-aware `encode_literal` the commit judgment seals, so a
        // fixed-width interval binding hashes its one-word form.
        let desc = schema.relation(side.relation).field(*field).value_type;
        // The literal COUNT precedes the literals: a one-literal binding
        // and a set binding can never alias, and the sealed side's
        // canonical (sorted, deduplicated) set order makes the stream a
        // function of the set, not its spelling.
        match literals {
            LiteralSet::One(literal) => {
                put_len(out, 1);
                put_literal(out, desc, literal);
            }
            LiteralSet::Many(values) => {
                put_len(out, values.len());
                for literal in values {
                    put_literal(out, desc, literal);
                }
            }
        }
    }
}

fn put_value_type(out: &mut Vec<u8>, value_type: &ValueType) {
    // Tag 1 is the deleted enum type's tombstone; it is never reused —
    // a reissued tag would collide theories across the vocabulary cut.
    match value_type {
        ValueType::Bool => out.push(ValueTypeTag::Bool.tag()),
        ValueType::U64 => out.push(ValueTypeTag::U64.tag()),
        ValueType::I64 => out.push(ValueTypeTag::I64.tag()),
        ValueType::String => out.push(ValueTypeTag::String.tag()),
        ValueType::FixedBytes { len } => {
            out.push(ValueTypeTag::FixedBytes.tag());
            out.extend_from_slice(&len.to_le_bytes());
        }
        ValueType::Interval { element } => {
            out.push(ValueTypeTag::Interval.tag());
            out.push(element_tag(*element));
        }
        ValueType::FixedInterval { element, width } => {
            out.push(ValueTypeTag::FixedInterval.tag());
            out.push(element_tag(*element));
            out.extend_from_slice(&width.to_le_bytes());
        }
    }
}

/// The element domain's fingerprint byte, shared by both interval tags.
fn element_tag(element: IntervalElement) -> u8 {
    match element {
        IntervalElement::U64 => IntervalElementTag::U64.tag(),
        IntervalElement::I64 => IntervalElementTag::I64.tag(),
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
fn put_literal(out: &mut Vec<u8>, desc: bumbledb_theory::schema::ValueType, literal: &Value) {
    match literal {
        Value::String(text) => put_bytes(out, text.as_bytes()),
        encoded => encode_literal(encoded, desc, out),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Bound, RelationDescriptor, SchemaDescriptor, StatementId};
    use super::*;
    use crate::schema::ValidateDescriptor as _;
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

    #[test]
    fn golden_fingerprint_pins_the_hash() {
        // Pinned: the canonical encoding (and therefore blake3 of it)
        // must not drift while the format label stays `v5`. `base()` covers
        // every literal-adjacent input: fresh auto-keys, a declared key,
        // and a containment with a selection literal. Pinned through
        // `Display` — the fingerprint's one rendering, hex of all 32
        // bytes, the same string the mismatch diagnostics print.
        assert_eq!(
            base_fingerprint().to_string(),
            "1959c7ceac2e7e8214b382db0bfafb17a2510d1713a441ef1b1df763b1973ced"
        );
    }

    #[test]
    fn mirror_links_never_reach_the_fingerprint() {
        // Identity golden: the sealed `==` pairing is derived from hashed
        // inputs exactly like enforcement, so a
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
            schema
                .containment(crate::schema::ContainmentId(0))
                .mirror_id(&schema),
            Some(StatementId(4))
        );
        assert_eq!(
            fingerprint(&schema).to_string(),
            "ee5bd9b1673ddac9c8aa32e6136901258c47ba9aa0e9d4f6dce98a5eda5c23d8"
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
        // X is ordered (the order defines the determinant key): the same field
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
    fn the_interval_width_is_a_fingerprint_input() {
        // A width change is a new theory — exactly as a bytes<N> width
        // change is — and the fixed family never aliases the general
        // type (distinct tags in the canonical stream).
        let of = |ty: ValueType| {
            fingerprint(&schema_of(SchemaDescriptor {
                relations: vec![RelationDescriptor {
                    extension: None,
                    name: "R".into(),
                    fields: vec![field("slot", ty)],
                }],
                statements: vec![],
            }))
        };
        assert_ne!(
            of(ValueType::FixedInterval {
                element: IntervalElement::U64,
                width: 1,
            }),
            of(ValueType::FixedInterval {
                element: IntervalElement::U64,
                width: 2,
            })
        );
        assert_ne!(
            of(ValueType::FixedInterval {
                element: IntervalElement::U64,
                width: 1,
            }),
            of(ValueType::Interval {
                element: IntervalElement::U64,
            })
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
        expected.extend_from_slice(b"bumbledb-schema-v5");
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
        expected.extend_from_slice(b"bumbledb-schema-v5");
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
        expected.extend_from_slice(&1u32.to_le_bytes()); // literal count (singleton)
        expected.extend_from_slice(&1u64.to_be_bytes()); // u64 literal, canonical encoding
        expected.extend_from_slice(&0u32.to_le_bytes()); // target relation id
        expected.extend_from_slice(&1u32.to_le_bytes()); // projection len
        expected.extend_from_slice(&0u16.to_le_bytes()); // field id
        expected.extend_from_slice(&0u32.to_le_bytes()); // selection len
        assert_eq!(bytes, expected);
    }

    /// The tag-4 body golden — the capacity statement's byte-level pin,
    /// covering every weight kind (unit / field / Duration) and every
    /// bound kind (`*`, literal, target field, Duration(target field))
    /// in the operator's read order (C2): target ‖ weight ‖ lo ‖ hi ‖
    /// source. The count-only tag 2 never had a byte golden; the
    /// most-changed encoding does not land without one — a codec drift
    /// here would otherwise surface first as a field decode failure.
    #[expect(
        clippy::too_many_lines,
        reason = "one byte-golden, every weight/bound kind pinned in read order — clearer kept together"
    )]
    #[test]
    fn golden_bytes_pin_the_capacity_encoding() {
        use crate::schema::tests::{capacity, capacity_weighted};
        let interval = ValueType::Interval {
            element: IntervalElement::U64,
        };
        let schema = schema_of(SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Pool".into(),
                    fields: vec![
                        field("id", ValueType::U64),
                        field("supply", ValueType::U64),
                        field("span", interval),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Dev".into(),
                    fields: vec![
                        field("pool", ValueType::U64),
                        field("watts", ValueType::U64),
                        field("busy", interval),
                    ],
                },
            ],
            statements: vec![
                fd(RelationId(0), &[FieldId(0)]),
                // Unit floor `{2..*}` and unit range `{0..3}`.
                capacity(
                    side(RelationId(1), &[FieldId(0)]),
                    2,
                    None,
                    side(RelationId(0), &[FieldId(0)]),
                ),
                capacity(
                    side(RelationId(1), &[FieldId(0)]),
                    0,
                    Some(3),
                    side(RelationId(0), &[FieldId(0)]),
                ),
                // `Pool(id) <=[watts]{0..supply} Dev(pool)`.
                capacity_weighted(
                    side(RelationId(0), &[FieldId(0)]),
                    Weight::Field(FieldId(1)),
                    0,
                    Some(Bound::TargetField(FieldId(1))),
                    side(RelationId(1), &[FieldId(0)]),
                ),
                // `Pool(id) <=[Duration(busy)]{0..Duration(span)} Dev(pool)`.
                capacity_weighted(
                    side(RelationId(0), &[FieldId(0)]),
                    Weight::DurationOf(FieldId(2)),
                    0,
                    Some(Bound::TargetDuration(FieldId(2))),
                    side(RelationId(1), &[FieldId(0)]),
                ),
            ],
        });
        let mut bytes = Vec::new();
        canonical_bytes(&schema, &mut bytes);

        let mut expected: Vec<u8> = Vec::new();
        expected.extend_from_slice(&18u32.to_le_bytes());
        expected.extend_from_slice(b"bumbledb-schema-v5");
        expected.extend_from_slice(&2u32.to_le_bytes()); // relation count
        let put_relation = |expected: &mut Vec<u8>, name: &str, fields: [&str; 3]| {
            expected.extend_from_slice(&u32::try_from(name.len()).expect("len").to_le_bytes());
            expected.extend_from_slice(name.as_bytes());
            expected.extend_from_slice(&3u32.to_le_bytes()); // field count
            for (idx, field_name) in fields.iter().enumerate() {
                expected.extend_from_slice(
                    &u32::try_from(field_name.len()).expect("len").to_le_bytes(),
                );
                expected.extend_from_slice(field_name.as_bytes());
                if idx == 2 {
                    expected.push(6); // ValueType::Interval (general) tag
                    expected.push(0); // IntervalElement::U64
                } else {
                    expected.push(2); // ValueType::U64 tag
                }
                expected.push(0); // Generation::None tag
            }
            expected.push(0); // ordinary: no extension
        };
        put_relation(&mut expected, "Pool", ["id", "supply", "span"]);
        put_relation(&mut expected, "Dev", ["pool", "watts", "busy"]);
        expected.extend_from_slice(&5u32.to_le_bytes()); // statement count
        expected.push(0); // Functionality form tag
        expected.extend_from_slice(&0u32.to_le_bytes()); // relation id
        expected.extend_from_slice(&1u32.to_le_bytes()); // projection len
        expected.extend_from_slice(&0u16.to_le_bytes()); // field id
        // One unselected side, `Rel(field 0)` — all four statements use it.
        let put_bare_side = |expected: &mut Vec<u8>, relation: u32| {
            expected.extend_from_slice(&relation.to_le_bytes());
            expected.extend_from_slice(&1u32.to_le_bytes()); // projection len
            expected.extend_from_slice(&0u16.to_le_bytes()); // field id
            expected.extend_from_slice(&0u32.to_le_bytes()); // selection len
        };
        // `Pool(id) <={2..*} Dev(pool)` — unit weight, star ceiling.
        expected.push(4); // Capacity form tag
        put_bare_side(&mut expected, 0); // target: Pool(id)
        expected.push(0); // Weight::Unit kind tag
        expected.extend_from_slice(&2u64.to_le_bytes()); // lo
        expected.push(0); // hi absent (`*`)
        put_bare_side(&mut expected, 1); // source: Dev(pool)
        // `Pool(id) <={0..3} Dev(pool)` — literal ceiling.
        expected.push(4);
        put_bare_side(&mut expected, 0);
        expected.push(0); // Weight::Unit
        expected.extend_from_slice(&0u64.to_le_bytes()); // lo
        expected.push(1); // hi present
        expected.push(0); // Bound::Lit kind tag
        expected.extend_from_slice(&3u64.to_le_bytes()); // literal value
        put_bare_side(&mut expected, 1);
        // `Pool(id) <=[watts]{0..supply} Dev(pool)` — field weight,
        // dependent ceiling.
        expected.push(4);
        put_bare_side(&mut expected, 0);
        expected.push(1); // Weight::Field kind tag
        expected.extend_from_slice(&1u16.to_le_bytes()); // watts
        expected.extend_from_slice(&0u64.to_le_bytes()); // lo
        expected.push(1); // hi present
        expected.push(1); // Bound::TargetField kind tag
        expected.extend_from_slice(&1u16.to_le_bytes()); // supply
        put_bare_side(&mut expected, 1);
        // `Pool(id) <=[Duration(busy)]{0..Duration(span)} Dev(pool)`.
        expected.push(4);
        put_bare_side(&mut expected, 0);
        expected.push(2); // Weight::DurationOf kind tag
        expected.extend_from_slice(&2u16.to_le_bytes()); // busy
        expected.extend_from_slice(&0u64.to_le_bytes()); // lo
        expected.push(1); // hi present
        expected.push(2); // Bound::TargetDuration kind tag
        expected.extend_from_slice(&2u16.to_le_bytes()); // span
        put_bare_side(&mut expected, 1);
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
        // pre-encoded rows (like enforcement and `mirror`) are deterministic
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
        expected.extend_from_slice(b"bumbledb-schema-v5");
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
