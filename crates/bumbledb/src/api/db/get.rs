//! Shared point-read machinery: keyed lookup by decoded value equality.
//!
//! A key statement's determinant is the projected value tuple. The
//! reference read walks the relation's final-state rows, decodes each, and
//! compares the projection exactly — value equality over canonical scalars,
//! never a fingerprint verdict. Determinant acceleration (order-preserving
//! or bucketed probes) is the recorded C04/C05 follow-up shared with
//! P01/P03; admission correctness never depends on it because the store's
//! reference indexer is judgment-complete without entries.
//!
//! Closed relations read from the schema's sealed extension (no store row
//! can exist for them); their sealed rows decode through the sealed-row
//! codec, which refuses text columns by construction.

use crate::error::{DynIdError, Result};
use crate::ir::Value;
use crate::schema::{KeyId, KeyStatement, Relation, Schema, StatementView};
use bumbledb_theory::schema::{FieldId, RelationId, StatementId, value_matches};

use super::collection::shape_mismatch;

pub(super) fn key_statement_of(
    schema: &Schema,
    relation: RelationId,
    key: StatementId,
) -> Result<(KeyId, &KeyStatement)> {
    let Some(rel) = schema.relation_checked(relation) else {
        return Err(DynIdError::UnknownRelation { relation }.into());
    };
    let Some(StatementView::Key(key_id, statement)) = schema.statement_checked(key) else {
        return Err(DynIdError::NotAKeyStatement {
            relation,
            statement: key,
        }
        .into());
    };
    if statement.relation != relation || !rel.keys().contains(&key_id) {
        return Err(DynIdError::NotAKeyStatement {
            relation,
            statement: key,
        }
        .into());
    }
    Ok((key_id, statement))
}

/// Judge the key tuple's shape against the projection's field types before
/// any row is read — a mistyped key is a typed refusal, not a miss.
pub(super) fn check_key_shape(
    schema: &Schema,
    relation: RelationId,
    projection: &[FieldId],
    key_values: &[Value],
) -> Result<()> {
    let rel = schema.relation(relation);
    if key_values.len() != projection.len() {
        return Err(crate::error::FactShapeError::ArityMismatch {
            relation,
            mismatch: crate::error::Mismatch {
                witnessed: key_values.len(),
                required: projection.len(),
            },
        }
        .into());
    }
    for (value, &field) in key_values.iter().zip(projection) {
        if let Err(mismatch) = value_matches(value, &rel.field(field).value_type) {
            return Err(shape_mismatch(relation, field, mismatch).into());
        }
    }
    Ok(())
}

/// Does this decoded row match the key on every projected field?
pub(super) fn projection_matches(
    row: &[Value],
    projection: &[FieldId],
    key_values: &[Value],
) -> bool {
    projection.iter().zip(key_values).all(|(&field, key)| {
        row.get(usize::from(field.0))
            .is_some_and(|value| value == key)
    })
}

/// Decode one closed relation's sealed row to canonical values. Sealed rows
/// are encoded once at schema validation and refuse text columns, so the
/// intern resolver is unreachable.
pub(super) fn decode_sealed_row(rel: &Relation, fact: &[u8]) -> Vec<Value> {
    crate::encoding::decode_values(rel.layout().encoded(fact), |_| {
        unreachable!("closed relations refuse str columns")
    })
    .expect("sealed extension rows decode by construction")
}

/// Reference keyed lookup over one committed snapshot: walk the relation's
/// rows, decode, compare the projection exactly.
pub(super) fn find_snapshot_row<'s>(
    snapshot: &'s crate::storage::store::OwnedSnapshot,
    schema: &Schema,
    relation: RelationId,
    projection: &[FieldId],
    key_values: &[Value],
    work: &crate::work::WorkContext,
) -> Result<Option<&'s [u8]>> {
    let fields = schema.relation(relation).fields();
    let iterator = snapshot
        .rows(relation)
        .map_err(crate::error::Error::from_store)?;
    for entry in iterator {
        work.step(1).map_err(|error| {
            crate::error::Error::from_store(crate::storage::store::StoreError::Work(error))
        })?;
        let (_, row) = entry.map_err(crate::error::Error::from_store)?;
        let decoded = crate::canonical::decode(fields, row, work).map_err(super::tx::row_error)?;
        if projection_matches(&decoded.values, projection, key_values) {
            return Ok(Some(row));
        }
    }
    Ok(None)
}

/// Find the closed-relation row matching the key projection.
pub(super) fn closed_row_by_key<'c>(
    rows: &'c [super::closed::ClosedRow],
    statement: &KeyStatement,
    key_values: &[Value],
) -> Option<&'c super::closed::ClosedRow> {
    rows.iter()
        .find(|row| projection_matches(&row.values, &statement.projection, key_values))
}
