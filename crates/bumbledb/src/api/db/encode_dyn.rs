use super::{InternMode, WriteTx};
use crate::encoding::{decode_field, encode_fact, FactLayout, ValueRef};
use crate::error::{FactShapeError, Result};
use crate::ir::Value;
use crate::schema::{FieldDescriptor, FieldId, RelationId};

/// The one [`crate::schema::ValueMismatch`] → [`FactShapeError`] translation,
/// shared by every dynamic write/read surface (`insert_dyn`/`delete_dyn`/
/// `get_dyn`).
pub(super) fn shape_mismatch(
    rel: RelationId,
    field: FieldId,
    mismatch: crate::schema::ValueMismatch,
) -> FactShapeError {
    match mismatch {
        crate::schema::ValueMismatch::Type => FactShapeError::TypeMismatch {
            relation: rel,
            field,
        },
        crate::schema::ValueMismatch::EnumOrdinal(ordinal) => {
            FactShapeError::EnumOrdinalOutOfRange {
                relation: rel,
                field,
                ordinal,
            }
        }
        crate::schema::ValueMismatch::Utf8 => FactShapeError::InvalidUtf8 {
            relation: rel,
            field,
        },
        crate::schema::ValueMismatch::IntervalEmpty => FactShapeError::EmptyInterval {
            relation: rel,
            field,
        },
    }
}

impl<S> WriteTx<'_, S> {
    /// Encodes a dynamic fact into `self.scratch`, interning through the
    /// delta ([`InternMode::Mint`]) or resolving without minting
    /// ([`InternMode::Resolve`] — `Ok(false)` = a value was never
    /// interned; the fact cannot exist). Shape problems are typed errors
    /// — ETL input is data (`docs/architecture/70-api.md`).
    pub(super) fn encode_dyn(
        &mut self,
        rel: RelationId,
        values: &[Value],
        mode: InternMode,
    ) -> Result<bool> {
        let Some(relation) = self.schema.relation_checked(rel) else {
            return Err(FactShapeError::UnknownRelation { relation: rel }.into());
        };
        let fields = relation.fields();
        if values.len() != fields.len() {
            return Err(FactShapeError::ArityMismatch {
                relation: rel,
                expected: fields.len(),
                supplied: values.len(),
            }
            .into());
        }
        // Take the ref scratch out for the fill, restoring it on every
        // exit — miss, shape error, or success — so the buffer's
        // capacity survives (the one restore point).
        let mut refs = std::mem::take(&mut self.refs);
        refs.clear();
        let encoded = self.dyn_refs(rel, values, fields, mode, &mut refs);
        if let Ok(true) = encoded {
            self.scratch.clear();
            encode_fact(&refs, relation.layout(), &mut self.scratch);
        }
        self.refs = refs;
        encoded
    }

    /// Fills `refs` with one column [`ValueRef`] per field, type-checked
    /// against the declaration and interned per `mode`. `Ok(false)` = a
    /// resolve-mode miss: the value was never interned, so the fact
    /// cannot exist.
    fn dyn_refs(
        &mut self,
        rel: RelationId,
        values: &[Value],
        fields: &[FieldDescriptor],
        mode: InternMode,
        refs: &mut Vec<ValueRef>,
    ) -> Result<bool> {
        for (idx, (value, field)) in values.iter().zip(fields).enumerate() {
            let field_id = FieldId(u16::try_from(idx).expect("validated schema: fields fit u16"));
            if let Err(mismatch) = crate::schema::value_matches(value, &field.value_type) {
                return Err(shape_mismatch(rel, field_id, mismatch).into());
            }
            let value_ref = match value {
                Value::AllenMask(_) => {
                    unreachable!("value_matches rejected mask values above: not a field type")
                }
                Value::Bool(v) => ValueRef::Bool(*v),
                Value::U64(v) => ValueRef::U64(*v),
                Value::I64(v) => ValueRef::I64(*v),
                Value::Enum(ordinal) => ValueRef::Enum(*ordinal),
                Value::IntervalU64(start, end) => ValueRef::IntervalU64(*start, *end),
                Value::IntervalI64(start, end) => ValueRef::IntervalI64(*start, *end),
                Value::String(raw) => {
                    let text =
                        std::str::from_utf8(raw).expect("value_matches validated UTF-8 above");
                    let id = match mode {
                        InternMode::Mint => Some(self.delta.intern_str(&self.view, text)?),
                        InternMode::Resolve => self.delta.resolve_str(&self.view, text)?,
                    };
                    let Some(id) = id else { return Ok(false) };
                    ValueRef::String(id)
                }
                Value::Bytes(raw) => {
                    let id = match mode {
                        InternMode::Mint => Some(self.delta.intern_bytes(&self.view, raw)?),
                        InternMode::Resolve => self.delta.resolve_bytes(&self.view, raw)?,
                    };
                    let Some(id) = id else { return Ok(false) };
                    ValueRef::Bytes(id)
                }
            };
            refs.push(value_ref);
        }
        Ok(true)
    }
}

/// Decodes canonical fact bytes into owned dynamic [`Value`]s — the one
/// body behind [`WriteTx::get_dyn`]'s point-read decode and
/// [`super::Snapshot::scan`]'s export decode; only intern resolution
/// differs by context (pending-first inside a write transaction, the
/// committed dictionary on a snapshot), so the resolvers are the
/// parameters.
pub(super) fn decode_values(
    fact: &[u8],
    layout: &FactLayout,
    mut resolve_str: impl FnMut(u64) -> Result<Box<[u8]>>,
    mut resolve_bytes: impl FnMut(u64) -> Result<Box<[u8]>>,
) -> Result<Vec<Value>> {
    (0..layout.field_count())
        .map(|idx| {
            Ok(match decode_field(fact, layout, idx)? {
                ValueRef::Bool(v) => Value::Bool(v),
                ValueRef::U64(v) => Value::U64(v),
                ValueRef::I64(v) => Value::I64(v),
                ValueRef::Enum(ordinal) => Value::Enum(ordinal),
                ValueRef::String(id) => Value::String(resolve_str(id)?),
                ValueRef::Bytes(id) => Value::Bytes(resolve_bytes(id)?),
                ValueRef::IntervalU64(start, end) => Value::IntervalU64(start, end),
                ValueRef::IntervalI64(start, end) => Value::IntervalI64(start, end),
            })
        })
        .collect()
}
