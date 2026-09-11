//! The one `ChangeSet` row codec. No duplicate row encoder lives here.
//! Decode converts each borrowed row into its final queued output.

use bumbledb::work::{WorkContext, WorkError};
use bumbledb::{ChangeSet, RelationId, Value};

use crate::marshal::{output_vec, row_out};
use crate::runtime::{QueuedBytes, QueuedOutput, RuntimeError};

use super::change_error;

/// Reserve only after the stated row count matches the actual input shape.
pub(super) fn reserve_input_rows(
    stated: u64,
    context: &WorkContext,
) -> Result<Vec<Vec<Value>>, RuntimeError> {
    let count = usize::try_from(stated).map_err(|_| RuntimeError::InvalidArgument)?;
    context.checkpoint()?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|_| WorkError::Allocation)?;
    Ok(rows)
}

/// Copy JS input into owned values before dispatch to the worker.
pub(crate) fn parse_input_rows(
    sealed: &crate::Sealed,
    relation: u32,
    stated: u64,
    cells: &napi::bindgen_prelude::Array,
    context: &WorkContext,
) -> Result<Vec<Vec<Value>>, RuntimeError> {
    let roster = sealed
        .rosters
        .get(relation as usize)
        .ok_or(RuntimeError::InvalidArgument)?;
    let arity = roster.fields.len();
    if u128::from(stated) * (arity as u128) != u128::from(cells.len()) {
        return Err(RuntimeError::InvalidArgument);
    }
    context.checkpoint()?;
    if arity == 0 {
        // The zero-column relation is a set: absent or the one empty tuple.
        let rows = if stated == 0 {
            Vec::new()
        } else {
            vec![Vec::new()]
        };
        return Ok(rows);
    }
    let mut rows = reserve_input_rows(stated, context)?;
    for start in (0..cells.len()).step_by(arity) {
        let mut row = output_vec(arity)?;
        for (offset, field) in roster.fields.iter().enumerate() {
            let index = start + u32::try_from(offset).expect("field count fits u32");
            let value = crate::marshal::req_at::<napi::Unknown>(cells, index, "row cells")
                .map_err(|_| RuntimeError::InvalidArgument)?;
            let value = crate::marshal::schema_value_in(
                &field.value_type,
                &value,
                &roster.name,
                &field.name,
            )
            .map_err(|_| RuntimeError::InvalidArgument)?;
            row.push(value);
        }
        context.checkpoint()?;
        rows.push(row);
    }
    Ok(rows)
}

pub(crate) fn encode_rows_bytes(
    schema: &bumbledb::schema::Schema,
    relation: RelationId,
    rows: &[Vec<Value>],
    context: &WorkContext,
) -> Result<QueuedBytes, RuntimeError> {
    let mut builder = ChangeSet::builder(schema, context.clone());
    for values in rows {
        context.checkpoint()?;
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
    let mut rows = output_vec(count)?;
    for record in changes.records() {
        context.checkpoint()?;
        if record.relation != relation || record.kind != bumbledb::changes::ChangeKind::Add {
            return Err(RuntimeError::Engine {
                diagnostic: None,
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
                    diagnostic: None,
                    kind: crate::tags::error_family::CORRUPTION,
                    message: format!("decodeRows: {error}"),
                },
            },
        )?;
        rows.push(row_out(context, &decoded)?);
    }
    Ok(QueuedOutput { rows })
}
