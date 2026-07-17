//! The canonical schema-descriptor DECODER — the exact inverse of
//! [`super::fingerprint`]'s `canonical_bytes` stream, one function per
//! production of that encoding (label, relations with fields and sealed
//! extensions, statements in materialized order). Readers: the exhume
//! entry ([`crate::exhume`]) — the only consumer that ever holds
//! descriptor bytes without the theory that produced them.
//!
//! The decoder returns the schema **as declared** (a [`SchemaDescriptor`]
//! ready for `.validate()`): the canonical stream carries the SEALED shape
//! — closed relations open with the synthetic `id` field, and the
//! statement list opens with the auto-materialized keys (one per `fresh`
//! field, then one per closed relation) — so decoding strips exactly what
//! validation re-materializes. The round trip is self-verifying at the
//! caller: re-encoding the validated schema must reproduce the input bytes
//! (and therefore the stored fingerprint), so a decoder drift can never
//! silently misread a store.
//!
//! Errors are `&'static str` shape diagnoses, wrapped by the caller as the
//! typed [`CorruptionError::MalformedValue`] — persisted descriptor bytes
//! that fail to parse are corrupt data, exactly as a mis-shaped `F` key is.
//!
//! [`CorruptionError::MalformedValue`]: crate::error::CorruptionError::MalformedValue

use crate::encoding::{FactLayout, TypeDesc, ValueRef, decode_field};
use bumbledb_theory::{Interval, Value};

use super::fingerprint::FORMAT_VERSION_LABEL;
use super::{
    FieldDescriptor, FieldId, Generation, IntervalElement, LiteralSet, RelationDescriptor,
    RelationId, Row, SchemaDescriptor, Side, StatementDescriptor, ValueType,
};

/// One decoded relation, both shapes: the declared descriptor (synthetic
/// id stripped, extension rows as values) and the sealed field list the
/// statement decoder needs for literal typing.
struct DecodedRelation {
    declared: RelationDescriptor,
    sealed_fields: Vec<FieldDescriptor>,
}

/// Decodes a persisted canonical descriptor back into the schema **as
/// declared**. The caller owns the two integrity checks the decode itself
/// cannot make: blake3 of the input against the stored fingerprint, and
/// the re-encode round trip after validation.
///
/// # Errors
///
/// A static shape diagnosis naming the failing production — truncated
/// stream, an unknown tag, a non-UTF-8 name, a mis-shaped extension row,
/// trailing bytes, or an auto-key prefix that does not re-materialize.
pub(crate) fn decode_descriptor(bytes: &[u8]) -> Result<SchemaDescriptor, &'static str> {
    let mut cur = Cursor(bytes);
    if cur.bytes()? != FORMAT_VERSION_LABEL {
        return Err("descriptor encoding label");
    }
    let relation_count = cur.len()?;
    let mut relations: Vec<DecodedRelation> = Vec::with_capacity(relation_count);
    for _ in 0..relation_count {
        relations.push(relation(&mut cur)?);
    }
    let statement_count = cur.len()?;
    let mut statements = Vec::with_capacity(statement_count);
    for _ in 0..statement_count {
        statements.push(statement(&mut cur, &relations)?);
    }
    if !cur.0.is_empty() {
        return Err("descriptor trailing bytes");
    }
    let declared_relations: Vec<RelationDescriptor> =
        relations.into_iter().map(|rel| rel.declared).collect();
    // The stream's statement list opens with the auto-materialized keys.
    // Re-materialize them from the decoded relations and strip exactly
    // that prefix — what remains is the declared statement list
    // (validation rejects a declared duplicate of an auto key, so the
    // split is unambiguous).
    let auto = SchemaDescriptor {
        relations: declared_relations.clone(),
        statements: Vec::new(),
    }
    .materialized_statements();
    if statements.len() < auto.len() || statements[..auto.len()] != auto[..] {
        return Err("descriptor auto-key prefix");
    }
    statements.drain(..auto.len());
    Ok(SchemaDescriptor {
        relations: declared_relations,
        statements,
    })
}

