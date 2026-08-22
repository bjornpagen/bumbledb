//! The `U` pass: one cursor over `U | relation | statement | determinant`.
//! Every entry's row id must resolve to a live fact that re-derives the
//! same determinant bytes, and pointwise keys additionally re-verify per-group
//! disjointness: within one scalar-prefix group the cursor is ordered by
//! interval start, so one lookback checks `prev.end <= next.start` — the
//! invariant the neighbor probe assumes but never re-checks globally.
use crate::error::{CorruptionError, Result};
use crate::schema::StatementView;
use crate::storage::catalog::CatalogRead;
use crate::storage::keys;

use super::{Sweep, for_namespace};

pub(super) fn sweep<C: CatalogRead + Copy>(s: &mut Sweep<'_, C>) -> Result<()> {
    let schema = s.schema;
    let mut derived = keys::DeterminantImage::scratch();

    let mut prev_pointwise: Option<Vec<u8>> = None;
    for_namespace(s.catalog, keys::Namespace::Determinant, |key, value| {
        let Some((rel, sid, determinant)) = keys::parse_determinant_key(key) else {
            s.malformed(key, "U key length");
            prev_pointwise = None;
            return Ok(());
        };
        let Some(relation) = schema.relation_checked(rel) else {
            s.malformed(key, "U key relation");
            prev_pointwise = None;
            return Ok(());
        };

        if relation.body().closed_rows().is_some() {
            s.corrupt(CorruptionError::ClosedRelationEntry {
                relation: rel,
                key: key.into(),
            });
            prev_pointwise = None;
            return Ok(());
        }
        let Some(StatementView::Key(key_id, statement)) = schema.statement_checked(sid) else {
            s.malformed(key, "U key statement");
            prev_pointwise = None;
            return Ok(());
        };
        if statement.relation != rel || !relation.keys().contains(&key_id) {
            s.malformed(key, "U key statement");
            prev_pointwise = None;
            return Ok(());
        }

        if statement.form().as_fresh_row().is_some() {
            s.corrupt(CorruptionError::FreshRowDeterminantEntry {
                relation: rel,
                statement: sid,
                determinant_key: key.into(),
            });
            prev_pointwise = None;
            return Ok(());
        }
        let Ok(row_bytes) = <[u8; 8]>::try_from(value) else {
            s.malformed(key, "U row id");
            prev_pointwise = None;
            return Ok(());
        };
        let row_id = u64::from_le_bytes(row_bytes);

        let backs = match s.fact(rel, row_id)? {
            None => false,
            Some(fact) if fact.as_ref().len() != relation.layout().fact_width() => true,
            Some(fact) => {
                keys::determinant_image(
                    relation.layout().encoded(fact.as_ref()),
                    &statement.projection,
                    &mut derived,
                );
                derived.as_bytes() == determinant
            }
        };
        if !backs {
            s.corrupt(CorruptionError::DeterminantWithoutFact {
                relation: rel,
                statement: sid,
                determinant_key: key.into(),
            });
        }

        let Some(tail) = schema.key_tail(statement) else {
            prev_pointwise = None;
            return Ok(());
        };
        if determinant.len() < tail.width() {
            prev_pointwise = None;
            return Ok(());
        }
        if let Some(prev) = &prev_pointwise {
            let same_group = prev.len() == key.len()
                && prev[..prev.len() - tail.width()] == key[..key.len() - tail.width()];
            let words = (
                crate::encoding::interval_words(tail, &prev[prev.len() - tail.width()..]),
                crate::encoding::interval_words(tail, &key[key.len() - tail.width()..]),
            );

            if let (Some((_, prev_end)), Some((next_start, _))) = words
                && same_group
                && prev_end > next_start
            {
                s.corrupt(CorruptionError::PointwiseOverlap {
                    relation: rel,
                    statement: sid,
                    first: prev.clone().into(),
                    second: key.into(),
                });
            }
        }
        prev_pointwise = Some(key.to_vec());
        Ok(())
    })
}
