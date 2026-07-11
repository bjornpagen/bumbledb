//! Test-only cold dual-output build (`40-storage.md`): the executable
//! record of the one-scan claim.

use std::sync::Arc;

use crate::error::Result;
use crate::image::{build, RelationImage};
use crate::schema::{RelationId, Schema};
use crate::storage::env::ReadTxn;

use super::{apply, Const, FilterPredicate, View};

/// Cold dual-output build (`40-storage.md`): one storage scan produces both
/// the cacheable unfiltered image and the query-local survivor view. The
/// caller inserts the image into the cache.
///
/// Test-only: the production cold path builds the unfiltered image and
/// then filters it (`get_or_build` + `apply`) — the same two passes this
/// fuses, kept as the executable record of 40-storage's one-scan claim.
///
/// The filter pass runs over the freshly decoded columns rather than being
/// interleaved into the decode loop — the one storage scan is the expensive
/// part, and sharing `apply`'s evaluator beats duplicating it inside the
/// builder (deliberate simplification of the 30-execution doc's parenthetical).
///
/// # Errors
///
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
    let image = build(txn, schema, rel)?;
    let view = apply(&image, predicates, params, buf)?;
    Ok((image, view))
}
