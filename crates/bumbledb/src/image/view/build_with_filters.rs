//! record of the one-scan claim.

use std::sync::Arc;

use crate::error::Result;
use crate::image::{RelationImage, build};
use crate::schema::Schema;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::RelationId;

use super::{Const, FilterPredicate, View, apply};

/// the cacheable unfiltered image and the query-local survivor view. The
/// caller inserts the image into the cache.
/// Test-only: the production cold path builds the unfiltered image and
/// then filters it (`get_or_build` + `apply`) — the same two passes this
/// fuses, kept as the executable record of 50-storage's one-scan claim.
/// part, and sharing `apply`'s evaluator beats duplicating it inside the
/// builder (deliberate simplification of the 40-execution doc's parenthetical).
/// # Errors
/// Build errors (`Lmdb`, `Corruption`) propagate.
#[cfg(test)]
pub fn build_with_filters(
    txn: &ReadTxn<'_>,
    schema: &Schema,
    rel: RelationId,
    predicates: &[FilterPredicate],
    params: &[Const],
    buf: Vec<u32>,
) -> Result<(Arc<RelationImage>, View)> {
    let image = build(&txn.catalog(), schema, rel)?;
    let view = apply(&image, predicates, params, buf);
    Ok((image, view))
}
