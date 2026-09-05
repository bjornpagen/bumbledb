//! Shared point-read machinery: keyed lookup by decoded value equality.
//!
//! A key statement's determinant is the projected value tuple. The indexed
//! read projects the key's scalar determinant through the store's one
//! projection convention, walks that determinant bucket, and confirms each
//! candidate with exact decoded values — value equality over canonical
//! scalars decides, never a fingerprint (a forced collision widens the
//! bucket, never an answer). Work is bucket-shaped, not relation-shaped.
//!
//! The bounded reference walk ([`find_snapshot_row_scan`]) stays available
//! as the exact oracle and the fallback for a projection the compiled
//! determinant table does not carry; it is a scan and is named one.
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

/// Indexed keyed lookup over one committed snapshot: project the key's
/// scalar determinant with the store's one projection convention, enumerate
/// that determinant bucket, and confirm each candidate row by exact decoded
/// projection equality (the full projection — a pointwise key's interval
/// tail included). Work is proportional to the bucket, never the relation.
pub(super) fn find_snapshot_row<'s>(
    snapshot: &'s crate::storage::store::OwnedSnapshot,
    schema: &Schema,
    relation: RelationId,
    projection: &[FieldId],
    key_values: &[Value],
    work: &crate::work::WorkContext,
) -> Result<Option<&'s [u8]>> {
    let Some(key) = snapshot.determinants().key_for(relation, projection) else {
        // Not a compiled key projection of this store's schema (callers
        // always hold a sealed key statement, so this arm is defensive):
        // the bounded reference walk answers exactly.
        return find_snapshot_row_scan(snapshot, schema, relation, projection, key_values, work);
    };
    let scalar_values: Vec<Value> = key
        .scalar_positions
        .iter()
        .map(|&position| key_values[position].clone())
        .collect();
    let projected = crate::storage::store::det_index::determinant_bytes(
        key,
        &scalar_values,
        work,
    )
    .map_err(crate::error::Error::from_store)?;
    let fields = schema.relation(relation).fields();
    let mut hit = None;
    snapshot
        .visit_projection(key.id, &projected, work, &mut |id, bytes| {
            work.step(1)?;
            let decoded = crate::canonical::decode(fields, bytes, work)?;
            if projection_matches(decoded.values(), projection, key_values) {
                hit = Some(id);
                return Ok(false);
            }
            Ok(true)
        })
        .map_err(crate::error::Error::from_store)?;
    match hit {
        Some(id) => snapshot
            .fetch(relation, id)
            .map_err(crate::error::Error::from_store),
        None => Ok(None),
    }
}

/// The bounded reference walk — the exact oracle for [`find_snapshot_row`]
/// and the fallback for an uncompiled projection: walk the relation's rows,
/// decode, compare the projection exactly. A scan, and named one.
pub(super) fn find_snapshot_row_scan<'s>(
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
        work.step(1).map_err(store_work)?;
        let (_, row) = entry.map_err(crate::error::Error::from_store)?;
        let decoded = crate::canonical::decode(fields, row, work).map_err(super::tx::row_error)?;
        if projection_matches(decoded.values(), projection, key_values) {
            return Ok(Some(row));
        }
    }
    Ok(None)
}

fn store_work(error: crate::work::WorkError) -> crate::error::Error {
    crate::error::Error::from_store(crate::storage::store::StoreError::Work(error))
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

/// One keyed row hit from [`get_with_work`]: closed relations carry
/// pre-decoded values; stored relations carry canonical row bytes.
pub(super) enum KeyedRowHit<'a> {
    Closed(&'a super::closed::ClosedRow),
    Store(&'a [u8]),
}

/// Keyed lookup under the caller's work allowance — the native runtime
/// threads each wire operation's bounded [`WorkContext`] through here so
/// determinant projection, bucket walks and row decode observe the
/// operation's policy, not a long-lived session lease's embedded ledger.
pub(super) fn get_with_work<'a>(
    snapshot: &'a crate::storage::store::OwnedSnapshot,
    schema: &Schema,
    closed: &'a super::closed::ClosedRows,
    relation: RelationId,
    key: StatementId,
    key_values: &[Value],
    work: &crate::work::WorkContext,
) -> Result<Option<KeyedRowHit<'a>>> {
    let (_, statement) = key_statement_of(schema, relation, key)?;
    check_key_shape(schema, relation, &statement.projection, key_values)?;
    if let Some(rows) = closed.get(relation) {
        return Ok(closed_row_by_key(rows, statement, key_values).map(KeyedRowHit::Closed));
    }
    find_snapshot_row(
        snapshot,
        schema,
        relation,
        &statement.projection,
        key_values,
        work,
    )
    .map(|row| row.map(KeyedRowHit::Store))
}
