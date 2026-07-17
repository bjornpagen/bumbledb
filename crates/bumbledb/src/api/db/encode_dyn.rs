use super::{InternMode, WriteTx};
use crate::encoding::{ValueRef, encode_fact};
use crate::error::{FactShapeError, Result};
use crate::ir::Value;
use bumbledb_theory::schema::{FieldDescriptor, FieldId, RelationId};

/// The one [`crate::schema::ValueMismatch`] → [`FactShapeError`] translation,
/// shared by every dynamic write/read surface (`insert_dyn`/`delete_dyn`/
/// `contains_dyn`/`get_dyn`, both transaction kinds).
pub(super) fn shape_mismatch(
    rel: RelationId,
    field: FieldId,
    mismatch: bumbledb_theory::schema::ValueMismatch,
) -> FactShapeError {
    match mismatch {
        bumbledb_theory::schema::ValueMismatch::Type => FactShapeError::TypeMismatch {
            relation: rel,
            field,
        },
        bumbledb_theory::schema::ValueMismatch::Utf8 => FactShapeError::InvalidUtf8 {
            relation: rel,
            field,
        },
    }
}

/// Fills `refs` with one column [`ValueRef`] per field — arity- and
/// type-checked against the declaration (typed [`FactShapeError`]s; ETL
/// input is data, `docs/architecture/70-api.md`) — the one dynamic
/// value→column body under both transaction kinds. Only string interning
/// differs by context, so the resolver is the parameter: the write
/// transaction resolves pending-first (minting on the insert path), the
/// snapshot reads the committed dictionary. `Ok(false)` = a resolve-mode
/// miss: the value was never interned, so the fact cannot exist.
pub(super) fn dyn_value_refs(
    rel: RelationId,
    values: &[Value],
    fields: &[FieldDescriptor],
    refs: &mut Vec<ValueRef>,
    mut resolve_str: impl FnMut(&str) -> Result<Option<u64>>,
) -> Result<bool> {
    if values.len() != fields.len() {
        return Err(FactShapeError::ArityMismatch {
            relation: rel,
            expected: fields.len(),
            supplied: values.len(),
        }
        .into());
    }
    for (idx, (value, field)) in values.iter().zip(fields).enumerate() {
        let field_id = FieldId(u16::try_from(idx).expect("field count fits u16"));
        if let Err(mismatch) = bumbledb_theory::schema::value_matches(value, &field.value_type) {
            return Err(shape_mismatch(rel, field_id, mismatch).into());
        }
        let value_ref = match value {
            Value::AllenMask(_) => {
                unreachable!("value_matches rejected mask values above: not a field type")
            }
            Value::Bool(v) => ValueRef::Bool(*v),
            Value::U64(v) => ValueRef::U64(*v),
            Value::I64(v) => ValueRef::I64(*v),
            // The interval family splits by the FIELD's width:
            // `value_matches` above already enforced the fixed
            // type's exact width and Q2 bound, so the fixed ref just
            // marks the one-word encoding.
            Value::IntervalU64(interval) => match field.value_type {
                bumbledb_theory::schema::ValueType::Interval { width: Some(_), .. } => {
                    ValueRef::FixedIntervalU64(*interval)
                }
                _ => ValueRef::IntervalU64(*interval),
            },
            Value::IntervalI64(interval) => match field.value_type {
                bumbledb_theory::schema::ValueType::Interval { width: Some(_), .. } => {
                    ValueRef::FixedIntervalI64(*interval)
                }
                _ => ValueRef::IntervalI64(*interval),
            },
            Value::String(raw) => {
                let text = std::str::from_utf8(raw).expect("value_matches validated UTF-8 above");
                let Some(id) = resolve_str(text)? else {
                    return Ok(false);
                };
                ValueRef::String(id)
            }
            // Identity-shaped: bytes<N> values encode inline —
            // no dictionary traffic in either mode.
            Value::FixedBytes(raw) => ValueRef::fixed_bytes(raw),
        };
        refs.push(value_ref);
    }
    Ok(true)
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
        // Take the ref scratch out for the fill, restoring it on every
        // exit — miss, shape error, or success — so the buffer's
        // capacity survives (the one restore point).
        let mut refs = std::mem::take(&mut self.refs);
        refs.clear();
        let delta = &mut self.delta;
        let view = &self.view;
        let encoded = dyn_value_refs(
            rel,
            values,
            relation.fields(),
            &mut refs,
            |text| match mode {
                InternMode::Mint => delta.intern_str(view, text).map(Some),
                InternMode::Resolve => delta.resolve_str(view, text),
            },
        );
        if let Ok(true) = encoded {
            self.scratch.clear();
            encode_fact(&refs, relation.layout(), &mut self.scratch);
        }
        self.refs = refs;
        encoded
    }
}
