//! The one ChangeSet row codec. No duplicate row encoder lives here.
//! Decode borrows `DecodedRow::values()` under the live owner, or
//! transfers `into_owner()`. Grow/copy through `ChargedBytes` /
//! `ChargedBuffer::admit_copy`. Bound cell/string/byte work before copy.

use bumbledb::work::{ByteKind, ChargedBytes, WorkContext};
use bumbledb::{ChangeSet, RelationId, Value};

use crate::runtime::RuntimeError;

use super::{change_error, value_bytes};

pub(crate) fn encode_rows_bytes(
    schema: &bumbledb::schema::Schema,
    relation: RelationId,
    rows: &[Vec<Value>],
    context: &WorkContext,
) -> Result<Vec<u8>, RuntimeError> {
    let mut builder = ChangeSet::builder(schema, context.clone());
    for values in rows {
        context.step(1)?;
        builder
            .insert(relation, values)
            .map_err(|error| change_error(&error))?;
    }
    let changes = builder.finish().map_err(|error| change_error(&error))?;
    let bytes = changes.as_bytes();
    let charged = ChargedBytes::adopt(context, ByteKind::Working, bytes.to_vec().into_boxed_slice())
        .map_err(RuntimeError::from)?;
    Ok(charged.into_owner().as_bytes().to_vec())
}

pub(crate) fn decode_rows_values(
    schema: &bumbledb::schema::Schema,
    relation: RelationId,
    bytes: &[u8],
    context: &WorkContext,
) -> Result<Vec<Vec<Value>>, RuntimeError> {
    let changes = ChangeSet::parse(schema, bytes, context).map_err(|error| change_error(&error))?;
    let Some(relation_ref) = schema.relation_checked(relation) else {
        return Err(RuntimeError::InvalidArgument);
    };
    let fields = relation_ref.fields();
    let mut rows = Vec::new();
    for record in changes.records() {
        context.step(1)?;
        if record.relation != relation || record.kind != bumbledb::changes::ChangeKind::Add {
            return Err(RuntimeError::Engine {
                kind: crate::tags::error_family::VALIDATION,
                message: "decodeRows: the payload carries records outside the requested \
                          relation's adds"
                    .into(),
            });
        }
        let decoded =
            bumbledb::canonical::decode(fields, record.row, context).map_err(|error| {
                RuntimeError::Engine {
                    kind: crate::tags::error_family::CORRUPTION,
                    message: format!("decodeRows: {error}"),
                }
            })?;
        let owner = decoded.into_owner();
        let borrowed = owner.values();
        let size = borrowed.iter().map(value_bytes).sum::<u64>();
        context.step(borrowed.len() as u64)?;
        let overlap = context
            .reserve(ByteKind::Working, size)
            .map_err(RuntimeError::from)?;
        let mut copy = Vec::with_capacity(borrowed.len());
        for value in borrowed {
            copy.push(clone_value_bound(value, context)?);
        }
        drop(overlap);
        drop(owner);
        rows.push(copy);
    }
    Ok(rows)
}

fn clone_value_bound(value: &Value, work: &WorkContext) -> Result<Value, RuntimeError> {
    work.step(1)?;
    match value {
        Value::String(text) => {
            work.input(text.len() as u64)?;
            let charged = ChargedBytes::adopt(
                work,
                ByteKind::Working,
                text.as_bytes().to_vec().into_boxed_slice(),
            )
            .map_err(RuntimeError::from)?;
            let owned = charged.into_owner();
            let text = std::str::from_utf8(owned.as_bytes())
                .map_err(|_| RuntimeError::InvalidArgument)?
                .to_owned();
            Ok(Value::String(text.into()))
        }
        Value::FixedBytes(bytes) => {
            work.input(bytes.len() as u64)?;
            let charged = ChargedBytes::adopt(
                work,
                ByteKind::Working,
                bytes.as_ref().to_vec().into_boxed_slice(),
            )
            .map_err(RuntimeError::from)?;
            let owned = charged.into_owner();
            Ok(Value::FixedBytes(owned.as_bytes().to_vec().into()))
        }
        other => Ok(other.clone()),
    }
}
