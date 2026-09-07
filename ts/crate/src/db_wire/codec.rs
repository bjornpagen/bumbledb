//! The one `ChangeSet` row codec. No duplicate row encoder lives here.
//! Decode converts each charged row directly into the queued output.
//! Source and destination reservations overlap during the one required copy.

use bumbledb::work::{ByteKind, WorkContext};
use bumbledb::{ChangeSet, RelationId, Value};

use crate::marshal::{ValueOut, output_vec, row_out};
use crate::runtime::{QueuedBytes, QueuedOutput, RuntimeError};

use super::change_error;

pub(crate) fn encode_rows_bytes(
    schema: &bumbledb::schema::Schema,
    relation: RelationId,
    rows: &[Vec<Value>],
    context: &WorkContext,
) -> Result<QueuedBytes, RuntimeError> {
    let mut builder = ChangeSet::builder(schema, context.clone());
    for values in rows {
        context.step(1)?;
        builder
            .insert(relation, values)
            .map_err(|error| change_error(&error))?;
    }
    let changes = builder.finish().map_err(|error| change_error(&error))?;
    QueuedBytes::copy_from(context, changes.as_bytes())
}

pub(crate) fn decode_rows_values(
    schema: &bumbledb::schema::Schema,
    relation: RelationId,
    bytes: &[u8],
    context: &WorkContext,
) -> Result<QueuedOutput, RuntimeError> {
    let changes = ChangeSet::parse(schema, bytes, context).map_err(|error| change_error(&error))?;
    let Some(relation_ref) = schema.relation_checked(relation) else {
        return Err(RuntimeError::InvalidArgument);
    };
    let fields = relation_ref.fields();
    let count = usize::try_from(changes.len()).map_err(|_| RuntimeError::InvalidArgument)?;
    let outer_bytes = count
        .checked_mul(size_of::<Vec<ValueOut>>())
        .ok_or_else(|| {
            crate::runtime::session::engine_error(&bumbledb::Error::ResultBytesOverflow)
        })?;
    let mut charge = context.reserve(ByteKind::Result, outer_bytes as u64)?;
    let mut rows = output_vec(count)?;
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
        let decoded = bumbledb::canonical::decode(fields, record.row, context).map_err(
            |error| match error {
                bumbledb::canonical::RowError::Work(error) => RuntimeError::Work(error),
                error => RuntimeError::Engine {
                    kind: crate::tags::error_family::CORRUPTION,
                    message: format!("decodeRows: {error}"),
                },
            },
        )?;
        rows.push(row_out(context, &decoded, &mut charge)?);
    }
    Ok(QueuedOutput { rows, charge })
}