/// One relation: name, sealed fields, closedness tag, and (closed) the
/// sealed extension rows — decoded back to the DECLARED shape.
fn relation(cur: &mut Cursor<'_>) -> Result<DecodedRelation, &'static str> {
    let name = utf8(cur.bytes()?, "descriptor relation name")?;
    let field_count = cur.len()?;
    let mut sealed_fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        let field_name = utf8(cur.bytes()?, "descriptor field name")?;
        let value_type = value_type(cur)?;
        let generation = match cur.byte()? {
            0 => Generation::None,
            1 => Generation::Fresh,
            _ => return Err("descriptor generation tag"),
        };
        sealed_fields.push(FieldDescriptor {
            name: field_name.into(),
            value_type,
            generation,
        });
    }
    match cur.byte()? {
        0 => Ok(DecodedRelation {
            declared: RelationDescriptor {
                name: name.into(),
                fields: sealed_fields.clone(),
                extension: None,
            },
            sealed_fields,
        }),
        1 => {
            // The sealed field list of a closed relation opens with the
            // synthetic (`id`, u64) field validation prepends; the
            // declared descriptor never carries it.
            let [id_field, declared_fields @ ..] = sealed_fields.as_slice() else {
                return Err("descriptor closed relation without fields");
            };
            if id_field.name.as_ref() != "id"
                || id_field.value_type != ValueType::U64
                || id_field.generation != Generation::None
            {
                return Err("descriptor closed synthetic id field");
            }
            let layout = FactLayout::new(
                &sealed_fields
                    .iter()
                    .map(|field| field.value_type.type_desc())
                    .collect::<Vec<_>>(),
            );
            let row_count = cur.len()?;
            let mut rows = Vec::with_capacity(row_count);
            for row_id in 0..row_count {
                let handle = utf8(cur.bytes()?, "descriptor extension handle")?;
                let fact = cur.bytes()?;
                rows.push(extension_row(handle, fact, &layout, row_id)?);
            }
            Ok(DecodedRelation {
                declared: RelationDescriptor {
                    name: name.into(),
                    fields: declared_fields.to_vec(),
                    extension: Some(rows.into_boxed_slice()),
                },
                sealed_fields,
            })
        }
        _ => Err("descriptor closedness tag"),
    }
}

/// One sealed extension row's canonical fact bytes, decoded back to the
/// declared [`Row`]: the leading synthetic id (which must equal the
/// declaration index) is stripped, the intrinsic values decode per the
/// sealed layout. Closed relations refuse `str` columns at declaration,
/// so no dictionary exists to consult — an intern-id field here is a
/// mis-shaped stream.
fn extension_row(
    handle: &str,
    fact: &[u8],
    layout: &FactLayout,
    row_id: usize,
) -> Result<Row, &'static str> {
    if fact.len() != layout.fact_width() {
        return Err("descriptor extension row width");
    }
    let mut values = Vec::with_capacity(layout.field_count().saturating_sub(1));
    for idx in 0..layout.field_count() {
        let decoded =
            decode_field(fact, layout, idx).map_err(|_| "descriptor extension row value")?;
        if idx == 0 {
            if decoded != ValueRef::U64(row_id as u64) {
                return Err("descriptor extension row id");
            }
            continue;
        }
        values.push(match decoded {
            ValueRef::Bool(v) => Value::Bool(v),
            ValueRef::U64(v) => Value::U64(v),
            ValueRef::I64(v) => Value::I64(v),
            ValueRef::String(_) => return Err("descriptor extension row str column"),
            ValueRef::FixedBytes(value) => Value::FixedBytes(value.as_bytes().into()),
            ValueRef::IntervalU64(interval) | ValueRef::FixedIntervalU64(interval) => {
                Value::IntervalU64(interval)
            }
            ValueRef::IntervalI64(interval) | ValueRef::FixedIntervalI64(interval) => {
                Value::IntervalI64(interval)
            }
        });
    }
    Ok(Row {
        handle: handle.into(),
        values: values.into_boxed_slice(),
    })
}

