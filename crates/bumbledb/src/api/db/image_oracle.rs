//! The copy-on-append column differential, engine-hosted test support
//! (feature `image-oracle`;
//! docs/prds/incremental-images/prd-I1-copy-on-append.md § coverage
//! item 3(b)): the image the engine SERVES for a relation — whatever
//! arm produced it (full build, append, carry-forward, or a plain
//! cache hit) — compared facet-by-facet against a from-scratch
//! [`crate::image::build`] in the SAME read transaction. Set-semantic
//! query parity already catches wrong tail CONTENT, but an append
//! landing the right multiset at wrong positions, or corrupting
//! lazily-observed metadata (spans, forced distincts), passes every
//! other referee — this is the only in-tree comparison of the two fill
//! paths (the one-shot integration referee lives in `append_tests.rs`).
//! Its former consumer, the detached fuzz crate's `ops` target, died
//! with the fuzzing apparatus (the 2026-07-20 hard-delete ruling,
//! docs/architecture/60-validation.md § the deletion record).

use bumbledb_theory::schema::{FieldId, RelationId};

use super::Db;
use crate::error::Result;

/// One facet on which the served image diverged from its from-scratch
/// rebuild — facts, never row ids (the error law).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDivergence {
    /// The row counts differ.
    RowCount { served: usize, rebuilt: usize },
    /// A field's column span differs (the field→column map desynced).
    Span { field: FieldId },
    /// A column's full slice differs — kind or bytes.
    Column { column: usize },
    /// A column's forced exact distinct count differs.
    Cardinality {
        column: usize,
        served: u64,
        rebuilt: u64,
    },
}

impl<S> Db<S> {
    /// The referee clause, engine-side: serve `rel`'s image through the
    /// production cache path, rebuild it from scratch in the same
    /// snapshot, and report the first facet on which they differ
    /// (`None` = indistinguishable at the column granularity —
    /// `row_count`, every field's span, every column's full slice,
    /// every forced distinct). A closed relation answers `None`
    /// vacuously: its storage is the theory itself and there is no
    /// second fill path to differ.
    ///
    /// # Errors
    ///
    /// `Lmdb` on snapshot failure; build/append errors (`Lmdb`,
    /// `Corruption`) propagate from either fill path.
    ///
    /// # Panics
    ///
    /// Only on a foreign `rel` (internal ids are dense and validated —
    /// a caller draws relations from the target schema).
    pub fn image_divergence(&self, rel: RelationId) -> Result<Option<ImageDivergence>> {
        let relation = self.schema.relation(rel);
        if relation.extension().is_some() {
            return Ok(None);
        }
        let txn = self.env.read_txn()?;
        let served = self.cache.get_or_build(&txn, &self.schema, rel)?;
        let rebuilt = crate::image::build(&txn, &self.schema, rel)?;
        if served.row_count() != rebuilt.row_count() {
            return Ok(Some(ImageDivergence::RowCount {
                served: served.row_count(),
                rebuilt: rebuilt.row_count(),
            }));
        }
        for field in 0..relation.fields().len() {
            let field = FieldId(u16::try_from(field).expect("validated field count fits u16"));
            if served.span(field) != rebuilt.span(field) {
                return Ok(Some(ImageDivergence::Span { field }));
            }
        }
        // Total image columns, derived exactly as the build derives
        // them (the field→column map).
        let types: Vec<bumbledb_theory::TypeDesc> = relation
            .fields()
            .iter()
            .map(|f| f.value_type.type_desc())
            .collect();
        let spans = crate::image::column_spans(&types);
        let columns = spans
            .last()
            .map_or(0, |s| usize::from(s.first_column + s.width.column_count()));
        for column in 0..columns {
            if served.column(column) != rebuilt.column(column) {
                return Ok(Some(ImageDivergence::Column { column }));
            }
            let (served_count, rebuilt_count) =
                (served.cardinality(column), rebuilt.cardinality(column));
            if served_count != rebuilt_count {
                return Ok(Some(ImageDivergence::Cardinality {
                    column,
                    served: served_count,
                    rebuilt: rebuilt_count,
                }));
            }
        }
        Ok(None)
    }
}
