use super::{InternMode, WriteTx};
use crate::encoding::{encode_fact, ValueRef};
use crate::error::{FactShapeError, Result};
use crate::ir::Value;
use crate::schema::{FieldId, RelationId};

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

impl WriteTx<'_> {
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
        let mut refs = std::mem::take(&mut self.refs);
        refs.clear();
        for (idx, (value, field)) in values.iter().zip(fields).enumerate() {
            let field_id = FieldId(u16::try_from(idx).expect("validated schema: fields fit u16"));
            if let Err(mismatch) = crate::schema::value_matches(value, &field.value_type) {
                self.refs = refs; // restore the scratch before erroring
                return Err(shape_mismatch(rel, field_id, mismatch).into());
            }
            let value_ref = match value {
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
                        InternMode::Mint => self.delta.intern_str(&self.view, text).map(Some),
                        InternMode::Resolve => self.delta.resolve_str(&self.view, text),
                    };
                    match id {
                        Ok(Some(id)) => ValueRef::String(id),
                        Ok(None) => {
                            self.refs = refs;
                            return Ok(false);
                        }
                        Err(err) => {
                            self.refs = refs;
                            return Err(err);
                        }
                    }
                }
                Value::Bytes(raw) => {
                    let id = match mode {
                        InternMode::Mint => self.delta.intern_bytes(&self.view, raw).map(Some),
                        InternMode::Resolve => self.delta.resolve_bytes(&self.view, raw),
                    };
                    match id {
                        Ok(Some(id)) => ValueRef::Bytes(id),
                        Ok(None) => {
                            self.refs = refs;
                            return Ok(false);
                        }
                        Err(err) => {
                            self.refs = refs;
                            return Err(err);
                        }
                    }
                }
            };
            refs.push(value_ref);
        }
        self.scratch.clear();
        encode_fact(&refs, relation.layout(), &mut self.scratch);
        self.refs = refs;
        Ok(true)
    }
}