/// One structural type description — the inverse of the encoder's
/// `put_value_type` tag table (tag 1 is the deleted enum tombstone and
/// never decodes).
fn value_type(cur: &mut Cursor<'_>) -> Result<ValueType, &'static str> {
    Ok(match cur.byte()? {
        0 => ValueType::Bool,
        2 => ValueType::U64,
        3 => ValueType::I64,
        4 => ValueType::String,
        5 => ValueType::FixedBytes { len: cur.u16()? },
        6 => ValueType::Interval {
            element: element(cur)?,
            width: None,
        },
        7 => ValueType::Interval {
            element: element(cur)?,
            width: Some(cur.u64()?),
        },
        _ => return Err("descriptor value-type tag"),
    })
}

fn element(cur: &mut Cursor<'_>) -> Result<IntervalElement, &'static str> {
    match cur.byte()? {
        0 => Ok(IntervalElement::U64),
        1 => Ok(IntervalElement::I64),
        _ => Err("descriptor interval element tag"),
    }
}

/// One statement: the form tag and its body, sides decoded with literal
/// typing resolved through the already-decoded relations' SEALED field
/// lists (relations encode before statements, so the shape of every
/// literal is a function of bytes already decoded).
fn statement(
    cur: &mut Cursor<'_>,
    relations: &[DecodedRelation],
) -> Result<StatementDescriptor, &'static str> {
    Ok(match cur.byte()? {
        0 => {
            let relation = relation_id(cur, relations)?;
            let projection_len = cur.len()?;
            let mut projection = Vec::with_capacity(projection_len);
            for _ in 0..projection_len {
                projection.push(FieldId(cur.u16()?));
            }
            StatementDescriptor::Functionality {
                relation,
                projection: projection.into_boxed_slice(),
            }
        }
        1 => StatementDescriptor::Containment {
            source: side(cur, relations)?,
            target: side(cur, relations)?,
        },
        2 => {
            let source = side(cur, relations)?;
            let lo = cur.u64()?;
            let hi = match cur.byte()? {
                0 => None,
                1 => Some(cur.u64()?),
                _ => return Err("descriptor window hi tag"),
            };
            let target = side(cur, relations)?;
            StatementDescriptor::Cardinality {
                source,
                lo,
                hi,
                target,
            }
        }
        _ => return Err("descriptor statement form tag"),
    })
}

/// One side: relation, projection, and the selection bindings — each
/// literal decoded at its selected field's encoding, exactly where the
/// encoder's `put_side` resolved the same [`TypeDesc`].
fn side(cur: &mut Cursor<'_>, relations: &[DecodedRelation]) -> Result<Side, &'static str> {
    let relation = relation_id(cur, relations)?;
    let sealed_fields = &relations[relation.0 as usize].sealed_fields;
    let projection_len = cur.len()?;
    let mut projection = Vec::with_capacity(projection_len);
    for _ in 0..projection_len {
        projection.push(FieldId(cur.u16()?));
    }
    let selection_len = cur.len()?;
    let mut selection = Vec::with_capacity(selection_len);
    for _ in 0..selection_len {
        let field = FieldId(cur.u16()?);
        let desc = sealed_fields
            .get(usize::from(field.0))
            .ok_or("descriptor selection field id")?
            .value_type
            .type_desc();
        let count = cur.len()?;
        let literals = match count {
            0 => return Err("descriptor empty literal set"),
            1 => LiteralSet::One(literal(cur, desc)?),
            _ => {
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(literal(cur, desc)?);
                }
                LiteralSet::Many(values.into_boxed_slice())
            }
        };
        selection.push((field, literals));
    }
    Ok(Side {
        relation,
        projection: projection.into_boxed_slice(),
        selection: selection.into_boxed_slice(),
    })
}

fn relation_id(
    cur: &mut Cursor<'_>,
    relations: &[DecodedRelation],
) -> Result<RelationId, &'static str> {
    let id = cur.u32()?;
    if (id as usize) < relations.len() {
        Ok(RelationId(id))
    } else {
        Err("descriptor relation id")
    }
}

/// One selection literal at its field's encoding — the inverse of
/// `put_literal`: `str` literals are length-prefixed raw bytes (the one
/// per-database-free string form); everything else is the shared
/// [`crate::encoding::encode_literal`] shape at the field's [`TypeDesc`].
fn literal(cur: &mut Cursor<'_>, desc: TypeDesc) -> Result<Value, &'static str> {
    Ok(match desc {
        TypeDesc::String => Value::String(cur.bytes()?.into()),
        TypeDesc::Bool => Value::Bool(
            crate::encoding::decode_bool(cur.byte()?).map_err(|_| "descriptor bool literal")?,
        ),
        TypeDesc::U64 => Value::U64(crate::encoding::decode_u64(cur.word()?)),
        TypeDesc::I64 => Value::I64(crate::encoding::decode_i64(cur.word()?)),
        TypeDesc::FixedBytes { len } => {
            let padded = cur.take(crate::encoding::fixed_bytes_words(len) * 8)?;
            let raw = padded
                .get(..usize::from(len))
                .ok_or("descriptor bytes literal width")?;
            if padded[usize::from(len)..].iter().any(|&byte| byte != 0) {
                return Err("descriptor bytes literal pad");
            }
            Value::FixedBytes(raw.into())
        }
        TypeDesc::Interval {
            element,
            width: None,
        } => {
            let (start, end) = (cur.word()?, cur.word()?);
            match element {
                IntervalElement::U64 => Value::IntervalU64(
                    Interval::<u64>::new(
                        crate::encoding::decode_u64(start),
                        crate::encoding::decode_u64(end),
                    )
                    .ok_or("descriptor interval literal bounds")?,
                ),
                IntervalElement::I64 => Value::IntervalI64(
                    Interval::<i64>::new(
                        crate::encoding::decode_i64(start),
                        crate::encoding::decode_i64(end),
                    )
                    .ok_or("descriptor interval literal bounds")?,
                ),
            }
        }
        TypeDesc::Interval {
            element,
            width: Some(width),
        } => {
            let (start_word, end_word) =
                crate::encoding::decode_fixed_interval_start(cur.word()?, width)
                    .map_err(|_| "descriptor fixed interval literal start")?;
            match element {
                IntervalElement::U64 => Value::IntervalU64(
                    Interval::<u64>::new(start_word, end_word)
                        .ok_or("descriptor fixed interval literal bounds")?,
                ),
                IntervalElement::I64 => Value::IntervalI64(
                    Interval::<i64>::new(
                        crate::encoding::decode_i64(start_word.to_be_bytes()),
                        crate::encoding::decode_i64(end_word.to_be_bytes()),
                    )
                    .ok_or("descriptor fixed interval literal bounds")?,
                ),
            }
        }
    })
}

fn utf8<'a>(bytes: &'a [u8], what: &'static str) -> Result<&'a str, &'static str> {
    std::str::from_utf8(bytes).map_err(|_| what)
}

/// A forward-only view over the descriptor bytes; every read is
/// length-checked (truncation is a typed shape diagnosis, never a panic —
/// persisted bytes are data).
struct Cursor<'a>(&'a [u8]);

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], &'static str> {
        if self.0.len() < n {
            return Err("descriptor truncated");
        }
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        Ok(head)
    }

    fn byte(&mut self) -> Result<u8, &'static str> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, &'static str> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("2")))
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }

    fn u64(&mut self) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("8")))
    }

    /// One canonical big-endian word (the value encodings' unit).
    fn word(&mut self) -> Result<[u8; 8], &'static str> {
        Ok(self.take(8)?.try_into().expect("8"))
    }

    /// A u32 length prefix, bounded by the remaining input so a forged
    /// length can never drive an allocation past the stream.
    fn len(&mut self) -> Result<usize, &'static str> {
        let len = self.u32()? as usize;
        if len > self.0.len() {
            return Err("descriptor length prefix");
        }
        Ok(len)
    }

    /// A length-prefixed byte string (`put_bytes`'s inverse).
    fn bytes(&mut self) -> Result<&'a [u8], &'static str> {
        let len = self.len()?;
        self.take(len)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ValidateDescriptor as _;
    use super::super::fingerprint::{canonical_descriptor, fingerprint};
    use super::super::tests::{
        cardinality, closed, containment, fd, field, fresh_field, row, side, side_where,
        side_where_sets,
    };
    use super::*;

    /// Every construct the canonical encoding can carry: both closed
    /// tiers' shapes, `fresh`, every field type (str, `bytes<N>`, general
    /// and fixed-width intervals over both elements), a declared FD, a
    /// containment with singleton and set selections (str, u64, bytes,
    /// ray-interval literals), and windows with and without a ceiling.
    #[expect(
        clippy::too_many_lines,
        reason = "the exhaustive fixture is clearer as one literal declaration"
    )]
    fn everything() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![
                closed(
                    "Kind",
                    vec![
                        field("mastered", ValueType::Bool),
                        field("weight", ValueType::U64),
                        field(
                            "span",
                            ValueType::Interval {
                                element: IntervalElement::U64,
                                width: None,
                            },
                        ),
                    ],
                    vec![
                        row(
                            "DirectPass",
                            vec![
                                Value::Bool(true),
                                Value::U64(2),
                                Value::IntervalU64(Interval::<u64>::new(1, 3).expect("interval")),
                            ],
                        ),
                        row(
                            "Failed",
                            vec![
                                Value::Bool(false),
                                Value::U64(5),
                                Value::IntervalU64(Interval::<u64>::new(3, 5).expect("interval")),
                            ],
                        ),
                    ],
                ),
                RelationDescriptor {
                    extension: None,
                    name: "Holder".into(),
                    fields: vec![
                        fresh_field("id"),
                        field("name", ValueType::String),
                        field("digest", ValueType::FixedBytes { len: 12 }),
                        field(
                            "at",
                            ValueType::Interval {
                                element: IntervalElement::U64,
                                width: None,
                            },
                        ),
                        field(
                            "lease",
                            ValueType::Interval {
                                element: IntervalElement::I64,
                                width: Some(7),
                            },
                        ),
                        field("balance", ValueType::I64),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Account".into(),
                    fields: vec![
                        fresh_field("id"),
                        field("holder", ValueType::U64),
                        field("kind", ValueType::U64),
                    ],
                },
            ],
            statements: vec![
                fd(RelationId(2), &[FieldId(1), FieldId(2)]),
                containment(
                    side_where(
                        RelationId(2),
                        &[FieldId(1)],
                        vec![(FieldId(2), Value::U64(0))],
                    ),
                    side(RelationId(1), &[FieldId(0)]),
                ),
                containment(
                    side(RelationId(2), &[FieldId(2)]),
                    side(RelationId(0), &[FieldId(0)]),
                ),
                containment(
                    side_where_sets(
                        RelationId(1),
                        &[FieldId(0)],
                        vec![
                            (
                                FieldId(1),
                                LiteralSet::Many(Box::new([
                                    Value::String(Box::from(&b"alpha"[..])),
                                    Value::String(Box::from(&b"beta"[..])),
                                ])),
                            ),
                            (
                                FieldId(2),
                                LiteralSet::One(Value::FixedBytes(Box::from(&b"0123456789ab"[..]))),
                            ),
                            (
                                FieldId(3),
                                LiteralSet::One(Value::IntervalU64(
                                    Interval::<u64>::new(5, u64::MAX).expect("ray"),
                                )),
                            ),
                            (
                                FieldId(4),
                                LiteralSet::One(Value::IntervalI64(
                                    Interval::<i64>::new(-3, 4).expect("width 7"),
                                )),
                            ),
                            (FieldId(5), LiteralSet::One(Value::I64(-42))),
                        ],
                    ),
                    side(RelationId(1), &[FieldId(0)]),
                ),
                cardinality(
                    side(RelationId(2), &[FieldId(1)]),
                    0,
                    Some(3),
                    side(RelationId(1), &[FieldId(0)]),
                ),
                cardinality(
                    side_where(
                        RelationId(2),
                        &[FieldId(1)],
                        vec![(FieldId(2), Value::U64(1))],
                    ),
                    2,
                    None,
                    side(RelationId(1), &[FieldId(0)]),
                ),
            ],
        }
    }

    #[test]
    fn the_canonical_stream_round_trips_to_the_declared_descriptor() {
        // The whole exhume premise in one assert chain: encode the sealed
        // schema, decode the stream, and the DECLARED descriptor comes
        // back structurally identical (the fixture's literal sets are
        // pre-canonical, so sealing changes nothing).
        let declared = everything();
        let schema = declared.clone().validate().expect("valid fixture");
        let bytes = canonical_descriptor(&schema);
        let decoded = decode_descriptor(&bytes).expect("decodes");
        assert_eq!(decoded, declared);
    }

    #[test]
    fn a_decoded_descriptor_revalidates_to_the_same_fingerprint_and_bytes() {
        // The self-verifying round trip the exhume entry pins per store:
        // decode → validate → re-encode reproduces the exact input bytes,
        // so the fingerprint is preserved through the whole cycle.
        let schema = everything().validate().expect("valid fixture");
        let bytes = canonical_descriptor(&schema);
        let reopened = decode_descriptor(&bytes)
            .expect("decodes")
            .validate()
            .expect("revalidates");
        assert_eq!(canonical_descriptor(&reopened), bytes);
        assert_eq!(fingerprint(&reopened), fingerprint(&schema));
    }

    #[test]
    fn truncation_anywhere_is_a_shape_diagnosis_never_a_panic() {
        let schema = everything().validate().expect("valid fixture");
        let bytes = canonical_descriptor(&schema);
        for cut in 0..bytes.len() {
            assert!(
                decode_descriptor(&bytes[..cut]).is_err(),
                "a {cut}-byte prefix decoded"
            );
        }
    }

    #[test]
    fn trailing_bytes_are_refused() {
        let schema = everything().validate().expect("valid fixture");
        let mut bytes = canonical_descriptor(&schema);
        bytes.push(0);
        assert_eq!(decode_descriptor(&bytes), Err("descriptor trailing bytes"));
    }

    #[test]
    fn a_foreign_label_is_refused() {
        assert_eq!(
            decode_descriptor(b"\x12\x00\x00\x00bumbledb-schema-v3"),
            Err("descriptor encoding label")
        );
    }
}
